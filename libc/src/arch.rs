// ============================================================
// Architecture selection
// ============================================================

/// Read the thread pointer.  errno and static TLS use this before any
/// architecture-specific TLS helpers are available.
#[cfg(target_arch = "x86_64")]
unsafe fn arch_get_tp() -> usize {
    const ARCH_PRCTL: i64 = 158;
    const ARCH_GET_FS: i64 = 0x1003;
    let mut tp: usize = 0;
    let _ = <Arch as Syscalls>::syscall2(ARCH_PRCTL, ARCH_GET_FS, &mut tp as *mut usize as i64);
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
}
pub use sysnr::*;
