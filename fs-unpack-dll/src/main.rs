use std::path::PathBuf;

use anyhow::{Result, bail, ensure};
use argh::FromArgs;
use base64ct::{Base64, Encoding};
use fs_lib::{buffer::BufferExtension, get_optional_path};
use miniz_oxide::inflate::decompress_to_vec_zlib;

#[derive(FromArgs, PartialEq, Debug)]
/// Extract defarm.dll from script.bms
pub struct Cmd {
    /// path to QuickBMS script file
    #[argh(positional)]
    input: PathBuf,

    /// output path
    #[argh(positional)]
    output_path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    let input = Vec::read_from_file(&cli.input)?;
    let pattern = b"set MEMORY_FILE3 compressed \"";

    let Some(mut start) = input.windows(pattern.len()).position(|w| w == pattern) else {
        bail!("Failed to find compressed file in script")
    };
    start = start + pattern.len();

    let Some(mut end) = input[start..].iter().position(|&b| b == b'"') else {
        bail!("Failed to find compressed file in script")
    };
    end = end + start;

    let slice = &input[start..end];

    ensure!(
        slice.len() == 92744,
        "Compressed file size mismatch ({} bytes, expected 92744)",
        slice.len()
    );

    let mut decoded = [0_u8; 92744];

    Base64::decode(slice, &mut decoded)?;

    let decompressed = match decompress_to_vec_zlib(&decoded) {
        Err(msg) => bail!("Decompression error: {}", msg),
        Ok(b) => b,
    };

    ensure!(
        decompressed.len() == 317952,
        "Decompressed file size mismatch ({} bytes, expected 317952)",
        decompressed.len()
    );

    let mut exe_path = std::env::current_exe().unwrap();
    exe_path.pop();

    let file_path = get_optional_path(cli.output_path, exe_path).join("defarm.dll");

    decompressed.write_to_file(&file_path)?;

    println!("Successfully extracted file to: {}\n", file_path.display());

    Ok(())
}
