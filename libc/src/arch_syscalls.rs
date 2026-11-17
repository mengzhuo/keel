// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared raw syscall ABI.  This file is include!-ed by the libc, ldso and
// standalone-loader architecture modules; the Syscalls trait and the per-arch
// marker structs are declared at each include site.  Keeping one copy avoids
// three drifting definitions of the same register convention.

#[cfg(target_arch = "x86_64")]
impl Syscalls for X86_64 {
    #[inline(always)]
    unsafe fn syscall0(n: i64) -> i64 {
        let result: i64;
        unsafe {
            core::arch::asm!(
                "syscall",
                inlateout("rax") n => result,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall1(n: i64, a1: i64) -> i64 {
        let result: i64;
        unsafe {
            core::arch::asm!(
                "syscall",
                inlateout("rax") n => result,
                in("rdi") a1,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall2(n: i64, a1: i64, a2: i64) -> i64 {
        let result: i64;
        unsafe {
            core::arch::asm!(
                "syscall",
                inlateout("rax") n => result,
                in("rdi") a1,
                in("rsi") a2,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall3(n: i64, a1: i64, a2: i64, a3: i64) -> i64 {
        let result: i64;
        unsafe {
            core::arch::asm!(
                "syscall",
                inlateout("rax") n => result,
                in("rdi") a1,
                in("rsi") a2,
                in("rdx") a3,
                lateout("rcx") _,
                lateout("r11") _,
            );
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall4(n: i64, a1: i64, a2: i64, a3: i64, a4: i64) -> i64 {
        let result: i64;
        unsafe {
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
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall5(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64) -> i64 {
        let result: i64;
        unsafe {
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
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall6(n: i64, a1: i64, a2: i64, a3: i64, a4: i64, a5: i64, a6: i64) -> i64 {
        let result: i64;
        unsafe {
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
        }
        result
    }

    #[inline(always)]
    unsafe fn syscall_noreturn1(n: i64, a1: i64) -> ! {
        unsafe {
            core::arch::asm!(
                "syscall",
                in("rax") n,
                in("rdi") a1,
                options(noreturn)
            );
        }
    }
}

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
