use memmap::{MmapMut, MmapOptions};
use std::fs::File;
use std::io;
use std::io::Write;

pub enum DataStore {
    File(MmapMut, File),
    Anon(MmapMut),
}

impl DataStore {
    pub fn file(file: File) -> io::Result<Self> {
        let mmap = unsafe { MmapOptions::new().map_copy(&file)? };
        Ok(DataStore::File(mmap, file))
    }

    pub fn anon(n_bytes: usize) -> io::Result<Self> {
        let mmap = MmapOptions::new().len(n_bytes).map_anon()?;
        Ok(DataStore::Anon(mmap))
    }

    pub fn data(&self) -> &[u8] {
        match self {
            DataStore::File(mmap, _) => mmap,
            DataStore::Anon(mmap) => mmap,
        }
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        match self {
            DataStore::File(mmap, _) => mmap,
            DataStore::Anon(mmap) => mmap,
        }
    }

    pub fn write(&mut self) -> io::Result<()> {
        match self {
            DataStore::File(mmap, file) => {
                file.write_all(mmap)?;
                file.flush()?;
            }
            _ => {}
        }
        Ok(())
    }
}
