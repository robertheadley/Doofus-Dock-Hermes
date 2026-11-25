use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use vt100::Parser;

enum Command {
    Write(Vec<u8>),
    Resize { rows: u16, cols: u16 },
    Stop,
}

#[derive(Default)]
struct TerminalQueryResponder {
    pending: Vec<u8>,
}

impl TerminalQueryResponder {
    fn responses(&mut self, bytes: &[u8], cursor: (u16, u16)) -> Vec<Vec<u8>> {
        self.pending.extend_from_slice(bytes);
        let mut responses = Vec::new();
        let mut index = 0;
        while index + 4 <= self.pending.len() {
            let sequence = &self.pending[index..index + 4];
            if sequence == b"\x1b[6n" {
                responses.push(format!("\x1b[{};{}R", cursor.0 + 1, cursor.1 + 1).into_bytes());
                index += 4;
            } else if sequence == b"\x1b[5n" {
                responses.push(b"\x1b[0n".to_vec());
                index += 4;
            } else {
                index += 1;
            }
        }

        let start = self.pending.len().saturating_sub(3);
        let partial = (start..self.pending.len())
            .find(|candidate| self.pending[*candidate] == 0x1b)
            .unwrap_or(self.pending.len());
        self.pending = self.pending[partial..].to_vec();
        responses
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    Exited(String),
    Failed(String),
}

/// Explicitly started local shell backed by a real pseudo-terminal.
/// The reader parses VT output off the UI thread and keeps only a bounded screen/scrollback.
pub struct LocalTerminal {
    commands: Sender<Command>,
    events: Receiver<TerminalEvent>,
    parser: Arc<Mutex<Parser>>,
    output_generation: Arc<AtomicU64>,
    last_output_generation: u64,
    worker: Option<JoinHandle<()>>,
    reader: Option<JoinHandle<()>>,
    exited: bool,
    size: (u16, u16),
}

impl LocalTerminal {
    pub fn spawn(cwd: &Path, rows: u16, cols: u16) -> Result<Self, String> {
        let rows = rows.clamp(4, 200);
        let cols = cols.clamp(20, 400);
        let pty = native_pty_system();
        let pair = pty
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| format!("Could not open local pseudo-terminal: {error}"))?;
        let mut command = CommandBuilder::new_default_prog();
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("Could not start the local shell: {error}"))?;
        drop(pair.slave);

        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("Could not read local terminal output: {error}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| format!("Could not write to local terminal: {error}"))?;
        let parser = Arc::new(Mutex::new(Parser::new(
            std::num::NonZeroU16::new(rows).expect("clamped nonzero rows"),
            std::num::NonZeroU16::new(cols).expect("clamped nonzero columns"),
            2000,
        )));
        let output_generation = Arc::new(AtomicU64::new(0));
        let (commands, command_rx) = mpsc::channel();
        let reader_commands = commands.clone();
        let (event_tx, events) = mpsc::channel();
        let reader_parser = Arc::clone(&parser);
        let reader_generation = Arc::clone(&output_generation);
        let reader_thread = thread::Builder::new()
            .name("hermes-local-terminal-reader".into())
            .spawn(move || {
                let mut buffer = [0u8; 4096];
                let mut query_responder = TerminalQueryResponder::default();
                loop {
                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(count) => {
                            if let Ok(mut parser) = reader_parser.lock() {
                                parser.process(&buffer[..count]);
                                let cursor = parser.screen().cursor_position();
                                for response in query_responder.responses(&buffer[..count], cursor)
                                {
                                    let _ = reader_commands.send(Command::Write(response));
                                }
                                reader_generation.fetch_add(1, Ordering::Relaxed);
                            } else {
                                break;
                            }
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| format!("Could not start terminal reader: {error}"))?;

        let master = pair.master;
        let worker = match thread::Builder::new()
            .name("hermes-local-terminal-session".into())
            .spawn(move || run_session(master, writer, &mut child, command_rx, event_tx))
        {
            Ok(worker) => worker,
            Err(error) => {
                let _ = reader_thread.join();
                return Err(format!("Could not start terminal session worker: {error}"));
            }
        };

        Ok(Self {
            commands,
            events,
            parser,
            output_generation,
            last_output_generation: 0,
            worker: Some(worker),
            reader: Some(reader_thread),
            exited: false,
            size: (rows, cols),
        })
    }

    pub fn send_line(&self, line: &str) -> Result<(), String> {
        if self.exited {
            return Err("The local shell has exited".into());
        }
        let mut bytes = line.as_bytes().to_vec();
        bytes.extend_from_slice(b"\r\n");
        self.commands
            .send(Command::Write(bytes))
            .map_err(|_| "The local shell is no longer available".into())
    }

    pub fn interrupt(&self) -> Result<(), String> {
        if self.exited {
            return Err("The local shell has exited".into());
        }
        self.commands
            .send(Command::Write(vec![3]))
            .map_err(|_| "The local shell is no longer available".into())
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let size = (rows.clamp(4, 200), cols.clamp(20, 400));
        if size == self.size || self.exited {
            return;
        }
        if self
            .commands
            .send(Command::Resize {
                rows: size.0,
                cols: size.1,
            })
            .is_ok()
        {
            self.size = size;
            if let Ok(mut parser) = self.parser.lock() {
                let _ = parser.set_size(
                    std::num::NonZeroU16::new(size.0).expect("clamped rows"),
                    std::num::NonZeroU16::new(size.1).expect("clamped columns"),
                );
            }
        }
    }

    pub fn take_events(&mut self) -> Vec<TerminalEvent> {
        let mut events = Vec::new();
        while let Ok(event) = self.events.try_recv() {
            if matches!(event, TerminalEvent::Exited(_) | TerminalEvent::Failed(_)) {
                self.exited = true;
            }
            events.push(event);
        }
        events
    }

    pub fn has_new_output(&mut self) -> bool {
        let generation = self.output_generation.load(Ordering::Relaxed);
        let changed = generation != self.last_output_generation;
        self.last_output_generation = generation;
        changed
    }

    pub fn screen_text(&self) -> String {
        let Ok(parser) = self.parser.lock() else {
            return String::new();
        };
        let screen = parser.screen();
        let (rows, cols) = screen.size();
        let mut output = String::new();
        for row in 0..rows.get() {
            let mut line = String::new();
            for col in 0..cols.get() {
                if let Some(cell) = screen.cell(row, col) {
                    line.push_str(cell.contents());
                }
            }
            output.push_str(line.trim_end());
            if row + 1 < rows.get() {
                output.push('\n');
            }
        }
        output
    }

    pub fn is_exited(&self) -> bool {
        self.exited
    }
}

fn run_session(
    master: Box<dyn MasterPty + Send>,
    mut writer: Box<dyn Write + Send>,
    child: &mut Box<dyn portable_pty::Child + Send + Sync>,
    commands: Receiver<Command>,
    events: Sender<TerminalEvent>,
) {
    loop {
        match commands.recv_timeout(Duration::from_millis(50)) {
            Ok(Command::Write(bytes)) => {
                if let Err(error) = writer.write_all(&bytes).and_then(|_| writer.flush()) {
                    let _ = events.send(TerminalEvent::Failed(format!(
                        "Terminal input failed: {error}"
                    )));
                    let _ = child.kill();
                    break;
                }
            }
            Ok(Command::Resize { rows, cols }) => {
                if let Err(error) = master.resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                }) {
                    let _ = events.send(TerminalEvent::Failed(format!(
                        "Terminal resize failed: {error}"
                    )));
                }
            }
            Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = events.send(TerminalEvent::Exited(format!(
                    "Shell exited with code {}",
                    status.exit_code()
                )));
                break;
            }
            Ok(None) => {}
            Err(error) => {
                let _ = events.send(TerminalEvent::Failed(format!(
                    "Could not inspect shell process: {error}"
                )));
                break;
            }
        }
    }
}

impl Drop for LocalTerminal {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir() -> std::path::PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after UNIX epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "hermes-native-terminal-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated terminal working directory");
        path
    }

    fn wait_for_text(terminal: &mut LocalTerminal, needle: &str) -> bool {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            for event in terminal.take_events() {
                if matches!(event, TerminalEvent::Exited(_) | TerminalEvent::Failed(_)) {
                    return false;
                }
            }
            if terminal.screen_text().contains(needle) {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn wait_for_exit(terminal: &mut LocalTerminal) -> bool {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            terminal.take_events();
            if terminal.is_exited() {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn vt_screen_interprets_control_sequences_and_bounds_dimensions() {
        let mut parser = Parser::new(
            std::num::NonZeroU16::new(4).unwrap(),
            std::num::NonZeroU16::new(12).unwrap(),
            16,
        );
        parser.process(b"hello\r\n\x1b[31mworld\x1b[0m");
        assert!(parser.screen().cell(0, 0).unwrap().contents().contains('h'));
        assert!(parser.screen().cell(1, 0).unwrap().contents().contains('w'));
    }

    #[test]
    fn terminal_query_responder_handles_sequences_split_between_reads() {
        let mut responder = TerminalQueryResponder::default();
        assert!(responder.responses(b"ready\x1b[", (2, 4)).is_empty());
        assert_eq!(
            responder.responses(b"6n", (2, 4)),
            vec![b"\x1b[3;5R".to_vec()]
        );
        assert_eq!(
            responder.responses(b"\x1b[5n", (0, 0)),
            vec![b"\x1b[0n".to_vec()]
        );
    }

    #[test]
    fn local_pty_echoes_output_resizes_and_stops_cleanly() {
        let cwd = unique_temp_dir();
        let mut terminal = LocalTerminal::spawn(&cwd, 30, 120).expect("start explicit PTY");

        let first_marker = format!("hermes-pty-first-{}", std::process::id());
        terminal
            .send_line(&format!("echo {first_marker}"))
            .expect("write first command");
        let first_output_arrived = wait_for_text(&mut terminal, &first_marker);
        let output_generation_advanced = terminal.has_new_output();

        terminal.resize(40, 140);
        let second_marker = format!("hermes-pty-resized-{}", std::process::id());
        terminal
            .send_line(&format!("echo {second_marker}"))
            .expect("write command after resize");
        let second_output_arrived = wait_for_text(&mut terminal, &second_marker);

        // Exit the shell explicitly so both PTY reader and worker can terminate on
        // platforms where closing a cloned PTY reader alone does not interrupt a read.
        terminal
            .send_line("exit")
            .expect("write shell exit command");
        let exited = wait_for_exit(&mut terminal);

        let stop_started = std::time::Instant::now();
        drop(terminal);
        assert!(
            stop_started.elapsed() < Duration::from_secs(5),
            "dropping the PTY did not stop and join its workers promptly"
        );
        std::fs::remove_dir_all(cwd).expect("remove isolated terminal working directory");

        assert!(first_output_arrived, "first command output never arrived");
        assert!(
            output_generation_advanced,
            "PTY output generation did not advance"
        );
        assert!(
            second_output_arrived,
            "PTY stopped producing output after resize"
        );
        assert!(exited, "shell did not report clean exit");
    }
}
