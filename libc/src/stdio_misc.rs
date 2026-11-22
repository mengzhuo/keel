// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// getdelim / getline
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getdelim(
    lineptr: *mut *mut c_char, n: *mut usize, delim: c_int, stream: *mut FILE,
) -> isize {
    if lineptr.is_null() || n.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let mut cap = *n;
    let mut buf = *lineptr;
    if buf.is_null() || cap == 0 {
        cap = 128;
        buf = realloc(buf as *mut c_void, cap) as *mut c_char;
        if buf.is_null() { return -1; }
        *lineptr = buf;
        *n = cap;
    }
    let mut len = 0usize;
    loop {
        let c = fgetc(stream);
        if c == -1 { if len == 0 { return -1; } break; }
        if len >= cap - 1 {
            let new_cap = match cap.checked_mul(2) {
                Some(v) => v,
                None => {
                    ERRNO!() = ENOMEM;
                    return -1;
                }
            };
            let new_buf = realloc(buf as *mut c_void, new_cap) as *mut c_char;
            if new_buf.is_null() {
                ERRNO!() = ENOMEM;
                return -1;
            }
            buf = new_buf; cap = new_cap;
            *lineptr = buf; *n = cap;
        }
        *buf.add(len) = c as c_char;
        len += 1;
        if c == delim { break; }
    }
    *buf.add(len) = 0;
    len as isize
}

#[no_mangle]
pub unsafe extern "C" fn getline(lineptr: *mut *mut c_char, n: *mut usize, stream: *mut FILE) -> isize {
    getdelim(lineptr, n, b'\n' as c_int, stream)
}

// ============================================================
// popen / pclose
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn popen(cmd: *const c_char, mode: *const c_char) -> *mut FILE {
    if cmd.is_null() || mode.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let m = *mode;
    if (m != b'r' as c_char && m != b'w' as c_char) || *mode.add(1) != 0 {
        ERRNO!() = EINVAL;
        return core::ptr::null_mut();
    }
    let mut p: [c_int; 2] = [0; 2];
    let pipe_ret = sys_pipe2(p.as_mut_ptr(), O_CLOEXEC);
    if pipe_ret < 0 {
        ERRNO!() = (-pipe_ret) as c_int;
        return core::ptr::null_mut();
    }
    let op = if m == b'r' as c_char { 0 } else { 1 };
    let child_fd = p[1 - op];
    let f = fdopen(p[op], mode);
    if f.is_null() { sys_close(p[0] as i64); sys_close(p[1] as i64); return core::ptr::null_mut(); }
    let pid = sys_fork();
    if pid < 0 {
        let e = (-pid) as c_int;
        fclose(f); sys_close(child_fd as i64);
        ERRNO!() = e;
        return core::ptr::null_mut();
    }
    if pid == 0 {
        sys_close(p[op] as i64);
        if child_fd != (1 - op) as c_int { sys_dup3(child_fd, (1 - op) as c_int, 0); sys_close(child_fd as i64); }
        let sh = b"/bin/sh\0".as_ptr() as *const c_char;
        let argv = [b"sh\0".as_ptr() as *const c_char, b"-c\0".as_ptr() as *const c_char, cmd, core::ptr::null()];
        sys_execve(sh, argv.as_ptr(), __environ as *const *const c_char);
        _exit(127);
    }
    sys_close(child_fd as i64);
    (*f).pipe_pid = pid as c_int;
    f
}

#[no_mangle]
pub unsafe extern "C" fn pclose(f: *mut FILE) -> c_int {
    if f.is_null() { ERRNO!() = EINVAL; return -1; }
    let pid = (*f).pipe_pid;
    if pid <= 0 { ERRNO!() = EINVAL; return -1; }
    let r = fclose(f);
    let mut status: c_int = 0;
    let w = loop {
        let w = waitpid(pid, &mut status, 0);
        if w >= 0 || ERRNO!() != EINTR { break w; }
    };
    if w < 0 || r != 0 { return -1; }
    status
}

// ============================================================
// tmpfile / tmpnam / remove / rename
// ============================================================

const L_TMPNAM: usize = 20;
static TMPNAM_COUNTER: AtomicU32 = AtomicU32::new(0);

unsafe fn tmp_seed() -> c_uint {
    let mut v: u32 = 0;
    if crate::getrandom(
        &mut v as *mut u32 as *mut u8,
        core::mem::size_of::<u32>(),
        0,
    ) == core::mem::size_of::<u32>() as isize
    {
        return v;
    }
    let mut ts: timespec = core::mem::zeroed();
    if sys_clock_gettime(CLOCK_REALTIME, &mut ts) < 0 {
        return 0;
    }
    (ts.tv_nsec as u32) ^ (crate::getpid() as u32).rotate_left(16)
}

unsafe fn fill_tmpname(out: *mut c_char, seed: c_uint) {
    let prefix = b"/tmp/tmp\0";
    let mut k = 0;
    while k < prefix.len() - 1 { *out.add(k) = prefix[k] as c_char; k += 1; }
    let (buf, len) = format_hex(seed as u64, false);
    for j in 0..len.min(L_TMPNAM - k - 1) { *out.add(k + j) = buf[j] as c_char; k += 1; }
    *out.add(k) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn tmpfile() -> *mut FILE {
    let mut name = [0u8; L_TMPNAM];
    let mut ctr: c_uint = 0;
    loop {
        ctr = ctr.wrapping_add(1);
        let seed = tmp_seed().wrapping_add(ctr);
        fill_tmpname(name.as_mut_ptr() as *mut c_char, seed);
        let fd = sys_open(name.as_ptr(), (O_RDWR | O_CREAT | O_EXCL) as i64, 0o600);
        if fd >= 0 {
            sys_unlinkat(AT_FDCWD, name.as_ptr(), 0);
            let f = fdopen(fd as c_int, b"w+\0".as_ptr() as *const c_char);
            if f.is_null() { sys_close(fd); }
            return f;
        }
        if ctr > 100 { return core::ptr::null_mut(); }
    }
}

#[no_mangle]
pub unsafe extern "C" fn tmpnam(s: *mut c_char) -> *mut c_char {
    static mut BUF: [c_char; L_TMPNAM] = [0; L_TMPNAM];
    let out = if s.is_null() { core::ptr::addr_of_mut!(BUF) as *mut c_char } else { s };
    let counter = TMPNAM_COUNTER.fetch_add(1, Ordering::Relaxed);
    let seed = tmp_seed().wrapping_add(counter);
    fill_tmpname(out, seed);
    out
}

#[no_mangle]
pub unsafe extern "C" fn remove(path: *const c_char) -> c_int {
    if path.is_null() { ERRNO!() = EINVAL; return -1; }
    let r = sys_unlinkat(AT_FDCWD, path as *const u8, 0);
    if r == 0 {
        return 0;
    }
    // POSIX remove() also removes directories; musl falls back to rmdir only
    // when unlink reports EISDIR.
    if (-r) as c_int == 21 {
        return rmdir(path);
    }
    ERRNO!() = (-r) as c_int;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn rename(old: *const c_char, new_: *const c_char) -> c_int {
    if old.is_null() || new_.is_null() { ERRNO!() = EINVAL; return -1; }
    let r = sys_renameat2(AT_FDCWD, old as *const u8, AT_FDCWD, new_ as *const u8, 0);
    if r == 0 { 0 } else { ERRNO!() = (-r) as c_int; -1 }
}

