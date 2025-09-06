use std::{
    collections::HashMap,
    env,
    fmt::UpperHex,
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Result, bail};

pub mod buffer;
pub mod cmd;
pub mod file;
pub mod path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PatchType {
    ArchiveCheck,
    CompileError,
    CorruptFile,
    RenameArchive,
}

#[allow(dead_code)]
pub struct PatternItem {
    pub find: Vec<u8>,
    pub replace: Vec<u8>,
    pub patch_type: PatchType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    Steam,
    Giants,
}

impl FromStr for Platform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "steam" => Ok(Platform::Steam),
            "giants" => Ok(Platform::Giants),
            _ => Err(format!("Unknown platform: {}", s)),
        }
    }
}

pub struct ByteshiftTable {
    pub bytes: Vec<u8>,
    pub offset: usize,
    pub mask: usize,
}

pub fn byte_array_hex_string<T: UpperHex>(v: &[T]) -> String {
    v.iter()
        .map(|k| format!("0x{:02X}", k))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn get_optional_path<P: AsRef<Path>>(path: Option<P>, fallback: P) -> PathBuf {
    match path {
        Some(p) => p.as_ref().to_path_buf(),
        _ => fallback.as_ref().to_path_buf(),
    }
}

pub fn list_files<P: AsRef<Path>>(path: P, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(path)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        } else if recursive && path.is_dir() {
            files.extend(list_files(path, true)?);
        }
    }
    Ok(files)
}

pub fn list_files_with_extension<P: AsRef<Path>>(
    path: P,
    extension: &str,
    recursive: bool,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let entries = fs::read_dir(path)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(file_ext) = path.extension() {
                if *file_ext == *extension {
                    files.push(path.components().collect());
                }
            }
        } else if recursive && path.is_dir() {
            files.extend(list_files_with_extension(path, extension, true)?);
        }
    }
    Ok(files)
}

pub fn try_get_command_path(file: &str) -> Result<PathBuf> {
    let mut dir = std::env::current_exe()?;
    dir.pop();

    let file_path = dir.join(file);

    if file_path.exists() {
        return Ok(file_path);
    }

    let file_path = dir.join("bin").join(file);

    if file_path.exists() {
        return Ok(file_path);
    }

    let file_path = env::current_dir()?.join(file);

    if file_path.exists() {
        return Ok(file_path);
    }

    let file_path = env::current_dir()?.join("bin").join(file);

    if file_path.exists() {
        return Ok(file_path);
    }

    bail!("Failed to locate '{}'", file)
}

lazy_static::lazy_static! {
    pub static ref KEYS_LIST: Vec<[u32;4]> = vec![
        [0x022DBB1E, 0x22EC2A94, 0x1B0C37E7, 0x2501A594],
        [0x23F0EA64, 0x317FAC94, 0x1B0C37E7, 0x2501A594],
        [0x30D0D6B6, 0x14B281C4, 0x2F28AC14, 0x29F53CB9],
    ];

    pub static ref EXECUTABLE_PATTERNS: HashMap<Platform, Vec<PatternItem>> = {
        let mut items = HashMap::new();

        items.insert(Platform::Steam, vec![
            PatternItem {
                patch_type: PatchType::ArchiveCheck,
                find: vec![0x74, 0x16, 0x48, 0x8B, 0x41, 0x18, 0x48],
                replace: vec![0x75, 0x16, 0x48, 0x8B, 0x41, 0x18, 0x48]
            },
            PatternItem {
                patch_type: PatchType::CompileError,
                find: vec![0x75, 0x27, 0x84, 0xC0, 0x74, 0x23, 0x48],
                replace: vec![0x71, 0x27, 0x84, 0xC0, 0x74, 0x23, 0x48]
            },
            PatternItem {
                patch_type: PatchType::CorruptFile,
                find: vec![0x0F, 0x84, 0xD4, 0x16, 0x00, 0x00, 0x41, 0x8B, 0xDE, 0x48],
                replace: vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x41, 0x8B, 0xDE, 0x48]
            },
        ]);

        items.insert(Platform::Giants, vec![
            PatternItem {
                patch_type: PatchType::ArchiveCheck,
                find: vec![0x74, 0x16, 0x48, 0x8B, 0x84, 0x24, 0xB8],
                replace: vec![0x75, 0x16, 0x48, 0x8B, 0x84, 0x24, 0xB8]
            },
            PatternItem {
                patch_type: PatchType::CompileError,
                find: vec![0x75, 0x27, 0x84, 0xC0, 0x74, 0x23, 0x48],
                replace: vec![0x71, 0x27, 0x84, 0xC0, 0x74, 0x23, 0x48]
            },
            // Version 1.10 and earlier
            // PatternItem {
            //     patch_type: PatchType::CorruptFile,
            //     find: vec![0x0F, 0x84, 0x79, 0x12, 0x00, 0x00, 0x41, 0x8B, 0xDE, 0x48],
            //     replace: vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x41, 0x8B, 0xDE, 0x48]
            // },

            // Version 1.11
            PatternItem {
                patch_type: PatchType::CorruptFile,
                find: vec![0x0F, 0x84, 0x7C, 0x12, 0x00, 0x00, 0x41, 0x8B, 0xDE, 0x48],
                replace: vec![0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x41, 0x8B, 0xDE, 0x48]
            },
        ]);

        items
    };

    pub static ref LUAJIT_DECODE_TABLES: HashMap<u8, ByteshiftTable> = {
        let mut entries = HashMap::new();

        entries.insert(3_u8, ByteshiftTable {
            bytes: vec![0x14, 0x0B, 0x09, 0x02, 0x08, 0x03, 0x03, 0x03],
            offset: 4,
            mask: 0x07,
        });

        entries.insert(4_u8, ByteshiftTable {
            bytes: vec![0x06, 0x10, 0x0C, 0x02, 0x09, 0x03, 0x04, 0x04, 0x09, 0x05, 0x04, 0x02, 0x05, 0x08, 0x09, 0x15],
            offset: 4,
            mask: 0x0f,
        });

        entries
    };

    pub static ref LUAU_DECODE_TABLES: HashMap<(u8, bool), ByteshiftTable> = {
        let mut entries = HashMap::new();

        // Table for dataS/scripts/
        entries.insert((3, false), ByteshiftTable {
            bytes: vec![0x02, 0x13, 0x0A, 0x08, 0x01, 0x07, 0x02, 0x02],
            offset: 0,
            mask: 0x07,
        });
        entries.insert((6, false), ByteshiftTable {
            bytes: vec![0x02, 0x13, 0x0A, 0x08, 0x01, 0x07, 0x02, 0x02],
            offset: 0,
            mask: 0x07,
        });


        // Table for DLC scripts
        entries.insert((3, true), ByteshiftTable {
            bytes: vec![
                0x14, 0x05, 0x0F, 0x0B, 0x01, 0x08, 0x02, 0x03,
                0x03, 0x08, 0x04, 0x03, 0x01, 0x04, 0x07, 0x08
            ],
            offset: 0,
            mask: 0x0f,
        });
        entries.insert((6, true), ByteshiftTable {
            bytes: vec![
                0x14, 0x05, 0x0F, 0x0B, 0x01, 0x08, 0x02, 0x03,
                0x03, 0x08, 0x04, 0x03, 0x01, 0x04, 0x07, 0x08
            ],
            offset: 0,
            mask: 0x0f,
        });

        entries
    };
}
