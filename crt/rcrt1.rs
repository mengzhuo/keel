// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! crabc static-PIE startup object (`rcrt1.o`).
//!
//! A static PIE is mapped by the kernel with no interpreter, so the entry
//! point must apply its own relative relocations before calling into libc.
//! The relocation walk is deliberately call-free and uses only stack and
//! pointer operations; `__libc_start_main` is reached only after the image
//! is self-relocated.

#![no_std]

use core::ffi::{c_char, c_int, c_void};

const DT_NULL: u64 = 0;
const DT_PLTRELSZ: u64 = 2;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_REL: u64 = 17;
const DT_RELSZ: u64 = 18;
const DT_RELR: u64 = 36;
const DT_RELRSZ: u64 = 35;
const DT_JMPREL: u64 = 23;

const AT_NULL: usize = 0;
const AT_PHDR: usize = 3;
const AT_PHENT: usize = 4;
const AT_PHNUM: usize = 5;
const AT_BASE: usize = 7;

const PT_DYNAMIC: u32 = 2;

#[cfg(target_arch = "x86_64")]
const R_RELATIVE: u64 = 8;
#[cfg(target_arch = "aarch64")]
const R_RELATIVE: u64 = 1027;
#[cfg(target_arch = "riscv64")]
const R_RELATIVE: u64 = 3;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .text
    .global _start
    .type _start,@function
_start:
    xor rbp, rbp
    mov rdi, rsp
    lea rsi, [rip + _DYNAMIC]
    and rsp, -16
    call crabc_rcrt1_start
    hlt
    .size _start, .-_start
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .text
    .global _start
    .type _start,%function
_start:
    mov x29, #0
    mov x30, #0
    mov x0, sp
    adrp x1, _DYNAMIC
    add x1, x1, :lo12:_DYNAMIC
    and sp, x0, #-16
    b crabc_rcrt1_start
    .size _start, .-_start
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .text
    .global _start
    .type _start,%function
_start:
    .option push
    .option norelax
    lla gp, __global_pointer$
    .option pop
    mv t0, sp
    mv a0, t0
    lla a1, _DYNAMIC
    andi sp, t0, -16
    tail crabc_rcrt1_start
    .size _start, .-_start
    .section .note.GNU-stack,"",@progbits
"#
);

type MainFn = unsafe extern "C" fn(c_int, *const *const c_char, *const *const c_char) -> c_int;

extern "C" {
    fn main(argc: c_int, argv: *const *const c_char, envp: *const *const c_char) -> c_int;
    fn __libc_start_main(
        main: MainFn,
        argc: c_int,
        argv: *const *const c_char,
        init: *const c_void,
        fini: *const c_void,
        rtld_fini: *const c_void,
        stack_end: *const c_void,
    ) -> !;
    static mut __auxv: *const usize;
    static mut __rc_static_base: usize;
}

#[no_mangle]
unsafe extern "C" fn crabc_rcrt1_start(sp: usize, dyn_addr: usize) -> ! {
    let argc = *(sp as *const usize);
    let argv = (sp + 8) as *const *const c_char;
    let mut envp = (sp + 8 * (argc + 2)) as *const *const c_char;
    while !(*envp).is_null() {
        envp = envp.add(1);
    }
    let auxv = envp.add(1) as *const usize;

    let mut at_phdr = 0usize;
    let mut at_phnum = 0usize;
    let mut at_phent = 0usize;
    let mut at_base = 0usize;
    let mut i = 0usize;
    loop {
        let tag = *auxv.add(i * 2);
        if tag == AT_NULL {
            break;
        }
        let val = *auxv.add(i * 2 + 1);
        if tag == AT_PHDR {
            at_phdr = val;
        } else if tag == AT_PHNUM {
            at_phnum = val;
        } else if tag == AT_PHENT {
            at_phent = val;
        } else if tag == AT_BASE {
            at_base = val;
        }
        i += 1;
    }

    let mut base = at_base;
    if base == 0 {
        if at_phdr == 0 || at_phnum == 0 || at_phent < 56 {
            // The kernel did not provide a usable program header table.
            loop {}
        }
        let mut ph = at_phdr;
        for _ in 0..at_phnum {
            let p_type = *(ph as *const u32);
            if p_type == PT_DYNAMIC {
                let p_vaddr = *((ph + 16) as *const u64) as usize;
                base = dyn_addr.wrapping_sub(p_vaddr);
                break;
            }
            ph += at_phent;
        }
    }

    let mut rela = 0usize;
    let mut rela_sz = 0usize;
    let mut rel = 0usize;
    let mut rel_sz = 0usize;
    let mut relr = 0usize;
    let mut relr_sz = 0usize;
    let mut jmprel = 0usize;
    let mut jmprel_sz = 0usize;
    let mut d = dyn_addr;
    loop {
        let tag = *(d as *const u64);
        if tag == DT_NULL {
            break;
        }
        let val = *((d + 8) as *const u64) as usize;
        if tag == DT_RELA {
            rela = val;
        } else if tag == DT_RELASZ {
            rela_sz = val;
        } else if tag == DT_REL {
            rel = val;
        } else if tag == DT_RELSZ {
            rel_sz = val;
        } else if tag == DT_RELR {
            relr = val;
        } else if tag == DT_RELRSZ {
            relr_sz = val;
        } else if tag == DT_JMPREL {
            jmprel = val;
        } else if tag == DT_PLTRELSZ {
            jmprel_sz = val;
        }
        d += 16;
    }

    if rela != 0 && rela_sz != 0 {
        let mut p = base + rela;
        let end = p + rela_sz;
        while p < end {
            let r_offset = *(p as *const usize);
            let r_info = *((p + 8) as *const u64);
            let r_addend = *((p + 16) as *const i64) as usize;
            if (r_info & 0xffff_ffff) == R_RELATIVE {
                let slot = (base + r_offset) as *mut usize;
                *slot = base.wrapping_add(r_addend);
            }
            p += 24;
        }
    }

    if rel != 0 && rel_sz != 0 {
        let mut p = base + rel;
        let end = p + rel_sz;
        while p < end {
            let r_offset = *(p as *const usize);
            let r_info = *((p + 8) as *const u64);
            if (r_info & 0xffff_ffff) == R_RELATIVE {
                let slot = (base + r_offset) as *mut usize;
                *slot = base.wrapping_add(*slot);
            }
            p += 16;
        }
    }

    if jmprel != 0 && jmprel_sz != 0 {
        let mut p = base + jmprel;
        let end = p + jmprel_sz;
        while p < end {
            let r_offset = *(p as *const usize);
            let r_info = *((p + 8) as *const u64);
            let r_addend = *((p + 16) as *const i64) as usize;
            if (r_info & 0xffff_ffff) == R_RELATIVE {
                let slot = (base + r_offset) as *mut usize;
                *slot = base.wrapping_add(r_addend);
            }
            p += 24;
        }
    }

    if relr != 0 && relr_sz != 0 {
        let mut p = base + relr;
        let end = p + relr_sz;
        let mut where_ = 0usize;
        while p < end {
            let entry = *(p as *const usize);
            p += 8;
            if entry & 1 == 0 {
                where_ = base.wrapping_add(entry);
                *(where_ as *mut usize) = (*(where_ as *const usize)).wrapping_add(base);
                where_ += core::mem::size_of::<usize>();
            } else {
                let mut bitmap = entry;
                let mut i = 0usize;
                while bitmap >> 1 != 0 {
                    if bitmap & 1 != 0 {
                        let slot = (where_ + i * core::mem::size_of::<usize>()) as *mut usize;
                        *slot = (*slot).wrapping_add(base);
                    }
                    bitmap >>= 1;
                    i += 1;
                }
                where_ += (core::mem::size_of::<usize>() * 8 - 1) * core::mem::size_of::<usize>();
            }
        }
    }

    __auxv = auxv;
    __rc_static_base = base;

    __libc_start_main(
        main,
        argc as c_int,
        argv,
        core::ptr::null(),
        core::ptr::null(),
        core::ptr::null(),
        sp as *const c_void,
    )
}
