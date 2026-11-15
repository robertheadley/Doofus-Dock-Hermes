use std::{env, fs, io::Write, path::PathBuf};

fn main() {
    let source = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("assets/pdfium/chromium-8125/bin/pdfium.dll");
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_ARCH");

    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ARCH").as_deref() != Ok("x86_64")
    {
        println!(
            "cargo:warning=Bundled PDFium runtime is currently available for Windows x64 only"
        );
        return;
    }

    let output_dir = PathBuf::from(env::var_os("OUT_DIR").expect("build output directory"));
    let profile_dir = output_dir
        .ancestors()
        .nth(3)
        .expect("Cargo OUT_DIR must have a profile ancestor");
    let destination = profile_dir.join("pdfium-8125.dll");
    let bytes = fs::read(&source).unwrap_or_else(|error| {
        panic!("read bundled PDFium runtime {}: {error}", source.display())
    });

    if destination.exists() {
        let existing = fs::read(&destination).unwrap_or_else(|error| {
            panic!(
                "read existing PDFium runtime {}: {error}",
                destination.display()
            )
        });
        if existing == bytes {
            return;
        }
        panic!(
            "refusing to replace a different existing PDFium runtime at {}",
            destination.display()
        );
    }

    let temporary = profile_dir.join(format!(".pdfium-8125-{}.dll.tmp", std::process::id()));
    let write_result = (|| -> std::io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, &destination)
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        panic!(
            "install bundled PDFium runtime to {}: {error}",
            destination.display()
        );
    }
}
