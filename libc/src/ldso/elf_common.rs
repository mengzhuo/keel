// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared ELF constants and segment-protection mapping for the in-libc
// dynamic linker and the standalone `keel` mapper. Included by both crates.

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_PHDR: u32 = 6;

// ELF64 program-header layout.  Both loaders parse phdrs from raw bytes
// because the executable's phdrs are already mapped by the kernel.
pub const PHDR_SIZE: usize = 56;
pub const PH_TYPE: usize = 0;
pub const PH_FLAGS: usize = 4;
pub const PH_OFFSET: usize = 8;
pub const PH_VADDR: usize = 16;
pub const PH_FILESZ: usize = 32;
pub const PH_MEMSZ: usize = 40;
pub const PH_ALIGN: usize = 48;

// ELF64 executable-header offsets used by the loader.
pub const EH_PHOFF: usize = 32;
pub const EH_PHENTSIZE: usize = 54;
pub const EH_PHNUM: usize = 56;

#[inline]
pub unsafe fn ehdr_u16(eh: *const u8, off: usize) -> u16 {
    u16::from_le_bytes(core::ptr::read_unaligned(eh.add(off) as *const [u8; 2]))
}

#[inline]
pub unsafe fn ehdr_u64(eh: *const u8, off: usize) -> u64 {
    u64::from_le_bytes(core::ptr::read_unaligned(eh.add(off) as *const [u8; 8]))
}

#[inline]
pub unsafe fn phdr_u32(ph: *const u8, off: usize) -> u32 {
    u32::from_le_bytes(core::ptr::read_unaligned(ph.add(off) as *const [u8; 4]))
}

#[inline]
pub unsafe fn phdr_u64(ph: *const u8, off: usize) -> u64 {
    u64::from_le_bytes(core::ptr::read_unaligned(ph.add(off) as *const [u8; 8]))
}

#[inline]
pub unsafe fn phdr_type(ph: *const u8) -> u32 {
    phdr_u32(ph, PH_TYPE)
}

pub const PF_R: u32 = 4;
pub const PF_W: u32 = 2;
pub const PF_X: u32 = 1;

pub const PROT_READ: i32 = 1;
pub const PROT_WRITE: i32 = 2;
pub const PROT_EXEC: i32 = 4;
pub const PROT_NONE: i32 = 0;

pub const MAP_PRIVATE: i32 = 0x02;
pub const MAP_FIXED: i32 = 0x10;
pub const MAP_ANONYMOUS: i32 = 0x20;

pub const DT_NULL: u64 = 0;
pub const DT_RELA: u64 = 7;
pub const DT_RELAENT: u64 = 9;
pub const DT_RELASZ: u64 = 8;

#[allow(dead_code)] // selected only on x86_64
pub const R_X86_64_RELATIVE: u64 = 8;
#[allow(dead_code)] // selected only on aarch64
pub const R_AARCH64_RELATIVE: u64 = 1027;
#[allow(dead_code)] // selected only on riscv64
pub const R_RISCV_RELATIVE: u64 = 3;

#[cfg(target_arch = "x86_64")]
#[allow(dead_code)]
pub const R_RELATIVE: u64 = R_X86_64_RELATIVE;
#[cfg(target_arch = "aarch64")]
#[allow(dead_code)]
pub const R_RELATIVE: u64 = R_AARCH64_RELATIVE;
#[cfg(target_arch = "riscv64")]
#[allow(dead_code)]
pub const R_RELATIVE: u64 = R_RISCV_RELATIVE;

pub const AT_NULL: u64 = 0;
pub const AT_PHDR: u64 = 3;
pub const AT_PHENT: u64 = 4;
pub const AT_PHNUM: u64 = 5;
pub const AT_PAGESZ: u64 = 6;
pub const AT_ENTRY: u64 = 9;
#[allow(dead_code)] // used by the dynamic linker, not the standalone mapper
pub const AT_BASE: u64 = 7;
pub const AT_UID: u64 = 11;
pub const AT_EUID: u64 = 12;
pub const AT_GID: u64 = 13;
pub const AT_EGID: u64 = 14;
pub const AT_SECURE: u64 = 23;
pub const AT_RANDOM: u64 = 25;
pub const AT_SYSINFO_EHDR: u64 = 33;

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
pub const EM_X86_64: u16 = 62;
pub const EM_AARCH64: u16 = 183;
pub const EM_RISCV: u16 = 243;

pub const PAGE_SIZE: usize = 4096;

const MMAP_FAILED: *mut u8 = !0 as *mut u8;

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

#[allow(unused_unsafe)]
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
        if reserve == MMAP_FAILED {
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
            if ptr == MMAP_FAILED {
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
            if ptr == MMAP_FAILED {
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
