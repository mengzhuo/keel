// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Remaining symbols - batch 1
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn __lgammal_r(x: f64, signgamp: *mut i32) -> f64 {
    *signgamp = 1;
    lgamma(x)
}

#[no_mangle]
pub unsafe extern "C" fn __libc_current_sigrtmax() -> i32 {
    64
}

#[no_mangle]
pub unsafe extern "C" fn _longjmp(env: *mut core::ffi::c_void, val: i32) {
    crate::longjmp(env as *mut u64, val)
}

#[no_mangle]
pub unsafe extern "C" fn __newlocale(_mask: i32, _name: *const core::ffi::c_char, base: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    base
}

#[no_mangle]
pub unsafe extern "C" fn __nl_langinfo(_item: i32) -> *mut core::ffi::c_char {
    b"\0".as_ptr() as *mut core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn __nl_langinfo_l(_item: i32, _loc: *mut core::ffi::c_void) -> *mut core::ffi::c_char {
    b"\0".as_ptr() as *mut core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn ns_get16(cp: *const u8) -> u16 {
    ((*cp.add(0) as u16) << 8) | *cp.add(1) as u16
}

#[no_mangle]
pub unsafe extern "C" fn ns_get32(cp: *const u8) -> u32 {
    ((*cp.add(0) as u32) << 24) | ((*cp.add(1) as u32) << 16) | ((*cp.add(2) as u32) << 8) | *cp.add(3) as u32
}

#[no_mangle]
pub unsafe extern "C" fn ns_put16(s: u16, dst: *mut u8) {
    *dst.add(0) = (s >> 8) as u8;
    *dst.add(1) = (s & 0xff) as u8;
}

#[no_mangle]
pub unsafe extern "C" fn ns_put32(l: u32, dst: *mut u8) {
    *dst.add(0) = (l >> 24) as u8;
    *dst.add(1) = ((l >> 16) & 0xff) as u8;
    *dst.add(2) = ((l >> 8) & 0xff) as u8;
    *dst.add(3) = (l & 0xff) as u8;
}

#[no_mangle]
pub unsafe extern "C" fn posix_close(fd: i32) -> i32 {
    crate::close(fd)
}

#[no_mangle]
pub unsafe extern "C" fn posix_openpt(flags: i32) -> i32 {
    crate::open(b"/dev/ptmx\0".as_ptr() as *const core::ffi::c_char, flags, 0)
}

#[no_mangle]
pub unsafe extern "C" fn putw(w: i32, f: *mut core::ffi::c_void) {
    putc((w & 0xff) as i32, f as *mut FILE);
    putc(((w >> 8) & 0xff) as i32, f as *mut FILE);
    putc(((w >> 16) & 0xff) as i32, f as *mut FILE);
    putc(((w >> 24) & 0xff) as i32, f as *mut FILE);
}



#[no_mangle]
pub unsafe extern "C" fn rand_r(seedp: *mut u32) -> i32 {
    let seed = (*seedp).wrapping_mul(1103515245).wrapping_add(12345);
    *seedp = seed;
    ((seed >> 16) & 0x7fff) as i32
}

#[no_mangle]
pub unsafe extern "C" fn reallocarray(ptr: *mut core::ffi::c_void, nmemb: usize, size: usize) -> *mut core::ffi::c_void {
    let total = match nmemb.checked_mul(size) {
        Some(v) => v,
        None => { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    };
    crate::realloc(ptr, total)
}

#[no_mangle]
pub unsafe extern "C" fn __res_state() -> *mut core::ffi::c_void {
    static mut STATE: [u8; 512] = [0; 512];
    STATE.as_mut_ptr() as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn sbrk(increment: isize) -> *mut u8 {
    // musl does not support growing the break directly (malloc uses mmap);
    // sbrk(nonzero) always fails with ENOMEM.
    if increment != 0 {
        ERRNO!() = ENOMEM;
        return -1isize as usize as *mut u8;
    }
    <Arch as Syscalls>::syscall1(SYS_BRK, 0) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn scalb(x: f64, exp: f64) -> f64 {
    if x.is_nan() || exp.is_nan() { return x * exp; }
    if !exp.is_finite() {
        let r = if exp > 0.0 { x * exp } else { x / (-exp) };
        if r.is_nan() && !x.is_nan() {
            feraiseexcept(FE_INVALID);
        }
        return r;
    }
    if libm::rint(exp) != exp {
        return __math_invalid(exp);
    }
    if exp > 65000.0 { return libm::scalbn(x, 65000); }
    if -exp > 65000.0 { return libm::scalbn(x, -65000); }
    libm::scalbn(x, exp as i32)
}

#[no_mangle]
pub unsafe extern "C" fn scalbf(x: f32, exp: f32) -> f32 {
    if x.is_nan() || exp.is_nan() { return x * exp; }
    if !exp.is_finite() {
        let r = if exp > 0.0 { x * exp } else { x / (-exp) };
        if r.is_nan() && !x.is_nan() {
            feraiseexcept(FE_INVALID);
        }
        return r;
    }
    if libm::rintf(exp) != exp {
        return __math_invalidf(exp);
    }
    if exp > 65000.0 { return libm::scalbnf(x, 65000); }
    if -exp > 65000.0 { return libm::scalbnf(x, -65000); }
    libm::scalbnf(x, exp as i32)
}

#[no_mangle]
pub unsafe extern "C" fn __sched_cpucount(setsize: usize, setp: *const u8) -> i32 {
    let mut count = 0;
    let mut i = 0;
    while i < setsize * 8 {
        if (*setp.add(i / 8) >> (i % 8)) & 1 != 0 { count += 1; }
        i += 1;
    }
    count
}



#[no_mangle]
pub unsafe extern "C" fn select(nfds: i32, readfds: *mut u8, writefds: *mut u8, exceptfds: *mut u8, timeout: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall6(SYS_SELECT, nfds as i64, readfds as i64, writefds as i64, exceptfds as i64, timeout as i64, 0)) as i32
}



#[no_mangle]
pub unsafe extern "C" fn sendfile(out_fd: i32, in_fd: i32, offset: *mut i64, count: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_SENDFILE, out_fd as i64, in_fd as i64, offset as i64, count as i64)) as isize
}





#[no_mangle]
#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
pub unsafe extern "C" fn __setjmp(env: *mut core::ffi::c_void) -> i32 {
    crate::setjmp(env as *mut u64)
}

#[no_mangle]
#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
pub unsafe extern "C" fn _setjmp(env: *mut core::ffi::c_void) -> i32 {
    crate::setjmp(env as *mut u64)
}

#[no_mangle]
pub unsafe extern "C" fn setlinebuf(stream: *mut core::ffi::c_void) {
    setvbuf(stream as *mut FILE, core::ptr::null_mut(), 1, 0);
}















#[no_mangle]
pub unsafe extern "C" fn significand(x: f64) -> f64 {
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp == 0 || exp == 0x7ff { return x; }
    f64::from_bits((bits & 0x000fffffffffffff) | (1023u64 << 52))
}

#[no_mangle]
pub unsafe extern "C" fn significandf(x: f32) -> f32 {
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32;
    if exp == 0 || exp == 0xff { return x; }
    f32::from_bits((bits & 0x007fffff) | (127u32 << 23))
}





#[no_mangle]
pub unsafe extern "C" fn sigset(sig: i32, handler: usize) -> usize {
    // BSD/glibc-compatible sigset() is an alias for signal().
    signal(sig, handler)
}

#[no_mangle]
pub unsafe extern "C" fn sincos(x: f64, sin: *mut f64, cos: *mut f64) {
    *sin = libm::sin(x);
    *cos = libm::cos(x);
}

#[no_mangle]
pub unsafe extern "C" fn sincosf(x: f32, sin: *mut f32, cos: *mut f32) {
    *sin = libm::sinf(x);
    *cos = libm::cosf(x);
}









unsafe fn nd_ascii_lower(c: c_char) -> c_int {
    let b = c as u8;
    if b >= b'A' && b <= b'Z' { (b + 32) as c_int } else { b as c_int }
}

unsafe fn nd_strncasecmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let c1 = nd_ascii_lower(*s1.add(i));
        let c2 = nd_ascii_lower(*s2.add(i));
        if c1 != c2 || c1 == 0 { return c1 - c2; }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strcasecmp(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char) -> i32 {
    nd_strncasecmp(s1, s2, usize::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn __strcasecmp_l(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char, _loc: *mut core::ffi::c_void) -> i32 {
    nd_strncasecmp(s1, s2, usize::MAX)
}

#[no_mangle]
pub unsafe extern "C" fn strcasestr(haystack: *const core::ffi::c_char, needle: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let nlen = crate::strlen(needle);
    if nlen == 0 { return haystack as *mut core::ffi::c_char; }
    let mut i = 0;
    loop {
        let h = haystack.add(i);
        if *h == 0 { return core::ptr::null_mut(); }
        if crate::strncasecmp(h, needle, nlen) == 0 { return h as *mut core::ffi::c_char; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn __strcoll_l(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char, _loc: *mut core::ffi::c_void) -> i32 {
    crate::strcmp(s1 as *const u8, s2 as *const u8)
}

#[no_mangle]
pub unsafe extern "C" fn strncasecmp(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char, n: usize) -> i32 {
    nd_strncasecmp(s1, s2, n)
}

#[no_mangle]
pub unsafe extern "C" fn __strncasecmp_l(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char, n: usize, _loc: *mut core::ffi::c_void) -> i32 {
    nd_strncasecmp(s1, s2, n)
}

#[no_mangle]
pub unsafe extern "C" fn strsep(stringp: *mut *mut core::ffi::c_char, delim: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    let s = *stringp;
    if s.is_null() { return core::ptr::null_mut(); }
    let mut p = s;
    while *p != 0 {
        let mut d = delim;
        while *d != 0 {
            if *p == *d { *p = 0; *stringp = p.add(1); return s; }
            d = d.add(1);
        }
        p = p.add(1);
    }
    *stringp = core::ptr::null_mut();
    s
}

#[no_mangle]
pub unsafe extern "C" fn strtoimax(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, base: i32) -> i64 {
    crate::strtol(nptr, endptr, base)
}

#[no_mangle]
pub unsafe extern "C" fn strtok_r(s: *mut core::ffi::c_char, delim: *const core::ffi::c_char, saveptr: *mut *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    let mut s = if s.is_null() { *saveptr } else { s };
    if s.is_null() { return core::ptr::null_mut(); }
    // Skip leading delimiters
    while *s != 0 {
        let mut d = delim;
        let mut found = false;
        while *d != 0 {
            if *s == *d { found = true; break; }
            d = d.add(1);
        }
        if !found { break; }
        s = s.add(1);
    }
    if *s == 0 { *saveptr = s; return core::ptr::null_mut(); }
    let start = s;
    // Find end of token
    while *s != 0 {
        let mut d = delim;
        while *d != 0 {
            if *s == *d { *s = 0; *saveptr = s.add(1); return start; }
            d = d.add(1);
        }
        s = s.add(1);
    }
    *saveptr = s;
    start
}

#[no_mangle]
pub unsafe extern "C" fn strtoumax(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, base: i32) -> u64 {
    crate::strtoul(nptr, endptr, base) as u64
}

#[no_mangle]
pub unsafe extern "C" fn __strxfrm_l(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char, n: usize, _loc: *mut core::ffi::c_void) -> usize {
    crate::strlcpy(dest as *mut u8, src as *const u8, n)
}

#[no_mangle]
pub unsafe extern "C" fn swab(src: *const u8, dest: *mut u8, nbytes: isize) {
    let mut i = 0;
    while i + 1 < nbytes {
        let a = *src.add(i as usize);
        let b = *src.add(i as usize + 1);
        *dest.add(i as usize) = b;
        *dest.add(i as usize + 1) = a;
        i += 2;
    }
}











#[no_mangle]
pub unsafe extern "C" fn __tolower_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    if c >= 'A' as i32 && c <= 'Z' as i32 { c + 32 } else { c }
}

#[no_mangle]
pub unsafe extern "C" fn __toupper_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    if c >= 'a' as i32 && c <= 'z' as i32 { c - 32 } else { c }
}

#[no_mangle]
pub unsafe extern "C" fn ulimit(cmd: c_int, mut args: ...) -> c_long {
    let mut rl: Rlimit = core::mem::zeroed();
    if getrlimit(RLIMIT_FSIZE, &mut rl) != 0 { return -1; }
    if cmd == 2 {
        let val = args.next_arg::<c_long>();
        rl.rlim_cur = 512u64.wrapping_mul(val as u64);
        if setrlimit(RLIMIT_FSIZE, &rl) != 0 { return -1; }
    }
    (rl.rlim_cur / 512) as c_long
}

#[repr(C)]
struct UtsName {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

#[no_mangle]
pub unsafe extern "C" fn uname(buf: *mut c_void) -> c_int {
    // Let the kernel validate the user pointer: an invalid but non-null
    // address must produce EFAULT instead of a fault in a local copy.
    let r = <Arch as Syscalls>::syscall1(SYS_UNAME, buf as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn unshare(flags: i32) -> i32 {
    let r = <Arch as Syscalls>::syscall1(SYS_UNSHARE, flags as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn updwtmpx(wtmpx_file: *const core::ffi::c_char, utx: *const core::ffi::c_void) {
    updwtmp(wtmpx_file, utx)
}

#[no_mangle]
pub unsafe extern "C" fn __uselocale(loc: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    loc
}

#[repr(C)]
pub struct Utimbuf { actime: i64, modtime: i64 }

#[no_mangle]
pub unsafe extern "C" fn utime(filename: *const c_char, times: *const Utimbuf) -> c_int {
    if times.is_null() {
        return utimensat(AT_FDCWD, filename, core::ptr::null(), 0);
    }
    let ts = [
        timespec { tv_sec: (*times).actime, tv_nsec: 0 },
        timespec { tv_sec: (*times).modtime, tv_nsec: 0 },
    ];
    utimensat(AT_FDCWD, filename, ts.as_ptr(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn vhangup() -> c_int {
    let r = <Arch as Syscalls>::syscall0(SYS_VHANGUP);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn vmsplice(fd: i32, iov: *const c_void, nr_segs: usize, flags: u32) -> isize {
    let r = <Arch as Syscalls>::syscall4(SYS_VMSPLICE, fd as i64, iov as i64, nr_segs as i64, flags as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}



#[no_mangle]
pub unsafe extern "C" fn wait4(pid: i32, wstatus: *mut i32, options: i32, rusage: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_WAIT4, pid as i64, wstatus as i64, options as i64, rusage as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn waitid(idtype: i32, id: u32, infop: *mut core::ffi::c_void, options: i32) -> i32 {
    // The kernel takes a fifth rusage pointer; it must be NULL, not whatever
    // happens to be in r10.
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_WAITID,
        idtype as i64,
        id as i64,
        infop as i64,
        options as i64,
        0,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __wcscoll_l(s1: *const u32, s2: *const u32, _loc: *mut core::ffi::c_void) -> i32 {
    crate::wcscmp(s1 as *const i32, s2 as *const i32)
}

#[no_mangle]
pub unsafe extern "C" fn __wcsxfrm_l(dest: *mut u32, src: *const u32, n: usize, _loc: *mut core::ffi::c_void) -> usize {
    wcsxfrm(dest as *mut wchar_t, src as *const wchar_t, n)
}

#[inline]
fn wctable_width(table: &[u8], wc: u32) -> bool {
    let page = table[(wc >> 8) as usize] as usize;
    (table[page * 32 + ((wc & 255) >> 3) as usize] >> (wc & 7)) & 1 != 0
}

#[no_mangle]
pub unsafe extern "C" fn wcwidth(c: u32) -> i32 {
    // Port of musl's wcwidth, using the nonspacing/wide tables.
    let wc = c;
    if wc < 0xff {
        return if ((wc + 1) & 0x7f) >= 0x21 {
            1
        } else if wc != 0 {
            -1
        } else {
            0
        };
    }
    if (wc & 0xfffe_ffff) < 0xfffe {
        if wctable_width(&WCTYPE_NONSPACING_TABLE, wc) {
            return 0;
        }
        if wctable_width(&WCTYPE_WIDE_TABLE, wc) {
            return 2;
        }
        return 1;
    }
    if (wc & 0xfffe) == 0xfffe {
        return -1;
    }
    if wc.wrapping_sub(0x20000) < 0x20000 {
        return 2;
    }
    if wc == 0xe0001 || wc.wrapping_sub(0xe0020) < 0x5f || wc.wrapping_sub(0xe0100) < 0xef {
        return 0;
    }
    1
}





