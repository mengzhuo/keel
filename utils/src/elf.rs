#![allow(dead_code)]

//! Minimal 64-bit little-endian ELF reader used by `ldd` and `ldconfig`.
//!
//! This intentionally only implements the parts needed by the two commands:
//! program headers, PT_INTERP, and the dynamic tags for DT_NEEDED, RPATH,
//! RUNPATH and SONAME.  It is not a general-purpose ELF library.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_STRTAB: u64 = 5;
const DT_STRSZ: u64 = 10;
const DT_SONAME: u64 = 14;
const DT_RPATH: u64 = 15;
const DT_RUNPATH: u64 = 29;

#[derive(Clone, Copy, Debug)]
pub struct Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[derive(Debug, Clone)]
pub struct ElfFile {
    pub path: PathBuf,
    pub e_type: u16,
    pub e_machine: u16,
    pub phdrs: Vec<Phdr>,
    pub interp: Option<String>,
    pub needed: Vec<String>,
    pub soname: Option<String>,
    pub rpath: Option<String>,
    pub runpath: Option<String>,
}

impl ElfFile {
    pub fn open(path: &Path) -> io::Result<Self> {
        let data = fs::read(path)?;
        Self::parse(path, &data)
    }

    pub fn parse(path: &Path, data: &[u8]) -> io::Result<Self> {
        if data.len() < 64 || &data[0..4] != b"\x7fELF" {
            return Err(invalid("not an ELF file"));
        }
        if data[4] != 2 {
            return Err(invalid("only 64-bit ELF files are supported"));
        }
        if data[5] != 1 {
            return Err(invalid("only little-endian ELF files are supported"));
        }

        let e_type = u16_at(data, 16)?;
        let e_machine = u16_at(data, 18)?;
        let e_phoff = u64_at(data, 32)? as usize;
        let e_phentsize = u16_at(data, 54)? as usize;
        let e_phnum = u16_at(data, 56)? as usize;

        if e_phentsize < 56 {
            return Err(invalid("invalid program header size"));
        }
        let table_len = e_phentsize
            .checked_mul(e_phnum)
            .ok_or_else(|| invalid("program header table overflow"))?;
        if e_phoff
            .checked_add(table_len)
            .map(|end| end > data.len())
            .unwrap_or(true)
        {
            return Err(invalid("program header table outside file"));
        }

        let mut phdrs = Vec::with_capacity(e_phnum);
        for i in 0..e_phnum {
            let off = e_phoff + i * e_phentsize;
            phdrs.push(Phdr {
                p_type: u32_at(data, off)?,
                p_flags: u32_at(data, off + 4)?,
                p_offset: u64_at(data, off + 8)?,
                p_vaddr: u64_at(data, off + 16)?,
                p_filesz: u64_at(data, off + 32)?,
                p_memsz: u64_at(data, off + 40)?,
                p_align: u64_at(data, off + 48)?,
            });
        }

        let interp = phdrs.iter().find(|p| p.p_type == PT_INTERP).and_then(|p| {
            let start = p.p_offset as usize;
            let end = start.checked_add(p.p_filesz as usize)?;
            if end <= data.len() {
                Some(cstr(&data[start..end]).to_string())
            } else {
                None
            }
        });

        let mut needed = Vec::new();
        let mut soname = None;
        let mut rpath = None;
        let mut runpath = None;

        if let Some(dynamic) = phdrs.iter().find(|p| p.p_type == PT_DYNAMIC) {
            let start = dynamic.p_offset as usize;
            let size = dynamic.p_filesz as usize;
            let end = start
                .checked_add(size)
                .ok_or_else(|| invalid("dynamic section overflow"))?;
            if end > data.len() {
                return Err(invalid("dynamic section outside file"));
            }

            let mut strtab_vaddr = 0u64;
            let mut strsz = 0usize;
            let mut needed_offsets = Vec::new();
            let mut soname_off = None;
            let mut rpath_off = None;
            let mut runpath_off = None;

            let mut pos = start;
            while pos + 16 <= end {
                let tag = u64_at(data, pos)?;
                let val = u64_at(data, pos + 8)?;
                pos += 16;
                if tag == DT_NULL {
                    break;
                }
                match tag {
                    DT_NEEDED => needed_offsets.push(val as usize),
                    DT_STRTAB => strtab_vaddr = val,
                    DT_STRSZ => strsz = val as usize,
                    DT_SONAME => soname_off = Some(val as usize),
                    DT_RPATH => rpath_off = Some(val as usize),
                    DT_RUNPATH => runpath_off = Some(val as usize),
                    _ => {}
                }
            }

            if strtab_vaddr != 0 && strsz != 0 {
                if let Some(strtab_off) = vaddr_to_offset(&phdrs, strtab_vaddr) {
                    let strtab_off = strtab_off as usize;
                    let strtab_end = strtab_off.saturating_add(strsz).min(data.len());
                    let strtab = &data[strtab_off..strtab_end];
                    for off in needed_offsets {
                        if off < strtab.len() {
                            needed.push(cstr(&strtab[off..]).to_string());
                        }
                    }
                    if let Some(off) = soname_off {
                        if off < strtab.len() {
                            soname = Some(cstr(&strtab[off..]).to_string());
                        }
                    }
                    if let Some(off) = rpath_off {
                        if off < strtab.len() {
                            rpath = Some(cstr(&strtab[off..]).to_string());
                        }
                    }
                    if let Some(off) = runpath_off {
                        if off < strtab.len() {
                            runpath = Some(cstr(&strtab[off..]).to_string());
                        }
                    }
                }
            }
        }

        Ok(ElfFile {
            path: path.to_path_buf(),
            e_type,
            e_machine,
            phdrs,
            interp,
            needed,
            soname,
            rpath,
            runpath,
        })
    }

    pub fn is_dynamic(&self) -> bool {
        self.interp.is_some() || !self.needed.is_empty()
    }
}

pub fn vaddr_to_offset(phdrs: &[Phdr], vaddr: u64) -> Option<u64> {
    for p in phdrs {
        if p.p_type != PT_LOAD {
            continue;
        }
        let end = p.p_vaddr.checked_add(p.p_filesz)?;
        if vaddr >= p.p_vaddr && vaddr < end {
            return Some(p.p_offset + (vaddr - p.p_vaddr));
        }
    }
    None
}

fn cstr(bytes: &[u8]) -> &str {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..end]).unwrap_or("")
}

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

fn u16_at(data: &[u8], off: usize) -> io::Result<u16> {
    let bytes = data
        .get(off..off + 2)
        .ok_or_else(|| invalid("truncated ELF file"))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn u32_at(data: &[u8], off: usize) -> io::Result<u32> {
    let bytes = data
        .get(off..off + 4)
        .ok_or_else(|| invalid("truncated ELF file"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u64_at(data: &[u8], off: usize) -> io::Result<u64> {
    let bytes = data
        .get(off..off + 8)
        .ok_or_else(|| invalid("truncated ELF file"))?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}
