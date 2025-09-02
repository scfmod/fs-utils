use std::path::PathBuf;

use anyhow::Result;
use argh::FromArgs;
use fs_lib::{EXECUTABLE_PATTERNS, buffer::BufferExtension};

use goblin::pe::{
    PE,
    options::{ParseMode, ParseOptions},
};

#[derive(FromArgs, PartialEq, Debug)]
/// Scan executable for valid patches
pub struct Cmd {
    /// path to executable
    #[argh(positional)]
    input: PathBuf,
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();
    let file_buffer = std::fs::read(&cli.input)?;

    let mut opts = ParseOptions::default();
    opts.with_parse_mode(ParseMode::Permissive);
    opts.parse_attribute_certificates = false;
    opts.parse_tls_data = false;
    opts.resolve_rva = true;

    let pe = PE::parse_with_opts(&file_buffer, &opts)?;

    println!("Executable: {}", cli.input.display());
    println!("Image Base: 0x{:X}", pe.image_base);

    // for section in &pe.sections {
    //     println!(
    //         "Section: {:8} RVA: 0x{:08X} VirtualSize: 0x{:08X} PointerToRawData: 0x{:08X}",
    //         section.name().unwrap_or(""),
    //         section.virtual_address,
    //         section.virtual_size,
    //         section.pointer_to_raw_data
    //     );
    // }

    for (platform, items) in EXECUTABLE_PATTERNS.iter() {
        println!("\nPlatform: {:?}", platform);

        for item in items.iter() {
            if let Some(offset) = file_buffer.find_bytes(&item.find) {
                println!("[+] {:?}", item.patch_type);
                println!("    offset: {}", offset);
                println!("    expect: {}", item.find.to_hex_string());
                println!("    replace: {}", item.replace.to_hex_string());
            } else {
                println!("[-] Could not locate pattern {:?}", item.patch_type);
                println!("    bytes: {}", item.find.to_hex_string());
            }
        }
    }

    Ok(())
}
