use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{
    LUAU_DECODE_TABLES, buffer::BufferExtension, list_files_with_extension, path::PathExtension,
};
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelIterator, ParallelIterator},
};
use std::path::{Path, PathBuf};

#[derive(FromArgs, PartialEq, Debug)]
/// Decode and decompile Luau .l64 bytecode files
pub struct Cmd {
    /// recursive mode if folder input
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// suppress output
    #[argh(switch, short = 's')]
    silent: bool,

    /// only decode files
    #[argh(switch, short = 'd')]
    decode_only: bool,

    /// set thread pool size when processing folders (0 = auto)
    #[argh(option, default = "0")]
    num_threads: u8,

    /// path to input file/folder
    #[argh(positional)]
    input: PathBuf,

    /// path to output file/folder (optional)
    #[argh(positional)]
    output: Option<PathBuf>,
}

// (version, is_encoded, is_dlc)
fn get_bytecode_info(buffer: &Vec<u8>) -> (u8, bool, bool) {
    match &buffer[0..3] {
        [0x03, 0x00, 0xF2] => (6, true, true),
        [0x02, 0xEF, ..] => (3, true, false),
        [0x03, 0xFD, ..] => (3, true, true),
        [0x02, 0xF0, ..] => (4, true, false),
        [0x02, 0xF2, ..] => (6, true, false),
        [0x06, 0x03, ..] => (6, false, false),
        [0x03, ..] => (3, false, false),
        [0x04, ..] => (4, false, false),
        _ => (0, false, false),
    }
}

fn decode_bytecode(buffer: &mut Vec<u8>, version: u8, is_dlc: bool) -> Result<()> {
    let Some(table) = LUAU_DECODE_TABLES.get(&(version, is_dlc)) else {
        bail!("Unable to decode, no valid byteshift table found")
    };

    buffer.shift_bytes(&table.bytes, table.offset, table.mask);
    buffer.remove(0);

    Ok(())
}

fn decompile_bytecode(bytecode: &mut Vec<u8>) -> Result<Vec<u8>> {
    let (version, is_encoded, is_dlc) = get_bytecode_info(&bytecode);

    if version == 0 {
        bail!("Unsupported/unknown bytecode");
    }

    if is_encoded {
        decode_bytecode(bytecode, version, is_dlc)?;
    }

    Ok(luau_lifter::decompile_bytecode(&bytecode, 1)
        .as_bytes()
        .to_vec())
}

fn decompile_file<P: AsRef<Path>>(file: P) -> Result<Vec<u8>> {
    let mut bytecode = Vec::read_from_file(&file)?;

    match decompile_bytecode(&mut bytecode) {
        Ok(result) => Ok(result),
        Err(e) => bail!("{}: {}", file.as_ref().display(), e),
    }
}

fn decode_file<P: AsRef<Path>>(file: P) -> Result<Vec<u8>> {
    let mut bytecode = Vec::read_from_file(&file)?;

    let (version, is_encoded, is_dlc) = get_bytecode_info(&bytecode);

    if version == 0 {
        bail!("Unsupported/unknown bytecode");
    }

    if is_encoded {
        decode_bytecode(&mut bytecode, version, is_dlc)?;
    }

    Ok(bytecode)
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.input.is_file() {
        let mut output_file: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        let result = match cli.decode_only {
            false => {
                if output_file.extension().unwrap() == "l64" {
                    output_file.set_extension("lua");
                }

                decompile_file(&cli.input)?
            }
            true => decode_file(&cli.input)?,
        };

        result.write_to_file(&output_file)?;

        if !cli.silent {
            if output_file != *cli.input {
                println!("{} -> {}", cli.input.display(), output_file.display());
            } else {
                println!("{}", cli.input.display());
            }
        }
    } else if cli.input.is_dir() {
        let output_path = cli.output.unwrap_or_else(|| cli.input.clone());

        if output_path.is_file() {
            bail!("Output path is a file")
        }

        ThreadPoolBuilder::new()
            .num_threads(cli.num_threads.into())
            .build_global()
            .unwrap();

        let files = list_files_with_extension(&cli.input, r"l64", cli.recursive)?;

        let iter_result = files.into_par_iter().try_for_each(|file| -> Result<()> {
            let mut output_file: PathBuf = file
                .convert_relative_path(&cli.input, &output_path)?
                .components()
                .collect();

            let result = match cli.decode_only {
                false => {
                    if output_file.extension().unwrap() == "l64" {
                        output_file.set_extension("lua");
                    }

                    decompile_file(&file)?
                }
                true => decode_file(&file)?,
            };

            result.write_to_file(&output_file)?;

            if !cli.silent {
                if output_file != *file {
                    println!("{} -> {}", file.display(), output_file.display());
                } else {
                    println!("{}", file.display());
                }
            }

            Ok(())
        });

        return iter_result;
    }

    Ok(())
}
