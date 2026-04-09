//! Dual-stream resource reader (system @ 0x50000000, graphics @ 0x60000000).

const SYSTEM_BASE: u64 = 0x5000_0000;
const GRAPHICS_BASE: u64 = 0x6000_0000;

#[derive(Debug)]
pub struct ResourceReader {
    system: Vec<u8>,
    graphics: Vec<u8>,
    pos: u64,
}

impl ResourceReader {
    pub fn new(system: Vec<u8>, graphics: Vec<u8>) -> Self {
        Self {
            system,
            graphics,
            pos: SYSTEM_BASE,
        }
    }

    pub fn set_pos(&mut self, p: u64) {
        self.pos = p;
    }

    pub fn pos(&self) -> u64 {
        self.pos
    }

    fn resolve(&self, addr: u64) -> anyhow::Result<(&[u8], usize)> {
        let a = addr as u32 as u64;
        if (a & SYSTEM_BASE) == SYSTEM_BASE {
            let off = (a & !SYSTEM_BASE) as usize;
            return Ok((&self.system, off));
        }
        if (a & GRAPHICS_BASE) == GRAPHICS_BASE {
            let off = (a & !GRAPHICS_BASE) as usize;
            return Ok((&self.graphics, off));
        }
        anyhow::bail!("Illegal resource address: 0x{:x}", addr);
    }

    pub fn read_u8(&mut self) -> anyhow::Result<u8> {
        let (buf, off) = self.resolve(self.pos)?;
        if off >= buf.len() {
            anyhow::bail!("read_u8 past end");
        }
        let v = buf[off];
        self.pos += 1;
        Ok(v)
    }

    pub fn read_u16(&mut self) -> anyhow::Result<u16> {
        let (buf, off) = self.resolve(self.pos)?;
        if off + 2 > buf.len() {
            anyhow::bail!("read_u16 past end");
        }
        let v = u16::from_le_bytes(buf[off..off + 2].try_into().unwrap());
        self.pos += 2;
        Ok(v)
    }

    pub fn read_u32(&mut self) -> anyhow::Result<u32> {
        let (buf, off) = self.resolve(self.pos)?;
        if off + 4 > buf.len() {
            anyhow::bail!("read_u32 past end");
        }
        let v = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    pub fn read_u64(&mut self) -> anyhow::Result<u64> {
        let (buf, off) = self.resolve(self.pos)?;
        if off + 8 > buf.len() {
            anyhow::bail!("read_u64 past end");
        }
        let v = u64::from_le_bytes(buf[off..off + 8].try_into().unwrap());
        self.pos += 8;
        Ok(v)
    }

    #[allow(dead_code)]
    pub fn read_f32(&mut self) -> anyhow::Result<f32> {
        let (buf, off) = self.resolve(self.pos)?;
        if off + 4 > buf.len() {
            anyhow::bail!("read_f32 past end");
        }
        let v = f32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    /// Null-terminated UTF-8 at virtual address (does not move `pos`).
    pub fn read_cstring_at(&self, vaddr: u64) -> Option<String> {
        if vaddr == 0 {
            return None;
        }
        let (buf, off) = self.resolve(vaddr).ok()?;
        let mut end = off;
        while end < buf.len() && buf[end] != 0 {
            end += 1;
        }
        std::str::from_utf8(&buf[off..end]).ok().map(|s| s.to_string())
    }

    pub fn at<T>(&mut self, vaddr: u64, f: impl FnOnce(&mut Self) -> anyhow::Result<T>) -> anyhow::Result<T> {
        if vaddr == 0 {
            anyhow::bail!("at(0)");
        }
        let back = self.pos;
        self.pos = vaddr;
        let r = f(self);
        self.pos = back;
        r
    }
}
