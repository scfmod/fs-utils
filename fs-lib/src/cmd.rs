use std::{ffi::OsStr, path::PathBuf, process};

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

/// Run command with an absolute path (no lookup)
pub fn run_command_absolute<I, S>(file_path: &PathBuf, args: I) -> Result<Vec<u8>>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut cmd = process::Command::new(file_path);
    let cmd = cmd.args(args);
    let output = cmd.output()?;

    let Some(code) = output.status.code() else {
        bail!("Failed to execute command: {}", file_path.display())
    };

    if code != 0 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "Error (code {}) when executing command: {}\n{}",
            code,
            file_path.display(),
            stderr
        );
    }

    Ok(output.stdout)
}

/// Find the luau-lifter binary for the current platform
/// On Windows: looks for luau-lifter.exe
/// On Mac/Linux: looks for luau-lifter (no extension)
pub fn find_luau_lifter() -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var("LUAU_LIFTER_PATH") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    // Platform-specific binary name
    let binary_name = if cfg!(target_os = "windows") {
        "luau-lifter.exe"
    } else {
        "luau-lifter"
    };

    // Try to find it using existing logic
    if let Ok(path) = try_get_command_path(binary_name) {
        return Ok(path);
    }

    // On Windows, also try .exe variant
    if cfg!(target_os = "windows") {
        if let Ok(path) = try_get_command_path("luau-lifter.exe") {
            return Ok(path);
        }
    }

    bail!(
        "Could not find luau-lifter binary. Set LUAU_LIFTER_PATH environment variable \
        or place '{}' in the bin directory.",
        binary_name
    )
}
