use std::path::PathBuf;

use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{
    LUAU_DECODE_TABLES, buffer::BufferExtension, byte_array_hex_string,
    cmd::run_command_return_stdout,
};

#[derive(FromArgs, PartialEq, Debug)]
/// Compile and encode Lua(u) file to valid bytecode file
pub struct Cmd {
    /// path to input file
    #[argh(positional)]
    input: PathBuf,

    /// path to output file
    #[argh(positional)]
    output: PathBuf,
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    let mut compiled_buffer = run_command_return_stdout(
        "luau-compile.exe",
        ["--binary", &cli.input.as_path().to_str().unwrap()],
    )?;

    let version = compiled_buffer[0];

    let Some(table) = LUAU_DECODE_TABLES.get(&(version, false)) else {
        bail!("Missing bytecode shift table")
    };

    compiled_buffer.insert(0, 0);
    compiled_buffer.shift_bytes_reversed(&table.bytes, table.offset, table.mask);
    compiled_buffer[0] = 0x02;
    compiled_buffer.write_to_file(cli.output)
}
