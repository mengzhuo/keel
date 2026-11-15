// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

#![allow(dead_code)]

// ELF parsing, segment mapping and relocation for the standalone loader.
// Architecture and raw syscall details live in arch.rs.

use crate::arch::sys_mmap;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Ehdr {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

pub const ET_DYN: u16 = 3;
pub const ET_EXEC: u16 = 2;
pub const EM_X86_64: u16 = 62;
pub const EM_AARCH64: u16 = 183;
pub const EM_RISCV: u16 = 243;

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_INTERP: u32 = 3;
pub const PT_PHDR: u32 = 6;

pub const PF_R: u32 = 4;
pub const PF_W: u32 = 2;
pub const PF_X: u32 = 1;

pub const DT_NULL: u64 = 0;
pub const DT_RELA: u64 = 7;
pub const DT_RELAENT: u64 = 9;
pub const DT_RELASZ: u64 = 8;
pub const DT_JMPREL: u64 = 23;
pub const DT_PLTRELSZ: u64 = 2;

pub const R_X86_64_RELATIVE: u64 = 8;
pub const R_AARCH64_RELATIVE: u64 = 1027;
pub const R_RISCV_RELATIVE: u64 = 3;

#[cfg(target_arch = "x86_64")]
const R_RELATIVE: u64 = R_X86_64_RELATIVE;
#[cfg(target_arch = "aarch64")]
const R_RELATIVE: u64 = R_AARCH64_RELATIVE;
#[cfg(target_arch = "riscv64")]
const R_RELATIVE: u64 = R_RISCV_RELATIVE;

pub const AT_NULL: u64 = 0;
pub const AT_PHDR: u64 = 3;
pub const AT_PHENT: u64 = 4;
pub const AT_PHNUM: u64 = 5;
pub const AT_PAGESZ: u64 = 6;
pub const AT_ENTRY: u64 = 9;
pub const AT_BASE: u64 = 7;
pub const AT_SYSINFO_EHDR: u64 = 33;

pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const PROT_EXEC: i32 = 4;
pub const PROT_NONE: i32 = 0;

pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANONYMOUS: i32 = 0x20;

pub const MAP_FAILED: *mut u8 = !0 as *mut u8;

pub const PAGE_SIZE: usize = 4096;

pub fn parse_ehdr(data: &[u8]) -> Result<Ehdr, &'static str> {
    if data.len() < 64 {
        return Err("file too small for ELF header");
    }
    if &data[..4] != b"\x7fELF" {
        return Err("bad ELF magic");
    }
    if data[4] != 2 {
        return Err("not ELFCLASS64");
    }
    let ehdr = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const Ehdr) };
    if ehdr.e_machine != EM_X86_64 && ehdr.e_machine != EM_AARCH64 && ehdr.e_machine != EM_RISCV {
        return Err("not a supported EM machine");
    }
    Ok(ehdr)
}

pub fn parse_phdrs<'a>(data: &'a [u8], ehdr: &Ehdr) -> Result<&'a [Phdr], &'static str> {
    if ehdr.e_phentsize as usize != core::mem::size_of::<Phdr>() {
        return Err("unsupported e_phentsize");
    }
    let count = ehdr.e_phnum as usize;
    let bytes = count
        .checked_mul(ehdr.e_phentsize as usize)
        .ok_or("phdr table overflow")?;
    let end = (ehdr.e_phoff as usize)
        .checked_add(bytes)
        .ok_or("phdr table overflow")?;
    if end > data.len() {
        return Err("phdr table extends past EOF");
    }
    let off = ehdr.e_phoff as usize;
    if off % core::mem::align_of::<Phdr>() != 0 {
        return Err("unaligned phdr table");
    }
    let base = unsafe { data.as_ptr().add(off) as *const Phdr };
    if (base as usize) % core::mem::align_of::<Phdr>() != 0 {
        return Err("unaligned phdr table");
    }
    Ok(unsafe { core::slice::from_raw_parts(base, count) })
}

pub fn map_segments(phdrs: &[Phdr], fd: i32, e_type: u16) -> Result<u64, &'static str> {
    let page = PAGE_SIZE as u64;

    let mut min = u64::MAX;
    let mut max = 0u64;
    for p in phdrs.iter().filter(|p| p.p_type == PT_LOAD) {
        let seg_start = p.p_vaddr & !(page - 1);
        let seg_end = p
            .p_vaddr
            .checked_add(p.p_memsz)
            .and_then(|v| v.checked_add(page - 1))
            .ok_or("PT_LOAD range overflow")?
            & !(page - 1);
        min = min.min(seg_start);
        max = max.max(seg_end);
    }
    if min >= max {
        return Err("no PT_LOAD segments");
    }

    let mut load_bias = 0u64;
    if e_type == ET_DYN {
        let span = max.checked_sub(min).ok_or("PT_LOAD span overflow")?;
        let reserve = unsafe {
            sys_mmap(
                core::ptr::null_mut(),
                span as usize,
                PROT_NONE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if reserve == MAP_FAILED {
            return Err("mmap failed while reserving ET_DYN span");
        }
        load_bias = (reserve as u64).checked_sub(min).ok_or("load bias overflow")?;
    }

    for ph in phdrs.iter().filter(|p| p.p_type == PT_LOAD) {
        let adj = ph.p_vaddr & (page - 1);
        let map_addr = load_bias
            .checked_add(ph.p_vaddr)
            .and_then(|v| v.checked_sub(adj))
            .ok_or("PT_LOAD address overflow")?;
        let map_off = ph.p_offset.checked_sub(adj).ok_or("PT_LOAD p_offset underflow")?;
        // Map the file-backed pages, not just p_filesz: the pages rounded up
        // to a page boundary may be needed before BSS starts.  Then extend
        // with anonymous pages so that BSS never lands outside the mapping.
        let file_len = ph
            .p_filesz
            .checked_add(adj)
            .and_then(|v| v.checked_add(page - 1))
            .ok_or("PT_LOAD file size overflow")?
            & !(page - 1);
        let mem_len = ph
            .p_memsz
            .checked_add(adj)
            .and_then(|v| v.checked_add(page - 1))
            .ok_or("PT_LOAD memory size overflow")?
            & !(page - 1);

        let prot = prot_from_flags(ph.p_flags);
        if file_len > 0 {
            let ptr = unsafe {
                sys_mmap(
                    map_addr as *mut u8,
                    file_len as usize,
                    prot,
                    MAP_PRIVATE | MAP_FIXED,
                    fd,
                    map_off as i64,
                )
            };
            if ptr == MAP_FAILED {
                return Err("mmap failed for PT_LOAD segment");
            }
        }
        if mem_len > file_len {
            let bss_map = map_addr.checked_add(file_len).ok_or("PT_LOAD BSS address overflow")?;
            let ptr = unsafe {
                sys_mmap(
                    bss_map as *mut u8,
                    (mem_len - file_len) as usize,
                    prot,
                    MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if ptr == MAP_FAILED {
                return Err("mmap failed for PT_LOAD BSS");
            }
        }

        if ph.p_memsz > ph.p_filesz {
            let bss_addr = ph
                .p_vaddr
                .checked_add(ph.p_filesz)
                .and_then(|v| load_bias.checked_add(v))
                .ok_or("PT_LOAD BSS address overflow")?;
            let bss_len = (ph.p_memsz - ph.p_filesz) as usize;
            unsafe { core::ptr::write_bytes(bss_addr as *mut u8, 0, bss_len) };
        }
    }

    Ok(load_bias)
}

pub fn apply_relocations(data: &[u8], phdrs: &[Phdr], base_addr: u64) -> Result<(), &'static str> {
    let dyn_ph = match phdrs.iter().find(|p| p.p_type == PT_DYNAMIC) {
        Some(p) => p,
        None => return Ok(()),
    };

    apply_relocations_from_dynamic(
        data,
        dyn_ph.p_offset as usize,
        dyn_ph.p_filesz as usize,
        base_addr,
        phdrs,
    )
}

pub fn apply_relocations_from_dynamic(
    data: &[u8],
    dyn_off: usize,
    dyn_filesz: usize,
    base_addr: u64,
    phdrs: &[Phdr],
) -> Result<(), &'static str> {
    let dyn_end = dyn_off
        .checked_add(dyn_filesz)
        .ok_or("PT_DYNAMIC size overflow")?;
    if dyn_end > data.len() {
        return Err("PT_DYNAMIC extends past EOF");
    }

    let mut rela_off = 0u64;
    let mut rela_sz = 0u64;
    let mut rela_ent = 24u64;

    let mut pos = dyn_off;
    loop {
        let next = match pos.checked_add(16) {
            Some(next) => next,
            None => break,
        };
        if next > data.len() {
            break;
        }
        let d_tag = u64::from_le_bytes(data[pos..pos + 8].try_into().unwrap());
        let d_val = u64::from_le_bytes(data[pos + 8..pos + 16].try_into().unwrap());
        if d_tag == DT_NULL {
            break;
        }
        match d_tag {
            DT_RELA => rela_off = d_val,
            DT_RELASZ => rela_sz = d_val,
            DT_RELAENT => rela_ent = d_val,
            _ => {}
        }
        pos = next;
    }

    if rela_sz == 0 {
        return Ok(());
    }
    if rela_ent == 0 {
        return Err("DT_RELAENT is zero");
    }
    let rela_off = match usize::try_from(rela_off) {
        Ok(v) => v,
        Err(_) => return Err("DT_RELA offset overflow"),
    };
    let rela_end = match rela_off.checked_add(rela_sz as usize) {
        Some(v) => v,
        None => return Err("DT_RELA range overflow"),
    };
    if rela_end > data.len() {
        return Err("DT_RELA extends past EOF");
    }

    let count = rela_sz / rela_ent;
    for i in 0..count {
        let off = match (i as usize)
            .checked_mul(rela_ent as usize)
            .and_then(|v| rela_off.checked_add(v))
        {
            Some(v) => v,
            None => return Err("relocation offset overflow"),
        };
        let end = match off.checked_add(24) {
            Some(v) => v,
            None => return Err("relocation entry overflow"),
        };
        if end > data.len() {
            return Err("relocation entry extends past EOF");
        }
        let r_offset = u64::from_le_bytes(data[off..off + 8].try_into().unwrap());
        let r_info = u64::from_le_bytes(data[off + 8..off + 16].try_into().unwrap());
        let r_addend = i64::from_le_bytes(data[off + 16..off + 24].try_into().unwrap());

        if r_info & 0xffffffff == R_RELATIVE {
            let writable = phdrs.iter().any(|p| {
                p.p_type == PT_LOAD
                    && (p.p_flags & PF_W) != 0
                    && r_offset >= p.p_vaddr
                    && r_offset < p.p_vaddr.saturating_add(p.p_memsz)
            });
            if !writable {
                return Err("RELATIVE relocation outside writable segment");
            }
            let addr = match base_addr.checked_add(r_offset) {
                Some(v) => v,
                None => return Err("RELATIVE relocation address overflow"),
            };
            let ptr = addr as *mut u64;
            let val = (base_addr as i64).wrapping_add(r_addend) as u64;
            unsafe { *ptr = val };
        }
    }

    Ok(())
}

// Self-relocate: read DT_RELA/DT_RELASZ from in-memory PT_DYNAMIC, apply RELATIVE entries.
// Only stack-local variables and raw pointer writes before this returns.
pub unsafe fn self_relocate(base: usize, phdrs: &[Phdr]) {
    for ph in phdrs.iter().filter(|p| p.p_type == PT_DYNAMIC) {
        let dyn_start = base + ph.p_vaddr as usize;
        let dyn_end = dyn_start + ph.p_memsz as usize;
        let mut pos = dyn_start;

        let mut rela_ptr: *const u8 = core::ptr::null();
        let mut rela_sz: usize = 0;

        while pos + 16 <= dyn_end {
            let d_tag = u64::from_le_bytes(unsafe { *(pos as *const [u8; 8]) });
            let d_val = u64::from_le_bytes(unsafe { *((pos + 8) as *const [u8; 8]) });
            if d_tag == DT_NULL {
                break;
            }
            match d_tag {
                DT_RELA => rela_ptr = (base + d_val as usize) as *const u8,
                DT_RELASZ => rela_sz = d_val as usize,
                _ => {}
            }
            pos += 16;
        }

        if rela_ptr.is_null() || rela_sz == 0 {
            continue;
        }

        let count = rela_sz / 24;
        for i in 0..count {
            let entry = unsafe { rela_ptr.add(i * 24) };
            let r_offset = u64::from_le_bytes(unsafe { *(entry as *const [u8; 8]) });
            let r_info = u64::from_le_bytes(unsafe { *((entry.add(8)) as *const [u8; 8]) });
            let r_addend = i64::from_le_bytes(unsafe { *((entry.add(16)) as *const [u8; 8]) });

            if r_info & 0xffffffff == R_RELATIVE {
                let ptr = (base as u64 + r_offset) as *mut u64;
                let val = (base as i64 + r_addend) as u64;
                unsafe { *ptr = val };
            }
        }
    }
}

pub fn prot_from_flags(flags: u32) -> i32 {
    let mut prot = 0;
    if flags & PF_R != 0 {
        prot |= PROT_READ;
    }
    if flags & PF_W != 0 {
        prot |= PROT_WRITE;
    }
    if flags & PF_X != 0 {
        prot |= PROT_EXEC;
    }
    prot
}
