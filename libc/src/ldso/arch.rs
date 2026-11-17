// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Architecture layer for the combined libc/loader.
//
// Included from lib.rs.  Contains the raw syscall ABI, the active Arch alias
// and the syscall numbers used by the loader.  The public syscall wrappers
// (sys_open, sys_mmap, ...) remain in lib.rs.

// ============================================================
// Architecture selection
// ============================================================

// ============================================================
// Syscall ABI
// ============================================================

trait Syscalls {
    unsafe fn syscall0(n: i64) -> i64;
    unsafe fn syscall1(n: i64, a1: i64) -> i64;
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64;
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64;
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64;
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64;
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64;
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> !;
}

struct X86_64;
struct Aarch64;
struct Riscv64;
struct PowerPc64;
struct LoongArch64;

// x86_64
include!("../arch_syscalls.rs");


// aarch64


// riscv64


// powerpc64


// loongarch64



// ============================================================
// Selected architecture
// ============================================================

// x86_64
#[cfg(target_arch = "x86_64")]
type Arch = X86_64;
// aarch64
#[cfg(target_arch = "aarch64")]
type Arch = Aarch64;
// riscv64
#[cfg(target_arch = "riscv64")]
type Arch = Riscv64;
// powerpc64
#[cfg(target_arch = "powerpc64")]
type Arch = PowerPc64;
// loongarch64
#[cfg(target_arch = "loongarch64")]
type Arch = LoongArch64;

// ============================================================
// Syscall numbers
// ============================================================

// x86_64
#[cfg(target_arch = "x86_64")]
mod sysnr {
    pub const SYS_READ: i64 = 0;
    pub const SYS_WRITE: i64 = 1;
    pub const SYS_OPENAT: i64 = 257;
    pub const SYS_CLOSE: i64 = 3;
    pub const SYS_LSEEK: i64 = 8;
    pub const SYS_MMAP: i64 = 9;
    pub const SYS_MUNMAP: i64 = 11;
    pub const SYS_FSTAT: i64 = 5;
    pub const SYS_READLINKAT: i64 = 267;
    pub const SYS_ARCH_PRCTL: i64 = 158;
    pub const SYS_GETRLIMIT: i64 = 97;
    pub const SYS_EXIT: i64 = 60;
}
// aarch64
#[cfg(target_arch = "aarch64")]
mod sysnr {
    pub const SYS_READ: i64 = 63;
    pub const SYS_WRITE: i64 = 64;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_CLOSE: i64 = 57;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_EXIT: i64 = 93;
}
// riscv64
#[cfg(target_arch = "riscv64")]
mod sysnr {
    pub const SYS_READ: i64 = 63;
    pub const SYS_WRITE: i64 = 64;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_CLOSE: i64 = 57;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_EXIT: i64 = 93;
}
// powerpc64
#[cfg(target_arch = "powerpc64")]
mod sysnr {
    pub const SYS_READ: i64 = 3;
    pub const SYS_WRITE: i64 = 4;
    pub const SYS_OPENAT: i64 = 286;
    pub const SYS_CLOSE: i64 = 6;
    pub const SYS_LSEEK: i64 = 19;
    pub const SYS_MMAP: i64 = 90;
    pub const SYS_MUNMAP: i64 = 91;
    pub const SYS_FSTAT: i64 = 108;
    pub const SYS_READLINKAT: i64 = 296;
    pub const SYS_GETRLIMIT: i64 = 76;
    pub const SYS_EXIT: i64 = 1;
}
// loongarch64
#[cfg(target_arch = "loongarch64")]
mod sysnr {
    pub const SYS_READ: i64 = 63;
    pub const SYS_WRITE: i64 = 64;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_CLOSE: i64 = 57;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_EXIT: i64 = 93;
}
pub use sysnr::*;
