// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! prologue halves of the `.init` and `.fini` functions (`crti.o`).

#![no_std]

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .section .init
    .global _init
    .type _init,@function
_init:
    push rax
    .section .fini
    .global _fini
    .type _fini,@function
_fini:
    push rax
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .section .init
    .global _init
    .type _init,%function
_init:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    .section .fini
    .global _fini
    .type _fini,%function
_fini:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .init
    .global _init
    .type _init,%function
_init:
    addi sp, sp, -16
    sd ra, 8(sp)
    sd s0, 0(sp)
    mv s0, sp
    .section .fini
    .global _fini
    .type _fini,%function
_fini:
    addi sp, sp, -16
    sd ra, 8(sp)
    sd s0, 0(sp)
    mv s0, sp
    .section .note.GNU-stack,"",@progbits
"#
);
