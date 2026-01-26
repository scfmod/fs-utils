use std::{ffi::OsStr, path::PathBuf, process};

use anyhow::{Result, bail};

use crate::try_get_command_path;

pub fn run_command_return_stdout<I, S>(file: &str, args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let file_path: PathBuf = try_get_command_path(file)?.components().collect();

    let mut cmd = process::Command::new(&file_path);
    let cmd = cmd.args(args);
    let output = cmd.output()?;

    let Some(code) = output.status.code() else {
        bail!("Failed to execute command: {}", file_path.display())
    };

    if code != 0 {
        bail!(
            "Error (code {}) when executing command: {}",
            code,
            file_path.display()
        );
    }

    Ok(output.stdout)
}
