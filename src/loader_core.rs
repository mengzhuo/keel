// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ELF parsing, segment mapping and relocation for the standalone loader.
// Architecture and raw syscall details live in arch.rs.

use crate::arch::sys_mmap;

#[allow(dead_code)] // the shared file also serves the in-libc loader
pub mod elf_common {
    use super::sys_mmap;
    include!("ldso/elf_common.rs");
}
pub use elf_common::*;

pub const MAP_FAILED: *mut u8 = !0 as *mut u8;

include!("ldso/startup_common.rs");

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
    let mut rela_ent = RELA_ENT_SIZE as u64;
    let mut relr_off = 0u64;
    let mut relr_sz = 0u64;

    let mut dyns = DynRange::new(data.as_ptr() as usize + dyn_off, dyn_filesz);
    while let Some((tag, val)) = unsafe { dyns.next() } {
        match tag {
            DT_RELA => rela_off = val,
            DT_RELASZ => rela_sz = val,
            DT_RELAENT => rela_ent = val,
            DT_RELR => relr_off = val,
            DT_RELRSZ => relr_sz = val,
            _ => {}
        }
    }

    if relr_sz != 0 {
        unsafe { apply_relr(base_addr, relr_off, relr_sz) };
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
        let end = match off.checked_add(RELA_ENT_SIZE) {
            Some(v) => v,
            None => return Err("relocation entry overflow"),
        };
        if end > data.len() {
            return Err("relocation entry extends past EOF");
        }
        let rela = unsafe { read_rela(data.as_ptr().add(off)) };
        let r_offset = rela.r_offset;
        let r_info = rela.r_info;
        let r_addend = rela.r_addend;

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
