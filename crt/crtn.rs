//! epilogue halves of the `.init` and `.fini` functions (`crtn.o`).

#![no_std]

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    r#"
    .section .init
    pop rax
    ret
    .section .fini
    pop rax
    ret
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    r#"
    .section .init
    ldp x29, x30, [sp], #16
    ret
    .section .fini
    ldp x29, x30, [sp], #16
    ret
    .section .note.GNU-stack,"",@progbits
"#
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    r#"
    .section .init
    ld ra, 8(sp)
    ld s0, 0(sp)
    addi sp, sp, 16
    ret
    .section .fini
    ld ra, 8(sp)
    ld s0, 0(sp)
    addi sp, sp, 16
    ret
    .section .note.GNU-stack,"",@progbits
"#
);
