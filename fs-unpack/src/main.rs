use std::{
    ffi::{CString, c_uint, c_void},
    fs::File,
    mem::transmute,
    path::PathBuf,
};

use winapi::um::libloaderapi::{GetProcAddress, LoadLibraryA};

use anyhow::{Result, bail, ensure};
use argh::FromArgs;
use fs_lib::{KEYS_LIST, buffer::BufferExtension, byte_array_hex_string, file::FileExtension};

#[derive(FromArgs, PartialEq, Debug)]
/// Extract .gar/.pdlc archive
pub struct Cmd {
    /// path to .gar/.pdlc archive
    #[argh(positional)]
    input: PathBuf,

    /// output path
    #[argh(positional)]
    output_path: PathBuf,
}

type DefarmFn = unsafe extern "C" fn(
    input: *const c_void,
    input_size: c_uint,
    output: *mut c_void,
    key1: c_uint,
    key2: c_uint,
    key3: c_uint,
    key4: c_uint,
);

type DecryptFn = dyn Fn(&Vec<u8>, &[u32; 4]) -> Result<Vec<u8>>;

fn find_keys(file: &mut File, decrypt_fn: &DecryptFn) -> Result<[u32; 4]> {
    let test_bytes = file.read_bytes(512, 8)?;

    for keys in KEYS_LIST.iter() {
        let result = decrypt_fn(&test_bytes, &keys)?;

        if result.read_u32(0) == 1 {
            return Ok(keys.clone());
        }
    }

    bail!("Failed to find valid decryption key")
}

fn load_decrypt_function() -> Result<impl Fn(&Vec<u8>, &[u32; 4]) -> Result<Vec<u8>>> {
    // Load the 32-bit DLL
    let dll_path = CString::new("defarm.dll").unwrap();
    let h_module = unsafe { LoadLibraryA(dll_path.as_ptr()) };
    if h_module == std::ptr::null_mut() {
        panic!("Failed to load 'defarm.dll'");
    }

    // Get the function address
    let func_name = CString::new("defarm").unwrap();
    let func_ptr = unsafe { GetProcAddress(h_module, func_name.as_ptr()) };
    if func_ptr.is_null() {
        panic!("Failed to find function entry pointer to 'defarm' in 'defarm.dll'");
    }

    let function: DefarmFn = unsafe { transmute(func_ptr) };

    Ok(move |input: &Vec<u8>, keys: &[u32; 4]| {
        let length = input.len();
        let mut output: Vec<u8> = vec![0; length];

        unsafe {
            (function)(
                input.as_ptr() as *const c_void,
                length as u32,
                output.as_mut_ptr() as *mut c_void,
                keys[0],
                keys[1],
                keys[2],
                keys[3],
            );
        }

        Ok(output)
    })
}

fn get_archive_header(
    file: &mut File,
    decrypt: &DecryptFn,
    keys: &[u32; 4],
) -> Result<(u16, u32, Vec<u8>)> {
    let version = file.read_u16(100)?;
    let mut file_count = file.read_u32(104)?;

    if version > 2 {
        file_count = file_count + file.read_u32(108)?;
    }

    let size = file_count.saturating_mul(512);
    let input = file.read_bytes(512, size as usize)?;
    let result = decrypt(&input, &keys)?;

    Ok((version, file_count, result))
}

fn main() -> Result<()> {
    let cli: Cmd = argh::from_env();

    let decrypt = load_decrypt_function()?;
    let mut file = File::open(cli.input)?;
    let keys = find_keys(&mut file, &decrypt)?;

    let signature = file.read_string(0, 20)?;
    let identifier = file.read_string(96, 4)?;
    let (version, file_count, header) = get_archive_header(&mut file, &decrypt, &keys)?;

    println!(
        "Using keys: {}
Signature: {}
Identifier: {}
Version: {}
File count: {}",
        byte_array_hex_string(&keys),
        signature,
        identifier,
        version,
        file_count
    );

    ensure!(file_count > 0, "No file entries found in archive");

    for i in 0..file_count {
        let offset = 512 * i as usize;

        let file_xsize = header.read_u32(offset + 8);
        let file_size = header.read_u32(offset + 16);
        let file_offset = header.read_u64(offset + 24);
        let file_name = header.read_cstring(offset + 40)?;

        let cipher_bytes = file.read_bytes(file_offset, file_xsize as usize)?;
        let deciphered_bytes = decrypt(&cipher_bytes, &keys)?;
        let file_bytes = deciphered_bytes[0..file_size as usize].to_vec();

        let file_path: PathBuf = cli.output_path.join(&file_name).components().collect();

        file_bytes.write_to_file(&file_path)?;

        println!("{}", file_path.display());
    }

    Ok(())
}
