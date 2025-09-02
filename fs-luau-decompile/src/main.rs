use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{
    LUAU_DECODE_TABLES, buffer::BufferExtension, cmd::run_command_return_stdout,
    list_files_with_extension, path::PathExtension,
};
use std::path::{Path, PathBuf};

use crate::luau::{
    DecompileOptions,
    util::{format_luau_buffer, parse_bytecode_info},
};

mod luau;

#[derive(FromArgs, PartialEq, Debug)]
/// Decode and decompile Luau .l64 bytecode files
pub struct Cmd {
    /// recursive mode if folder input
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// suppress output
    #[argh(switch, short = 's')]
    silent: bool,

    /// do not include local values and upvalues
    #[argh(switch, short = 'd')]
    disable_formatting: bool,

    /// include line number for functions when applicable
    #[argh(switch, short = 'l')]
    enable_line_numbers: bool,

    /// include symbol table
    #[argh(switch, short = 't')]
    enable_symbol_table: bool,

    /// path to input file/folder
    #[argh(positional)]
    input: PathBuf,

    /// path to output file/folder (optional)
    #[argh(positional)]
    output: Option<PathBuf>,
}

// (version, is_encoded, is_dlc)
fn get_version(buffer: &Vec<u8>) -> (u8, bool, bool) {
    match buffer[0..2] {
        [0x02, 0xEF] => (3, true, false),
        [0x03, 0xFD] => (3, true, true),
        [0x02, 0xF0] => (4, true, false),
        [0x02, 0xF2] => (6, true, false),
        [0x06, 0x03] => (6, false, true),
        _ => match buffer[0] {
            0x03 => (3, false, false),
            0x04 => (4, false, false),
            _ => (0, false, false),
        },
    }
}

fn decode(buffer: &mut Vec<u8>, version: u8, is_dlc: bool) -> Result<()> {
    let Some(table) = LUAU_DECODE_TABLES.get(&(version, is_dlc)) else {
        bail!("Unable to decode, no valid byteshift table found")
    };

    buffer.shift_bytes(&table.bytes, table.offset, table.mask);
    buffer.remove(0);

    Ok(())
}

fn decompile<P: AsRef<Path>>(file: P, output_file: P, opts: &DecompileOptions) -> Result<PathBuf> {
    let mut bytecode = Vec::read_from_file(&file)?;

    let (version, is_encoded, is_dlc) = get_version(&bytecode);

    if version == 0 {
        bail!(
            "Unsupported/unknown bytecode file: {}",
            file.as_ref().display()
        );
    }

    if is_encoded {
        decode(&mut bytecode, version, is_dlc)?;
        bytecode.write_to_file(&file)?;
    }

    let mut result = run_command_return_stdout("luau-lifter.exe", [&file.as_ref()])?;

    let (main, prototypes, symbol_table) = parse_bytecode_info(&bytecode)?;

    format_luau_buffer(&mut result, main, &prototypes, &symbol_table, &opts)?;

    let mut output_file: PathBuf = output_file.as_ref().to_path_buf();

    if output_file.extension().unwrap() == "l64" {
        output_file.set_extension("lua");
    }

    result.write_to_file(&output_file)?;

    Ok(output_file)
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    let opts = DecompileOptions {
        use_symbol_table: cli.enable_symbol_table,
        use_line_numbers: cli.enable_line_numbers,
        use_variables: !cli.disable_formatting,
    };

    if cli.input.is_dir() {
        let output_path = cli.output.unwrap_or_else(|| cli.input.clone());

        if output_path.is_file() {
            bail!("Output path is a file")
        }

        let files = list_files_with_extension(&cli.input, r"l64", cli.recursive)?;

        for file in files.iter() {
            // Decompiler will end up in an endless loop trying to decompile this specific file.
            if file.file_name().unwrap() == "XMLSchema.l64" {
                continue;
            }

            let output_file: PathBuf = file
                .convert_relative_path(&cli.input, &output_path)?
                .components()
                .collect();

            let output_file = decompile(&file, &&output_file, &opts)?;

            if !cli.silent {
                if output_file != *file {
                    println!("{} -> {}", file.display(), output_file.display());
                } else {
                    println!("{}", file.display());
                }
            }
        }
    } else {
        let output_file: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        let output_file = decompile(&cli.input, &output_file, &opts)?;

        if !cli.silent {
            println!("{}", output_file.display());
        }
    }

    Ok(())
}
