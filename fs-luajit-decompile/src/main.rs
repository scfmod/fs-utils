use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::path::PathExtension;
use fs_lib::{
    LUAJIT_DECODE_TABLES, buffer::BufferExtension, cmd::run_command_return_stdout,
    list_files_with_extension,
};
use rayon::ThreadPoolBuilder;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};

#[derive(FromArgs, PartialEq, Debug)]
/// Decode and decompile LuaJIT .l64 bytecode files
pub struct Cmd {
    /// recursive mode if folder input
    #[argh(switch, short = 'r')]
    recursive: bool,

    /// suppress output
    #[argh(switch, short = 's')]
    silent: bool,

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

fn is_valid(buffer: &Vec<u8>) -> bool {
    buffer[0..3] == [0x1B, 0x4C, 0x4A]
}

fn is_encoded(buffer: &Vec<u8>) -> bool {
    buffer[4] == 0xFC
}

fn decode(buffer: &mut Vec<u8>) -> Result<()> {
    let Some(table) = LUAJIT_DECODE_TABLES.get(&buffer[3]) else {
        bail!("Unable to decode, no valid byteshift table found")
    };

    buffer.shift_bytes(&table.bytes, table.offset, table.mask);
    buffer[3] = 0x02;

    Ok(())
}

fn decompile<P: AsRef<Path>>(file: P, output_file: P) -> Result<PathBuf> {
    let mut file_buffer = std::fs::read(&file)?;

    if !is_valid(&file_buffer) {
        bail!("Unsupported bytecode file")
    }

    if is_encoded(&file_buffer) {
        decode(&mut file_buffer)?;
        file_buffer.write_to_file(&file)?;
    }

    let result = run_command_return_stdout("luajit-decompiler.exe", [&file.as_ref()])?;
    let mut output_file: PathBuf = output_file.as_ref().to_path_buf();

    if output_file.extension().unwrap() == "l64" {
        output_file.set_extension("lua");
    }

    result.write_to_file(&output_file)?;

    Ok(output_file)
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.input.is_dir() {
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
            let output_file: PathBuf = file
                .convert_relative_path(&cli.input, &output_path)?
                .components()
                .collect();

            let output_file = decompile(&file, &output_file)?;

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
    } else {
        let output_file: PathBuf = cli
            .output
            .unwrap_or(cli.input.clone())
            .components()
            .collect();

        let output_file = decompile(&cli.input, &output_file)?;

        if !cli.silent {
            println!("{}", output_file.display());
        }
    }

    Ok(())
}
