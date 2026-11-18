use serde::{Deserialize, Serialize};
use std::{
    io::{self, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    time::Duration,
};

pub const READINESS_TIMEOUT: Duration = Duration::from_secs(20);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(150);
const READ_TIMEOUT: Duration = Duration::from_millis(300);
const MAX_RECEIPT_BYTES: u64 = 512;
const NONCE_BYTES: usize = 32;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadinessReceipt {
    pub version: u8,
    pub window_id: u64,
    pub nonce: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadinessTarget {
    pub address: SocketAddr,
    pub nonce: String,
}

pub struct ReadinessReceiver {
    listener: TcpListener,
    window_id: u64,
    nonce: String,
}

impl ReadinessTarget {
    pub fn parse(address: &str, nonce: &str) -> Result<Self, String> {
        let address = address
            .parse::<SocketAddr>()
            .map_err(|_| "Native window readiness address is invalid".to_owned())?;
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(
                "Native window readiness address must be a nonzero loopback endpoint".into(),
            );
        }
        if nonce.len() != NONCE_BYTES * 2 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("Native window readiness nonce is invalid".into());
        }
        Ok(Self {
            address,
            nonce: nonce.to_ascii_lowercase(),
        })
    }
}

impl ReadinessReceiver {
    pub fn bind(window_id: u64) -> io::Result<(Self, ReadinessTarget)> {
        if window_id == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "native window ID must be nonzero",
            ));
        }
        let listener = TcpListener::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0))?;
        listener.set_nonblocking(true)?;
        let mut bytes = [0_u8; NONCE_BYTES];
        getrandom::fill(&mut bytes).map_err(|error| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("generate native window nonce: {error}"),
            )
        })?;
        let nonce: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
        let target = ReadinessTarget {
            address: listener.local_addr()?,
            nonce: nonce.clone(),
        };
        Ok((
            Self {
                listener,
                window_id,
                nonce,
            },
            target,
        ))
    }

    pub fn poll(&self) -> io::Result<Option<ReadinessReceipt>> {
        let (stream, peer) = match self.listener.accept() {
            Ok(connection) => connection,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };
        if !peer.ip().is_loopback() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "readiness receipt did not originate from loopback",
            ));
        }
        stream.set_read_timeout(Some(READ_TIMEOUT))?;
        let mut bytes = Vec::new();
        stream.take(MAX_RECEIPT_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_RECEIPT_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "readiness receipt exceeds 512 bytes",
            ));
        }
        let receipt: ReadinessReceipt = serde_json::from_slice(&bytes).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "readiness receipt is malformed")
        })?;
        if receipt.version != 1
            || receipt.window_id != self.window_id
            || !constant_time_eq(receipt.nonce.as_bytes(), self.nonce.as_bytes())
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "readiness receipt did not match the pending window request",
            ));
        }
        Ok(Some(receipt))
    }
}

pub fn send_readiness(target: &ReadinessTarget, window_id: u64) -> io::Result<()> {
    let mut stream = TcpStream::connect_timeout(&target.address, CONNECT_TIMEOUT)?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))?;
    let receipt = ReadinessReceipt {
        version: 1,
        window_id,
        nonce: target.nonce.clone(),
    };
    let mut bytes = serde_json::to_vec(&receipt).map_err(io::Error::other)?;
    if bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "readiness receipt exceeds 512 bytes",
        ));
    }
    bytes.push(b'\n');
    stream.write_all(&bytes)?;
    stream.shutdown(std::net::Shutdown::Write)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_receipt(address: SocketAddr, receipt: &ReadinessReceipt) {
        let mut stream = TcpStream::connect(address).expect("connect to test readiness channel");
        let mut bytes = serde_json::to_vec(receipt).expect("serialize test receipt");
        bytes.push(b'\n');
        stream.write_all(&bytes).expect("write test receipt");
        stream
            .shutdown(std::net::Shutdown::Write)
            .expect("finish test receipt");
    }

    #[test]
    fn matching_one_use_receipt_is_accepted() {
        let (receiver, target) = ReadinessReceiver::bind(42).expect("bind loopback listener");
        assert_eq!(target.address.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(receiver.poll().expect("empty poll"), None);
        send_readiness(&target, 42).expect("send valid readiness");
        let receipt = receiver
            .poll()
            .expect("valid receipt accepted")
            .expect("receipt arrived");
        assert_eq!(receipt.window_id, 42);
        assert_eq!(receipt.version, 1);
        assert_eq!(receiver.poll().expect("channel is one use"), None);
    }

    #[test]
    fn mismatched_id_or_nonce_is_rejected() {
        for receipt in [
            ReadinessReceipt {
                version: 1,
                window_id: 43,
                nonce: "00".repeat(NONCE_BYTES),
            },
            ReadinessReceipt {
                version: 1,
                window_id: 42,
                nonce: "00".repeat(NONCE_BYTES),
            },
        ] {
            let (receiver, target) = ReadinessReceiver::bind(42).expect("bind loopback listener");
            send_receipt(target.address, &receipt);
            assert_eq!(
                receiver
                    .poll()
                    .expect_err("mismatched receipt rejected")
                    .kind(),
                io::ErrorKind::PermissionDenied
            );
        }
    }

    #[test]
    fn malformed_and_oversized_receipts_fail_closed() {
        for payload in [
            b"{bad json}".to_vec(),
            vec![b'x'; (MAX_RECEIPT_BYTES + 1) as usize],
        ] {
            let (receiver, target) = ReadinessReceiver::bind(42).expect("bind loopback listener");
            let mut stream = TcpStream::connect(target.address).expect("connect to test listener");
            stream.write_all(&payload).expect("write test payload");
            stream
                .shutdown(std::net::Shutdown::Write)
                .expect("finish test payload");
            assert_eq!(
                receiver
                    .poll()
                    .expect_err("invalid receipt rejected")
                    .kind(),
                io::ErrorKind::InvalidData
            );
        }
    }

    #[test]
    fn target_parser_requires_loopback_and_a_256_bit_hex_nonce() {
        assert!(ReadinessTarget::parse("127.0.0.1:1234", &"ab".repeat(NONCE_BYTES)).is_ok());
        assert!(ReadinessTarget::parse("0.0.0.0:1234", &"ab".repeat(NONCE_BYTES)).is_err());
        assert!(ReadinessTarget::parse("127.0.0.1:0", &"ab".repeat(NONCE_BYTES)).is_err());
        assert!(ReadinessTarget::parse("127.0.0.1:1234", "short").is_err());
    }
}
