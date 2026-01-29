use anyhow::{Result, bail};
use argh::FromArgs;
use ast::formatter::IndentationMode;
use fs_lib::{
    LUAU_DECODE_TABLES, buffer::BufferExtension, list_files_with_extension, path::PathExtension,
};
use gar_lib::{GarArchive, GarPath};
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

    /// include line number info for functions when applicable
    #[argh(switch, short = 'l')]
    function_line_info: bool,

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

fn decompile_bytecode(bytecode: &mut Vec<u8>, write_function_line_info: bool) -> Result<Vec<u8>> {
    let (version, is_encoded, is_dlc) = get_bytecode_info(&bytecode);

    if version == 0 {
        bail!("Unsupported/unknown bytecode");
    }

    if is_encoded {
        decode_bytecode(bytecode, version, is_dlc)?;
    }

    Ok(luau_lifter::decompile_bytecode_with_opts(
        &bytecode,
        1,
        write_function_line_info,
        IndentationMode::default(),
    )
    .as_bytes()
    .to_vec())
}

fn decompile_file<P: AsRef<Path>>(file: P, write_function_line_info: bool) -> Result<Vec<u8>> {
    let mut bytecode = Vec::read_from_file(&file)?;

    match decompile_bytecode(&mut bytecode, write_function_line_info) {
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

fn decompile_from_archive(
    archive: &GarArchive,
    path: &str,
    write_function_line_info: bool,
) -> Result<Vec<u8>> {
    let mut bytecode = archive
        .read_file(path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match decompile_bytecode(&mut bytecode, write_function_line_info) {
        Ok(result) => Ok(result),
        Err(e) => bail!("{}: {}", path, e),
    }
}

fn decode_from_archive(archive: &GarArchive, path: &str) -> Result<Vec<u8>> {
    let mut bytecode = archive
        .read_file(path)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let (version, is_encoded, is_dlc) = get_bytecode_info(&bytecode);

    if version == 0 {
        bail!("Unsupported/unknown bytecode");
    }

    if is_encoded {
        decode_bytecode(&mut bytecode, version, is_dlc)?;
    }

    Ok(bytecode)
}

fn list_l64_files<'a>(archive: &'a GarArchive, base: &str, recursive: bool) -> Vec<&'a str> {
    archive
        .files_with_extension(base, "l64", recursive)
        .into_iter()
        .filter(|f| !f.contains("XMLSchema.l64"))
        .collect()
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    match GarPath::parse(&cli.input) {
        GarPath::Filesystem(path) => {
            if path.is_file() {
                let mut output_file: PathBuf =
                    cli.output.unwrap_or(path.clone()).components().collect();

                let result = match cli.decode_only {
                    false => {
                        if output_file.extension().unwrap() == "l64" {
                            output_file.set_extension("lua");
                        }

                        decompile_file(&path, cli.function_line_info)?
                    }
                    true => decode_file(&path)?,
                };

                result.write_to_file(&output_file)?;

                if !cli.silent {
                    if output_file != path {
                        println!("{} -> {}", path.display(), output_file.display());
                    } else {
                        println!("{}", path.display());
                    }
                }
            } else if path.is_dir() {
                let output_path = cli.output.unwrap_or_else(|| path.clone());

                if output_path.is_file() {
                    bail!("Output path is a file")
                }

                ThreadPoolBuilder::new()
                    .num_threads(cli.num_threads.into())
                    .build_global()
                    .unwrap();

                let files: Vec<_> = list_files_with_extension(&path, r"l64", cli.recursive)?
                    .into_iter()
                    .filter(|f| {
                        let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
                        !name.contains("XMLSchema")
                    })
                    .collect();

                let iter_result = files.into_par_iter().try_for_each(|file| -> Result<()> {
                    let mut output_file: PathBuf = file
                        .convert_relative_path(&path, &output_path)?
                        .components()
                        .collect();

                    let result = match cli.decode_only {
                        false => {
                            if output_file.extension().unwrap() == "l64" {
                                output_file.set_extension("lua");
                            }

                            decompile_file(&file, cli.function_line_info)?
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
        }
        GarPath::Archive {
            archive_path,
            internal_path,
        } => {
            let archive = GarArchive::open(&archive_path).map_err(|e| anyhow::anyhow!("{}", e))?;
            let base = internal_path.as_deref().unwrap_or("");
            let output_path = cli.output.unwrap_or_else(|| PathBuf::from("."));

            ThreadPoolBuilder::new()
                .num_threads(cli.num_threads.into())
                .build_global()
                .unwrap();

            // Check if internal path is a single file
            if base.ends_with(".l64") {
                let result = if cli.decode_only {
                    decode_from_archive(&archive, base)?
                } else {
                    decompile_from_archive(&archive, base, cli.function_line_info)?
                };

                let filename = Path::new(base).file_name().unwrap();
                let mut out_file = output_path.join(filename);
                if !cli.decode_only {
                    out_file.set_extension("lua");
                }

                result.write_to_file(&out_file)?;

                if !cli.silent {
                    println!("{} -> {}", base, out_file.display());
                }
            } else {
                // Directory - process multiple files
                let files = list_l64_files(&archive, base, cli.recursive);

                if files.is_empty() {
                    bail!("No .l64 files found in archive path: {}", base);
                }

                let iter_result = files.into_par_iter().try_for_each(|file| -> Result<()> {
                    let result = if cli.decode_only {
                        decode_from_archive(&archive, file)?
                    } else {
                        decompile_from_archive(&archive, file, cli.function_line_info)?
                    };

                    let rel_path = file
                        .strip_prefix(base)
                        .unwrap_or(file)
                        .trim_start_matches('/');
                    let mut out_file = output_path.join(rel_path);
                    if !cli.decode_only {
                        out_file.set_extension("lua");
                    }

                    result.write_to_file(&out_file)?;

                    if !cli.silent {
                        println!("{} -> {}", file, out_file.display());
                    }

                    Ok(())
                });

                return iter_result;
            }
        }
    }

    Ok(())
}
