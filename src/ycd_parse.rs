//! GTA V .ycd (clip dictionary) parsing — mirrors CodeWalker ClipDictionary layout.

use std::collections::{BTreeSet, HashSet};
use std::io::Read;

use flate2::read::{DeflateDecoder, ZlibDecoder};

use crate::resource_reader::ResourceReader;

const RSC7_MAGIC: u32 = 0x3743_5352;

const CLIP_TYPE_ANIMATION_LIST: u32 = 2;

pub fn resource_flags_size(flags: u32) -> u64 {
    let s0 = ((flags >> 27) & 0x1) << 0;
    let s1 = ((flags >> 26) & 0x1) << 1;
    let s2 = ((flags >> 25) & 0x1) << 2;
    let s3 = ((flags >> 24) & 0x1) << 3;
    let s4 = ((flags >> 17) & 0x7f) << 4;
    let s5 = ((flags >> 11) & 0x3f) << 5;
    let s6 = ((flags >> 7) & 0xf) << 6;
    let s7 = ((flags >> 5) & 0x3) << 7;
    let s8 = ((flags >> 4) & 0x1) << 8;
    let ss = flags & 0xf;
    let base_size = 0x200u64 << ss;
    let sum: u64 = (s0 + s1 + s2 + s3 + s4 + s5 + s6 + s7 + s8) as u64;
    base_size * sum
}

// Matches CodeWalker: when ss > 0, size is adjusted without updating blockcount.
#[allow(unused_assignments)]
pub fn get_flags_from_size(size: i32, version: u32) -> u32 {
    let origsize = size;
    let mut size = size;
    let mut remainder = size & 0x1ff;
    let mut blocksize = 0x200i32;
    if remainder != 0 {
        size = size - remainder + blocksize;
    }
    let mut blockcount = (size as u32) >> 9;
    let mut ss = 0u32;
    while blockcount > 1024 {
        ss += 1;
        blockcount >>= 1;
    }
    if ss > 0 {
        size = origsize;
        blocksize = blocksize << ss;
        remainder = size & blocksize;
        if remainder != 0 {
            size = size - remainder + blocksize;
        }
    }
    let s0 = (blockcount >> 0) & 0x1;
    let s1 = (blockcount >> 1) & 0x1;
    let s2 = (blockcount >> 2) & 0x1;
    let s3 = (blockcount >> 3) & 0x1;
    let s4 = (blockcount >> 4) & 0x7f;
    let mut f = 0u32;
    f |= (version & 0xf) << 28;
    f |= (s0 & 0x1) << 27;
    f |= (s1 & 0x1) << 26;
    f |= (s2 & 0x1) << 25;
    f |= (s3 & 0x1) << 24;
    f |= (s4 & 0x7f) << 17;
    f |= ss & 0xf;
    f
}

fn try_zlib_inflate(body: &[u8]) -> Option<Vec<u8>> {
    let mut dec = ZlibDecoder::new(body);
    let mut out = Vec::new();
    if dec.read_to_end(&mut out).is_ok() && !out.is_empty() {
        return Some(out);
    }
    let mut dec2 = DeflateDecoder::new(body);
    let mut out2 = Vec::new();
    if dec2.read_to_end(&mut out2).is_ok() && !out2.is_empty() {
        return Some(out2);
    }
    None
}

pub struct Decompressed {
    pub system: Vec<u8>,
    pub graphics: Vec<u8>,
}

pub fn decompress_ycd_buffer(input: &[u8]) -> anyhow::Result<Decompressed> {
    if input.len() < 4 {
        anyhow::bail!("buffer too small");
    }
    let magic = u32::from_le_bytes(input[0..4].try_into().unwrap());
    let mut body = input;
    let mut system_flags = 0u32;
    let mut graphics_flags = 0u32;

    if magic == RSC7_MAGIC {
        if input.len() < 16 {
            anyhow::bail!("RSC7 header truncated");
        }
        system_flags = u32::from_le_bytes(input[8..12].try_into().unwrap());
        graphics_flags = u32::from_le_bytes(input[12..16].try_into().unwrap());
        body = &input[16..];
    }

    let data = try_zlib_inflate(body).unwrap_or_else(|| body.to_vec());

    if magic != RSC7_MAGIC {
        system_flags = get_flags_from_size(data.len() as i32, 0);
        graphics_flags = get_flags_from_size(0, 46);
    }

    let mut sys_size = resource_flags_size(system_flags) as usize;
    let mut gfx_size = resource_flags_size(graphics_flags) as usize;

    if sys_size + gfx_size > data.len() {
        system_flags = get_flags_from_size(data.len() as i32, 0);
        graphics_flags = get_flags_from_size(0, 6);
        sys_size = resource_flags_size(system_flags) as usize;
        gfx_size = resource_flags_size(graphics_flags) as usize;
    }

    if sys_size + gfx_size > data.len() {
        sys_size = data.len();
        gfx_size = 0;
    }

    let sys = data[..sys_size].to_vec();
    let gfx = if gfx_size > 0 {
        data[sys_size..sys_size + gfx_size].to_vec()
    } else {
        Vec::new()
    };

    Ok(Decompressed {
        system: sys,
        graphics: gfx,
    })
}

/// CodeWalker ClipBase ShortName
pub fn to_short_name(name: Option<&str>) -> Option<String> {
    let name = name?;
    let mut n = name.replace('\\', "/");
    if let Some(sl) = n.rfind('/') {
        if sl < n.len() - 1 {
            n = n[sl + 1..].to_string();
        }
    }
    if let Some(d) = n.find('.') {
        if d > 0 && d < n.len() {
            n = n[..d].to_string();
        }
    }
    Some(n.to_lowercase())
}

fn collect_clip_names(r: &mut ResourceReader, clip_ptr: u64, out: &mut BTreeSet<String>) -> anyhow::Result<()> {
    if clip_ptr == 0 {
        return Ok(());
    }
    r.at(clip_ptr, |rr| {
        rr.read_u32()?;
        rr.read_u32()?;
        rr.read_u32()?;
        rr.read_u32()?;
        let ty = rr.read_u32()?;
        rr.read_u32()?;
        let name_ptr = rr.read_u64()?;
        rr.read_u16()?;
        rr.read_u16()?;
        rr.read_u32()?;
        rr.read_u64()?;
        rr.read_u32()?;
        rr.read_u32()?;
        rr.read_u64()?;
        rr.read_u64()?;
        rr.read_u32()?;
        rr.read_u32()?;

        let full = rr.read_cstring_at(name_ptr);
        if let Some(sn) = to_short_name(full.as_deref()) {
            out.insert(sn);
        }

        if ty == CLIP_TYPE_ANIMATION_LIST {
            let _anim_list_ptr = rr.read_u64()?;
            let _count = rr.read_u16()?;
            rr.read_u16()?;
        }
        Ok(())
    })
}

fn walk_clip_map_chain(
    r: &mut ResourceReader,
    entry_ptr: u64,
    names: &mut BTreeSet<String>,
) -> anyhow::Result<()> {
    let mut p = entry_ptr;
    let mut seen = HashSet::new();
    while p != 0 && !seen.contains(&p) {
        seen.insert(p);
        let next = r.at(p, |er| {
            er.read_u32()?;
            er.read_u32()?;
            let clip_ptr = er.read_u64()?;
            let next_ptr = er.read_u64()?;
            er.read_u32()?;
            er.read_u32()?;
            collect_clip_names(er, clip_ptr, names)?;
            Ok::<u64, anyhow::Error>(next_ptr)
        })?;
        p = next;
    }
    Ok(())
}

pub struct ParseResult {
    pub animations: Vec<String>,
    pub error: Option<String>,
}

pub fn parse_ycd_animations(file_bytes: &[u8]) -> ParseResult {
    let mut names = BTreeSet::new();
    let dc = match decompress_ycd_buffer(file_bytes) {
        Ok(d) => d,
        Err(e) => {
            return ParseResult {
                animations: Vec::new(),
                error: Some(e.to_string()),
            };
        }
    };

    let mut reader = ResourceReader::new(dc.system, dc.graphics);
    reader.set_pos(0x5000_0000);

    let res = (|| -> anyhow::Result<()> {
        reader.read_u32()?;
        reader.read_u32()?;
        let file_pages_info_ptr = reader.read_u64()?;
        if file_pages_info_ptr != 0 {
            reader.at(file_pages_info_ptr, |pr| {
                pr.read_u32()?;
                pr.read_u32()?;
                let sys_pages = pr.read_u8()? as u64;
                let gfx_pages = pr.read_u8()? as u64;
                pr.read_u16()?;
                pr.read_u32()?;
                pr.set_pos(pr.pos() + 8 * (sys_pages + gfx_pages));
                Ok(())
            })?;
        }

        reader.read_u32()?;
        reader.read_u32()?;
        let _animations_map_ptr = reader.read_u64()?;
        reader.read_u32()?;
        reader.read_u32()?;
        let clips_ptr = reader.read_u64()?;
        let clips_map_capacity = reader.read_u16()?;
        reader.read_u16()?;
        reader.read_u32()?;
        reader.read_u32()?;
        reader.read_u32()?;

        if clips_ptr == 0 || clips_map_capacity == 0 {
            return Ok(());
        }

        reader.at(clips_ptr, |cr| {
            for _ in 0..clips_map_capacity {
                let ptr = cr.read_u64()?;
                if ptr != 0 {
                    walk_clip_map_chain(cr, ptr, &mut names)?;
                }
            }
            Ok(())
        })?;
        Ok(())
    })();

    match res {
        Ok(()) => ParseResult {
            animations: names.into_iter().collect(),
            error: None,
        },
        Err(e) => ParseResult {
            animations: Vec::new(),
            error: Some(e.to_string()),
        },
    }
}
