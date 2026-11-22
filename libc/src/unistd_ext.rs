// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// unistd extensions: pipe, dup, dup2, fcntl, access, unlink, etc.
// ============================================================

#[inline]
unsafe fn sys_dup(oldfd: i32) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_DUP, oldfd as i64)
}

#[inline]unsafe fn sys_dup2(oldfd: i32, newfd: i32) -> i64 {
    // dup3 with flags=0 is equivalent to dup2
    sys_dup3(oldfd, newfd, 0)
}

#[inline]
unsafe fn sys_access(path: *const u8, mode: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_FACCESSAT, AT_FDCWD as i64, path as i64, mode as i64, 0)
}

#[inline]unsafe fn sys_unlink(path: *const u8) -> i64 {
    sys_unlinkat(AT_FDCWD, path, 0)
}

#[inline]unsafe fn sys_rmdir(path: *const u8) -> i64 {
    sys_unlinkat(AT_FDCWD, path, AT_REMOVEDIR)
}

#[inline]
unsafe fn sys_chdir(path: *const u8) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_CHDIR, path as i64)
}

#[inline]
unsafe fn sys_getcwd(buf: *mut u8, size: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_GETCWD, buf as i64, size as i64)
}

#[inline]
unsafe fn sys_sethostname(name: *const u8, len: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SETHOSTNAME, name as i64, len as i64)
}

#[inline]
unsafe fn sys_gethostname(buf: *mut u8, len: usize) -> i64 {
    #[repr(C)]
    struct UtsName {
        sysname: [u8; 65],
        nodename: [u8; 65],
        release: [u8; 65],
        version: [u8; 65],
        machine: [u8; 65],
        domainname: [u8; 65],
    }
    let mut uts: UtsName = core::mem::zeroed();
    let result = <Arch as Syscalls>::syscall1(SYS_UNAME, &mut uts as *mut UtsName as i64);
    if result < 0 {
        return result;
    }
    let nlen = strlen(uts.nodename.as_ptr() as *const c_char);
    let copylen = if nlen < len { nlen } else { if len > 0 { len - 1 } else { 0 } };
    core::ptr::copy_nonoverlapping(uts.nodename.as_ptr(), buf, copylen);
    if len > 0 { *buf.add(copylen) = 0; }
    0
}

unsafe fn sys_truncate(path: *const u8, length: i64) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_TRUNCATE, path as i64, length as i64)
}

unsafe fn sys_ftruncate(fd: i32, length: i64) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_FTRUNCATE, fd as i64, length as i64)
}

unsafe fn sys_nanosleep(req: *const timespec, rem: *mut timespec) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_NANOSLEEP, req as i64, rem as i64)
}

const ITIMER_REAL: i32 = 0;

#[repr(C)]
struct itimerval {
    it_interval: timeval,
    it_value: timeval,
}

unsafe fn sys_alarm(seconds: c_uint) -> i64 {
    let mut new: itimerval = core::mem::zeroed();
    new.it_value.tv_sec = seconds as c_long;
    let mut old: itimerval = core::mem::zeroed();
    let r = <Arch as Syscalls>::syscall3(
        SYS_SETITIMER,
        ITIMER_REAL as i64,
        &new as *const _ as i64,
        &mut old as *mut _ as i64,
    );
    if r < 0 {
        return r;
    }
    old.it_value.tv_sec as i64 + (old.it_value.tv_usec != 0) as i64
}

unsafe fn sys_pause() -> i64 {
    #[cfg(target_arch = "x86_64")]
    { <Arch as Syscalls>::syscall0(SYS_PAUSE) }
    #[cfg(target_arch = "aarch64")]
    { <Arch as Syscalls>::syscall4(SYS_PPOLL, 0, 0, 0, 0) }
    #[cfg(target_arch = "riscv64")]
    { <Arch as Syscalls>::syscall4(SYS_PPOLL, 0, 0, 0, 0) }
    #[cfg(target_arch = "powerpc64")]
    { <Arch as Syscalls>::syscall0(SYS_PAUSE) }
    #[cfg(target_arch = "loongarch64")]
    { <Arch as Syscalls>::syscall4(SYS_PPOLL, 0, 0, 0, 0) }
}

unsafe fn sys_fsync(fd: i32) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_FSYNC, fd as i64)
}

unsafe fn sys_fdatasync(fd: i32) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_FDATASYNC, fd as i64)
}

unsafe fn sys_sync() {
    let _ = <Arch as Syscalls>::syscall0(SYS_SYNC);
}

unsafe fn sys_symlink(target: *const u8, linkpath: *const u8) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SYMLINKAT, target as i64, AT_FDCWD as i64, linkpath as i64)
}

unsafe fn sys_symlinkat(target: *const u8, newdirfd: i32, linkpath: *const u8) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SYMLINKAT, target as i64, newdirfd as i64, linkpath as i64)
}

unsafe fn sys_readlinkat(dirfd: i32, path: *const u8, buf: *mut u8, bufsiz: usize) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_READLINKAT, dirfd as i64, path as i64, buf as i64, bufsiz as i64)
}

unsafe fn sys_linkat(olddirfd: i32, oldpath: *const u8, newdirfd: i32, newpath: *const u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall5(SYS_LINKAT, olddirfd as i64, oldpath as i64, newdirfd as i64, newpath as i64, flags as i64)
}

unsafe fn sys_fchmod(fd: i32, mode: u32) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_FCHMOD, fd as i64, mode as i64)
}

unsafe fn sys_fchmodat(dirfd: i32, path: *const u8, mode: u32, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_FCHMODAT, dirfd as i64, path as i64, mode as i64, flags as i64)
}

unsafe fn sys_umask(mask: u32) -> u32 {
    <Arch as Syscalls>::syscall1(SYS_UMASK, mask as i64) as u32
}

unsafe fn sys_getgroups(size: i32, list: *mut c_uint) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_GETGROUPS, size as i64, list as i64)
}

unsafe fn sys_setuid(uid: c_uint) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_SETUID, uid as i64)
}

unsafe fn sys_setgid(gid: c_uint) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_SETGID, gid as i64)
}

unsafe fn sys_setsid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_SETSID)
}

unsafe fn sys_setpgid(pid: c_int, pgid: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SETPGID, pid as i64, pgid as i64)
}

unsafe fn sys_getpgid(pid: c_int) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_GETPGID, pid as i64)
}

unsafe fn sys_getsid(pid: c_int) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_GETSID, pid as i64)
}

unsafe fn sys_mkdirat(dirfd: i32, path: *const u8, mode: u32) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_MKDIRAT, dirfd as i64, path as i64, mode as i64)
}

// Public C ABI wrappers for unistd

#[no_mangle]
pub unsafe extern "C" fn pipe(fds: *mut c_int) -> c_int {
    let r = sys_pipe2(fds, 0);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn pipe2(fds: *mut c_int, flags: c_int) -> c_int {
    let r = sys_pipe2(fds, flags);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn dup(oldfd: c_int) -> c_int {
    let r = sys_dup(oldfd);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn dup2(oldfd: c_int, newfd: c_int) -> c_int {
    if oldfd == newfd {
        // dup3 rejects equal descriptors with EINVAL; dup2 must validate the
        // descriptor and return newfd unchanged.
        let r = sys_fcntl(oldfd, 1 /* F_GETFD */, 0);
        if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
        return newfd;
    }
    let r = sys_dup2(oldfd, newfd);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn dup3(oldfd: c_int, newfd: c_int, flags: c_int) -> c_int {
    let r = sys_dup3(oldfd, newfd, flags);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fcntl(fd: c_int, cmd: c_int, mut args: ...) -> c_int {
    // Commands without an argument. Everything else (including seals,
    // pipe sizes, ownership and locks) takes the third argument.
    let no_arg = matches!(cmd, F_GETFD | F_GETFL | F_GETOWN | F_GETSIG
        | F_GETLEASE | F_GETPIPE_SZ | F_GET_SEALS);
    let r = if no_arg {
        sys_fcntl(fd, cmd, 0)
    } else {
        let arg = args.next_arg::<c_long>();
        sys_fcntl(fd, cmd, arg)
    };
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn access(path: *const c_char, mode: c_int) -> c_int {
    let r = sys_access(path as *const u8, mode);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn unlink(path: *const c_char) -> c_int {
    let r = sys_unlink(path as *const u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn rmdir(path: *const c_char) -> c_int {
    let r = sys_rmdir(path as *const u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn chdir(path: *const c_char) -> c_int {
    let r = sys_chdir(path as *const u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char {
    if buf.is_null() {
        // allocate
        let alloc_size = if size == 0 { 256 } else { size };
        let p = malloc(alloc_size) as *mut c_char;
        if p.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
        let r = sys_getcwd(p as *mut u8, alloc_size);
        if r < 0 { free(p as *mut c_void); ERRNO!() = (-r) as c_int; return core::ptr::null_mut(); }
        // The kernel returns "(unreachable)..." when cwd is outside the
        // current root; POSIX getcwd must fail with ENOENT in that case.
        if *p as u8 != b'/' {
            free(p as *mut c_void);
            ERRNO!() = ENOENT;
            return core::ptr::null_mut();
        }
        return p;
    }
    if size == 0 { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let r = sys_getcwd(buf as *mut u8, size);
    if r < 0 { ERRNO!() = (-r) as c_int; return core::ptr::null_mut(); }
    if *buf as u8 != b'/' {
        ERRNO!() = ENOENT;
        return core::ptr::null_mut();
    }
    buf
}

#[no_mangle]
pub unsafe extern "C" fn gethostname(name: *mut c_char, len: usize) -> c_int {
    let r = sys_gethostname(name as *mut u8, len);
    syscall_ret(r) as c_int
}

// Linux page size.  This is hardcoded to 4096 on every architecture; that
// is correct for the CI hosts but not for aarch64, powerpc64le, riscv64 or
// loongarch64 kernels configured with a larger base page size.  See
// sysconf(_SC_PAGE_SIZE) for the same value.
#[no_mangle]
pub extern "C" fn getpagesize() -> c_int {
    PAGE as c_int
}

#[no_mangle]
pub unsafe extern "C" fn truncate(path: *const c_char, length: i64) -> c_int {
    let r = sys_truncate(path as *const u8, length);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn ftruncate(fd: c_int, length: i64) -> c_int {
    let r = sys_ftruncate(fd, length);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn nanosleep(req: *const timespec, rem: *mut timespec) -> c_int {
    pthread_testcancel();
    let r = sys_nanosleep(req, rem);
    pthread_testcancel();
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sleep(seconds: c_uint) -> c_uint {
    let req = timespec { tv_sec: seconds as c_long, tv_nsec: 0 };
    let mut rem: timespec = core::mem::zeroed();
    pthread_testcancel();
    let r = sys_nanosleep(&req, &mut rem);
    pthread_testcancel();
    if r < 0 {
        let e = (-r) as c_int;
        if e == EINTR { return rem.tv_sec as c_uint; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn usleep(usec: c_uint) -> c_int {
    let req = timespec { tv_sec: (usec / 1000000) as c_long, tv_nsec: ((usec % 1000000) * 1000) as c_long };
    pthread_testcancel();
    let r = sys_nanosleep(&req, core::ptr::null_mut());
    pthread_testcancel();
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn alarm(seconds: c_uint) -> c_uint {
    // POSIX rounds a nonzero fractional remainder up to one second;
    // sys_alarm() already performs that rounding.
    let r = sys_alarm(seconds);
    if r < 0 { 0 } else { r as c_uint }
}

#[no_mangle]
pub unsafe extern "C" fn pause() -> c_int {
    pthread_testcancel();
    sys_pause();
    pthread_testcancel();
    ERRNO!() = EINTR;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn fsync(fd: c_int) -> c_int {
    let r = sys_fsync(fd);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fdatasync(fd: c_int) -> c_int {
    let r = sys_fdatasync(fd);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sync() {
    sys_sync();
}

#[no_mangle]
pub unsafe extern "C" fn symlink(target: *const c_char, linkpath: *const c_char) -> c_int {
    let r = sys_symlinkat(target as *const u8, AT_FDCWD, linkpath as *const u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn readlink(path: *const c_char, buf: *mut c_char, bufsiz: usize) -> isize {
    let r = sys_readlinkat(AT_FDCWD, path as *const u8, buf as *mut u8, bufsiz);
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn link(oldpath: *const c_char, newpath: *const c_char) -> c_int {
    let r = sys_linkat(AT_FDCWD, oldpath as *const u8, AT_FDCWD, newpath as *const u8, 0);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn chmod(path: *const c_char, mode: c_uint) -> c_int {
    let r = sys_fchmodat(AT_FDCWD, path as *const u8, mode, 0);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fchmod(fd: c_int, mode: c_uint) -> c_int {
    let r = sys_fchmod(fd, mode);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn umask(mask: c_uint) -> c_uint {
    sys_umask(mask) as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn isatty(fd: c_int) -> c_int {
    let mut ws: winsize = core::mem::zeroed();
    if sys_ioctl(fd, TIOCGWINSZ, &mut ws as *mut winsize as *mut u8) == 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn getlogin() -> *mut c_char {
    crate::getenv(b"LOGNAME\0".as_ptr() as *const c_char)
}

#[no_mangle]
pub unsafe extern "C" fn getgroups(size: c_int, list: *mut c_uint) -> c_int {
    let r = sys_getgroups(size, list);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setuid(uid: c_uint) -> c_int {
    let r = sys_setuid(uid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setgid(gid: c_uint) -> c_int {
    let r = sys_setgid(gid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn seteuid(euid: c_uint) -> c_int {
    // Setting only the effective uid must keep the real and saved uids, so a
    // process can drop privileges and later restore them (setuid would clear
    // the saved uid for root).
    setresuid(u32::MAX, euid, u32::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn setegid(egid: c_uint) -> c_int {
    setresgid(u32::MAX, egid, u32::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn setreuid(ruid: c_uint, euid: c_uint) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_SETREUID, ruid as i64, euid as i64);
    syscall_ret(r) as c_int
}



#[no_mangle]
pub unsafe extern "C" fn setsid() -> c_int {
    let r = sys_setsid();
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setpgid(pid: c_int, pgid: c_int) -> c_int {
    let r = sys_setpgid(pid, pgid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getpgid(pid: c_int) -> c_int {
    let r = sys_getpgid(pid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getsid(pid: c_int) -> c_int {
    let r = sys_getsid(pid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getpgrp() -> c_int {
    sys_getpgid(0) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setpgrp() -> c_int {
    setpgid(0, 0)
}

#[no_mangle]
unsafe fn mkstemps_impl(template: *mut c_char, suffixlen: i32) -> c_int {
    mkstemps_flags(template, suffixlen, 0)
}

unsafe fn mkstemps_flags(template: *mut c_char, suffixlen: i32, flags: i32) -> c_int {
    if template.is_null() || suffixlen < 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    let len = strlen(template as *const c_char);
    let suffix = suffixlen as usize;
    if len < 6 + suffix {
        ERRNO!() = EINVAL;
        return -1;
    }
    let x_start = len - suffix - 6;
    for i in x_start..x_start + 6 {
        if *template.add(i) != b'X' as c_char {
            ERRNO!() = EINVAL;
            return -1;
        }
    }
    // Prefer the kernel CSPRNG for the suffix. O_EXCL is the real
    // protection, but unpredictable names also avoid trivial pre-creation
    // denial of service in a shared /tmp.
    let mut ctr: u32 = 0;
    if crate::getrandom(&mut ctr as *mut u32 as *mut u8, core::mem::size_of::<u32>(), 0)
        != core::mem::size_of::<u32>() as isize
    {
        let mut ts: timespec = core::mem::zeroed();
        let _ = sys_clock_gettime(CLOCK_REALTIME, &mut ts);
        ctr = (ts.tv_nsec as u32) ^ (crate::getpid() as u32).rotate_left(16);
    }
    for _ in 0..100 {
        ctr = ctr.wrapping_mul(1103515245).wrapping_add(12345);
        let mut seed = ctr;
        let hex = b"0123456789abcdef";
        for i in x_start..x_start + 6 {
            *template.add(i) = hex[(seed & 0xf) as usize] as c_char;
            seed >>= 4;
        }
        let fd = sys_open(template as *const u8, (O_RDWR | O_CREAT | O_EXCL | flags) as i64, 0o600);
        if fd >= 0 {
            return fd as c_int;
        }
        let e = (-fd) as c_int;
        if e != EEXIST {
            ERRNO!() = e;
            return -1;
        }
    }
    ERRNO!() = EEXIST;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn mkstemp(template: *mut c_char) -> c_int {
    mkstemps_impl(template, 0)
}

#[no_mangle]
pub unsafe extern "C" fn mkdir(path: *const c_char, mode: c_uint) -> c_int {
    let r = sys_mkdirat(AT_FDCWD, path as *const u8, mode);
    syscall_ret(r) as c_int
}

