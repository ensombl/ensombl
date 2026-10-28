//! [`PtySpawner`] backed by `portable-pty` (native PTYs on macOS/Linux, ConPTY on Windows).

use std::io::Write;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};

use crate::application::ports::{PtyControl, PtySpawner, SpawnSpec, SpawnedPty};
use crate::application::{Error, Result};

pub struct NativePty;

struct Control {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

fn pty_err(e: impl std::fmt::Display) -> Error {
    Error::Invalid(format!("pty: {e}"))
}

impl PtyControl for Control {
    fn write(&mut self, data: &[u8]) -> Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }

    fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_err)
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
    }

    fn wait(&mut self) -> Option<i32> {
        self.child.wait().ok().map(|s| s.exit_code() as i32)
    }
}

impl PtySpawner for NativePty {
    fn spawn(&self, spec: SpawnSpec) -> Result<SpawnedPty> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: spec.rows,
                cols: spec.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(pty_err)?;

        let mut cmd = CommandBuilder::new(&spec.program);
        cmd.args(&spec.args);
        cmd.cwd(&spec.cwd);
        cmd.env_clear();
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd).map_err(pty_err)?;
        drop(pair.slave);

        let output = pair.master.try_clone_reader().map_err(pty_err)?;
        let writer = pair.master.take_writer().map_err(pty_err)?;
        Ok(SpawnedPty {
            output,
            control: Box::new(Control {
                master: pair.master,
                writer,
                child,
            }),
        })
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::io::Read;

    use super::*;

    #[test]
    fn runs_a_process_and_reads_its_output() {
        let mut spawned = NativePty
            .spawn(SpawnSpec {
                program: "/bin/sh".into(),
                args: vec!["-c".into(), "printf crm-ok".into()],
                cwd: std::env::temp_dir(),
                env: vec![("PATH".into(), "/usr/bin:/bin".into())],
                cols: 80,
                rows: 24,
            })
            .unwrap();
        let mut out = String::new();
        let _ = spawned.output.read_to_string(&mut out);
        assert!(out.contains("crm-ok"), "got {out:?}");
        assert_eq!(spawned.control.wait(), Some(0));
    }
}
