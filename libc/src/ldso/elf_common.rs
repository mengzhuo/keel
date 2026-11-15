// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared ELF constants and segment-protection mapping for the in-libc
// dynamic linker and the standalone `keel` mapper. Included by both crates.

pub const PT_LOAD: u32 = 1;
pub const PT_DYNAMIC: u32 = 2;
pub const PT_PHDR: u32 = 6;

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
