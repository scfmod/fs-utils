use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use anyhow::{Result, bail};

use crate::buffer::BufferExtension;

pub trait FileExtension {
    fn read_bytes(&mut self, offset: u64, length: usize) -> Result<Vec<u8>>;
    fn read_u8(&mut self, offset: u64) -> Result<u8>;
    fn read_u16(&mut self, offset: u64) -> Result<u16>;
    fn read_u32(&mut self, offset: u64) -> Result<u32>;
    fn read_string(&mut self, offset: u64, length: usize) -> Result<String>;
}

impl FileExtension for File {
    fn read_bytes(&mut self, offset: u64, length: usize) -> Result<Vec<u8>> {
        let mut buffer: Vec<u8> = vec![0; length];

        self.seek(SeekFrom::Start(offset as u64))?;
        let n = self.read(&mut buffer)?;

        if n < length {
            bail!(
                "Failed to read requested {} bytes at offset {}",
                offset,
                length
            )
        }

        Ok(buffer)
    }

    fn read_u8(&mut self, offset: u64) -> Result<u8> {
        let buffer = self.read_bytes(offset, 1)?;

        Ok(buffer[0])
    }

    fn read_u16(&mut self, offset: u64) -> Result<u16> {
        let buffer = self.read_bytes(offset, 2)?;

        Ok(buffer.read_u16(0))
    }

    fn read_u32(&mut self, offset: u64) -> Result<u32> {
        let buffer = self.read_bytes(offset, 4)?;

        Ok(buffer.read_u32(0))
    }

    fn read_string(&mut self, offset: u64, length: usize) -> Result<String> {
        let buffer = self.read_bytes(offset, length)?;

        buffer.read_string(0, length)
    }
}
