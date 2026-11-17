// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Architecture layer for libc.
//
// Included from lib.rs.  Everything below is selected with target_arch:
// the syscall ABI, the active Arch alias and the syscall-number tables.
// Keep per-architecture changes in this file rather than scattering cfg
// blocks through the libc functional areas.

// ============================================================
// Architecture selection
// ============================================================

/// Linux returns -errno in [-4095, -1]; anything else is a valid pointer.
const SYSCALL_ERR_MAX: i64 = 4096;

#[cfg(target_arch = "x86_64")]
const SYS_ARCH_PRCTL: i64 = 158;

/// Read the thread pointer.  errno and static TLS use this before any
/// architecture-specific TLS helpers are available.
#[cfg(target_arch = "x86_64")]
unsafe fn arch_get_tp() -> usize {
    const ARCH_GET_FS: i64 = 0x1003;
    let mut tp: usize = 0;
    let _ = <Arch as Syscalls>::syscall2(SYS_ARCH_PRCTL, ARCH_GET_FS, &mut tp as *mut usize as i64);
    tp
}

#[cfg(target_arch = "aarch64")]
unsafe fn arch_get_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mrs {}, tpidr_el0", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "riscv64")]
unsafe fn arch_get_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mv {}, tp", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "powerpc64")]
unsafe fn arch_get_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mr {}, 13", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "loongarch64")]
unsafe fn arch_get_tp() -> usize {
    let tp: usize;
    core::arch::asm!("move {}, $tp", out(reg) tp, options(nostack, preserves_flags));
    tp
}

/// Read the thread pointer without a syscall.  Used on hot paths (vDSO
/// getrandom) where TLS is guaranteed to be set up already.
#[cfg(target_arch = "x86_64")]
unsafe fn arch_get_tp_fast() -> usize {
    let tp: usize;
    core::arch::asm!("mov {}, fs:[0]", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "aarch64")]
unsafe fn arch_get_tp_fast() -> usize {
    let tp: usize;
    core::arch::asm!("mrs {}, tpidr_el0", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "riscv64")]
unsafe fn arch_get_tp_fast() -> usize {
    let tp: usize;
    core::arch::asm!("mv {}, tp", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "powerpc64")]
unsafe fn arch_get_tp_fast() -> usize {
    let tp: usize;
    core::arch::asm!("mr {}, 13", out(reg) tp, options(nostack, preserves_flags));
    tp
}

#[cfg(target_arch = "loongarch64")]
unsafe fn arch_get_tp_fast() -> usize {
    let tp: usize;
    core::arch::asm!("move {}, $tp", out(reg) tp, options(nostack, preserves_flags));
    tp
}

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
#[cfg(target_arch = "x86_64")]
impl Syscalls for X86_64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            in("rsi") a2,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "syscall",
            inlateout("rax") n => result,
            in("rdi") a1,
            in("rsi") a2,
            in("rdx") a3,
            in("r10") a4,
            in("r8") a5,
            in("r9") a6,
            lateout("rcx") _,
            lateout("r11") _,
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        core::arch::asm!(
            "syscall",
            in("rax") n,
            in("rdi") a1,
            options(noreturn)
        );
    }
}

// aarch64
#[cfg(target_arch = "aarch64")]
impl Syscalls for Aarch64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            lateout("x0") result,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            inlateout("x1") a2 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            inlateout("x1") a2 => _,
            inlateout("x2") a3 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            inlateout("x1") a2 => _,
            inlateout("x2") a3 => _,
            inlateout("x3") a4 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            inlateout("x1") a2 => _,
            inlateout("x2") a3 => _,
            inlateout("x3") a4 => _,
            inlateout("x4") a5 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "svc #0",
            inlateout("x8") n => _,
            inlateout("x0") a1 => result,
            inlateout("x1") a2 => _,
            inlateout("x2") a3 => _,
            inlateout("x3") a4 => _,
            inlateout("x4") a5 => _,
            inlateout("x5") a6 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        core::arch::asm!(
            "svc #0",
            in("x8") n,
            in("x0") a1,
            options(noreturn, nostack),
        );
    }
}

// riscv64
#[cfg(target_arch = "riscv64")]
impl Syscalls for Riscv64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            lateout("a0") result,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            inlateout("a1") a2 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            inlateout("a1") a2 => _,
            inlateout("a2") a3 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            inlateout("a1") a2 => _,
            inlateout("a2") a3 => _,
            inlateout("a3") a4 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            inlateout("a1") a2 => _,
            inlateout("a2") a3 => _,
            inlateout("a3") a4 => _,
            inlateout("a4") a5 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "ecall",
            inlateout("a7") n => _,
            inlateout("a0") a1 => result,
            inlateout("a1") a2 => _,
            inlateout("a2") a3 => _,
            inlateout("a3") a4 => _,
            inlateout("a4") a5 => _,
            inlateout("a5") a6 => _,
            options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        core::arch::asm!(
            "ecall",
            in("a7") n,
            in("a0") a1,
            options(noreturn, nostack),
        );
    }
}


// powerpc64
#[cfg(target_arch = "powerpc64")]
impl Syscalls for PowerPc64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, lateout("r3") result,
            lateout("r4") _, lateout("r5") _, lateout("r6") _, lateout("r7") _,
            lateout("r8") _, lateout("r9") _, lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result,
            lateout("r4") _, lateout("r5") _, lateout("r6") _, lateout("r7") _,
            lateout("r8") _, lateout("r9") _, lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result, inlateout("r4") a2 => _,
            lateout("r5") _, lateout("r6") _, lateout("r7") _, lateout("r8") _,
            lateout("r9") _, lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result, inlateout("r4") a2 => _, inlateout("r5") a3 => _,
            lateout("r6") _, lateout("r7") _, lateout("r8") _, lateout("r9") _,
            lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result, inlateout("r4") a2 => _,
            inlateout("r5") a3 => _, inlateout("r6") a4 => _,
            lateout("r7") _, lateout("r8") _, lateout("r9") _, lateout("r10") _,
            lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result, inlateout("r4") a2 => _,
            inlateout("r5") a3 => _, inlateout("r6") a4 => _, inlateout("r7") a5 => _,
            lateout("r8") _, lateout("r9") _, lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        core::arch::asm!(
            "sc", "bns+ 1f", "neg 3, 3", "1:",
            inlateout("r0") n => _, inlateout("r3") a1 => result, inlateout("r4") a2 => _,
            inlateout("r5") a3 => _, inlateout("r6") a4 => _, inlateout("r7") a5 => _, inlateout("r8") a6 => _,
            lateout("r9") _, lateout("r10") _, lateout("r11") _, lateout("r12") _,
            out("cr0") _, out("ctr") _, out("xer") _, options(nostack),
        );
        result
    }
    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        core::arch::asm!("sc", in("r0") n, in("r3") a1, options(noreturn, nostack));
    }
}

// loongarch64
#[cfg(target_arch = "loongarch64")]
impl Syscalls for LoongArch64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") 0i64 => result, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a1") a2, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a1") a2, in("$a2") a3, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a1") a2, in("$a2") a3, in("$a3") a4, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a1") a2, in("$a2") a3, in("$a3") a4, in("$a4") a5, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        core::arch::asm!("syscall 0", inlateout("$a0") a1 => result, in("$a1") a2, in("$a2") a3, in("$a3") a4, in("$a4") a5, in("$a5") a6, in("$a7") n,
            lateout("$t0") _, lateout("$t1") _, lateout("$t2") _, lateout("$t3") _,
            lateout("$t4") _, lateout("$t5") _, lateout("$t6") _, lateout("$t7") _, lateout("$t8") _,
            options(nostack));
        result
    }
    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        core::arch::asm!("syscall 0", in("$a0") a1, in("$a7") n, options(noreturn, nostack));
    }
}

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
    pub const SYS_OPEN: i64 = 2;
    pub const SYS_CLOSE: i64 = 3;
    pub const SYS_STAT: i64 = 4;
    pub const SYS_FSTAT: i64 = 5;
    pub const SYS_LSEEK: i64 = 8;
    pub const SYS_MMAP: i64 = 9;
    pub const SYS_MUNMAP: i64 = 11;
    pub const SYS_MPROTECT: i64 = 10;
    pub const SYS_RT_SIGACTION: i64 = 13;
    pub const SYS_RT_SIGPROCMASK: i64 = 14;
    pub const SYS_IOCTL: i64 = 16;
    pub const SYS_ACCESS: i64 = 21;
    pub const SYS_SHMGET: i64 = 29;
    pub const SYS_SHMAT: i64 = 30;
    pub const SYS_SHMCTL: i64 = 31;
    pub const SYS_DUP: i64 = 32;
    pub const SYS_NANOSLEEP: i64 = 35;
    pub const SYS_SETITIMER: i64 = 38;
    pub const SYS_ALARM: i64 = 37;
    pub const SYS_SOCKET: i64 = 41;
    pub const SYS_CONNECT: i64 = 42;
    pub const SYS_ACCEPT: i64 = 43;
    pub const SYS_ACCEPT4: i64 = 288;
    pub const SYS_SENDTO: i64 = 44;
    pub const SYS_RECVFROM: i64 = 45;
    pub const SYS_SENDMSG: i64 = 46;
    pub const SYS_RECVMSG: i64 = 47;
    pub const SYS_SHUTDOWN: i64 = 48;
    pub const SYS_BIND: i64 = 49;
    pub const SYS_LISTEN: i64 = 50;
    pub const SYS_GETSOCKNAME: i64 = 51;
    pub const SYS_SOCKETPAIR: i64 = 53;
    pub const SYS_SETSOCKOPT: i64 = 54;
    pub const SYS_EXECVE: i64 = 59;
    pub const SYS_WAIT4: i64 = 61;
    pub const SYS_KILL: i64 = 62;
    pub const SYS_UNAME: i64 = 63;
    pub const SYS_SEMGET: i64 = 64;
    pub const SYS_SEMOP: i64 = 65;
    pub const SYS_SEMCTL: i64 = 66;
    pub const SYS_SHMDT: i64 = 67;
    pub const SYS_MSGGET: i64 = 68;
    pub const SYS_MSGSND: i64 = 69;
    pub const SYS_MSGRCV: i64 = 70;
    pub const SYS_MSGCTL: i64 = 71;
    pub const SYS_FCNTL: i64 = 72;
    pub const SYS_FSYNC: i64 = 74;
    pub const SYS_TRUNCATE: i64 = 76;
    pub const SYS_FTRUNCATE: i64 = 77;
    pub const SYS_GETCWD: i64 = 79;
    pub const SYS_CHDIR: i64 = 80;
    pub const SYS_SYMLINK: i64 = 88;
    pub const SYS_FCHMOD: i64 = 91;
    pub const SYS_UMASK: i64 = 95;
    pub const SYS_GETRLIMIT: i64 = 97;
    pub const SYS_SETUID: i64 = 105;
    pub const SYS_SETGID: i64 = 106;
    pub const SYS_SETPGID: i64 = 109;
    pub const SYS_GETGROUPS: i64 = 115;
    pub const SYS_GETPGID: i64 = 121;
    pub const SYS_GETSID: i64 = 124;
    pub const SYS_RT_SIGPENDING: i64 = 127;
    pub const SYS_RT_SIGTIMEDWAIT: i64 = 128;
    pub const SYS_RT_SIGSUSPEND: i64 = 130;
    pub const SYS_SIGALTSTACK: i64 = 131;
    pub const SYS_SETRLIMIT: i64 = 160;
    pub const SYS_SETHOSTNAME: i64 = 170;
    pub const SYS_FUTEX: i64 = 202;
    pub const SYS_CLOCK_GETTIME: i64 = 228;
    pub const SYS_CLOCK_GETRES: i64 = 229;
    pub const SYS_CLOCK_NANOSLEEP: i64 = 230;
    pub const SYS_EXIT_GROUP: i64 = 231;
    pub const SYS_TGKILL: i64 = 234;
    pub const SYS_MKDIRAT: i64 = 258;
    pub const SYS_NEWFSTATAT: i64 = 262;
    pub const SYS_UNLINKAT: i64 = 263;
    pub const SYS_LINKAT: i64 = 265;
    pub const SYS_SYMLINKAT: i64 = 266;
    pub const SYS_READLINKAT: i64 = 267;
    pub const SYS_FCHMODAT: i64 = 268;
    pub const SYS_FCHMODAT2: i64 = 452;
    pub const SYS_SET_ROBUST_LIST: i64 = 273;
    pub const SYS_UTIMENSAT: i64 = 280;
    pub const SYS_OPENAT: i64 = 257;
    pub const SYS_FACCESSAT: i64 = 269;
    pub const SYS_DUP3: i64 = 292;
    pub const SYS_PIPE2: i64 = 293;
    pub const SYS_SYNCFS: i64 = 306;
    pub const SYS_RENAMEAT2: i64 = 316;
    pub const SYS_STATFS: i64 = 137;
    pub const SYS_FSTATFS: i64 = 138;
    pub const SYS_FDATASYNC: i64 = 75;
    pub const SYS_PAUSE: i64 = 34;
    pub const SYS_POLL: i64 = 7;
    pub const SYS_PPOLL: i64 = 271;
    pub const SYS_PSELECT6: i64 = 270;
    pub const SYS_GETPID: i64 = 39;
    pub const SYS_FORK: i64 = 57;
    pub const SYS_EXIT: i64 = 60;
    pub const SYS_GETPPID: i64 = 110;
    pub const SYS_SETSID: i64 = 112;
    pub const SYS_SYNC: i64 = 162;
    pub const SYS_GETUID: i64 = 102;
    pub const SYS_GETGID: i64 = 104;
    pub const SYS_GETEUID: i64 = 107;
    pub const SYS_GETEGID: i64 = 108;
    pub const SYS_GETTID: i64 = 186;
    pub const SYS_CLOCK_SETTIME: i64 = 227;
    pub const SYS_CLONE: i64 = 56;
    pub const SYS_PREAD64: i64 = 17;
    pub const SYS_MREMAP: i64 = 25;
    pub const SYS_SYSINFO: i64 = 99;
    pub const SYS_MEMBARRIER: i64 = 324;
    pub const SYS_VHANGUP: i64 = 153;
    pub const SYS_VMSPLICE: i64 = 278;
    pub const SYS_GETDENTS64: i64 = 217;
    pub const SYS_EPOLL_CREATE: i64 = 213;
    pub const SYS_EPOLL_CREATE1: i64 = 291;
    pub const SYS_EPOLL_CTL: i64 = 233;
    pub const SYS_EPOLL_WAIT: i64 = 232;
    pub const SYS_EPOLL_PWAIT: i64 = 281;
    pub const SYS_EVENTFD2: i64 = 290;
    pub const SYS_SIGNALFD4: i64 = 289;
    pub const SYS_TIMERFD_CREATE: i64 = 283;
    pub const SYS_TIMERFD_SETTIME: i64 = 286;
    pub const SYS_TIMERFD_GETTIME: i64 = 287;
    pub const SYS_PRCTL: i64 = 157;
    pub const SYS_FALLOCATE: i64 = 285;
    pub const SYS_FLOCK: i64 = 73;
    pub const SYS_SETGROUPS: i64 = 116;
    pub const SYS_SETRESUID: i64 = 117;
    pub const SYS_SETRESGID: i64 = 119;
    pub const SYS_GETRESUID: i64 = 118;
    pub const SYS_GETRESGID: i64 = 120;
    pub const SYS_MKNOD: i64 = 133;
    pub const SYS_FCHDIR: i64 = 81;
    pub const SYS_FCHOWN: i64 = 93;
    pub const SYS_ACCT: i64 = 163;
    pub const SYS_ADJTIMEX: i64 = 159;
    pub const SYS_CAPGET: i64 = 125;
    pub const SYS_CAPSET: i64 = 126;
    pub const SYS_CHROOT: i64 = 161;
    pub const SYS_COPY_FILE_RANGE: i64 = 326;
    pub const SYS_DELETE_MODULE: i64 = 176;
    pub const SYS_INIT_MODULE: i64 = 175;
    pub const SYS_FCHOWNAT: i64 = 260;
    pub const SYS_MKNODAT: i64 = 259;
    pub const SYS_SETNS: i64 = 308;
    pub const SYS_UNSHARE: i64 = 272;
    pub const SYS_REBOOT: i64 = 169;
    pub const SYS_NICE: i64 = 34;
    pub const SYS_SETPRIORITY: i64 = 141;
    pub const SYS_GETPRIORITY: i64 = 140;
    pub const SYS_IOPERM: i64 = 173;
    pub const SYS_IOPL: i64 = 172;
    pub const SYS_SWAPON: i64 = 167;
    pub const SYS_SWAPOFF: i64 = 168;
    pub const SYS_MOUNT: i64 = 165;
    pub const SYS_UMOUNT2: i64 = 166;
    pub const SYS_PIVOT_ROOT: i64 = 155;
    pub const SYS_SETTIMEOFDAY: i64 = 164;
    pub const SYS_GETTIMEOFDAY: i64 = 96;

    // Less-common syscalls used by lib.rs.
    pub const SYS_SETREUID: i64 = 113;
    pub const SYS_QUOTACTL: i64 = 179;
    pub const SYS_SYSLOG: i64 = 103;
    pub const SYS_SYNC_FILE_RANGE: i64 = 277;
    pub const SYS_FADVISE64: i64 = 221;
    pub const SYS_READAHEAD: i64 = 187;
    pub const SYS_MQ_OPEN: i64 = 240;
    pub const SYS_MQ_UNLINK: i64 = 241;
    pub const SYS_MQ_TIMEDSEND: i64 = 242;
    pub const SYS_MQ_TIMEDRECEIVE: i64 = 243;
    pub const SYS_MQ_NOTIFY: i64 = 244;
    pub const SYS_MQ_GETSETATTR: i64 = 245;
    pub const SYS_SCHED_YIELD: i64 = 24;
    pub const SYS_SCHED_GET_PRIORITY_MAX: i64 = 146;
    pub const SYS_SCHED_GET_PRIORITY_MIN: i64 = 147;
    pub const SYS_SCHED_GETPARAM: i64 = 143;
    pub const SYS_SCHED_SETPARAM: i64 = 142;
    pub const SYS_SCHED_GETSCHEDULER: i64 = 145;
    pub const SYS_SCHED_SETSCHEDULER: i64 = 144;
    pub const SYS_SCHED_SETAFFINITY: i64 = 203;
    pub const SYS_SCHED_GETAFFINITY: i64 = 204;
    pub const SYS_GETCPU: i64 = 309;
    pub const SYS_SCHED_RR_GET_INTERVAL: i64 = 148;
    pub const SYS_TIMER_CREATE: i64 = 222;
    pub const SYS_TIMER_DELETE: i64 = 226;
    pub const SYS_TIMER_SETTIME: i64 = 223;
    pub const SYS_TIMER_GETTIME: i64 = 224;
    pub const SYS_TIMER_GETOVERRUN: i64 = 225;
    pub const SYS_INOTIFY_INIT: i64 = 253;
    pub const SYS_INOTIFY_INIT1: i64 = 294;
    pub const SYS_INOTIFY_ADD_WATCH: i64 = 254;
    pub const SYS_INOTIFY_RM_WATCH: i64 = 255;
    pub const SYS_FANOTIFY_INIT: i64 = 300;
    pub const SYS_FANOTIFY_MARK: i64 = 301;
    pub const SYS_GETRANDOM: i64 = 318;
    pub const SYS_PWRITE64: i64 = 18;
    pub const SYS_READV: i64 = 19;
    pub const SYS_WRITEV: i64 = 20;
    pub const SYS_SETDOMAINNAME: i64 = 171;
    pub const SYS_SENDMMSG: i64 = 307;
    pub const SYS_RECVMMSG: i64 = 299;
    pub const SYS_GETPEERNAME: i64 = 52;
    pub const SYS_GETSOCKOPT: i64 = 55;
    pub const SYS_GETITIMER: i64 = 36;
    pub const SYS_GETRUSAGE: i64 = 98;
    pub const SYS_PRLIMIT64: i64 = 302;
    pub const SYS_GETXATTR: i64 = 191;
    pub const SYS_LGETXATTR: i64 = 192;
    pub const SYS_FGETXATTR: i64 = 193;
    pub const SYS_LISTXATTR: i64 = 194;
    pub const SYS_LLISTXATTR: i64 = 195;
    pub const SYS_FLISTXATTR: i64 = 196;
    pub const SYS_SETXATTR: i64 = 188;
    pub const SYS_LSETXATTR: i64 = 189;
    pub const SYS_FSETXATTR: i64 = 190;
    pub const SYS_REMOVEXATTR: i64 = 197;
    pub const SYS_LREMOVEXATTR: i64 = 198;
    pub const SYS_FREMOVEXATTR: i64 = 199;
    pub const SYS_BRK: i64 = 12;
    pub const SYS_PTRACE: i64 = 101;
    pub const SYS_PERSONALITY: i64 = 135;
    pub const SYS_PREADV: i64 = 295;
    pub const SYS_PWRITEV: i64 = 296;
    pub const SYS_PREADV2: i64 = 327;
    pub const SYS_PWRITEV2: i64 = 328;
    pub const SYS_MEMFD_CREATE: i64 = 319;
    pub const SYS_MLOCK2: i64 = 325;
    pub const SYS_REMAP_FILE_PAGES: i64 = 216;
    pub const SYS_SELECT: i64 = 23;
    pub const SYS_SENDFILE: i64 = 40;
    pub const SYS_WAITID: i64 = 247;
    pub const SYS_MSYNC: i64 = 26;
    pub const SYS_MLOCK: i64 = 149;
    pub const SYS_MUNLOCK: i64 = 150;
    pub const SYS_MLOCKALL: i64 = 151;
    pub const SYS_MUNLOCKALL: i64 = 152;
    pub const SYS_MADVISE: i64 = 28;
    pub const SYS_MINCORE: i64 = 27;
    pub const SYS_TIMES: i64 = 100;
    pub const SYS_SPLICE: i64 = 275;
    pub const SYS_TEE: i64 = 276;
    pub const SYS_STATX: i64 = 332;
    pub const SYS_SETFSUID: i64 = 122;
    pub const SYS_SETFSGID: i64 = 123;
    pub const SYS_SETREGID: i64 = 114;
    pub const SYS_PROCESS_VM_READV: i64 = 310;
    pub const SYS_PROCESS_VM_WRITEV: i64 = 311;
    pub const SYS_SEMTIMEDOP: i64 = 220;
    pub const SYS_NAME_TO_HANDLE_AT: i64 = 303;
    pub const SYS_OPEN_BY_HANDLE_AT: i64 = 304;
    pub const SYS_RT_SIGQUEUEINFO: i64 = 129;
}
// aarch64
#[cfg(target_arch = "aarch64")]
mod sysnr {
    pub const SYS_READ: i64 = 63;
    pub const SYS_WRITE: i64 = 64;
    // pub const SYS_OPEN: i64 = ???; // missing in aarch64 table
    pub const SYS_CLOSE: i64 = 57;
    // pub const SYS_STAT: i64 = ???; // missing in aarch64 table
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_RT_SIGACTION: i64 = 134;
    pub const SYS_RT_SIGPROCMASK: i64 = 135;
    pub const SYS_IOCTL: i64 = 29;
    // pub const SYS_ACCESS: i64 = ???; // missing in aarch64 table
    pub const SYS_SHMGET: i64 = 194;
    pub const SYS_SHMAT: i64 = 196;
    pub const SYS_SHMCTL: i64 = 195;
    pub const SYS_DUP: i64 = 23;
    pub const SYS_NANOSLEEP: i64 = 101;
    pub const SYS_SETITIMER: i64 = 103;
    // pub const SYS_ALARM: i64 = ???; // missing in aarch64 table
    pub const SYS_SOCKET: i64 = 198;
    pub const SYS_CONNECT: i64 = 203;
    pub const SYS_ACCEPT: i64 = 202;
    pub const SYS_SENDTO: i64 = 206;
    pub const SYS_RECVFROM: i64 = 207;
    pub const SYS_SHUTDOWN: i64 = 210;
    pub const SYS_BIND: i64 = 200;
    pub const SYS_LISTEN: i64 = 201;
    pub const SYS_GETSOCKNAME: i64 = 204;
    pub const SYS_SOCKETPAIR: i64 = 199;
    pub const SYS_SETSOCKOPT: i64 = 208;
    pub const SYS_EXECVE: i64 = 221;
    pub const SYS_WAIT4: i64 = 260;
    pub const SYS_KILL: i64 = 129;
    pub const SYS_UNAME: i64 = 160;
    pub const SYS_SEMGET: i64 = 190;
    pub const SYS_SEMOP: i64 = 193;
    pub const SYS_SEMCTL: i64 = 191;
    pub const SYS_SHMDT: i64 = 197;
    pub const SYS_MSGGET: i64 = 186;
    pub const SYS_MSGSND: i64 = 189;
    pub const SYS_MSGRCV: i64 = 188;
    pub const SYS_MSGCTL: i64 = 187;
    pub const SYS_FCNTL: i64 = 25;
    pub const SYS_FSYNC: i64 = 82;
    pub const SYS_TRUNCATE: i64 = 45;
    pub const SYS_FTRUNCATE: i64 = 46;
    pub const SYS_GETCWD: i64 = 17;
    pub const SYS_CHDIR: i64 = 49;
    // pub const SYS_SYMLINK: i64 = ???; // missing in aarch64 table
    pub const SYS_FCHMOD: i64 = 52;
    pub const SYS_UMASK: i64 = 166;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_SETUID: i64 = 146;
    pub const SYS_SETGID: i64 = 144;
    pub const SYS_SETPGID: i64 = 154;
    pub const SYS_GETGROUPS: i64 = 158;
    pub const SYS_GETPGID: i64 = 155;
    pub const SYS_GETSID: i64 = 156;
    pub const SYS_RT_SIGPENDING: i64 = 136;
    pub const SYS_RT_SIGTIMEDWAIT: i64 = 137;
    pub const SYS_RT_SIGSUSPEND: i64 = 133;
    pub const SYS_SIGALTSTACK: i64 = 132;
    pub const SYS_SETRLIMIT: i64 = 164;
    pub const SYS_SETHOSTNAME: i64 = 161;
    pub const SYS_FUTEX: i64 = 98;
    pub const SYS_CLOCK_GETTIME: i64 = 113;
    pub const SYS_CLOCK_GETRES: i64 = 114;
    pub const SYS_CLOCK_NANOSLEEP: i64 = 115;
    pub const SYS_EXIT_GROUP: i64 = 94;
    pub const SYS_TGKILL: i64 = 131;
    pub const SYS_MKDIRAT: i64 = 34;
    pub const SYS_NEWFSTATAT: i64 = 79;
    pub const SYS_UNLINKAT: i64 = 35;
    pub const SYS_LINKAT: i64 = 37;
    pub const SYS_SYMLINKAT: i64 = 36;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_FCHMODAT: i64 = 53;
    pub const SYS_FCHMODAT2: i64 = 452;
    pub const SYS_SET_ROBUST_LIST: i64 = 99;
    pub const SYS_UTIMENSAT: i64 = 88;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_FACCESSAT: i64 = 48;
    pub const SYS_DUP3: i64 = 24;
    pub const SYS_PIPE2: i64 = 59;
    pub const SYS_SYNCFS: i64 = 267;
    pub const SYS_RENAMEAT2: i64 = 276;
    pub const SYS_STATFS: i64 = 43;
    pub const SYS_FSTATFS: i64 = 44;
    pub const SYS_FDATASYNC: i64 = 83;
    pub const SYS_GETPID: i64 = 172;
    pub const SYS_EXIT: i64 = 93;
    pub const SYS_GETPPID: i64 = 173;
    pub const SYS_SETSID: i64 = 157;
    pub const SYS_SYNC: i64 = 81;
    pub const SYS_GETUID: i64 = 174;
    pub const SYS_GETGID: i64 = 176;
    pub const SYS_GETEUID: i64 = 175;
    pub const SYS_GETEGID: i64 = 177;
    pub const SYS_GETTID: i64 = 178;
    pub const SYS_CLOCK_SETTIME: i64 = 112;
    pub const SYS_CLONE: i64 = 220;
    pub const SYS_PPOLL: i64 = 73;
    pub const SYS_PREAD64: i64 = 67;
    pub const SYS_MREMAP: i64 = 216;
    pub const SYS_SYSINFO: i64 = 179;
    pub const SYS_MEMBARRIER: i64 = 283;
    pub const SYS_VHANGUP: i64 = 58;
    pub const SYS_VMSPLICE: i64 = 75;
    pub const SYS_GETTIMEOFDAY: i64 = 169;
    pub const SYS_SETTIMEOFDAY: i64 = 170;
    pub const SYS_ADJTIMEX: i64 = 171;
    pub const SYS_EPOLL_CREATE1: i64 = 20;
    pub const SYS_EPOLL_CTL: i64 = 21;
    pub const SYS_EPOLL_WAIT: i64 = 22;
    pub const SYS_EPOLL_PWAIT: i64 = 22;
    pub const SYS_EVENTFD2: i64 = 19;
    pub const SYS_SIGNALFD4: i64 = 74;
    pub const SYS_TIMERFD_CREATE: i64 = 85;
    pub const SYS_TIMERFD_SETTIME: i64 = 86;
    pub const SYS_TIMERFD_GETTIME: i64 = 87;
    pub const SYS_ACCEPT4: i64 = 242;
    pub const SYS_ACCT: i64 = 89;
    pub const SYS_CAPGET: i64 = 90;
    pub const SYS_CAPSET: i64 = 91;
    pub const SYS_CHROOT: i64 = 51;
    pub const SYS_COPY_FILE_RANGE: i64 = 285;
    pub const SYS_DELETE_MODULE: i64 = 106;
    pub const SYS_FALLOCATE: i64 = 47;
    pub const SYS_FCHDIR: i64 = 50;
    pub const SYS_FCHOWN: i64 = 55;
    pub const SYS_FCHOWNAT: i64 = 54;
    pub const SYS_FLOCK: i64 = 32;
    pub const SYS_GETDENTS64: i64 = 61;
    pub const SYS_GETPRIORITY: i64 = 141;
    pub const SYS_GETRESGID: i64 = 150;
    pub const SYS_GETRESUID: i64 = 148;
    pub const SYS_INIT_MODULE: i64 = 105;
    pub const SYS_MKNODAT: i64 = 33;
    pub const SYS_MOUNT: i64 = 40;
    pub const SYS_MPROTECT: i64 = 226;
    pub const SYS_PIVOT_ROOT: i64 = 41;
    pub const SYS_PRCTL: i64 = 167;
    pub const SYS_PSELECT6: i64 = 72;
    pub const SYS_REBOOT: i64 = 142;
    pub const SYS_RECVMSG: i64 = 212;
    pub const SYS_SENDMSG: i64 = 211;
    pub const SYS_SETGROUPS: i64 = 159;
    pub const SYS_SETNS: i64 = 268;
    pub const SYS_SETPRIORITY: i64 = 140;
    pub const SYS_SETRESGID: i64 = 149;
    pub const SYS_SETRESUID: i64 = 147;
    pub const SYS_SWAPOFF: i64 = 225;
    pub const SYS_SWAPON: i64 = 224;
    pub const SYS_UMOUNT2: i64 = 39;
    pub const SYS_UNSHARE: i64 = 97;

    // Less-common syscalls used by lib.rs.
    pub const SYS_SETREUID: i64 = 145;
    pub const SYS_QUOTACTL: i64 = 60;
    pub const SYS_SYSLOG: i64 = 116;
    pub const SYS_SYNC_FILE_RANGE: i64 = 84;
    pub const SYS_FADVISE64: i64 = 223;
    pub const SYS_READAHEAD: i64 = 213;
    pub const SYS_MQ_OPEN: i64 = 180;
    pub const SYS_MQ_UNLINK: i64 = 181;
    pub const SYS_MQ_TIMEDSEND: i64 = 182;
    pub const SYS_MQ_TIMEDRECEIVE: i64 = 183;
    pub const SYS_MQ_NOTIFY: i64 = 184;
    pub const SYS_MQ_GETSETATTR: i64 = 185;
    pub const SYS_SCHED_YIELD: i64 = 124;
    pub const SYS_SCHED_GET_PRIORITY_MAX: i64 = 125;
    pub const SYS_SCHED_GET_PRIORITY_MIN: i64 = 126;
    pub const SYS_SCHED_GETPARAM: i64 = 121;
    pub const SYS_SCHED_SETPARAM: i64 = 118;
    pub const SYS_SCHED_GETSCHEDULER: i64 = 120;
    pub const SYS_SCHED_SETSCHEDULER: i64 = 119;
    pub const SYS_SCHED_SETAFFINITY: i64 = 122;
    pub const SYS_SCHED_GETAFFINITY: i64 = 123;
    pub const SYS_GETCPU: i64 = 168;
    pub const SYS_SCHED_RR_GET_INTERVAL: i64 = 127;
    pub const SYS_TIMER_CREATE: i64 = 107;
    pub const SYS_TIMER_DELETE: i64 = 111;
    pub const SYS_TIMER_SETTIME: i64 = 110;
    pub const SYS_TIMER_GETTIME: i64 = 108;
    pub const SYS_TIMER_GETOVERRUN: i64 = 109;
    pub const SYS_INOTIFY_INIT: i64 = 26;
    pub const SYS_INOTIFY_INIT1: i64 = 26;
    pub const SYS_INOTIFY_ADD_WATCH: i64 = 27;
    pub const SYS_INOTIFY_RM_WATCH: i64 = 28;
    pub const SYS_FANOTIFY_INIT: i64 = 262;
    pub const SYS_FANOTIFY_MARK: i64 = 263;
    pub const SYS_GETRANDOM: i64 = 278;
    pub const SYS_PWRITE64: i64 = 68;
    pub const SYS_READV: i64 = 65;
    pub const SYS_WRITEV: i64 = 66;
    pub const SYS_SETDOMAINNAME: i64 = 162;
    pub const SYS_SENDMMSG: i64 = 269;
    pub const SYS_RECVMMSG: i64 = 243;
    pub const SYS_GETPEERNAME: i64 = 205;
    pub const SYS_GETSOCKOPT: i64 = 209;
    pub const SYS_GETITIMER: i64 = 102;
    pub const SYS_GETRUSAGE: i64 = 165;
    pub const SYS_PRLIMIT64: i64 = 261;
    pub const SYS_GETXATTR: i64 = 8;
    pub const SYS_LGETXATTR: i64 = 9;
    pub const SYS_FGETXATTR: i64 = 10;
    pub const SYS_LISTXATTR: i64 = 11;
    pub const SYS_LLISTXATTR: i64 = 12;
    pub const SYS_FLISTXATTR: i64 = 13;
    pub const SYS_SETXATTR: i64 = 5;
    pub const SYS_LSETXATTR: i64 = 6;
    pub const SYS_FSETXATTR: i64 = 7;
    pub const SYS_REMOVEXATTR: i64 = 14;
    pub const SYS_LREMOVEXATTR: i64 = 15;
    pub const SYS_FREMOVEXATTR: i64 = 16;
    pub const SYS_BRK: i64 = 214;
    pub const SYS_PTRACE: i64 = 117;
    pub const SYS_PERSONALITY: i64 = 92;
    pub const SYS_PREADV: i64 = 69;
    pub const SYS_PWRITEV: i64 = 70;
    pub const SYS_PREADV2: i64 = 286;
    pub const SYS_PWRITEV2: i64 = 287;
    pub const SYS_MEMFD_CREATE: i64 = 279;
    pub const SYS_MLOCK2: i64 = 284;
    pub const SYS_REMAP_FILE_PAGES: i64 = 234;
    pub const SYS_SELECT: i64 = 72;
    pub const SYS_SENDFILE: i64 = 71;
    pub const SYS_WAITID: i64 = 95;
    pub const SYS_MSYNC: i64 = 227;
    pub const SYS_MLOCK: i64 = 228;
    pub const SYS_MUNLOCK: i64 = 229;
    pub const SYS_MLOCKALL: i64 = 230;
    pub const SYS_MUNLOCKALL: i64 = 231;
    pub const SYS_MADVISE: i64 = 233;
    pub const SYS_MINCORE: i64 = 232;
    pub const SYS_TIMES: i64 = 153;
    pub const SYS_SPLICE: i64 = 76;
    pub const SYS_TEE: i64 = 77;
    pub const SYS_STATX: i64 = 291;
    pub const SYS_SETFSUID: i64 = 151;
    pub const SYS_SETFSGID: i64 = 152;
    pub const SYS_SETREGID: i64 = 143;
    pub const SYS_PROCESS_VM_READV: i64 = 270;
    pub const SYS_PROCESS_VM_WRITEV: i64 = 271;
    pub const SYS_SEMTIMEDOP: i64 = 192;
    pub const SYS_NAME_TO_HANDLE_AT: i64 = 264;
    pub const SYS_OPEN_BY_HANDLE_AT: i64 = 265;
    pub const SYS_RT_SIGQUEUEINFO: i64 = 138;
}
// riscv64 uses the same new-style syscall table as aarch64
// riscv64 uses the same new-style syscall table as aarch64
#[cfg(target_arch = "riscv64")]
mod sysnr {
    pub const SYS_READ: i64 = 63;
    pub const SYS_WRITE: i64 = 64;
    pub const SYS_CLOSE: i64 = 57;
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_RT_SIGACTION: i64 = 134;
    pub const SYS_RT_SIGPROCMASK: i64 = 135;
    pub const SYS_IOCTL: i64 = 29;
    pub const SYS_SHMGET: i64 = 194;
    pub const SYS_SHMAT: i64 = 196;
    pub const SYS_SHMCTL: i64 = 195;
    pub const SYS_DUP: i64 = 23;
    pub const SYS_NANOSLEEP: i64 = 101;
    pub const SYS_SETITIMER: i64 = 103;
    pub const SYS_SOCKET: i64 = 198;
    pub const SYS_CONNECT: i64 = 203;
    pub const SYS_ACCEPT: i64 = 202;
    pub const SYS_SENDTO: i64 = 206;
    pub const SYS_RECVFROM: i64 = 207;
    pub const SYS_SHUTDOWN: i64 = 210;
    pub const SYS_BIND: i64 = 200;
    pub const SYS_LISTEN: i64 = 201;
    pub const SYS_GETSOCKNAME: i64 = 204;
    pub const SYS_SOCKETPAIR: i64 = 199;
    pub const SYS_SETSOCKOPT: i64 = 208;
    pub const SYS_EXECVE: i64 = 221;
    pub const SYS_WAIT4: i64 = 260;
    pub const SYS_KILL: i64 = 129;
    pub const SYS_UNAME: i64 = 160;
    pub const SYS_SEMGET: i64 = 190;
    pub const SYS_SEMOP: i64 = 193;
    pub const SYS_SEMCTL: i64 = 191;
    pub const SYS_SHMDT: i64 = 197;
    pub const SYS_MSGGET: i64 = 186;
    pub const SYS_MSGSND: i64 = 189;
    pub const SYS_MSGRCV: i64 = 188;
    pub const SYS_MSGCTL: i64 = 187;
    pub const SYS_FCNTL: i64 = 25;
    pub const SYS_FSYNC: i64 = 82;
    pub const SYS_TRUNCATE: i64 = 45;
    pub const SYS_FTRUNCATE: i64 = 46;
    pub const SYS_GETCWD: i64 = 17;
    pub const SYS_CHDIR: i64 = 49;
    pub const SYS_FCHMOD: i64 = 52;
    pub const SYS_UMASK: i64 = 166;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_SETUID: i64 = 146;
    pub const SYS_SETGID: i64 = 144;
    pub const SYS_SETPGID: i64 = 154;
    pub const SYS_GETGROUPS: i64 = 158;
    pub const SYS_GETPGID: i64 = 155;
    pub const SYS_GETSID: i64 = 156;
    pub const SYS_RT_SIGPENDING: i64 = 136;
    pub const SYS_RT_SIGTIMEDWAIT: i64 = 137;
    pub const SYS_RT_SIGSUSPEND: i64 = 133;
    pub const SYS_SIGALTSTACK: i64 = 132;
    pub const SYS_SETRLIMIT: i64 = 164;
    pub const SYS_SETHOSTNAME: i64 = 161;
    pub const SYS_FUTEX: i64 = 98;
    pub const SYS_CLOCK_GETTIME: i64 = 113;
    pub const SYS_CLOCK_GETRES: i64 = 114;
    pub const SYS_CLOCK_NANOSLEEP: i64 = 115;
    pub const SYS_EXIT_GROUP: i64 = 94;
    pub const SYS_TGKILL: i64 = 131;
    pub const SYS_MKDIRAT: i64 = 34;
    pub const SYS_NEWFSTATAT: i64 = 79;
    pub const SYS_UNLINKAT: i64 = 35;
    pub const SYS_LINKAT: i64 = 37;
    pub const SYS_SYMLINKAT: i64 = 36;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_FCHMODAT: i64 = 53;
    pub const SYS_FCHMODAT2: i64 = 452;
    pub const SYS_SET_ROBUST_LIST: i64 = 99;
    pub const SYS_UTIMENSAT: i64 = 88;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_FACCESSAT: i64 = 48;
    pub const SYS_DUP3: i64 = 24;
    pub const SYS_PIPE2: i64 = 59;
    pub const SYS_SYNCFS: i64 = 267;
    pub const SYS_RENAMEAT2: i64 = 276;
    pub const SYS_STATFS: i64 = 43;
    pub const SYS_FSTATFS: i64 = 44;
    pub const SYS_FDATASYNC: i64 = 83;
    pub const SYS_GETPID: i64 = 172;
    pub const SYS_EXIT: i64 = 93;
    pub const SYS_GETPPID: i64 = 173;
    pub const SYS_SETSID: i64 = 157;
    pub const SYS_SYNC: i64 = 81;
    pub const SYS_GETUID: i64 = 174;
    pub const SYS_GETGID: i64 = 176;
    pub const SYS_GETEUID: i64 = 175;
    pub const SYS_GETEGID: i64 = 177;
    pub const SYS_GETTID: i64 = 178;
    pub const SYS_CLOCK_SETTIME: i64 = 112;
    pub const SYS_CLONE: i64 = 220;
    pub const SYS_PPOLL: i64 = 73;
    pub const SYS_PREAD64: i64 = 67;
    pub const SYS_MREMAP: i64 = 216;
    pub const SYS_SYSINFO: i64 = 179;
    pub const SYS_MEMBARRIER: i64 = 283;
    pub const SYS_VHANGUP: i64 = 58;
    pub const SYS_VMSPLICE: i64 = 75;
    pub const SYS_GETTIMEOFDAY: i64 = 169;
    pub const SYS_SETTIMEOFDAY: i64 = 170;
    pub const SYS_ADJTIMEX: i64 = 171;
    pub const SYS_EPOLL_CREATE1: i64 = 20;
    pub const SYS_EPOLL_CTL: i64 = 21;
    pub const SYS_EPOLL_WAIT: i64 = 22;
    pub const SYS_EPOLL_PWAIT: i64 = 22;
    pub const SYS_EVENTFD2: i64 = 19;
    pub const SYS_SIGNALFD4: i64 = 74;
    pub const SYS_TIMERFD_CREATE: i64 = 85;
    pub const SYS_TIMERFD_SETTIME: i64 = 86;
    pub const SYS_TIMERFD_GETTIME: i64 = 87;
    pub const SYS_ACCEPT4: i64 = 242;
    pub const SYS_ACCT: i64 = 89;
    pub const SYS_CAPGET: i64 = 90;
    pub const SYS_CAPSET: i64 = 91;
    pub const SYS_CHROOT: i64 = 51;
    pub const SYS_COPY_FILE_RANGE: i64 = 285;
    pub const SYS_DELETE_MODULE: i64 = 106;
    pub const SYS_FALLOCATE: i64 = 47;
    pub const SYS_FCHDIR: i64 = 50;
    pub const SYS_FCHOWN: i64 = 55;
    pub const SYS_FCHOWNAT: i64 = 54;
    pub const SYS_FLOCK: i64 = 32;
    pub const SYS_GETDENTS64: i64 = 61;
    pub const SYS_GETPRIORITY: i64 = 141;
    pub const SYS_GETRESGID: i64 = 150;
    pub const SYS_GETRESUID: i64 = 148;
    pub const SYS_INIT_MODULE: i64 = 105;
    pub const SYS_MKNODAT: i64 = 33;
    pub const SYS_MOUNT: i64 = 40;
    pub const SYS_MPROTECT: i64 = 226;
    pub const SYS_PIVOT_ROOT: i64 = 41;
    pub const SYS_PRCTL: i64 = 167;
    pub const SYS_PSELECT6: i64 = 72;
    pub const SYS_REBOOT: i64 = 142;
    pub const SYS_RECVMSG: i64 = 212;
    pub const SYS_SENDMSG: i64 = 211;
    pub const SYS_SETGROUPS: i64 = 159;
    pub const SYS_SETNS: i64 = 268;
    pub const SYS_SETPRIORITY: i64 = 140;
    pub const SYS_SETRESGID: i64 = 149;
    pub const SYS_SETRESUID: i64 = 147;
    pub const SYS_SWAPOFF: i64 = 225;
    pub const SYS_SWAPON: i64 = 224;
    pub const SYS_UMOUNT2: i64 = 39;
    pub const SYS_UNSHARE: i64 = 97;

    // Less-common syscalls used by lib.rs.
    pub const SYS_SETREUID: i64 = 145;
    pub const SYS_QUOTACTL: i64 = 60;
    pub const SYS_SYSLOG: i64 = 116;
    pub const SYS_SYNC_FILE_RANGE: i64 = 84;
    pub const SYS_FADVISE64: i64 = 223;
    pub const SYS_READAHEAD: i64 = 213;
    pub const SYS_MQ_OPEN: i64 = 180;
    pub const SYS_MQ_UNLINK: i64 = 181;
    pub const SYS_MQ_TIMEDSEND: i64 = 182;
    pub const SYS_MQ_TIMEDRECEIVE: i64 = 183;
    pub const SYS_MQ_NOTIFY: i64 = 184;
    pub const SYS_MQ_GETSETATTR: i64 = 185;
    pub const SYS_SCHED_YIELD: i64 = 124;
    pub const SYS_SCHED_GET_PRIORITY_MAX: i64 = 125;
    pub const SYS_SCHED_GET_PRIORITY_MIN: i64 = 126;
    pub const SYS_SCHED_GETPARAM: i64 = 121;
    pub const SYS_SCHED_SETPARAM: i64 = 118;
    pub const SYS_SCHED_GETSCHEDULER: i64 = 120;
    pub const SYS_SCHED_SETSCHEDULER: i64 = 119;
    pub const SYS_SCHED_SETAFFINITY: i64 = 122;
    pub const SYS_SCHED_GETAFFINITY: i64 = 123;
    pub const SYS_GETCPU: i64 = 168;
    pub const SYS_SCHED_RR_GET_INTERVAL: i64 = 127;
    pub const SYS_TIMER_CREATE: i64 = 107;
    pub const SYS_TIMER_DELETE: i64 = 111;
    pub const SYS_TIMER_SETTIME: i64 = 110;
    pub const SYS_TIMER_GETTIME: i64 = 108;
    pub const SYS_TIMER_GETOVERRUN: i64 = 109;
    pub const SYS_INOTIFY_INIT: i64 = 26;
    pub const SYS_INOTIFY_INIT1: i64 = 26;
    pub const SYS_INOTIFY_ADD_WATCH: i64 = 27;
    pub const SYS_INOTIFY_RM_WATCH: i64 = 28;
    pub const SYS_FANOTIFY_INIT: i64 = 262;
    pub const SYS_FANOTIFY_MARK: i64 = 263;
    pub const SYS_GETRANDOM: i64 = 278;
    pub const SYS_PWRITE64: i64 = 68;
    pub const SYS_READV: i64 = 65;
    pub const SYS_WRITEV: i64 = 66;
    pub const SYS_SETDOMAINNAME: i64 = 162;
    pub const SYS_SENDMMSG: i64 = 269;
    pub const SYS_RECVMMSG: i64 = 243;
    pub const SYS_GETPEERNAME: i64 = 205;
    pub const SYS_GETSOCKOPT: i64 = 209;
    pub const SYS_GETITIMER: i64 = 102;
    pub const SYS_GETRUSAGE: i64 = 165;
    pub const SYS_PRLIMIT64: i64 = 261;
    pub const SYS_GETXATTR: i64 = 8;
    pub const SYS_LGETXATTR: i64 = 9;
    pub const SYS_FGETXATTR: i64 = 10;
    pub const SYS_LISTXATTR: i64 = 11;
    pub const SYS_LLISTXATTR: i64 = 12;
    pub const SYS_FLISTXATTR: i64 = 13;
    pub const SYS_SETXATTR: i64 = 5;
    pub const SYS_LSETXATTR: i64 = 6;
    pub const SYS_FSETXATTR: i64 = 7;
    pub const SYS_REMOVEXATTR: i64 = 14;
    pub const SYS_LREMOVEXATTR: i64 = 15;
    pub const SYS_FREMOVEXATTR: i64 = 16;
    pub const SYS_BRK: i64 = 214;
    pub const SYS_PTRACE: i64 = 117;
    pub const SYS_PERSONALITY: i64 = 92;
    pub const SYS_PREADV: i64 = 69;
    pub const SYS_PWRITEV: i64 = 70;
    pub const SYS_PREADV2: i64 = 286;
    pub const SYS_PWRITEV2: i64 = 287;
    pub const SYS_MEMFD_CREATE: i64 = 279;
    pub const SYS_MLOCK2: i64 = 284;
    pub const SYS_REMAP_FILE_PAGES: i64 = 234;
    pub const SYS_SELECT: i64 = 72;
    pub const SYS_SENDFILE: i64 = 71;
    pub const SYS_WAITID: i64 = 95;
    pub const SYS_MSYNC: i64 = 227;
    pub const SYS_MLOCK: i64 = 228;
    pub const SYS_MUNLOCK: i64 = 229;
    pub const SYS_MLOCKALL: i64 = 230;
    pub const SYS_MUNLOCKALL: i64 = 231;
    pub const SYS_MADVISE: i64 = 233;
    pub const SYS_MINCORE: i64 = 232;
    pub const SYS_TIMES: i64 = 153;
    pub const SYS_SPLICE: i64 = 76;
    pub const SYS_TEE: i64 = 77;
    pub const SYS_STATX: i64 = 291;
    pub const SYS_SETFSUID: i64 = 151;
    pub const SYS_SETFSGID: i64 = 152;
    pub const SYS_SETREGID: i64 = 143;
    pub const SYS_PROCESS_VM_READV: i64 = 270;
    pub const SYS_PROCESS_VM_WRITEV: i64 = 271;
    pub const SYS_SEMTIMEDOP: i64 = 192;
    pub const SYS_NAME_TO_HANDLE_AT: i64 = 264;
    pub const SYS_OPEN_BY_HANDLE_AT: i64 = 265;
    pub const SYS_RT_SIGQUEUEINFO: i64 = 138;
}
// powerpc64
#[cfg(target_arch = "powerpc64")]
mod sysnr {
    pub const SYS_ACCEPT: i64 = 330;
    pub const SYS_ACCEPT4: i64 = 344;
    pub const SYS_ACCESS: i64 = 33;
    pub const SYS_ACCT: i64 = 51;
    pub const SYS_ADD_KEY: i64 = 269;
    pub const SYS_ADJTIMEX: i64 = 124;
    pub const SYS_AFS_SYSCALL: i64 = 137;
    pub const SYS_ALARM: i64 = 27;
    pub const SYS_BDFLUSH: i64 = 134;
    pub const SYS_BIND: i64 = 327;
    pub const SYS_BPF: i64 = 361;
    pub const SYS_BREAK: i64 = 17;
    pub const SYS_BRK: i64 = 45;
    pub const SYS_CACHESTAT: i64 = 451;
    pub const SYS_CAPGET: i64 = 183;
    pub const SYS_CAPSET: i64 = 184;
    pub const SYS_CHDIR: i64 = 12;
    pub const SYS_CHMOD: i64 = 15;
    pub const SYS_CHOWN: i64 = 181;
    pub const SYS_CHROOT: i64 = 61;
    pub const SYS_CLOCK_ADJTIME: i64 = 347;
    pub const SYS_CLOCK_GETRES: i64 = 247;
    pub const SYS_CLOCK_GETTIME: i64 = 246;
    pub const SYS_CLOCK_NANOSLEEP: i64 = 248;
    pub const SYS_CLOCK_SETTIME: i64 = 245;
    pub const SYS_CLONE: i64 = 120;
    pub const SYS_CLONE3: i64 = 435;
    pub const SYS_CLOSE: i64 = 6;
    pub const SYS_CLOSE_RANGE: i64 = 436;
    pub const SYS_CONNECT: i64 = 328;
    pub const SYS_COPY_FILE_RANGE: i64 = 379;
    pub const SYS_CREAT: i64 = 8;
    pub const SYS_CREATE_MODULE: i64 = 127;
    pub const SYS_DELETE_MODULE: i64 = 129;
    pub const SYS_DUP: i64 = 41;
    pub const SYS_DUP2: i64 = 63;
    pub const SYS_DUP3: i64 = 316;
    pub const SYS_EPOLL_CREATE: i64 = 236;
    pub const SYS_EPOLL_CREATE1: i64 = 315;
    pub const SYS_EPOLL_CTL: i64 = 237;
    pub const SYS_EPOLL_PWAIT: i64 = 303;
    pub const SYS_EPOLL_PWAIT2: i64 = 441;
    pub const SYS_EPOLL_WAIT: i64 = 238;
    pub const SYS_EVENTFD: i64 = 307;
    pub const SYS_EVENTFD2: i64 = 314;
    pub const SYS_EXECVE: i64 = 11;
    pub const SYS_EXECVEAT: i64 = 362;
    pub const SYS_EXIT: i64 = 1;
    pub const SYS_EXIT_GROUP: i64 = 234;
    pub const SYS_FACCESSAT: i64 = 298;
    pub const SYS_FACCESSAT2: i64 = 439;
    pub const SYS_FADVISE64: i64 = 233;
    pub const SYS_FALLOCATE: i64 = 309;
    pub const SYS_FANOTIFY_INIT: i64 = 323;
    pub const SYS_FANOTIFY_MARK: i64 = 324;
    pub const SYS_FCHDIR: i64 = 133;
    pub const SYS_FCHMOD: i64 = 94;
    pub const SYS_FCHMODAT: i64 = 297;
    pub const SYS_FCHMODAT2: i64 = 452;
    pub const SYS_FCHOWN: i64 = 95;
    pub const SYS_FCHOWNAT: i64 = 289;
    pub const SYS_FCNTL: i64 = 55;
    pub const SYS_FDATASYNC: i64 = 148;
    pub const SYS_FGETXATTR: i64 = 214;
    pub const SYS_FINIT_MODULE: i64 = 353;
    pub const SYS_FLISTXATTR: i64 = 217;
    pub const SYS_FLOCK: i64 = 143;
    pub const SYS_FORK: i64 = 2;
    pub const SYS_FREMOVEXATTR: i64 = 220;
    pub const SYS_FSCONFIG: i64 = 431;
    pub const SYS_FSETXATTR: i64 = 211;
    pub const SYS_FSMOUNT: i64 = 432;
    pub const SYS_FSOPEN: i64 = 430;
    pub const SYS_FSPICK: i64 = 433;
    pub const SYS_FSTAT: i64 = 108;
    pub const SYS_FSTATFS: i64 = 100;
    pub const SYS_FSTATFS64: i64 = 253;
    pub const SYS_FSYNC: i64 = 118;
    pub const SYS_FTIME: i64 = 35;
    pub const SYS_FTRUNCATE: i64 = 93;
    pub const SYS_FUTEX: i64 = 221;
    pub const SYS_FUTEX_REQUEUE: i64 = 456;
    pub const SYS_FUTEX_WAIT: i64 = 455;
    pub const SYS_FUTEX_WAITV: i64 = 449;
    pub const SYS_FUTEX_WAKE: i64 = 454;
    pub const SYS_FUTIMESAT: i64 = 290;
    pub const SYS_GETCPU: i64 = 302;
    pub const SYS_GETCWD: i64 = 182;
    pub const SYS_GETDENTS: i64 = 141;
    pub const SYS_GETDENTS64: i64 = 202;
    pub const SYS_GETEGID: i64 = 50;
    pub const SYS_GETEUID: i64 = 49;
    pub const SYS_GETGID: i64 = 47;
    pub const SYS_GETGROUPS: i64 = 80;
    pub const SYS_GETITIMER: i64 = 105;
    pub const SYS_GETPEERNAME: i64 = 332;
    pub const SYS_GETPGID: i64 = 132;
    pub const SYS_GETPGRP: i64 = 65;
    pub const SYS_GETPID: i64 = 20;
    pub const SYS_GETPMSG: i64 = 187;
    pub const SYS_GETPPID: i64 = 64;
    pub const SYS_GETPRIORITY: i64 = 96;
    pub const SYS_GETRANDOM: i64 = 359;
    pub const SYS_GETRESGID: i64 = 170;
    pub const SYS_GETRESUID: i64 = 165;
    pub const SYS_GETRLIMIT: i64 = 76;
    pub const SYS_GETRUSAGE: i64 = 77;
    pub const SYS_GETSID: i64 = 147;
    pub const SYS_GETSOCKNAME: i64 = 331;
    pub const SYS_GETSOCKOPT: i64 = 340;
    pub const SYS_GETTID: i64 = 207;
    pub const SYS_GETTIMEOFDAY: i64 = 78;
    pub const SYS_GETUID: i64 = 24;
    pub const SYS_GETXATTR: i64 = 212;
    pub const SYS_GET_KERNEL_SYMS: i64 = 130;
    pub const SYS_GET_MEMPOLICY: i64 = 260;
    pub const SYS_GET_ROBUST_LIST: i64 = 299;
    pub const SYS_GTTY: i64 = 32;
    pub const SYS_IDLE: i64 = 112;
    pub const SYS_INIT_MODULE: i64 = 128;
    pub const SYS_INOTIFY_ADD_WATCH: i64 = 276;
    pub const SYS_INOTIFY_INIT: i64 = 275;
    pub const SYS_INOTIFY_INIT1: i64 = 318;
    pub const SYS_INOTIFY_RM_WATCH: i64 = 277;
    pub const SYS_IOCTL: i64 = 54;
    pub const SYS_IOPERM: i64 = 101;
    pub const SYS_IOPL: i64 = 110;
    pub const SYS_IOPRIO_GET: i64 = 274;
    pub const SYS_IOPRIO_SET: i64 = 273;
    pub const SYS_IO_CANCEL: i64 = 231;
    pub const SYS_IO_DESTROY: i64 = 228;
    pub const SYS_IO_GETEVENTS: i64 = 229;
    pub const SYS_IO_PGETEVENTS: i64 = 388;
    pub const SYS_IO_SETUP: i64 = 227;
    pub const SYS_IO_SUBMIT: i64 = 230;
    pub const SYS_IO_URING_ENTER: i64 = 426;
    pub const SYS_IO_URING_REGISTER: i64 = 427;
    pub const SYS_IO_URING_SETUP: i64 = 425;
    pub const SYS_IPC: i64 = 117;
    pub const SYS_KCMP: i64 = 354;
    pub const SYS_KEXEC_FILE_LOAD: i64 = 382;
    pub const SYS_KEXEC_LOAD: i64 = 268;
    pub const SYS_KEYCTL: i64 = 271;
    pub const SYS_KILL: i64 = 37;
    pub const SYS_LANDLOCK_ADD_RULE: i64 = 445;
    pub const SYS_LANDLOCK_CREATE_RULESET: i64 = 444;
    pub const SYS_LANDLOCK_RESTRICT_SELF: i64 = 446;
    pub const SYS_LCHOWN: i64 = 16;
    pub const SYS_LGETXATTR: i64 = 213;
    pub const SYS_LINK: i64 = 9;
    pub const SYS_LINKAT: i64 = 294;
    pub const SYS_LISTEN: i64 = 329;
    pub const SYS_LISTMOUNT: i64 = 458;
    pub const SYS_LISTXATTR: i64 = 215;
    pub const SYS_LLISTXATTR: i64 = 216;
    pub const SYS_LOCK: i64 = 53;
    pub const SYS_LOOKUP_DCOOKIE: i64 = 235;
    pub const SYS_LREMOVEXATTR: i64 = 219;
    pub const SYS_LSEEK: i64 = 19;
    pub const SYS_LSETXATTR: i64 = 210;
    pub const SYS_LSM_GET_SELF_ATTR: i64 = 459;
    pub const SYS_LSM_LIST_MODULES: i64 = 461;
    pub const SYS_LSM_SET_SELF_ATTR: i64 = 460;
    pub const SYS_LSTAT: i64 = 107;
    pub const SYS_MADVISE: i64 = 205;
    pub const SYS_MAP_SHADOW_STACK: i64 = 453;
    pub const SYS_MBIND: i64 = 259;
    pub const SYS_MEMBARRIER: i64 = 365;
    pub const SYS_MEMFD_CREATE: i64 = 360;
    pub const SYS_MIGRATE_PAGES: i64 = 258;
    pub const SYS_MINCORE: i64 = 206;
    pub const SYS_MKDIR: i64 = 39;
    pub const SYS_MKDIRAT: i64 = 287;
    pub const SYS_MKNOD: i64 = 14;
    pub const SYS_MKNODAT: i64 = 288;
    pub const SYS_MLOCK: i64 = 150;
    pub const SYS_MLOCK2: i64 = 378;
    pub const SYS_MLOCKALL: i64 = 152;
    pub const SYS_MMAP: i64 = 90;
    pub const SYS_MODIFY_LDT: i64 = 123;
    pub const SYS_MOUNT: i64 = 21;
    pub const SYS_MOUNT_SETATTR: i64 = 442;
    pub const SYS_MOVE_MOUNT: i64 = 429;
    pub const SYS_MOVE_PAGES: i64 = 301;
    pub const SYS_MPROTECT: i64 = 125;
    pub const SYS_MPX: i64 = 56;
    pub const SYS_MQ_GETSETATTR: i64 = 267;
    pub const SYS_MQ_NOTIFY: i64 = 266;
    pub const SYS_MQ_OPEN: i64 = 262;
    pub const SYS_MQ_TIMEDRECEIVE: i64 = 265;
    pub const SYS_MQ_TIMEDSEND: i64 = 264;
    pub const SYS_MQ_UNLINK: i64 = 263;
    pub const SYS_MREMAP: i64 = 163;
    pub const SYS_MSEAL: i64 = 462;
    pub const SYS_MSGCTL: i64 = 402;
    pub const SYS_MSGGET: i64 = 399;
    pub const SYS_MSGRCV: i64 = 401;
    pub const SYS_MSGSND: i64 = 400;
    pub const SYS_MSYNC: i64 = 144;
    pub const SYS_MULTIPLEXER: i64 = 201;
    pub const SYS_MUNLOCK: i64 = 151;
    pub const SYS_MUNLOCKALL: i64 = 153;
    pub const SYS_MUNMAP: i64 = 91;
    pub const SYS_NAME_TO_HANDLE_AT: i64 = 345;
    pub const SYS_NANOSLEEP: i64 = 162;
    pub const SYS_NEWFSTATAT: i64 = 291;
    pub const SYS_NFSSERVCTL: i64 = 168;
    pub const SYS_NICE: i64 = 34;
    pub const SYS_OLDFSTAT: i64 = 28;
    pub const SYS_OLDLSTAT: i64 = 84;
    pub const SYS_OLDOLDUNAME: i64 = 59;
    pub const SYS_OLDSTAT: i64 = 18;
    pub const SYS_OLDUNAME: i64 = 109;
    pub const SYS_OPEN: i64 = 5;
    pub const SYS_OPENAT: i64 = 286;
    pub const SYS_OPENAT2: i64 = 437;
    pub const SYS_OPEN_BY_HANDLE_AT: i64 = 346;
    pub const SYS_OPEN_TREE: i64 = 428;
    pub const SYS_PAUSE: i64 = 29;
    pub const SYS_PCICONFIG_IOBASE: i64 = 200;
    pub const SYS_PCICONFIG_READ: i64 = 198;
    pub const SYS_PCICONFIG_WRITE: i64 = 199;
    pub const SYS_PERF_EVENT_OPEN: i64 = 319;
    pub const SYS_PERSONALITY: i64 = 136;
    pub const SYS_PIDFD_GETFD: i64 = 438;
    pub const SYS_PIDFD_OPEN: i64 = 434;
    pub const SYS_PIDFD_SEND_SIGNAL: i64 = 424;
    pub const SYS_PIPE: i64 = 42;
    pub const SYS_PIPE2: i64 = 317;
    pub const SYS_PIVOT_ROOT: i64 = 203;
    pub const SYS_PKEY_ALLOC: i64 = 384;
    pub const SYS_PKEY_FREE: i64 = 385;
    pub const SYS_PKEY_MPROTECT: i64 = 386;
    pub const SYS_POLL: i64 = 167;
    pub const SYS_PPOLL: i64 = 281;
    pub const SYS_PRCTL: i64 = 171;
    pub const SYS_PREAD64: i64 = 179;
    pub const SYS_PREADV: i64 = 320;
    pub const SYS_PREADV2: i64 = 380;
    pub const SYS_PRLIMIT64: i64 = 325;
    pub const SYS_PROCESS_MADVISE: i64 = 440;
    pub const SYS_PROCESS_MRELEASE: i64 = 448;
    pub const SYS_PROCESS_VM_READV: i64 = 351;
    pub const SYS_PROCESS_VM_WRITEV: i64 = 352;
    pub const SYS_PROF: i64 = 44;
    pub const SYS_PROFIL: i64 = 98;
    pub const SYS_PSELECT6: i64 = 280;
    pub const SYS_PTRACE: i64 = 26;
    pub const SYS_PUTPMSG: i64 = 188;
    pub const SYS_PWRITE64: i64 = 180;
    pub const SYS_PWRITEV: i64 = 321;
    pub const SYS_PWRITEV2: i64 = 381;
    pub const SYS_QUERY_MODULE: i64 = 166;
    pub const SYS_QUOTACTL: i64 = 131;
    pub const SYS_QUOTACTL_FD: i64 = 443;
    pub const SYS_READ: i64 = 3;
    pub const SYS_READAHEAD: i64 = 191;
    pub const SYS_READDIR: i64 = 89;
    pub const SYS_READLINK: i64 = 85;
    pub const SYS_READLINKAT: i64 = 296;
    pub const SYS_READV: i64 = 145;
    pub const SYS_REBOOT: i64 = 88;
    pub const SYS_RECV: i64 = 336;
    pub const SYS_RECVFROM: i64 = 337;
    pub const SYS_RECVMMSG: i64 = 343;
    pub const SYS_RECVMSG: i64 = 342;
    pub const SYS_REMAP_FILE_PAGES: i64 = 239;
    pub const SYS_REMOVEXATTR: i64 = 218;
    pub const SYS_RENAME: i64 = 38;
    pub const SYS_RENAMEAT: i64 = 293;
    pub const SYS_RENAMEAT2: i64 = 357;
    pub const SYS_REQUEST_KEY: i64 = 270;
    pub const SYS_RESTART_SYSCALL: i64 = 0;
    pub const SYS_RMDIR: i64 = 40;
    pub const SYS_RSEQ: i64 = 387;
    pub const SYS_RTAS: i64 = 255;
    pub const SYS_RT_SIGACTION: i64 = 173;
    pub const SYS_RT_SIGPENDING: i64 = 175;
    pub const SYS_RT_SIGPROCMASK: i64 = 174;
    pub const SYS_RT_SIGQUEUEINFO: i64 = 177;
    pub const SYS_RT_SIGRETURN: i64 = 172;
    pub const SYS_RT_SIGSUSPEND: i64 = 178;
    pub const SYS_RT_SIGTIMEDWAIT: i64 = 176;
    pub const SYS_RT_TGSIGQUEUEINFO: i64 = 322;
    pub const SYS_SCHED_GETAFFINITY: i64 = 223;
    pub const SYS_SCHED_GETATTR: i64 = 356;
    pub const SYS_SCHED_GETPARAM: i64 = 155;
    pub const SYS_SCHED_GETSCHEDULER: i64 = 157;
    pub const SYS_SCHED_GET_PRIORITY_MAX: i64 = 159;
    pub const SYS_SCHED_GET_PRIORITY_MIN: i64 = 160;
    pub const SYS_SCHED_RR_GET_INTERVAL: i64 = 161;
    pub const SYS_SCHED_SETAFFINITY: i64 = 222;
    pub const SYS_SCHED_SETATTR: i64 = 355;
    pub const SYS_SCHED_SETPARAM: i64 = 154;
    pub const SYS_SCHED_SETSCHEDULER: i64 = 156;
    pub const SYS_SCHED_YIELD: i64 = 158;
    pub const SYS_SECCOMP: i64 = 358;
    pub const SYS_SELECT: i64 = 82;
    pub const SYS_SEMCTL: i64 = 394;
    pub const SYS_SEMGET: i64 = 393;
    pub const SYS_SEMOP: i64 = 392;
    pub const SYS_SEMTIMEDOP: i64 = 392;
    pub const SYS_SEND: i64 = 334;
    pub const SYS_SENDFILE: i64 = 186;
    pub const SYS_SENDMMSG: i64 = 349;
    pub const SYS_SENDMSG: i64 = 341;
    pub const SYS_SENDTO: i64 = 335;
    pub const SYS_SETDOMAINNAME: i64 = 121;
    pub const SYS_SETFSGID: i64 = 139;
    pub const SYS_SETFSUID: i64 = 138;
    pub const SYS_SETGID: i64 = 46;
    pub const SYS_SETGROUPS: i64 = 81;
    pub const SYS_SETHOSTNAME: i64 = 74;
    pub const SYS_SETITIMER: i64 = 104;
    pub const SYS_SETNS: i64 = 350;
    pub const SYS_SETPGID: i64 = 57;
    pub const SYS_SETPRIORITY: i64 = 97;
    pub const SYS_SETREGID: i64 = 71;
    pub const SYS_SETRESGID: i64 = 169;
    pub const SYS_SETRESUID: i64 = 164;
    pub const SYS_SETREUID: i64 = 70;
    pub const SYS_SETRLIMIT: i64 = 75;
    pub const SYS_SETSID: i64 = 66;
    pub const SYS_SETSOCKOPT: i64 = 339;
    pub const SYS_SETTIMEOFDAY: i64 = 79;
    pub const SYS_SETUID: i64 = 23;
    pub const SYS_SETXATTR: i64 = 209;
    pub const SYS_SET_MEMPOLICY: i64 = 261;
    pub const SYS_SET_MEMPOLICY_HOME_NODE: i64 = 450;
    pub const SYS_SET_ROBUST_LIST: i64 = 300;
    pub const SYS_SET_TID_ADDRESS: i64 = 232;
    pub const SYS_SGETMASK: i64 = 68;
    pub const SYS_SHMAT: i64 = 397;
    pub const SYS_SHMCTL: i64 = 396;
    pub const SYS_SHMDT: i64 = 398;
    pub const SYS_SHMGET: i64 = 395;
    pub const SYS_SHUTDOWN: i64 = 338;
    pub const SYS_SIGACTION: i64 = 67;
    pub const SYS_SIGALTSTACK: i64 = 185;
    pub const SYS_SIGNAL: i64 = 48;
    pub const SYS_SIGNALFD: i64 = 305;
    pub const SYS_SIGNALFD4: i64 = 313;
    pub const SYS_SIGPENDING: i64 = 73;
    pub const SYS_SIGPROCMASK: i64 = 126;
    pub const SYS_SIGRETURN: i64 = 119;
    pub const SYS_SIGSUSPEND: i64 = 72;
    pub const SYS_SOCKET: i64 = 326;
    pub const SYS_SOCKETCALL: i64 = 102;
    pub const SYS_SOCKETPAIR: i64 = 333;
    pub const SYS_SPLICE: i64 = 283;
    pub const SYS_SPU_CREATE: i64 = 279;
    pub const SYS_SPU_RUN: i64 = 278;
    pub const SYS_SSETMASK: i64 = 69;
    pub const SYS_STAT: i64 = 106;
    pub const SYS_STATFS: i64 = 99;
    pub const SYS_STATFS64: i64 = 252;
    pub const SYS_STATMOUNT: i64 = 457;
    pub const SYS_STATX: i64 = 383;
    pub const SYS_STIME: i64 = 25;
    pub const SYS_STTY: i64 = 31;
    pub const SYS_SUBPAGE_PROT: i64 = 310;
    pub const SYS_SWAPCONTEXT: i64 = 249;
    pub const SYS_SWAPOFF: i64 = 115;
    pub const SYS_SWAPON: i64 = 87;
    pub const SYS_SWITCH_ENDIAN: i64 = 363;
    pub const SYS_SYMLINK: i64 = 83;
    pub const SYS_SYMLINKAT: i64 = 295;
    pub const SYS_SYNC: i64 = 36;
    pub const SYS_SYNCFS: i64 = 348;
    pub const SYS_SYNC_FILE_RANGE2: i64 = 308;
    pub const SYS_SYSFS: i64 = 135;
    pub const SYS_SYSINFO: i64 = 116;
    pub const SYS_SYSLOG: i64 = 103;
    pub const SYS_SYS_DEBUG_SETCONTEXT: i64 = 256;
    pub const SYS_TEE: i64 = 284;
    pub const SYS_TGKILL: i64 = 250;
    pub const SYS_TIME: i64 = 13;
    pub const SYS_TIMERFD_CREATE: i64 = 306;
    pub const SYS_TIMERFD_GETTIME: i64 = 312;
    pub const SYS_TIMERFD_SETTIME: i64 = 311;
    pub const SYS_TIMER_CREATE: i64 = 240;
    pub const SYS_TIMER_DELETE: i64 = 244;
    pub const SYS_TIMER_GETOVERRUN: i64 = 243;
    pub const SYS_TIMER_GETTIME: i64 = 242;
    pub const SYS_TIMER_SETTIME: i64 = 241;
    pub const SYS_TIMES: i64 = 43;
    pub const SYS_TKILL: i64 = 208;
    pub const SYS_TRUNCATE: i64 = 92;
    pub const SYS_TUXCALL: i64 = 225;
    pub const SYS_UGETRLIMIT: i64 = 190;
    pub const SYS_ULIMIT: i64 = 58;
    pub const SYS_UMASK: i64 = 60;
    pub const SYS_UMOUNT: i64 = 22;
    pub const SYS_UMOUNT2: i64 = 52;
    pub const SYS_UNAME: i64 = 122;
    pub const SYS_UNLINK: i64 = 10;
    pub const SYS_UNLINKAT: i64 = 292;
    pub const SYS_UNSHARE: i64 = 282;
    pub const SYS_USELIB: i64 = 86;
    pub const SYS_USERFAULTFD: i64 = 364;
    pub const SYS_USTAT: i64 = 62;
    pub const SYS_UTIME: i64 = 30;
    pub const SYS_UTIMENSAT: i64 = 304;
    pub const SYS_UTIMES: i64 = 251;
    pub const SYS_VFORK: i64 = 189;
    pub const SYS_VHANGUP: i64 = 111;
    pub const SYS_VM86: i64 = 113;
    pub const SYS_VMSPLICE: i64 = 285;
    pub const SYS_WAIT4: i64 = 114;
    pub const SYS_WAITID: i64 = 272;
    pub const SYS_WAITPID: i64 = 7;
    pub const SYS_WRITE: i64 = 4;
    pub const SYS_WRITEV: i64 = 146;
    pub const SYS__LLSEEK: i64 = 140;
    pub const SYS__NEWSELECT: i64 = 142;
    pub const SYS__SYSCTL: i64 = 149;
}

// loongarch64
#[cfg(target_arch = "loongarch64")]
mod sysnr {
    pub const SYS_ACCEPT: i64 = 202;
    pub const SYS_ACCEPT4: i64 = 242;
    pub const SYS_ACCT: i64 = 89;
    pub const SYS_ADD_KEY: i64 = 217;
    pub const SYS_ADJTIMEX: i64 = 171;
    pub const SYS_BIND: i64 = 200;
    pub const SYS_BPF: i64 = 280;
    pub const SYS_BRK: i64 = 214;
    pub const SYS_CACHESTAT: i64 = 451;
    pub const SYS_CAPGET: i64 = 90;
    pub const SYS_CAPSET: i64 = 91;
    pub const SYS_CHDIR: i64 = 49;
    pub const SYS_CHROOT: i64 = 51;
    pub const SYS_CLOCK_ADJTIME: i64 = 266;
    pub const SYS_CLOCK_GETRES: i64 = 114;
    pub const SYS_CLOCK_GETTIME: i64 = 113;
    pub const SYS_CLOCK_NANOSLEEP: i64 = 115;
    pub const SYS_CLOCK_SETTIME: i64 = 112;
    pub const SYS_CLONE: i64 = 220;
    pub const SYS_CLONE3: i64 = 435;
    pub const SYS_CLOSE: i64 = 57;
    pub const SYS_CLOSE_RANGE: i64 = 436;
    pub const SYS_CONNECT: i64 = 203;
    pub const SYS_COPY_FILE_RANGE: i64 = 285;
    pub const SYS_DELETE_MODULE: i64 = 106;
    pub const SYS_DUP: i64 = 23;
    pub const SYS_DUP3: i64 = 24;
    pub const SYS_EPOLL_CREATE1: i64 = 20;
    pub const SYS_EPOLL_CTL: i64 = 21;
    pub const SYS_EPOLL_PWAIT: i64 = 22;
    pub const SYS_EPOLL_PWAIT2: i64 = 441;
    pub const SYS_EVENTFD2: i64 = 19;
    pub const SYS_EXECVE: i64 = 221;
    pub const SYS_EXECVEAT: i64 = 281;
    pub const SYS_EXIT: i64 = 93;
    pub const SYS_EXIT_GROUP: i64 = 94;
    pub const SYS_FACCESSAT: i64 = 48;
    pub const SYS_FACCESSAT2: i64 = 439;
    pub const SYS_FADVISE64: i64 = 223;
    pub const SYS_FALLOCATE: i64 = 47;
    pub const SYS_FANOTIFY_INIT: i64 = 262;
    pub const SYS_FANOTIFY_MARK: i64 = 263;
    pub const SYS_FCHDIR: i64 = 50;
    pub const SYS_FCHMOD: i64 = 52;
    pub const SYS_FCHMODAT: i64 = 53;
    pub const SYS_FCHMODAT2: i64 = 452;
    pub const SYS_FCHOWN: i64 = 55;
    pub const SYS_FCHOWNAT: i64 = 54;
    pub const SYS_FCNTL: i64 = 25;
    pub const SYS_FDATASYNC: i64 = 83;
    pub const SYS_FGETXATTR: i64 = 10;
    pub const SYS_FINIT_MODULE: i64 = 273;
    pub const SYS_FLISTXATTR: i64 = 13;
    pub const SYS_FLOCK: i64 = 32;
    pub const SYS_FREMOVEXATTR: i64 = 16;
    pub const SYS_FSCONFIG: i64 = 431;
    pub const SYS_FSETXATTR: i64 = 7;
    pub const SYS_FSMOUNT: i64 = 432;
    pub const SYS_FSOPEN: i64 = 430;
    pub const SYS_FSPICK: i64 = 433;
    pub const SYS_FSTAT: i64 = 80;
    pub const SYS_FSTATFS: i64 = 44;
    pub const SYS_FSYNC: i64 = 82;
    pub const SYS_FTRUNCATE: i64 = 46;
    pub const SYS_FUTEX: i64 = 98;
    pub const SYS_FUTEX_REQUEUE: i64 = 456;
    pub const SYS_FUTEX_WAIT: i64 = 455;
    pub const SYS_FUTEX_WAITV: i64 = 449;
    pub const SYS_FUTEX_WAKE: i64 = 454;
    pub const SYS_GETCPU: i64 = 168;
    pub const SYS_GETCWD: i64 = 17;
    pub const SYS_GETDENTS64: i64 = 61;
    pub const SYS_GETEGID: i64 = 177;
    pub const SYS_GETEUID: i64 = 175;
    pub const SYS_GETGID: i64 = 176;
    pub const SYS_GETGROUPS: i64 = 158;
    pub const SYS_GETITIMER: i64 = 102;
    pub const SYS_GETPEERNAME: i64 = 205;
    pub const SYS_GETPGID: i64 = 155;
    pub const SYS_GETPID: i64 = 172;
    pub const SYS_GETPPID: i64 = 173;
    pub const SYS_GETPRIORITY: i64 = 141;
    pub const SYS_GETRLIMIT: i64 = 163;
    pub const SYS_SETRLIMIT: i64 = 164;
    pub const SYS_GETRANDOM: i64 = 278;
    pub const SYS_GETRESGID: i64 = 150;
    pub const SYS_GETRESUID: i64 = 148;
    pub const SYS_GETRUSAGE: i64 = 165;
    pub const SYS_GETSID: i64 = 156;
    pub const SYS_GETSOCKNAME: i64 = 204;
    pub const SYS_GETSOCKOPT: i64 = 209;
    pub const SYS_GETTID: i64 = 178;
    pub const SYS_GETTIMEOFDAY: i64 = 169;
    pub const SYS_GETUID: i64 = 174;
    pub const SYS_GETXATTR: i64 = 8;
    pub const SYS_GET_MEMPOLICY: i64 = 236;
    pub const SYS_GET_ROBUST_LIST: i64 = 100;
    pub const SYS_INIT_MODULE: i64 = 105;
    pub const SYS_INOTIFY_ADD_WATCH: i64 = 27;
    pub const SYS_INOTIFY_INIT: i64 = 26;
    pub const SYS_INOTIFY_INIT1: i64 = 26;
    pub const SYS_INOTIFY_RM_WATCH: i64 = 28;
    pub const SYS_IOCTL: i64 = 29;
    pub const SYS_IOPRIO_GET: i64 = 31;
    pub const SYS_IOPRIO_SET: i64 = 30;
    pub const SYS_IO_CANCEL: i64 = 3;
    pub const SYS_IO_DESTROY: i64 = 1;
    pub const SYS_IO_GETEVENTS: i64 = 4;
    pub const SYS_IO_PGETEVENTS: i64 = 292;
    pub const SYS_IO_SETUP: i64 = 0;
    pub const SYS_IO_SUBMIT: i64 = 2;
    pub const SYS_IO_URING_ENTER: i64 = 426;
    pub const SYS_IO_URING_REGISTER: i64 = 427;
    pub const SYS_IO_URING_SETUP: i64 = 425;
    pub const SYS_KCMP: i64 = 272;
    pub const SYS_KEXEC_FILE_LOAD: i64 = 294;
    pub const SYS_KEXEC_LOAD: i64 = 104;
    pub const SYS_KEYCTL: i64 = 219;
    pub const SYS_KILL: i64 = 129;
    pub const SYS_LANDLOCK_ADD_RULE: i64 = 445;
    pub const SYS_LANDLOCK_CREATE_RULESET: i64 = 444;
    pub const SYS_LANDLOCK_RESTRICT_SELF: i64 = 446;
    pub const SYS_LGETXATTR: i64 = 9;
    pub const SYS_LINKAT: i64 = 37;
    pub const SYS_LISTEN: i64 = 201;
    pub const SYS_LISTMOUNT: i64 = 458;
    pub const SYS_LISTXATTR: i64 = 11;
    pub const SYS_LLISTXATTR: i64 = 12;
    pub const SYS_LOOKUP_DCOOKIE: i64 = 18;
    pub const SYS_LREMOVEXATTR: i64 = 15;
    pub const SYS_LSEEK: i64 = 62;
    pub const SYS_LSETXATTR: i64 = 6;
    pub const SYS_LSM_GET_SELF_ATTR: i64 = 459;
    pub const SYS_LSM_LIST_MODULES: i64 = 461;
    pub const SYS_LSM_SET_SELF_ATTR: i64 = 460;
    pub const SYS_MADVISE: i64 = 233;
    pub const SYS_MAP_SHADOW_STACK: i64 = 453;
    pub const SYS_MBIND: i64 = 235;
    pub const SYS_MEMBARRIER: i64 = 283;
    pub const SYS_MEMFD_CREATE: i64 = 279;
    pub const SYS_MIGRATE_PAGES: i64 = 238;
    pub const SYS_MINCORE: i64 = 232;
    pub const SYS_MKDIRAT: i64 = 34;
    pub const SYS_MKNODAT: i64 = 33;
    pub const SYS_MLOCK: i64 = 228;
    pub const SYS_MLOCK2: i64 = 284;
    pub const SYS_MLOCKALL: i64 = 230;
    pub const SYS_MMAP: i64 = 222;
    pub const SYS_MOUNT: i64 = 40;
    pub const SYS_MOUNT_SETATTR: i64 = 442;
    pub const SYS_MOVE_MOUNT: i64 = 429;
    pub const SYS_MOVE_PAGES: i64 = 239;
    pub const SYS_MPROTECT: i64 = 226;
    pub const SYS_MQ_GETSETATTR: i64 = 185;
    pub const SYS_MQ_NOTIFY: i64 = 184;
    pub const SYS_MQ_OPEN: i64 = 180;
    pub const SYS_MQ_TIMEDRECEIVE: i64 = 183;
    pub const SYS_MQ_TIMEDSEND: i64 = 182;
    pub const SYS_MQ_UNLINK: i64 = 181;
    pub const SYS_MREMAP: i64 = 216;
    pub const SYS_MSEAL: i64 = 462;
    pub const SYS_MSGCTL: i64 = 187;
    pub const SYS_MSGGET: i64 = 186;
    pub const SYS_MSGRCV: i64 = 188;
    pub const SYS_MSGSND: i64 = 189;
    pub const SYS_MSYNC: i64 = 227;
    pub const SYS_MUNLOCK: i64 = 229;
    pub const SYS_MUNLOCKALL: i64 = 231;
    pub const SYS_MUNMAP: i64 = 215;
    pub const SYS_NAME_TO_HANDLE_AT: i64 = 264;
    pub const SYS_NANOSLEEP: i64 = 101;
    pub const SYS_NEWFSTATAT: i64 = 79;
    pub const SYS_NFSSERVCTL: i64 = 42;
    pub const SYS_OPENAT: i64 = 56;
    pub const SYS_OPENAT2: i64 = 437;
    pub const SYS_OPEN_BY_HANDLE_AT: i64 = 265;
    pub const SYS_OPEN_TREE: i64 = 428;
    pub const SYS_PERF_EVENT_OPEN: i64 = 241;
    pub const SYS_PERSONALITY: i64 = 92;
    pub const SYS_PIDFD_GETFD: i64 = 438;
    pub const SYS_PIDFD_OPEN: i64 = 434;
    pub const SYS_PIDFD_SEND_SIGNAL: i64 = 424;
    pub const SYS_PIPE2: i64 = 59;
    pub const SYS_PIVOT_ROOT: i64 = 41;
    pub const SYS_PKEY_ALLOC: i64 = 289;
    pub const SYS_PKEY_FREE: i64 = 290;
    pub const SYS_PKEY_MPROTECT: i64 = 288;
    pub const SYS_PPOLL: i64 = 73;
    pub const SYS_PRCTL: i64 = 167;
    pub const SYS_PREAD64: i64 = 67;
    pub const SYS_PREADV: i64 = 69;
    pub const SYS_PREADV2: i64 = 286;
    pub const SYS_PRLIMIT64: i64 = 261;
    pub const SYS_PROCESS_MADVISE: i64 = 440;
    pub const SYS_PROCESS_MRELEASE: i64 = 448;
    pub const SYS_PROCESS_VM_READV: i64 = 270;
    pub const SYS_PROCESS_VM_WRITEV: i64 = 271;
    pub const SYS_PSELECT6: i64 = 72;
    pub const SYS_PTRACE: i64 = 117;
    pub const SYS_PWRITE64: i64 = 68;
    pub const SYS_PWRITEV: i64 = 70;
    pub const SYS_PWRITEV2: i64 = 287;
    pub const SYS_QUOTACTL: i64 = 60;
    pub const SYS_QUOTACTL_FD: i64 = 443;
    pub const SYS_READ: i64 = 63;
    pub const SYS_READAHEAD: i64 = 213;
    pub const SYS_READLINKAT: i64 = 78;
    pub const SYS_READV: i64 = 65;
    pub const SYS_REBOOT: i64 = 142;
    pub const SYS_RECVFROM: i64 = 207;
    pub const SYS_RECVMMSG: i64 = 243;
    pub const SYS_RECVMSG: i64 = 212;
    pub const SYS_REMAP_FILE_PAGES: i64 = 234;
    pub const SYS_REMOVEXATTR: i64 = 14;
    pub const SYS_RENAMEAT2: i64 = 276;
    pub const SYS_REQUEST_KEY: i64 = 218;
    pub const SYS_RESTART_SYSCALL: i64 = 128;
    pub const SYS_RSEQ: i64 = 293;
    pub const SYS_RT_SIGACTION: i64 = 134;
    pub const SYS_RT_SIGPENDING: i64 = 136;
    pub const SYS_RT_SIGPROCMASK: i64 = 135;
    pub const SYS_RT_SIGQUEUEINFO: i64 = 138;
    pub const SYS_RT_SIGRETURN: i64 = 139;
    pub const SYS_RT_SIGSUSPEND: i64 = 133;
    pub const SYS_RT_SIGTIMEDWAIT: i64 = 137;
    pub const SYS_RT_TGSIGQUEUEINFO: i64 = 240;
    pub const SYS_SCHED_GETAFFINITY: i64 = 123;
    pub const SYS_SCHED_GETATTR: i64 = 275;
    pub const SYS_SCHED_GETPARAM: i64 = 121;
    pub const SYS_SCHED_GETSCHEDULER: i64 = 120;
    pub const SYS_SCHED_GET_PRIORITY_MAX: i64 = 125;
    pub const SYS_SCHED_GET_PRIORITY_MIN: i64 = 126;
    pub const SYS_SCHED_RR_GET_INTERVAL: i64 = 127;
    pub const SYS_SCHED_SETAFFINITY: i64 = 122;
    pub const SYS_SCHED_SETATTR: i64 = 274;
    pub const SYS_SCHED_SETPARAM: i64 = 118;
    pub const SYS_SCHED_SETSCHEDULER: i64 = 119;
    pub const SYS_SCHED_YIELD: i64 = 124;
    pub const SYS_SECCOMP: i64 = 277;
    pub const SYS_SEMCTL: i64 = 191;
    pub const SYS_SELECT: i64 = 72;
    pub const SYS_SEMGET: i64 = 190;
    pub const SYS_SEMOP: i64 = 193;
    pub const SYS_SEMTIMEDOP: i64 = 192;
    pub const SYS_SENDFILE: i64 = 71;
    pub const SYS_SENDMMSG: i64 = 269;
    pub const SYS_SENDMSG: i64 = 211;
    pub const SYS_SENDTO: i64 = 206;
    pub const SYS_SETDOMAINNAME: i64 = 162;
    pub const SYS_SETFSGID: i64 = 152;
    pub const SYS_SETFSUID: i64 = 151;
    pub const SYS_SETGID: i64 = 144;
    pub const SYS_SETGROUPS: i64 = 159;
    pub const SYS_SETHOSTNAME: i64 = 161;
    pub const SYS_SETITIMER: i64 = 103;
    pub const SYS_SETNS: i64 = 268;
    pub const SYS_SETPGID: i64 = 154;
    pub const SYS_SETPRIORITY: i64 = 140;
    pub const SYS_SETREGID: i64 = 143;
    pub const SYS_SETRESGID: i64 = 149;
    pub const SYS_SETRESUID: i64 = 147;
    pub const SYS_SETREUID: i64 = 145;
    pub const SYS_SETSID: i64 = 157;
    pub const SYS_SETSOCKOPT: i64 = 208;
    pub const SYS_SETTIMEOFDAY: i64 = 170;
    pub const SYS_SETUID: i64 = 146;
    pub const SYS_SETXATTR: i64 = 5;
    pub const SYS_SET_MEMPOLICY: i64 = 237;
    pub const SYS_SET_MEMPOLICY_HOME_NODE: i64 = 450;
    pub const SYS_SET_ROBUST_LIST: i64 = 99;
    pub const SYS_SET_TID_ADDRESS: i64 = 96;
    pub const SYS_SHMAT: i64 = 196;
    pub const SYS_SHMCTL: i64 = 195;
    pub const SYS_SHMDT: i64 = 197;
    pub const SYS_SHMGET: i64 = 194;
    pub const SYS_SHUTDOWN: i64 = 210;
    pub const SYS_SIGALTSTACK: i64 = 132;
    pub const SYS_SIGNALFD4: i64 = 74;
    pub const SYS_SOCKET: i64 = 198;
    pub const SYS_SOCKETPAIR: i64 = 199;
    pub const SYS_SPLICE: i64 = 76;
    pub const SYS_STATFS: i64 = 43;
    pub const SYS_STATMOUNT: i64 = 457;
    pub const SYS_STATX: i64 = 291;
    pub const SYS_SWAPOFF: i64 = 225;
    pub const SYS_SWAPON: i64 = 224;
    pub const SYS_SYMLINKAT: i64 = 36;
    pub const SYS_SYNC: i64 = 81;
    pub const SYS_SYNCFS: i64 = 267;
    pub const SYS_SYNC_FILE_RANGE: i64 = 84;
    pub const SYS_SYSINFO: i64 = 179;
    pub const SYS_SYSLOG: i64 = 116;
    pub const SYS_TEE: i64 = 77;
    pub const SYS_TGKILL: i64 = 131;
    pub const SYS_TIMERFD_CREATE: i64 = 85;
    pub const SYS_TIMERFD_GETTIME: i64 = 87;
    pub const SYS_TIMERFD_SETTIME: i64 = 86;
    pub const SYS_TIMER_CREATE: i64 = 107;
    pub const SYS_TIMER_DELETE: i64 = 111;
    pub const SYS_TIMER_GETOVERRUN: i64 = 109;
    pub const SYS_TIMER_GETTIME: i64 = 108;
    pub const SYS_TIMER_SETTIME: i64 = 110;
    pub const SYS_TIMES: i64 = 153;
    pub const SYS_TKILL: i64 = 130;
    pub const SYS_TRUNCATE: i64 = 45;
    pub const SYS_UMASK: i64 = 166;
    pub const SYS_UMOUNT2: i64 = 39;
    pub const SYS_UNAME: i64 = 160;
    pub const SYS_UNLINKAT: i64 = 35;
    pub const SYS_UNSHARE: i64 = 97;
    pub const SYS_USERFAULTFD: i64 = 282;
    pub const SYS_UTIMENSAT: i64 = 88;
    pub const SYS_VHANGUP: i64 = 58;
    pub const SYS_VMSPLICE: i64 = 75;
    pub const SYS_WAIT4: i64 = 260;
    pub const SYS_WAITID: i64 = 95;
    pub const SYS_WRITE: i64 = 64;
    pub const SYS_WRITEV: i64 = 66;
}

pub use sysnr::*;
