use std::{ffi::OsStr, process};

use anyhow::{Result, bail};

use crate::try_get_command_path;

pub fn run_command_return_stdout<I, S>(file: &str, args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut file_path = try_get_command_path(file)?;
    file_path = file_path.components().collect();

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
