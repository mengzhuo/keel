// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// setjmp.h
// jmp_buf is unsigned long[8]: rbx, rbp, r12-r15, rsp, rip
// ============================================================

#[no_mangle]
#[inline(never)]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn setjmp(env: *mut c_ulong) -> c_int {
    core::arch::asm!(
        "mov rax, [rsp]",
        "lea rcx, [rsp + 8]",
        "mov [r8], rbx",
        "mov [r8 + 8], rbp",
        "mov [r8 + 16], r12",
        "mov [r8 + 24], r13",
        "mov [r8 + 32], r14",
        "mov [r8 + 40], r15",
        "mov [r8 + 48], rcx",
        "mov [r8 + 56], rax",
        in("r8") env,
        lateout("rax") _,
        lateout("rcx") _,
        options(nostack),
    );
    0
}

#[no_mangle]
#[inline(never)]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn longjmp(env: *const c_ulong, val: c_int) -> ! {
    let ret = if val == 0 { 1 } else { val } as u32;
    core::arch::asm!(
        "mov rbx, [r8]",
        "mov rbp, [r8 + 8]",
        "mov r12, [r8 + 16]",
        "mov r13, [r8 + 24]",
        "mov r14, [r8 + 32]",
        "mov r15, [r8 + 40]",
        "mov rsp, [r8 + 48]",
        "mov eax, edx",
        "jmp [r8 + 56]",
        in("r8") env,
        in("edx") ret,
        options(noreturn),
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn setjmp(env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "stp x19, x20, [x0, #0]",
        "stp x21, x22, [x0, #16]",
        "stp x23, x24, [x0, #32]",
        "stp x25, x26, [x0, #48]",
        "stp x27, x28, [x0, #64]",
        "stp x29, x30, [x0, #80]",
        "mov x2, sp",
        "str x2, [x0, #104]",
        "stp d8, d9, [x0, #112]",
        "stp d10, d11, [x0, #128]",
        "stp d12, d13, [x0, #144]",
        "stp d14, d15, [x0, #160]",
        "mov x0, #0",
        "ret",
    );
}

#[no_mangle]
#[inline(never)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn longjmp(env: *const c_ulong, val: c_int) -> ! {
    let ret = if val == 0 { 1 } else { val } as u64;
    core::arch::asm!(
        "ldp x19, x20, [x0]",
        "ldp x21, x22, [x0, #16]",
        "ldp x23, x24, [x0, #32]",
        "ldp x25, x26, [x0, #48]",
        "ldp x27, x28, [x0, #64]",
        "ldp x29, x30, [x0, #80]",
        "ldr x2, [x0, #104]",
        "mov sp, x2",
        "ldp d8, d9, [x0, #112]",
        "ldp d10, d11, [x0, #128]",
        "ldp d12, d13, [x0, #144]",
        "ldp d14, d15, [x0, #160]",
        "mov x0, x1",
        "br x30",
        in("x0") env,
        in("x1") ret,
        options(noreturn),
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn setjmp(env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "sd s0, 0(a0)",
        "sd s1, 8(a0)",
        "sd s2, 16(a0)",
        "sd s3, 24(a0)",
        "sd s4, 32(a0)",
        "sd s5, 40(a0)",
        "sd s6, 48(a0)",
        "sd s7, 56(a0)",
        "sd s8, 64(a0)",
        "sd s9, 72(a0)",
        "sd s10, 80(a0)",
        "sd s11, 88(a0)",
        "sd sp, 96(a0)",
        "sd ra, 104(a0)",
        "li a0, 0",
        "ret",
    );
}

#[no_mangle]
#[inline(never)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn longjmp(env: *const c_ulong, val: c_int) -> ! {
    let ret = if val == 0 { 1 } else { val } as u64;
    core::arch::asm!(
        "ld s0, 0(a0)",
        "ld s1, 8(a0)",
        "ld s2, 16(a0)",
        "ld s3, 24(a0)",
        "ld s4, 32(a0)",
        "ld s5, 40(a0)",
        "ld s6, 48(a0)",
        "ld s7, 56(a0)",
        "ld s8, 64(a0)",
        "ld s9, 72(a0)",
        "ld s10, 80(a0)",
        "ld s11, 88(a0)",
        "ld sp, 96(a0)",
        "ld ra, 104(a0)",
        "mv a0, a1",
        "ret",
        in("a0") env,
        in("a1") ret,
        options(noreturn),
    );
}

// sigsetjmp is implemented in raw assembly because the Rust compiler
// rewrites naked_asm that touches rbx into incorrect push/pop sequences,
// corrupting the caller's rbx across the initial return. The exported wrappers
// jump to a local hidden assembly implementation; the layout matches musl:
// env[0..7] = __jmp_buf, env[8] = saved return address, env[9] = saved signal
// mask, env[10] = saved rbx.

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!("jmp sigsetjmp_real");
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn __sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!("jmp sigsetjmp_real");
}

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".type sigsetjmp_real, @function",
    "sigsetjmp_real:",
    "   test esi, esi",
    "   jz 1f",
    "   pop qword ptr [rdi + 64]",
    "   mov qword ptr [rdi + 80], rbx",
    // Do not push rbx here: it would move the stack by 8 bytes and make
    // setjmp save an rsp that is 8 bytes below the caller's, misaligning the
    // caller after longjmp (postgres bootstrap crashes in AVX memset).
    "   mov rbx, rdi",
    "   call setjmp",
    "   push qword ptr [rbx + 64]",
    "   mov rdi, rbx",
    "   mov esi, eax",
    "   mov rbx, qword ptr [rbx + 80]",
    "   lea rdx, [rdi + 72]",
    "   test esi, esi",
    "   jz 2f",
    "   mov rsi, rdx",
    "   xor rdx, rdx",
    "   mov edi, 2",
    "   jmp 3f",
    "2:",
    "   xor rsi, rsi",
    "   mov edi, 2",
    "3:",
    "   mov eax, 14",
    "   mov r10d, 8",
    "   syscall",
    "   test esi, esi",
    "   jnz 4f",
    "   mov rax, 0xfffffffe7fffffff",
    "   and qword ptr [rdx], rax",
    "4:",
    "   mov eax, esi",
    "   ret",
    "1:",
    "   jmp setjmp",
);


#[no_mangle]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn __sigsetjmp_tail(env: *mut c_ulong, ret: c_int) -> c_int {
    const SIG_SETMASK: c_int = 2;
    let ss = env.add(23) as *mut SigSetT;
    let set = if ret != 0 { ss as *const SigSetT } else { core::ptr::null() };
    let old = if ret != 0 { core::ptr::null_mut() } else { ss };
    let _ = sys_rt_sigprocmask(SIG_SETMASK, set, old, core::mem::size_of::<SigSetT>());
    ret
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn sigsetjmp(env: *mut c_ulong, savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "cbz w1, setjmp",
        "str x30, [x0, #176]",
        "str x19, [x0, #192]",
        "mov x19, x0",
        "bl setjmp",
        "mov w1, w0",
        "mov x0, x19",
        "ldr x30, [x0, #176]",
        "ldr x19, [x0, #192]",
        "b __sigsetjmp_tail",
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn __sigsetjmp(env: *mut c_ulong, savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "cbz w1, setjmp",
        "str x30, [x0, #176]",
        "str x19, [x0, #192]",
        "mov x19, x0",
        "bl setjmp",
        "mov w1, w0",
        "mov x0, x19",
        "ldr x30, [x0, #176]",
        "ldr x19, [x0, #192]",
        "b __sigsetjmp_tail",
    );
}


#[no_mangle]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn __sigsetjmp_tail(env: *mut c_ulong, ret: c_int) -> c_int {
    const SIG_SETMASK: c_int = 2;
    let ss = env.add(27) as *mut SigSetT;
    let set = if ret != 0 { ss as *const SigSetT } else { core::ptr::null() };
    let old = if ret != 0 { core::ptr::null_mut() } else { ss };
    let _ = sys_rt_sigprocmask(SIG_SETMASK, set, old, core::mem::size_of::<SigSetT>());
    ret
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn sigsetjmp(env: *mut c_ulong, savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "bnez a1, 1f",
        "tail setjmp",
        "1:",
        "sd ra, 208(a0)",
        "sd s0, 224(a0)",
        "mv s0, a0",
        "call setjmp",
        "mv a1, a0",
        "mv a0, s0",
        "ld s0, 224(a0)",
        "ld ra, 208(a0)",
        "tail __sigsetjmp_tail",
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn __sigsetjmp(env: *mut c_ulong, savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "bnez a1, 1f",
        "tail setjmp",
        "1:",
        "sd ra, 208(a0)",
        "sd s0, 224(a0)",
        "mv s0, a0",
        "call setjmp",
        "mv a1, a0",
        "mv a0, s0",
        "ld s0, 224(a0)",
        "ld ra, 208(a0)",
        "tail __sigsetjmp_tail",
    );
}

#[no_mangle]
#[inline(never)]
pub unsafe extern "C" fn siglongjmp(env: *const c_ulong, val: c_int) -> ! {
    longjmp(env, val);
}

// Adapted from musl's powerpc64 setjmp.s / longjmp.s / sigsetjmp.s.
// The save body is local to libc; the exported entry points are naked Rust
// functions so rustc's cdylib export list includes them.
#[cfg(target_arch = "powerpc64")]
core::arch::global_asm!(
    ".globl __keel_setjmp_toc",
    ".hidden __keel_setjmp_toc",
    ".type __keel_setjmp_toc,@function",
    "__keel_setjmp_toc:",
    "mflr 0",
    "std 0, 0(3)",
    "mfcr 0",
    "std 0, 8(3)",
    "std 1, 16(3)",
    "std 5, 24(3)",
    "std 14, 32(3)",
    "std 15, 40(3)",
    "std 16, 48(3)",
    "std 17, 56(3)",
    "std 18, 64(3)",
    "std 19, 72(3)",
    "std 20, 80(3)",
    "std 21, 88(3)",
    "std 22, 96(3)",
    "std 23, 104(3)",
    "std 24, 112(3)",
    "std 25, 120(3)",
    "std 26, 128(3)",
    "std 27, 136(3)",
    "std 28, 144(3)",
    "std 29, 152(3)",
    "std 30, 160(3)",
    "std 31, 168(3)",
    "stfd 14, 176(3)",
    "stfd 15, 184(3)",
    "stfd 16, 192(3)",
    "stfd 17, 200(3)",
    "stfd 18, 208(3)",
    "stfd 19, 216(3)",
    "stfd 20, 224(3)",
    "stfd 21, 232(3)",
    "stfd 22, 240(3)",
    "stfd 23, 248(3)",
    "stfd 24, 256(3)",
    "stfd 25, 264(3)",
    "stfd 26, 272(3)",
    "stfd 27, 280(3)",
    "stfd 28, 288(3)",
    "stfd 29, 296(3)",
    "stfd 30, 304(3)",
    "stfd 31, 312(3)",
    "addi 3, 3, 320",
    "stvx 20, 0, 3",
    "addi 3, 3, 16",
    "stvx 21, 0, 3",
    "addi 3, 3, 16",
    "stvx 22, 0, 3",
    "addi 3, 3, 16",
    "stvx 23, 0, 3",
    "addi 3, 3, 16",
    "stvx 24, 0, 3",
    "addi 3, 3, 16",
    "stvx 25, 0, 3",
    "addi 3, 3, 16",
    "stvx 26, 0, 3",
    "addi 3, 3, 16",
    "stvx 27, 0, 3",
    "addi 3, 3, 16",
    "stvx 28, 0, 3",
    "addi 3, 3, 16",
    "stvx 29, 0, 3",
    "addi 3, 3, 16",
    "stvx 30, 0, 3",
    "addi 3, 3, 16",
    "stvx 31, 0, 3",
    "li 3, 0",
    "blr",
);

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-setjmp@ha",
        "addi 2, 2, .TOC.-setjmp@l",
        "ld 5, 24(1)",
        "b __keel_setjmp_toc",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn _setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-_setjmp@ha",
        "addi 2, 2, .TOC.-_setjmp@l",
        "ld 5, 24(1)",
        "b __keel_setjmp_toc",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn __setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-__setjmp@ha",
        "addi 2, 2, .TOC.-__setjmp@l",
        "ld 5, 24(1)",
        "b __keel_setjmp_toc",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn longjmp(_env: *const c_ulong, _val: c_int) -> ! {
    core::arch::naked_asm!(
        "ld 0, 0(3)",
        "mtlr 0",
        "ld 0, 8(3)",
        "mtcr 0",
        "ld 1, 16(3)",
        "ld 2, 24(3)",
        "std 2, 24(1)",
        "ld 14, 32(3)",
        "ld 15, 40(3)",
        "ld 16, 48(3)",
        "ld 17, 56(3)",
        "ld 18, 64(3)",
        "ld 19, 72(3)",
        "ld 20, 80(3)",
        "ld 21, 88(3)",
        "ld 22, 96(3)",
        "ld 23, 104(3)",
        "ld 24, 112(3)",
        "ld 25, 120(3)",
        "ld 26, 128(3)",
        "ld 27, 136(3)",
        "ld 28, 144(3)",
        "ld 29, 152(3)",
        "ld 30, 160(3)",
        "ld 31, 168(3)",
        "lfd 14, 176(3)",
        "lfd 15, 184(3)",
        "lfd 16, 192(3)",
        "lfd 17, 200(3)",
        "lfd 18, 208(3)",
        "lfd 19, 216(3)",
        "lfd 20, 224(3)",
        "lfd 21, 232(3)",
        "lfd 22, 240(3)",
        "lfd 23, 248(3)",
        "lfd 24, 256(3)",
        "lfd 25, 264(3)",
        "lfd 26, 272(3)",
        "lfd 27, 280(3)",
        "lfd 28, 288(3)",
        "lfd 29, 296(3)",
        "lfd 30, 304(3)",
        "lfd 31, 312(3)",
        "addi 3, 3, 320",
        "lvx 20, 0, 3",
        "addi 3, 3, 16",
        "lvx 21, 0, 3",
        "addi 3, 3, 16",
        "lvx 22, 0, 3",
        "addi 3, 3, 16",
        "lvx 23, 0, 3",
        "addi 3, 3, 16",
        "lvx 24, 0, 3",
        "addi 3, 3, 16",
        "lvx 25, 0, 3",
        "addi 3, 3, 16",
        "lvx 26, 0, 3",
        "addi 3, 3, 16",
        "lvx 27, 0, 3",
        "addi 3, 3, 16",
        "lvx 28, 0, 3",
        "addi 3, 3, 16",
        "lvx 29, 0, 3",
        "addi 3, 3, 16",
        "lvx 30, 0, 3",
        "addi 3, 3, 16",
        "lvx 31, 0, 3",
        "mr 3, 4",
        "cmpwi cr7, 4, 0",
        "bne cr7, 1f",
        "li 3, 1",
        "1:",
        "blr",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-sigsetjmp@ha",
        "addi 2, 2, .TOC.-sigsetjmp@l",
        "cmpwi cr7, 4, 0",
        "beq- cr7, 2f",
        "ld 5, 24(1)",
        "mflr 6",
        "std 6, 512(3)",
        "std 2, 528(3)",
        "std 16, 536(3)",
        "mr 16, 3",
        "bl __keel_setjmp_toc",
        "mr 4, 3",
        "mr 3, 16",
        "ld 5, 512(3)",
        "mtlr 5",
        "ld 2, 528(3)",
        "ld 16, 536(3)",
        "b __sigsetjmp_tail",
        "2:",
        "ld 5, 24(1)",
        "b __keel_setjmp_toc",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn __sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-__sigsetjmp@ha",
        "addi 2, 2, .TOC.-__sigsetjmp@l",
        "cmpwi cr7, 4, 0",
        "beq- cr7, 2f",
        "ld 5, 24(1)",
        "mflr 6",
        "std 6, 512(3)",
        "std 2, 528(3)",
        "std 16, 536(3)",
        "mr 16, 3",
        "bl __keel_setjmp_toc",
        "mr 4, 3",
        "mr 3, 16",
        "ld 5, 512(3)",
        "mtlr 5",
        "ld 2, 528(3)",
        "ld 16, 536(3)",
        "b __sigsetjmp_tail",
        "2:",
        "ld 5, 24(1)",
        "b __keel_setjmp_toc",
    );
}

#[cfg(target_arch = "powerpc64")]
#[no_mangle]
pub unsafe extern "C" fn __sigsetjmp_tail(env: *mut c_ulong, ret: c_int) -> c_int {
    const SIG_SETMASK: c_int = 2;
    let ss = env.add(65) as *mut SigSetT;
    let set = if ret != 0 { ss as *const SigSetT } else { core::ptr::null() };
    let old = if ret != 0 { core::ptr::null_mut() } else { ss };
    let _ = sys_rt_sigprocmask(SIG_SETMASK, set, old, core::mem::size_of::<SigSetT>());
    ret
}

// Adapted from musl's loongarch64 setjmp.S / longjmp.S / sigsetjmp.s.
#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "st.d $ra, $a0, 0",
        "st.d $sp, $a0, 8",
        "st.d $r21, $a0, 16",
        "st.d $fp, $a0, 24",
        "st.d $s0, $a0, 32",
        "st.d $s1, $a0, 40",
        "st.d $s2, $a0, 48",
        "st.d $s3, $a0, 56",
        "st.d $s4, $a0, 64",
        "st.d $s5, $a0, 72",
        "st.d $s6, $a0, 80",
        "st.d $s7, $a0, 88",
        "st.d $s8, $a0, 96",
        "fst.d $fs0, $a0, 104",
        "fst.d $fs1, $a0, 112",
        "fst.d $fs2, $a0, 120",
        "fst.d $fs3, $a0, 128",
        "fst.d $fs4, $a0, 136",
        "fst.d $fs5, $a0, 144",
        "fst.d $fs6, $a0, 152",
        "fst.d $fs7, $a0, 160",
        "move $a0, $zero",
        "jr $ra",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn _setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "b setjmp",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn __setjmp(_env: *mut c_ulong) -> c_int {
    core::arch::naked_asm!(
        "b setjmp",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn longjmp(_env: *const c_ulong, _val: c_int) -> ! {
    core::arch::naked_asm!(
        "ld.d $ra, $a0, 0",
        "ld.d $sp, $a0, 8",
        "ld.d $r21, $a0, 16",
        "ld.d $fp, $a0, 24",
        "ld.d $s0, $a0, 32",
        "ld.d $s1, $a0, 40",
        "ld.d $s2, $a0, 48",
        "ld.d $s3, $a0, 56",
        "ld.d $s4, $a0, 64",
        "ld.d $s5, $a0, 72",
        "ld.d $s6, $a0, 80",
        "ld.d $s7, $a0, 88",
        "ld.d $s8, $a0, 96",
        "fld.d $fs0, $a0, 104",
        "fld.d $fs1, $a0, 112",
        "fld.d $fs2, $a0, 120",
        "fld.d $fs3, $a0, 128",
        "fld.d $fs4, $a0, 136",
        "fld.d $fs5, $a0, 144",
        "fld.d $fs6, $a0, 152",
        "fld.d $fs7, $a0, 160",
        "sltui $a0, $a1, 1",
        "add.d $a0, $a0, $a1",
        "jr $ra",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "beq $a1, $zero, 2f",
        "st.d $ra, $a0, 184",
        "st.d $s0, $a0, 200",
        "move $s0, $a0",
        "la.local $t0, setjmp",
        "jirl $ra, $t0, 0",
        "move $a1, $a0",
        "move $a0, $s0",
        "ld.d $ra, $a0, 184",
        "ld.d $s0, $a0, 200",
        "b __sigsetjmp_tail",
        "2:",
        "la.local $t0, setjmp",
        "jr $t0",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
#[unsafe(naked)]
pub unsafe extern "C" fn __sigsetjmp(_env: *mut c_ulong, _savemask: c_int) -> c_int {
    core::arch::naked_asm!(
        "b sigsetjmp",
    );
}

#[cfg(target_arch = "loongarch64")]
#[no_mangle]
pub unsafe extern "C" fn __sigsetjmp_tail(env: *mut c_ulong, ret: c_int) -> c_int {
    const SIG_SETMASK: c_int = 2;
    let ss = env.add(24) as *mut SigSetT;
    let set = if ret != 0 { ss as *const SigSetT } else { core::ptr::null() };
    let old = if ret != 0 { core::ptr::null_mut() } else { ss };
    let _ = sys_rt_sigprocmask(SIG_SETMASK, set, old, core::mem::size_of::<SigSetT>());
    ret
}

