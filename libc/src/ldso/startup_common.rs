// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared initial-stack construction and entry jump for both the standalone
// mapper and the in-libc dynamic linker.

const STACK_GUARD: usize = 1024;

unsafe fn c_str_len(s: *const u8) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Lay out argv, envp and auxv on a fresh stack and return the initial `sp`.
///
/// `auxv` must not contain AT_NULL; a terminating entry is appended.  When
/// `random` is non-null, 16 bytes are copied and an AT_RANDOM entry is added.
/// `scratch` must have room for `argv.len() + envp.len()` pointers.
pub unsafe fn build_initial_stack(
    stack_base: usize,
    stack_size: usize,
    argv: &[*const u8],
    envp: &[*const u8],
    auxv: &[(u64, u64)],
    random: *const u8,
    scratch: *mut usize,
) -> Option<usize> {
    let bottom = stack_base.checked_add(STACK_GUARD)?;
    let mut sp = stack_base.checked_add(stack_size)?;

    for (i, &s) in argv.iter().enumerate() {
        let need = c_str_len(s).checked_add(1)?;
        sp = sp.checked_sub(need)?;
        if sp < bottom {
            return None;
        }
        core::ptr::copy_nonoverlapping(s, sp as *mut u8, need);
        *scratch.add(i) = sp;
    }
    let argc = argv.len();
    for (i, &s) in envp.iter().enumerate() {
        let need = c_str_len(s).checked_add(1)?;
        sp = sp.checked_sub(need)?;
        if sp < bottom {
            return None;
        }
        core::ptr::copy_nonoverlapping(s, sp as *mut u8, need);
        *scratch.add(argc + i) = sp;
    }

    let mut random_ptr = 0usize;
    if !random.is_null() {
        sp = sp.checked_sub(16)?;
        sp &= !15usize;
        if sp < bottom {
            return None;
        }
        random_ptr = sp;
        core::ptr::copy_nonoverlapping(random, sp as *mut u8, 16);
    }
    sp &= !15usize;

    let random_entry = if random.is_null() { 0 } else { 1 };
    let aux_count = auxv.len().checked_add(random_entry)?.checked_add(1)?;
    let aux_bytes = aux_count.checked_mul(16)?;
    let envc = envp.len();
    let pointer_bytes = argc.checked_add(envc)?.checked_add(3)?.checked_mul(8)?;
    let pad = if (argc + envc).is_multiple_of(2) { 8 } else { 0 };
    sp = sp.checked_sub(aux_bytes.checked_add(pointer_bytes)?.checked_add(pad)?)?;
    if sp < bottom {
        return None;
    }

    if pad != 0 {
        sp -= 8;
        *(sp as *mut u64) = 0;
    }

    let aux = sp as *mut u64;
    let mut ai = 0usize;
    for &(tag, val) in auxv {
        *aux.add(ai * 2) = tag;
        *aux.add(ai * 2 + 1) = val;
        ai += 1;
    }
    if !random.is_null() {
        *aux.add(ai * 2) = AT_RANDOM;
        *aux.add(ai * 2 + 1) = random_ptr as u64;
        ai += 1;
    }
    *aux.add(ai * 2) = AT_NULL;
    *aux.add(ai * 2 + 1) = 0;

    sp -= (envc + 1) * 8;
    for i in 0..envc {
        *((sp + i * 8) as *mut u64) = *scratch.add(argc + i) as u64;
    }
    *((sp + envc * 8) as *mut u64) = 0;

    sp -= (argc + 1) * 8;
    for i in 0..argc {
        *((sp + i * 8) as *mut u64) = *scratch.add(i) as u64;
    }
    *((sp + argc * 8) as *mut u64) = 0;

    sp -= 8;
    *(sp as *mut u64) = argc as u64;
    Some(sp)
}

#[inline(never)]
pub unsafe fn jump_to_entry(entry: u64, sp: usize) -> ! {
    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "mov rsp, {sp}",
        "jmp {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "aarch64")]
    core::arch::asm!(
        "mov sp, {sp}",
        "br {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "powerpc64")]
    core::arch::asm!(
        "mr 1, {sp}",
        "mr 12, {entry}",
        "mtctr 12",
        "bctr",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "loongarch64")]
    core::arch::asm!(
        "move $sp, {sp}",
        "jr {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "riscv64")]
    {
        // The compiler reuses the entry register during address computation;
        // reload it volatile immediately before the jump.
        let entry_ref = &entry as *const u64;
        let entry_val = core::ptr::read_volatile(entry_ref);
        core::arch::asm!(
            "mv sp, {sp}",
            "jr {entry}",
            sp = in(reg) sp,
            entry = in(reg) entry_val,
            options(noreturn)
        );
    }
}
