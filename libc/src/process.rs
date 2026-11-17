// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// syscall wrappers: fstat/newfstatat/getrlimit/setrlimit/utimensat
// ============================================================

#[inline]
unsafe fn sys_newfstatat(dirfd: i32, path: *const c_char, buf: *mut u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_NEWFSTATAT, dirfd as i64, path as i64, buf as i64, flags as i64)
}

#[inline]
unsafe fn sys_fstat(fd: i32, buf: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_FSTAT, fd as i64, buf as i64)
}

// Linux removed the legacy getrlimit/setrlimit entry points on some
// architectures (qemu-ppc64le reports syscall 76 as unknown); prlimit64
// with pid 0 provides the same semantics.
#[cfg(any(target_arch = "powerpc64", target_arch = "loongarch64"))]
#[inline]
unsafe fn sys_getrlimit(resource: i32, rlim: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_PRLIMIT64, 0, resource as i64, 0, rlim as i64)
}

#[cfg(any(target_arch = "powerpc64", target_arch = "loongarch64"))]
#[inline]
unsafe fn sys_setrlimit(resource: i32, rlim: *const u8) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_PRLIMIT64, 0, resource as i64, rlim as i64, 0)
}

#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
#[inline]
unsafe fn sys_getrlimit(resource: i32, rlim: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_GETRLIMIT, resource as i64, rlim as i64)
}

#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
#[inline]
unsafe fn sys_setrlimit(resource: i32, rlim: *const u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SETRLIMIT, resource as i64, rlim as i64)
}

#[inline]
unsafe fn sys_utimensat(dirfd: i32, path: *const c_char, times: *const u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_UTIMENSAT, dirfd as i64, path as i64, times as i64, flags as i64)
}

// ============================================================
// unistd.h: process primitives
// ============================================================

#[inline]
unsafe fn sys_fork() -> i64 {
    #[cfg(target_arch = "x86_64")]
    { <Arch as Syscalls>::syscall0(SYS_FORK) }
    #[cfg(target_arch = "aarch64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
    #[cfg(target_arch = "riscv64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
    #[cfg(target_arch = "powerpc64")]
    { <Arch as Syscalls>::syscall0(SYS_FORK) }
    #[cfg(target_arch = "loongarch64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
}


#[inline]
unsafe fn sys_execve(path: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_EXECVE, path as i64, argv as i64, envp as i64)
}

#[inline]
unsafe fn sys_wait4(pid: c_int, status: *mut c_int, options: c_int, rusage: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_WAIT4, pid as i64, status as i64, options as i64, rusage as i64)
}

#[inline]
unsafe fn sys_getppid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETPPID)
}

#[inline]
unsafe fn sys_getuid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETUID)
}

#[inline]
unsafe fn sys_getgid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETGID)
}

#[inline]
unsafe fn sys_geteuid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETEUID)
}

#[inline]
unsafe fn sys_getegid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETEGID)
}

#[no_mangle]
pub unsafe extern "C" fn fork() -> c_int {
    __fork_handler(-1);
    let ret = sys_fork();
    // Run the atfork handlers before publishing errno, exactly like musl's
    // __syscall_ret: a failing fork must report the kernel's errno even if a
    // handler changes errno. RLIMIT_NPROC is enforced by the kernel and does
    // not apply to root, so do not second-guess it here.
    if ret < 0 {
        let err = (-ret) as c_int;
        __fork_handler(0);
        ERRNO!() = err;
        return -1;
    }
    if ret == 0 {
        allocator_after_fork();
        thread_allocator_after_fork();
        __fork_handler(1);
    } else {
        __fork_handler(0);
    }
    ret as c_int
}

#[no_mangle]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let r = sys_execve(path, argv, envp);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn wait(status: *mut c_int) -> c_int {
    let r = sys_wait4(-1, status, 0, core::ptr::null_mut());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int {
    let r = sys_wait4(pid, status, options, core::ptr::null_mut());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getppid() -> c_int {
    sys_getppid() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getuid() -> c_uint {
    sys_getuid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn getgid() -> c_uint {
    sys_getgid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn geteuid() -> c_uint {
    sys_geteuid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn getegid() -> c_uint {
    sys_getegid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn vfork() -> c_int {
    // keel uses the portable fork semantics; fork() also runs the atfork
    // handlers and resets the allocator/thread caches in the child.
    fork()
}

#[no_mangle]
pub unsafe extern "C" fn execl(path: *const c_char, arg: *const c_char, mut args: ...) -> c_int {
    let mut cap = 16usize;
    let mut argv = malloc(cap * core::mem::size_of::<*const c_char>()) as *mut *const c_char;
    if argv.is_null() { ERRNO!() = ENOMEM; return -1; }
    *argv.add(0) = arg;
    let mut n: usize = 1;
    loop {
        let a: *const c_char = args.next_arg();
        if a.is_null() { break; }
        if n + 1 >= cap {
            cap = match cap.checked_mul(2) { Some(v) => v, None => { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; } };
            let grown = realloc(argv as *mut c_void, cap * core::mem::size_of::<*const c_char>()) as *mut *const c_char;
            if grown.is_null() { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            argv = grown;
        }
        *argv.add(n) = a;
        n += 1;
    }
    *argv.add(n) = core::ptr::null();
    let r = sys_execve(path, argv, __environ as *const *const c_char);
    free(argv as *mut c_void);
    syscall_ret(r) as c_int
}

// ============================================================
// spawn.h: posix_spawn family
// ============================================================

// Matches musl's posix_spawn_file_actions_t layout.  Actions are a growable
// linked list; PATH search uses a stack buffer and an O(n) scan per entry.
unsafe fn spawn_execvp(file: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> ! {
    let flen = strlen(file as *const c_char);
    let f = file as *const u8;
    let mut has_slash = false;
    for i in 0..flen {
        if *f.add(i) == b'/' { has_slash = true; break; }
    }
    if has_slash {
        sys_execve(file, argv, envp);
        _exit(127);
    }
    let path = getenv(b"PATH\0".as_ptr() as *const c_char);
    let default_path = b"/usr/local/bin:/usr/bin:/bin\0";
    let p = if path.is_null() { default_path.as_ptr() } else { path as *const u8 };
    let plen = strlen(p as *const c_char);
    let mut buf = [0u8; PATH_MAX_V];
    let mut i: usize = 0;
    while i <= plen {
        let start = i;
        while i < plen && *p.add(i) != b':' { i += 1; }
        let dlen = i - start;
        if dlen > 0 && dlen + 1 + flen < PATH_MAX_V {
            core::ptr::copy_nonoverlapping(p.add(start), buf.as_mut_ptr(), dlen);
            *buf.as_mut_ptr().add(dlen) = b'/';
            core::ptr::copy_nonoverlapping(f, buf.as_mut_ptr().add(dlen + 1), flen + 1);
            sys_execve(buf.as_ptr() as *const c_char, argv, envp);
            // execve failed, try next
        }
        i += 1; // skip ':'
    }
    _exit(127);
}

#[no_mangle]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut c_int,
    file: *const c_char,
    fa: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let child = sys_fork();
    if child < 0 {
        ERRNO!() = (-child) as c_int;
        return -1;
    }
    if child == 0 {
        // child: apply attributes and file actions, then exec
        spawn_apply_attr(attrp);
        spawn_apply_actions(fa);
        let actual_envp = if envp.is_null() { __environ as *const *const c_char } else { envp };
        spawn_execvp(file, argv, actual_envp);
    }
    *pid = child as c_int;
    0
}

#[no_mangle]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut c_int,
    path: *const c_char,
    fa: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    // posix_spawnp() skips PATH search when the path contains '/', so it is
    // also the correct implementation of posix_spawn().
    posix_spawnp(pid, path, fa, attrp, argv, envp)
}

// ============================================================
// sys/stat.h: stat / fstat / utimensat / futimens
// ============================================================

#[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    _pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    _unused: [i64; 3],
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    __pad2: i32,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    __unused4: u32,
    __unused5: u32,
}

pub const S_IFMT: u32 = 0o170000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFIFO: u32 = 0o010000;
pub const S_IFSOCK: u32 = 0o140000;

pub const AT_FDCWD: i32 = -100;

const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
const AT_EACCESS: c_int = 0x200;
const AT_REMOVEDIR: c_int = 0x200;

#[no_mangle]
pub unsafe extern "C" fn stat(path: *const c_char, buf: *mut Stat) -> c_int {
    let r = sys_newfstatat(AT_FDCWD, path, buf as *mut u8, 0);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut Stat) -> c_int {
    let r = sys_fstat(fd, buf as *mut u8);
    syscall_ret(r) as c_int
}

// ============================================================
// sys/resource.h: getrlimit / setrlimit
// ============================================================

#[repr(C)]
pub struct Rlimit {
    pub rlim_cur: u64,
    pub rlim_max: u64,
}

pub const RLIMIT_NOFILE: c_int = 7;
pub const RLIMIT_STACK: c_int = 3;
pub const RLIMIT_DATA: c_int = 2;
pub const RLIMIT_AS: c_int = 9;
pub const RLIMIT_CPU: c_int = 0;
pub const RLIMIT_FSIZE: c_int = 1;
pub const RLIMIT_CORE: c_int = 4;
pub const RLIMIT_RSS: c_int = 5;
pub const RLIMIT_NPROC: c_int = 6;
pub const RLIMIT_MEMLOCK: c_int = 8;
pub const RLIMIT_LOCKS: c_int = 10;
pub const RLIMIT_SIGPENDING: c_int = 11;
pub const RLIMIT_MSGQUEUE: c_int = 12;
pub const RLIMIT_NICE: c_int = 13;
pub const RLIMIT_RTPRIO: c_int = 14;
pub const RLIMIT_RTTIME: c_int = 15;
pub const RLIM_INFINITY: u64 = !0u64;

#[no_mangle]
pub unsafe extern "C" fn getrlimit(resource: c_int, rlim: *mut Rlimit) -> c_int {
    let r = sys_getrlimit(resource, rlim as *mut u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int {
    let r = sys_setrlimit(resource, rlim as *const u8);
    syscall_ret(r) as c_int
}

// ============================================================
// utimensat / futimens
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn utimensat(dirfd: c_int, path: *const c_char, times: *const timespec, flags: c_int) -> c_int {
    let r = sys_utimensat(dirfd, path, times as *const u8, flags);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn futimens(fd: c_int, times: *const timespec) -> c_int {
    let r = sys_utimensat(fd, core::ptr::null(), times as *const u8, 0);
    syscall_ret(r) as c_int
}

include!("socket.rs");
include!("shm_sem.rs");
