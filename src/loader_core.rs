// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ELF parsing, segment mapping and relocation for the standalone loader.
// Architecture and raw syscall details live in arch.rs.

use crate::arch::sys_mmap;

#[allow(dead_code)] // the shared file also serves the in-libc loader
pub mod elf_common {
    use super::sys_mmap;
    include!("../libc/src/ldso/elf_common.rs");
}
pub use elf_common::*;

pub const MAP_FAILED: *mut u8 = !0 as *mut u8;

include!("../libc/src/ldso/startup_common.rs");

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

