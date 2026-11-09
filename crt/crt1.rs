//! crabc startup object (`crt1.o` / `Scrt1.o`).
//!
//! The entry point only marshals the kernel stack into the
//! `__libc_start_main(main, argc, argv, init, fini, rtld_fini)` ABI.
//! Dynamic constructors are run by the crabc loader before the entry point,
//! so no init/fini callbacks are passed here.

#![no_std]

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .text
    .global _start
    .type _start,@function
_start:
    xor rbp, rbp
    mov rsi, [rsp]
    lea rdx, [rsp + 8]
    lea rdi, [rip + main]
    xor ecx, ecx
    xor r8d, r8d
    xor r9d, r9d
    and rsp, -16
    call __libc_start_main
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
    mov x9, sp
    ldr x1, [x9]
    add x2, x9, #8
    adrp x0, main
    add x0, x0, :lo12:main
    mov x3, #0
    mov x4, #0
    mov x5, #0
    and sp, x9, #-16
    b __libc_start_main
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
    ld a1, 0(t0)
    addi a2, t0, 8
    lla a0, main
    li a3, 0
    li a4, 0
    li a5, 0
    andi sp, t0, -16
    tail __libc_start_main
    .size _start, .-_start
    .section .note.GNU-stack,"",@progbits
"#
);
