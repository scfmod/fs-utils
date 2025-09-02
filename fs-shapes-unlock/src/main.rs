use std::path::{Path, PathBuf};

/**
fs-shapes-unlock [-r|--recursive] <file|folder> [<output>]
*/
use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{buffer::BufferExtension, list_files_with_extension, path::PathExtension};

#[derive(FromArgs, PartialEq, Debug)]
/// Unlock .i3d.shapes files
pub struct Cmd {
    /// recursive mode if folder input
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// suppress output
    #[argh(switch, short = 's')]
    silent: bool,

    /// path to input file/folder
    #[argh(positional)]
    input: PathBuf,

    /// path to output file/folder (optional)
    #[argh(positional)]
    output: Option<PathBuf>,
}

fn unlock_shapes_file<P: AsRef<Path>>(file: P, output_file: P) -> Result<()> {
    let mut buffer = Vec::read_from_file(&file)?;

    if !is_locked(&buffer)? {
        return Ok(());
    }

    unlock(&mut buffer)?;

    buffer.write_to_file(&output_file)
}

fn is_locked(buffer: &Vec<u8>) -> Result<bool> {
    match buffer[0] {
        0x05 | 0x07 | 0x0A => Ok(buffer[1] != 0 || buffer[3] != 0),
        0x00 | 0x01 => Ok(buffer[2] != 0),
        _ => bail!("Unknown format"),
    }
}

fn unlock(buffer: &mut Vec<u8>) -> Result<()> {
    match buffer[0] {
        0x05 | 0x07 | 0x0A => {
            // FS22, FS25 (0x0A)
            buffer[1] = 0;
            buffer[2] = buffer[2].wrapping_sub(0x0D);
            buffer[3] = 0;
        }
        0x00 | 0x01 => {
            // Legacy
            buffer[0] = 0;
            buffer[1] = buffer[1].wrapping_sub(0x0D);
            buffer[2] = 0;
        }
        _ => bail!("Unknown format"),
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.input.is_dir() {
        let output_path = cli.output.unwrap_or_else(|| cli.input.clone());

        if output_path.is_file() {
            bail!("Output path is a file")
        }

        let files = list_files_with_extension(&cli.input, r"shapes", cli.recursive)?;

        for file in files.iter() {
            let output_file: PathBuf = file
                .convert_relative_path(&cli.input, &output_path)?
                .components()
                .collect();

            if !cli.silent {
                if output_file != *file {
                    println!("{} -> {}", file.display(), output_file.display());
                } else {
                    println!("{}", file.display());
                }
            }

            unlock_shapes_file(&file, &&output_file)?;
        }
    } else {
        let output: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        unlock_shapes_file(&cli.input, &output)?;

        if !cli.silent {
            println!("{}", output.display());
        }
    }

    Ok(())
}
