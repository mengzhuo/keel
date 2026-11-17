// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// signal.h
// ============================================================

pub const SA_RESTORER: c_ulong = 0x04000000;

pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;
pub const SIG_ERR: usize = !0usize;

pub const SIGHUP: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGQUIT: c_int = 3;
pub const SIGILL: c_int = 4;
pub const SIGTRAP: c_int = 5;
pub const SIGABRT: c_int = 6;
pub const SIGIOT: c_int = 6;
pub const SIGBUS: c_int = 7;
pub const SIGFPE: c_int = 8;
pub const SIGKILL: c_int = 9;
pub const SIGUSR1: c_int = 10;
pub const SIGSEGV: c_int = 11;
pub const SIGUSR2: c_int = 12;
pub const SIGPIPE: c_int = 13;
pub const SIGALRM: c_int = 14;
pub const SIGTERM: c_int = 15;
pub const SIGSTKFLT: c_int = 16;
pub const SIGCHLD: c_int = 17;
pub const SIGCONT: c_int = 18;
pub const SIGSTOP: c_int = 19;
pub const SIGTSTP: c_int = 20;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGURG: c_int = 23;
pub const SIGXCPU: c_int = 24;
pub const SIGXFSZ: c_int = 25;
pub const SIGVTALRM: c_int = 26;
pub const SIGPROF: c_int = 27;
pub const SIGWINCH: c_int = 28;
pub const SIGIO: c_int = 29;
pub const SIGPOLL: c_int = 29;
pub const SIGPWR: c_int = 30;
pub const SIGSYS: c_int = 31;
pub const SIGUNUSED: c_int = 31;
pub const _NSIG: c_int = 65;

pub const SIG_BLOCK: c_int = 0;
pub const SIG_UNBLOCK: c_int = 1;
pub const SIG_SETMASK: c_int = 2;

pub const SA_NOCLDSTOP: c_ulong = 1;
pub const SA_NOCLDWAIT: c_ulong = 2;
pub const SA_SIGINFO: c_ulong = 4;
pub const SA_ONSTACK: c_ulong = 0x08000000;
pub const SA_RESTART: c_ulong = 0x10000000;
pub const SA_NODEFER: c_ulong = 0x40000000;
pub const SA_RESETHAND: c_ulong = 0x80000000;

pub const SS_ONSTACK: c_int = 1;
pub const SS_DISABLE: c_int = 2;
#[cfg(target_arch = "aarch64")]
pub const MINSIGSTKSZ: usize = 6144;
#[cfg(target_arch = "aarch64")]
pub const SIGSTKSZ: usize = 12288;
#[cfg(not(target_arch = "aarch64"))]
pub const MINSIGSTKSZ: usize = 2048;
#[cfg(not(target_arch = "aarch64"))]
pub const SIGSTKSZ: usize = 8192;

pub const SI_USER: c_int = 0;
pub const SI_TKILL: c_int = -6;

#[repr(C)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

#[repr(C)]
pub struct siginfo_t {
    pub si_signo: c_int,
    pub si_errno: c_int,
    pub si_code: c_int,
    // The remaining bytes of the 128-byte siginfo_t are not accessed.
    _pad: [u8; 128 - 3 * core::mem::size_of::<c_int>()],
}

#[repr(C)]
pub struct stack_t {
    pub ss_sp: *mut c_void,
    pub ss_flags: c_int,
    pub ss_size: usize,
}

// Kernel ABI for rt_sigaction.  All supported architectures use the same
// handler/flags/restorer/8-byte-mask layout.
#[repr(C)]
pub struct KSigAction {
    pub sa_handler: usize,
    pub sa_flags: c_ulong,
    pub sa_restorer: usize,
    pub sa_mask: [c_ulong; 1],
}

// musl's public layout: handler, 128-byte sigset_t, flags, restorer.
#[repr(C)]
pub struct sigaction {
    pub sa_handler: usize,
    pub sa_mask: [u64; 16],
    pub sa_flags: c_int,
    pub sa_restorer: usize,
}

pub type SigSetT = c_ulong;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "mov eax, 15",
    "syscall",
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "mov x8, #139",
    "svc #0",
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "li a7, 139",
    "ecall",
);

extern "C" {
    fn sig_restorer();
}

#[inline]
unsafe fn sys_rt_sigaction(sig: c_int,
    act: *const KSigAction,
    oldact: *mut KSigAction,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGACTION, sig as i64, act as i64, oldact as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigprocmask(how: c_int,
    set: *const SigSetT,
    oldset: *mut SigSetT,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGPROCMASK, how as i64, set as i64, oldset as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigpending(set: *mut SigSetT, sigsetsize: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_RT_SIGPENDING, set as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigsuspend(mask: *const SigSetT, sigsetsize: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_RT_SIGSUSPEND, mask as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigtimedwait(set: *const SigSetT,
    info: *mut siginfo_t,
    timeout: *const timespec,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGTIMEDWAIT, set as i64, info as i64, timeout as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_sigaltstack(ss: *const stack_t, old_ss: *mut stack_t) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SIGALTSTACK, ss as i64, old_ss as i64)
}

#[inline]
unsafe fn sys_tgkill(tgid: c_int, tid: c_int, sig: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_TGKILL, tgid as i64, tid as i64, sig as i64)
}

const CLOCK_REALTIME: c_int = 0;
const CLOCK_MONOTONIC: c_int = 1;

#[inline]
unsafe fn sys_clock_gettime(clockid: c_int, ts: *mut timespec) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_CLOCK_GETTIME, clockid as i64, ts as i64)
}

// clock_gettime lives in vdso.rs: it prefers the kernel's vDSO entry point
// and only falls back to sys_clock_gettime() there.

#[no_mangle]
pub unsafe extern "C" fn sigprocmask(
    how: c_int,
    set: *const SigSetT,
    oldset: *mut SigSetT,
) -> c_int {
    // mask out internal rt signals (32, 33) before touching the kernel
    let internal_mask: SigSetT = (1u64 << 31) | (1u64 << 32);
    let filtered: SigSetT;
    let set_ptr: *const SigSetT = if set.is_null() {
        set
    } else {
        filtered = *set & !internal_mask;
        &filtered
    };
    let r = sys_rt_sigprocmask(how, set_ptr, oldset, core::mem::size_of::<SigSetT>());
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    if !oldset.is_null() {
        *oldset &= !internal_mask;
    }
    0
}

#[inline]
unsafe fn sys_kill(pid: c_int, sig: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_KILL, pid as i64, sig as i64)
}

#[inline]
unsafe fn sys_getpid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETPID)
}

#[no_mangle]
pub unsafe extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction,
    oldact: *mut sigaction,
) -> c_int {
    let mut kact: KSigAction = core::mem::zeroed();
    let act_ptr = if act.is_null() {
        core::ptr::null()
    } else {
        kact.sa_handler = (*act).sa_handler;
        kact.sa_flags = ((*act).sa_flags as c_ulong) | SA_RESTORER;
        kact.sa_restorer = if (*act).sa_flags as c_ulong & SA_RESTORER != 0
            && (*act).sa_restorer != 0
        {
            (*act).sa_restorer
        } else {
            sig_restorer as *const () as usize
        };
        kact.sa_mask[0] = (*act).sa_mask[0];
        &kact
    };
    let mut kold: KSigAction = core::mem::zeroed();
    let old_ptr = if oldact.is_null() { core::ptr::null_mut() } else { &mut kold };
    let r = sys_rt_sigaction(signum, act_ptr, old_ptr, 8);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    if !oldact.is_null() {
        (*oldact).sa_handler = kold.sa_handler;
        (*oldact).sa_flags = kold.sa_flags as c_int;
        (*oldact).sa_restorer = kold.sa_restorer;
        (*oldact).sa_mask = [0; 16];
        (*oldact).sa_mask[0] = kold.sa_mask[0];
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn signal(signum: c_int, handler: usize) -> usize {
    // musl's signal() installs the handler with SA_RESTART, so interrupted
    // syscalls (waitpid, read, ...) are restarted automatically.
    let act = sigaction {
        sa_handler: handler,
        sa_mask: [0; 16],
        sa_flags: SA_RESTART as c_int,
        sa_restorer: 0,
    };
    let mut old: sigaction = core::mem::zeroed();
    if sigaction(signum, &act, &mut old) == -1 {
        SIG_ERR
    } else {
        old.sa_handler
    }
}

#[no_mangle]
pub unsafe extern "C" fn kill(pid: c_int, sig: c_int) -> c_int {
    let r = sys_kill(pid, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn tgkill(tgid: c_int, tid: c_int, sig: c_int) -> c_int {
    let r = sys_tgkill(tgid, tid, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getpid() -> c_int {
    sys_getpid() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn raise(sig: c_int) -> c_int {
    let r = sys_tgkill(sys_getpid() as c_int, sys_gettid() as c_int, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigemptyset(set: *mut SigSetT) -> c_int {
    *set = 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigfillset(set: *mut SigSetT) -> c_int {
    // musl: all signals except 32 (SIGCANCEL) and 33 (SIGSYNCCALL)
    *set = 0xFFFF_FFFC_7FFF_FFFFu64;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigaddset(set: *mut SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 {
        ERRNO!() = EINVAL;
        return -1;
    }
    // internal rt signals (32, 33) cannot be manipulated
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    *set |= 1u64 << s;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigdelset(set: *mut SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 {
        ERRNO!() = EINVAL;
        return -1;
    }
    // internal rt signals (32, 33) are never members
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    *set &= !(1u64 << s);
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigismember(set: *const SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 { return 0; }
    // internal rt signals (32, 33) are never members
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    ((*set & (1u64 << s)) != 0) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigpending(set: *mut SigSetT) -> c_int {
    let r = sys_rt_sigpending(set, core::mem::size_of::<SigSetT>());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigsuspend(mask: *const SigSetT) -> c_int {
    let r = sys_rt_sigsuspend(mask, core::mem::size_of::<SigSetT>());
    if r < 0 {
        ERRNO!() = (-r) as c_int;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn sigtimedwait(
    mask: *const SigSetT,
    info: *mut siginfo_t,
    timeout: *const timespec,
) -> c_int {
    loop {
        let r = sys_rt_sigtimedwait(mask, info, timeout, core::mem::size_of::<SigSetT>());
        if r >= 0 {
            return r as c_int;
        }
        let e = (-r) as c_int;
        if e != EINTR {
            ERRNO!() = e;
            return -1;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sigwaitinfo(
    mask: *const SigSetT,
    info: *mut siginfo_t,
) -> c_int {
    sigtimedwait(mask, info, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn sigwait(
    mask: *const SigSetT,
    sig: *mut c_int,
) -> c_int {
    let mut info: siginfo_t = core::mem::zeroed();
    if sigtimedwait(mask, &mut info, core::ptr::null()) < 0 {
        return ERRNO!();
    }
    *sig = info.si_signo;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigaltstack(ss: *const stack_t, old_ss: *mut stack_t) -> c_int {
    if !ss.is_null() {
        if (*ss).ss_flags & SS_ONSTACK != 0 {
            ERRNO!() = EINVAL;
            return -1;
        }
        if (*ss).ss_flags & SS_DISABLE == 0 && (*ss).ss_size < MINSIGSTKSZ {
            ERRNO!() = ENOMEM;
            return -1;
        }
    }
    let r = sys_sigaltstack(ss, old_ss);
    syscall_ret(r) as c_int
}

include!("setjmp.rs");
