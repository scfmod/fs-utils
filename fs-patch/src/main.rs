use std::{path::PathBuf, vec};

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
    #[argh(switch)]
    revert: bool,
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();
    let file_path: PathBuf = cli.input.components().collect();
    let mut file_buffer = std::fs::read(&file_path)?;

    let Some(items) = EXECUTABLE_PATTERNS.get(&cli.platform) else {
        bail!("No patch items found")
    };

    let mut patches: Vec<Patch> = vec![];

    for item in items.iter() {
        if let Some(offset) = file_buffer.find_bytes(&item.find) {
            patches.push(Patch {
                offset,
                find: item.find.clone(),
                replace: item.replace.clone(),
                patch_type: item.patch_type.clone(),
                is_applied: false,
            });
        } else if let Some(offset) = file_buffer.find_bytes(&item.replace) {
            patches.push(Patch {
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

    let mut is_modified = false;

    for patch in patches {
        if !cli.revert {
            if patch.is_applied {
                println!("[*] {:?} is already applied", patch.patch_type);
            } else {
                file_buffer.replace_bytes(&patch.replace, patch.offset);
                println!("[+] Applied {:?}", patch.patch_type);
                is_modified = true;
            }
        } else {
            if patch.is_applied {
                file_buffer.replace_bytes(&patch.find, patch.offset);
                println!("[-] Reverted {:?}", patch.patch_type);
                is_modified = true;
            } else {
                println!("[*] {:?} is already reverted", patch.patch_type);
            }
        }
    }

    if is_modified {
        file_buffer.write_to_file(&file_path)?;
        println!("\nExecutable updated: {}", file_path.display());
    } else {
        println!("\nNo changes required");
    }

    Ok(())
}
