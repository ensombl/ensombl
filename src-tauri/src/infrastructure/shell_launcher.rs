//! [`DetachedLauncher`] that runs a command line through the login shell without waiting.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::application::ports::DetachedLauncher;
use crate::application::{Error, Result};

pub struct ShellLauncher {
    pub shell: std::path::PathBuf,
}

impl DetachedLauncher for ShellLauncher {
    fn launch(&self, command_line: &str, cwd: &Path, env: &[(String, String)]) -> Result<()> {
        Command::new(&self.shell)
            .args(["-lc", command_line])
            .current_dir(cwd)
            .env_clear()
            .envs(env.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| Error::Invalid(format!("couldn't launch editor: {e}")))?;
        Ok(())
    }
}
