use std::ops::{Deref, DerefMut};

use memmap2::MmapMut;

/// Pixel buffer allocated straight from the OS; memory is returned on drop.
pub struct ImageBuf {
    map: Option<MmapMut>,
    pub width: u32,
    pub height: u32,
}

#[allow(unused)]
impl ImageBuf {
    /// Zero-initialized RGBA8 buffer.
    pub fn new(width: u32, height: u32) -> std::io::Result<Self> {
        let len = width as usize * height as usize * 4;
        let map = if len == 0 {
            None
        } else {
            Some(MmapMut::map_anon(len)?)
        };
        Ok(Self { map, width, height })
    }

    pub fn from_bytes(width: u32, height: u32, data: &[u8]) -> std::io::Result<Self> {
        let mut buf = Self::new(width, height)?;
        buf.copy_from_slice(data); // panics if sizes don't match
        Ok(buf)
    }
}

impl Deref for ImageBuf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.map.as_deref().unwrap_or(&[])
    }
}
impl DerefMut for ImageBuf {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.map.as_deref_mut().unwrap_or(&mut [])
    }
}
