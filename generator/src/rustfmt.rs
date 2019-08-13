use std::fs::File;
use std::io;
use std::process::{Child, Command, Stdio};
use std::path::PathBuf;

pub enum RustFmtWriter {
    Formatted(Child),
    Unformatted(File),
}

impl RustFmtWriter {
    pub(crate) fn new(output_file: File) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(match rustfmt_path() {
            Some(path) => RustFmtWriter::Formatted(
                Command::new(path)
                    .arg("--edition=2018")
                    .stderr(Stdio::null())
                    .stdout(output_file)
                    .stdin(Stdio::piped())
                    .spawn()?,
            ),
            None => RustFmtWriter::Unformatted(output_file),
        })
    }

    pub(crate) fn close(self) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            RustFmtWriter::Formatted(mut cmd) => {
                if cmd.wait()?.success() {
                    Ok(())
                } else {
                    Err("rustfmt exited with error".to_owned().into())
                }
            },
            RustFmtWriter::Unformatted(file) => Ok(file.sync_all()?),
        }
    }
}

impl io::Write for RustFmtWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            RustFmtWriter::Formatted(cmd) => cmd.stdin.as_mut().unwrap().write(buf),
            RustFmtWriter::Unformatted(file) => file.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            RustFmtWriter::Formatted(_) => Ok(()),
            RustFmtWriter::Unformatted(file) => file.flush(),
        }

    }
}

fn rustfmt_path() -> Option<PathBuf> {
    match std::env::var_os("RUSTFMT") {
        Some(which) => {
            if which.is_empty() {
                None
            } else {
                Some(PathBuf::from(which))
            }
        }
        None => toolchain_find::find_installed_component("rustfmt"),
    }
}
