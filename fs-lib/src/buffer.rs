use std::{fs::create_dir_all, path::Path};

use anyhow::{Result, bail};

use crate::byte_array_hex_string;

#[allow(unused)]
pub trait BufferExtension {
    fn from_string(str: &str) -> Vec<u8>;
    fn to_string(&self) -> Result<String>;
    fn to_hex_string(&self) -> String;

    fn read_u8(&self, offset: usize) -> u8;
    fn read_u16(&self, offset: usize) -> u16;
    fn read_u32(&self, offset: usize) -> u32;
    fn read_u64(&self, offset: usize) -> u64;
    fn read_string(&self, offset: usize, length: usize) -> Result<String>;
    fn read_cstring(&self, offset: usize) -> Result<String>;

    fn read_from_file<P: AsRef<Path>>(file: P) -> Result<Vec<u8>>;
    fn write_to_file<P: AsRef<Path>>(&self, file: P) -> Result<()>;

    fn find_bytes(&self, bytes: &Vec<u8>) -> Option<usize>;
    fn find_bytes_from(&self, bytes: &Vec<u8>, offset: usize) -> Option<usize>;
    fn replace_bytes(&mut self, bytes: &Vec<u8>, offset: usize);
    fn find_and_replace(&mut self, find: &Vec<u8>, replace: &Vec<u8>, offset: usize);
    fn find_and_replace_string(&mut self, find: &str, replace: &str, offset: usize);

    fn shift_bytes(&mut self, table: &Vec<u8>, offset: usize, mask: usize);
    fn shift_bytes_reversed(&mut self, bytes: &Vec<u8>, offset: usize, mask: usize);
}

impl BufferExtension for Vec<u8> {
    fn from_string(str: &str) -> Vec<u8> {
        String::from(str).into_bytes()
    }

    fn to_string(&self) -> Result<String> {
        let str = std::str::from_utf8(self)?;

        Ok(String::from(str))
    }

    fn to_hex_string(&self) -> String {
        byte_array_hex_string(self)
    }

    fn read_u8(&self, offset: usize) -> u8 {
        self[offset]
    }

    fn read_u16(&self, offset: usize) -> u16 {
        u16::from_le_bytes([self[offset], self[offset + 1]])
    }

    fn read_u32(&self, offset: usize) -> u32 {
        u32::from_le_bytes([
            self[offset],
            self[offset + 1],
            self[offset + 2],
            self[offset + 3],
        ])
    }

    fn read_u64(&self, offset: usize) -> u64 {
        u64::from_le_bytes([
            self[offset],
            self[offset + 1],
            self[offset + 2],
            self[offset + 3],
            self[offset + 4],
            self[offset + 5],
            self[offset + 6],
            self[offset + 7],
        ])
    }

    fn read_string(&self, offset: usize, length: usize) -> Result<String> {
        let bytes = &self[offset..offset + length];

        Ok(String::from(std::str::from_utf8(&bytes)?))
    }

    fn read_cstring(&self, offset: usize) -> Result<String> {
        let slice = &self[offset..];
        let Some(end) = slice.iter().position(|&b| b == 0) else {
            bail!("Failed to read cstring at offset {}", offset)
        };

        Ok(String::from(std::str::from_utf8(
            &self[offset..offset + end],
        )?))
    }

    fn read_from_file<P: AsRef<Path>>(file: P) -> Result<Vec<u8>> {
        Ok(std::fs::read(file)?)
    }

    fn write_to_file<P: AsRef<Path>>(&self, file: P) -> Result<()> {
        let mut path = file.as_ref().to_path_buf();

        path.pop();

        if !path.exists() {
            create_dir_all(path)?;
        }

        Ok(std::fs::write(file, &self)?)
    }

    fn find_bytes(&self, bytes: &Vec<u8>) -> Option<usize> {
        self.windows(bytes.len()).position(|window| window == bytes)
    }

    fn find_bytes_from(&self, bytes: &Vec<u8>, offset: usize) -> Option<usize> {
        self[offset..].to_vec().find_bytes(&bytes)
    }

    fn replace_bytes(&mut self, bytes: &Vec<u8>, offset: usize) {
        for i in 0..bytes.len() {
            self[i + offset] = bytes[i];
        }
    }

    fn shift_bytes(&mut self, table: &Vec<u8>, offset: usize, mask: usize) {
        for i in offset..self.len() {
            let value = self[i];
            let bytecode = table[i & mask];

            self[i] = value.wrapping_add(bytecode).wrapping_add(i as u8);
        }
    }

    fn shift_bytes_reversed(&mut self, bytes: &Vec<u8>, offset: usize, mask: usize) {
        for i in offset..self.len() {
            let value = self[i];
            let bytecode = bytes[i & mask];

            self[i] = value.wrapping_sub(bytecode).wrapping_sub(i as u8);
        }
    }

    fn find_and_replace(&mut self, find: &Vec<u8>, replace: &Vec<u8>, offset: usize) {
        let mut i = offset;

        while i <= self.len() - find.len() {
            if self[i..i + find.len()] == *find {
                self.splice(i..i + find.len(), replace.clone());
                i += replace.len();
            } else {
                i += 1;
            }
        }
    }

    fn find_and_replace_string(&mut self, find: &str, replace: &str, offset: usize) {
        self.find_and_replace(&Vec::from_string(find), &Vec::from_string(replace), offset);
    }
}
