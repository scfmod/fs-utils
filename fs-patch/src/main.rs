use std::{
    path::{Path, PathBuf},
    vec,
};

use anyhow::{Result, bail};
use argh::FromArgs;
use fs_lib::{EXECUTABLE_PATTERNS, PatchType, Platform, buffer::BufferExtension};

#[allow(dead_code)]
struct Patch {
    pub offset: usize,
    pub find: Vec<u8>,
    pub replace: Vec<u8>,
    pub patch_type: PatchType,
    pub is_applied: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Patch executable
pub struct Cmd {
    /// path to executable
    #[argh(positional)]
    input: PathBuf,

    /// platform: steam, giants (default: steam)
    #[argh(option, default = "Platform::Steam")]
    platform: Platform,

    /// revert (applied) patches
    #[argh(switch, short = 'r')]
    revert: bool,

    /// scan executable for valid patches
    #[argh(switch, short = 's')]
    scan: bool,

    /// check executable for active/inactive patches
    #[argh(switch, short = 'c')]
    check: bool,
}

fn scan_executable<P: AsRef<Path>>(file: P) -> Result<()> {
    let file_buffer = std::fs::read(&file)?;

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

fn find_valid_patches(file_buffer: &Vec<u8>, platform: &Platform) -> Result<Vec<Patch>> {
    let Some(items) = EXECUTABLE_PATTERNS.get(&platform) else {
        bail!("No patch items found")
    };

    let mut result: Vec<Patch> = vec![];

    for item in items.iter() {
        if let Some(offset) = file_buffer.find_bytes(&item.find) {
            result.push(Patch {
                offset,
                find: item.find.clone(),
                replace: item.replace.clone(),
                patch_type: item.patch_type.clone(),
                is_applied: false,
            });
        } else if let Some(offset) = file_buffer.find_bytes(&item.replace) {
            result.push(Patch {
                offset,
                find: item.find.clone(),
                replace: item.replace.clone(),
                patch_type: item.patch_type.clone(),
                is_applied: true,
            });
        } else {
            bail!("No valid offsets found for patch {:?}", item.patch_type)
        }
    }

    Ok(result)
}

fn patch_executable<P: AsRef<Path>>(file: P, platform: &Platform) -> Result<(Vec<u8>, bool)> {
    let mut file_buffer = Vec::read_from_file(file)?;
    let patches = find_valid_patches(&file_buffer, platform)?;

    let mut is_modified = false;

    for patch in patches {
        if patch.is_applied {
            println!("[*] {:?} is already applied", patch.patch_type);
        } else {
            file_buffer.replace_bytes(&patch.replace, patch.offset);
            println!("[+] Applied {:?}", patch.patch_type);
            is_modified = true;
        }
    }

    Ok((file_buffer, is_modified))
}

fn patch_revert_executable<P: AsRef<Path>>(
    file: P,
    platform: &Platform,
) -> Result<(Vec<u8>, bool)> {
    let mut file_buffer = Vec::read_from_file(file)?;
    let patches = find_valid_patches(&file_buffer, platform)?;

    let mut is_modified = false;

    for patch in patches {
        if patch.is_applied {
            file_buffer.replace_bytes(&patch.find, patch.offset);
            println!("[-] Reverted {:?}", patch.patch_type);
            is_modified = true;
        } else {
            println!("[*] {:?} is already reverted", patch.patch_type);
        }
    }

    Ok((file_buffer, is_modified))
}

fn check_executable<P: AsRef<Path>>(file: P, platform: &Platform) -> Result<()> {
    let file_buffer = Vec::read_from_file(file)?;
    let patches = find_valid_patches(&file_buffer, platform)?;

    for patch in patches {
        if patch.is_applied {
            println!("[+] Patch is active {:?}", patch.patch_type);
        } else {
            println!("[-] Patch is not active {:?}", patch.patch_type);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    if cli.scan {
        return scan_executable(&cli.input);
    }

    if cli.check {
        return check_executable(&cli.input, &cli.platform);
    }

    let (file_buffer, is_modified) = match cli.revert {
        true => patch_revert_executable(&cli.input, &cli.platform)?,
        false => patch_executable(&cli.input, &cli.platform)?,
    };

    if is_modified {
        file_buffer.write_to_file(&cli.input)?;
        println!("\nExecutable updated: {}", &cli.input.display());
    } else {
        println!("\nNo changes required");
    }

    Ok(())
}
