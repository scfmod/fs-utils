use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{LUAU_DECODE_TABLES, buffer::BufferExtension};
use mlua::Compiler;
use walkdir::WalkDir;

#[derive(FromArgs, PartialEq, Debug)]
/// Compile and encode Lua(u) file to valid bytecode file
pub struct Cmd {
    /// path to input file (.lua) or directory
    #[argh(positional)]
    input: PathBuf,

    /// path to output file (optional, defaults to input with .l64 extension)
    #[argh(positional)]
    output: Option<PathBuf>,

    /// process directory recursively
    #[argh(switch, short = 'r')]
    recursive: bool,
}

fn compile_file(input: &PathBuf, output: &PathBuf) -> Result<()> {
    println!("Compiling {:?}", input);
    let source = fs::read_to_string(input)?;
    let compiler = Compiler::new();
    let mut bytecode = compiler.compile(&source)
        .map_err(|e| anyhow::anyhow!("Lua compile error: {}", e))?;

    let version = bytecode[0];
    println!("Bytecode version: {}", version);

    let Some(table) = LUAU_DECODE_TABLES.get(&(version, false)) else {
        bail!("Missing bytecode shift table for version {}", version)
    };

    bytecode.insert(0, 0);
    bytecode.shift_bytes_reversed(&table.bytes, table.offset, table.mask);
    bytecode[0] = 0x02;

    println!("Writing encoded l64 to {:?}", output);
    bytecode.write_to_file(output)
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.input.is_dir() || cli.recursive {
        let walker = if cli.recursive {
            WalkDir::new(&cli.input)
        } else {
            WalkDir::new(&cli.input).max_depth(1)
        };

        let mut count = 0;
        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "lua") {
                let output_path = path.with_extension("l64");
                if let Err(e) = compile_file(&path.to_path_buf(), &output_path) {
                    eprintln!("Error compiling {:?}: {}", path, e);
                } else {
                    count += 1;
                }
            }
        }
        println!("\nCompiled {} files", count);
    } else {
        let output = cli.output.unwrap_or_else(|| cli.input.with_extension("l64"));
        compile_file(&cli.input, &output)?;
    }

    Ok(())
}
