// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Legacy syscall/stdio/signal compatibility
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn settimeofday(tv: *const c_void, tz: *const c_void) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_SETTIMEOFDAY, tv as i64, tz as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn stime(t: *const i64) -> c_int {
    if t.is_null() { ERRNO!() = EINVAL; return -1; }
    let tv = timeval { tv_sec: *t, tv_usec: 0 };
    settimeofday(&tv as *const timeval as *const c_void, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn times(buf: *mut c_void) -> c_long {
    let r = <Arch as Syscalls>::syscall1(SYS_TIMES, buf as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as c_long }
}

#[no_mangle]
pub unsafe extern "C" fn splice(fd_in: c_int, off_in: *mut i64, fd_out: c_int, off_out: *mut i64, len: usize, flags: u32) -> isize {
    let r = <Arch as Syscalls>::syscall6(SYS_SPLICE, fd_in as i64, off_in as i64, fd_out as i64, off_out as i64, len as i64, flags as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

#[no_mangle]
pub unsafe extern "C" fn tee(fd_in: c_int, fd_out: c_int, len: usize, flags: u32) -> isize {
    let r = <Arch as Syscalls>::syscall4(SYS_TEE, fd_in as i64, fd_out as i64, len as i64, flags as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

#[no_mangle]
pub unsafe extern "C" fn statx(dirfd: c_int, path: *const c_char, flags: c_int, mask: u32, buf: *mut c_void) -> c_int {
    let r = <Arch as Syscalls>::syscall5(SYS_STATX, dirfd as i64, path as i64, flags as i64, mask as i64, buf as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setns(fd: c_int, nstype: c_int) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_SETNS, fd as i64, nstype as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setfsuid(uid: u32) -> c_int {
    <Arch as Syscalls>::syscall1(SYS_SETFSUID, uid as i64) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setfsgid(gid: u32) -> c_int {
    <Arch as Syscalls>::syscall1(SYS_SETFSGID, gid as i64) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setregid(rgid: u32, egid: u32) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_SETREGID, rgid as i64, egid as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn process_vm_readv(pid: c_int, local: *const c_void, liovcnt: usize, remote: *const c_void, riovcnt: usize, flags: u64) -> isize {
    let r = <Arch as Syscalls>::syscall6(SYS_PROCESS_VM_READV, pid as i64, local as i64, liovcnt as i64, remote as i64, riovcnt as i64, flags as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

#[no_mangle]
pub unsafe extern "C" fn process_vm_writev(pid: c_int, local: *const c_void, liovcnt: usize, remote: *const c_void, riovcnt: usize, flags: u64) -> isize {
    let r = <Arch as Syscalls>::syscall6(SYS_PROCESS_VM_WRITEV, pid as i64, local as i64, liovcnt as i64, remote as i64, riovcnt as i64, flags as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

#[no_mangle]
pub unsafe extern "C" fn tcgetwinsize(fd: c_int, ws: *mut c_void) -> c_int {
    if fd < 0 || ws.is_null() { ERRNO!() = EINVAL; return -1; }
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5413, ws as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn tcsetwinsize(fd: c_int, ws: *const c_void) -> c_int {
    if fd < 0 || ws.is_null() { ERRNO!() = EINVAL; return -1; }
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5414, ws as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sockatmark(fd: c_int) -> c_int {
    let mut at: c_int = 0;
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x8905, &mut at as *mut c_int as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { at }
}

#[no_mangle]
pub unsafe extern "C" fn isastream(fd: c_int) -> c_int {
    let mut buf = [0u8; 64];
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5401, buf.as_mut_ptr() as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; 0 } else { 1 }
}

#[no_mangle]
pub unsafe extern "C" fn posix_madvise(addr: *mut c_void, len: usize, advice: c_int) -> c_int {
    madvise(addr as *mut u8, len, advice)
}

#[no_mangle]
pub unsafe extern "C" fn secure_getenv(name: *const c_char) -> *mut c_char {
    if getauxval(23) != 0 { return core::ptr::null_mut(); }
    getenv(name)
}

#[no_mangle]
pub unsafe extern "C" fn sigandset(dest: *mut c_ulong, left: *const c_ulong, right: *const c_ulong) -> c_int {
    if dest.is_null() || left.is_null() || right.is_null() { return -1; }
    *dest = *left & *right;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigorset(dest: *mut c_ulong, left: *const c_ulong, right: *const c_ulong) -> c_int {
    if dest.is_null() || left.is_null() || right.is_null() { return -1; }
    *dest = *left | *right;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigisemptyset(set: *const c_ulong) -> c_int {
    if set.is_null() { return 1; }
    (*set == 0) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sighold(sig: c_int) -> c_int {
    let set: c_ulong = 1u64 << (sig - 1) as u32;
    sigprocmask(0, &set, core::ptr::null_mut());
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigignore(sig: c_int) -> c_int {
    let _ = signal(sig, 1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigrelse(sig: c_int) -> c_int {
    let set: c_ulong = 1u64 << (sig - 1) as u32;
    sigprocmask(1, &set, core::ptr::null_mut());
    0
}

#[no_mangle]
pub unsafe extern "C" fn siginterrupt(sig: c_int, flag: c_int) -> c_int {
    let mut sa: sigaction = core::mem::zeroed();
    if sigaction(sig, core::ptr::null(), &mut sa) != 0 { return -1; }
    if flag != 0 { sa.sa_flags &= !(SA_RESTART as c_int); } else { sa.sa_flags |= SA_RESTART as c_int; }
    sigaction(sig, &sa, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn sigpause(sig: c_int) -> c_int {
    let set: c_ulong = 1u64 << (sig - 1) as u32;
    sigsuspend(&set)
}

#[no_mangle]
pub unsafe extern "C" fn semtimedop(semid: c_int, sops: *mut c_void, nsops: usize, timeout: *const timespec) -> c_int {
    let r = <Arch as Syscalls>::syscall4(SYS_SEMTIMEDOP, semid as i64, sops as i64, nsops as i64, timeout as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn wait3(status: *mut c_int, options: c_int, rusage: *mut c_void) -> i32 {
    wait4(-1, status, options, rusage) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tempnam(dir: *const c_char, pfx: *const c_char) -> *mut c_char {
    let mut path = [0 as c_char; 256];
    let mut i = 0;
    if !dir.is_null() {
        while *dir.add(i) != 0 && i < 200 { path[i] = *dir.add(i); i += 1; }
        if i > 0 && path[i-1] != b'/' as c_char { path[i] = b'/' as c_char; i += 1; }
    } else {
        for &c in b"/tmp/" { path[i] = c as c_char; i += 1; }
    }
    if !pfx.is_null() {
        let mut j = 0;
        while *pfx.add(j) != 0 && i < 220 { path[i] = *pfx.add(j); i += 1; j += 1; }
    }
    for &c in b"XXXXXX" { path[i] = c as c_char; i += 1; }
    path[i] = 0;
    let fd = mkstemp(path.as_mut_ptr());
    if fd < 0 { return core::ptr::null_mut(); }
    close(fd);
    unlink(path.as_ptr());
    let n = strlen(path.as_ptr());
    let out = malloc(n + 1) as *mut c_char;
    if out.is_null() { return core::ptr::null_mut(); }
    core::ptr::copy_nonoverlapping(path.as_ptr(), out, n + 1);
    out
}

#[no_mangle]
pub unsafe extern "C" fn putwc(c: u32, f: *mut c_void) -> u32 {
    fputwc(c as c_int, f as *mut FILE) as u32
}

#[no_mangle]
pub unsafe extern "C" fn __uflow(f: *mut c_void) -> c_int {
    fgetc(f as *mut FILE)
}

#[no_mangle]
pub unsafe extern "C" fn __xmknod(_ver: c_int, path: *const c_char, mode: c_uint, dev: u64) -> c_int {
    mknod(path, mode, dev)
}

#[no_mangle]
pub unsafe extern "C" fn __xmknodat(_ver: c_int, dirfd: c_int, path: *const c_char, mode: c_uint, dev: u64) -> c_int {
    mknodat(dirfd, path, mode, dev)
}

#[no_mangle]
pub unsafe extern "C" fn name_to_handle_at(dirfd: c_int, path: *const c_char, handle: *mut c_void, mount_id: *mut c_int, flags: c_int) -> c_int {
    let r = <Arch as Syscalls>::syscall5(SYS_NAME_TO_HANDLE_AT, dirfd as i64, path as i64, handle as i64, mount_id as i64, flags as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn open_by_handle_at(mount_fd: c_int, handle: *mut c_void, flags: c_int) -> c_int {
    let r = <Arch as Syscalls>::syscall3(SYS_OPEN_BY_HANDLE_AT, mount_fd as i64, handle as i64, flags as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn dn_skipname(comp: *const u8, eom: *const u8) -> c_int {
    let len = match (eom as usize).checked_sub(comp as usize) {
        Some(n) => n,
        None => return -1,
    };
    let msg = match InBuf::new(comp, len) {
        Some(msg) => msg,
        None => return -1,
    };
    match dns_skip_name(&msg, 0) {
        Some(n) if n <= c_int::MAX as usize => n as c_int,
        _ => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn dn_comp(exp: *const c_char, dst: *mut u8, dstsiz: c_int, _dnptrs: *mut *mut u8, _lastdnptr: *mut *mut u8) -> c_int {
    if exp.is_null() || dst.is_null() || dstsiz <= 0 { return -1; }
    let mut o = 0usize;
    let mut p = exp;
    loop {
        let start = p;
        while *p != 0 && *p != b'.' as c_char { p = p.add(1); }
        let l = p as usize - start as usize;
        if l > 63 || o + l + 2 > dstsiz as usize { return -1; }
        *dst.add(o) = l as u8; o += 1;
        core::ptr::copy_nonoverlapping(start as *const u8, dst.add(o), l);
        o += l;
        if *p == 0 { break; }
        p = p.add(1);
    }
    if o + 1 > dstsiz as usize { return -1; }
    *dst.add(o) = 0;
    (o + 1) as c_int
}

