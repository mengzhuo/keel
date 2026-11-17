// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Architecture layer for the standalone loader.
//
// Used as a normal module from main.rs.  Contains the raw syscall ABI, the
// active Arch alias, SYS_MMAP and the loader's sys_mmap wrapper.

/// Linux returns -errno in [-4095, -1]; anything else is a valid pointer.
const SYSCALL_ERR_MAX: i64 = 4096;

#[cfg(target_arch = "x86_64")]
const SYS_ARCH_PRCTL: i64 = 158;

// ============================================================
// Syscall ABI
// ============================================================

#[allow(dead_code)]
pub trait Syscalls {
    unsafe fn syscall0(n: i64) -> i64;
    unsafe fn syscall1(n: i64, a1: i64) -> i64;
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64;
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64;
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64;
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64;
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64;
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> !;
}

#[cfg(target_arch = "x86_64")]
pub struct X86_64;

// x86_64
include!("arch_syscalls.rs");


#[cfg(target_arch = "aarch64")]
pub struct Aarch64;

// aarch64


#[cfg(target_arch = "riscv64")]
pub struct Riscv64;
#[cfg(target_arch = "powerpc64")]
pub struct PowerPc64;
#[cfg(target_arch = "loongarch64")]
pub struct LoongArch64;

// riscv64


// ============================================================
// Selected architecture
// ============================================================

// powerpc64


// loongarch64



// x86_64
#[cfg(target_arch = "x86_64")]
pub type Arch = X86_64;
// aarch64
#[cfg(target_arch = "aarch64")]
pub type Arch = Aarch64;
// riscv64
#[cfg(target_arch = "riscv64")]
pub type Arch = Riscv64;
// powerpc64
#[cfg(target_arch = "powerpc64")]
pub type Arch = PowerPc64;
// loongarch64
#[cfg(target_arch = "loongarch64")]
pub type Arch = LoongArch64;

// ============================================================
// Syscall numbers
// ============================================================

// x86_64
#[cfg(target_arch = "x86_64")]
mod sysnr {
    pub const SYS_MMAP: i64 = 9;
}
// aarch64
#[cfg(target_arch = "aarch64")]
mod sysnr {
    pub const SYS_MMAP: i64 = 222;
}
// riscv64
#[cfg(target_arch = "riscv64")]
mod sysnr {
    pub const SYS_MMAP: i64 = 222;
}
// powerpc64
#[cfg(target_arch = "powerpc64")]
mod sysnr {
    pub const SYS_MMAP: i64 = 90;
}
// loongarch64
#[cfg(target_arch = "loongarch64")]
mod sysnr {
    pub const SYS_MMAP: i64 = 222;
}
pub use sysnr::*;

// ============================================================
// Syscall wrappers
// ============================================================

/// Drop the loader's own thread pointer before jumping to a static target.
/// The target inherits the address space, and Rust's glibc TLS block is not
/// a keel TCB: leaving %fs set makes keel's static startup think TLS is
/// already initialized and skip building its own block.
#[inline]
pub unsafe fn clear_thread_pointer() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        const ARCH_SET_FS: i64 = 0x1002;
        let _ = <Arch as Syscalls>::syscall2(SYS_ARCH_PRCTL, ARCH_SET_FS, 0);
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr tpidr_el0, {}", in(reg) 0usize, options(nostack));
    }
    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!("mv tp, {}", in(reg) 0usize, options(nostack));
    }
}

#[inline]
pub unsafe fn sys_mmap(
    addr: *mut u8,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> *mut u8 {
    let result = unsafe {
        <Arch as Syscalls>::syscall6(
            SYS_MMAP,
            addr as i64,
            length as i64,
            prot as i64,
            flags as i64,
            fd as i64,
            offset,
        )
    };
    if result < 0 && result > -SYSCALL_ERR_MAX {
        !0 as *mut u8
    } else {
        result as *mut u8
    }
}
