// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// musl ABI compatibility helpers
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn stpcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
    let mut d = dest;
    let mut s = src;
    while *s != 0 {
        *d = *s;
        d = d.add(1);
        s = s.add(1);
    }
    *d = 0;
    d
}

#[no_mangle]
pub unsafe extern "C" fn stpncpy(dest: *mut c_char, src: *const c_char, n: usize) -> *mut c_char {
    let mut i = 0;
    while i < n && *src.add(i) != 0 {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    let end = dest.add(i);
    while i < n {
        *dest.add(i) = 0;
        i += 1;
    }
    end
}

#[no_mangle]
pub unsafe extern "C" fn strchrnul(s: *const c_char, c: c_int) -> *mut c_char {
    let mut p = s;
    while *p != 0 && *p as c_int != c {
        p = p.add(1);
    }
    p as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn fstatat(dirfd: c_int, path: *const c_char, buf: *mut Stat, flags: c_int) -> c_int {
    let r = sys_newfstatat(dirfd, path, buf as *mut u8, flags);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn execvpe(
    file: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    ERRNO!() = ENOENT;
    if file.is_null() || *file == 0 {
        return -1;
    }
    let mut p = file;
    while *p != 0 {
        if *p == b'/' as c_char {
            return execve(file, argv, envp);
        }
        p = p.add(1);
    }
    let file_len = crate::strlen(file);
    let path = crate::getenv(b"PATH\0".as_ptr() as *const c_char);
    let default_path = b"/usr/local/bin:/bin:/usr/bin\0";
    let path = if path.is_null() { default_path.as_ptr() as *const c_char } else { path };

    let mut seen_eacces = false;
    let mut buf = [0 as c_char; PATH_MAX_V];
    let mut dir = path;
    loop {
        let mut end = dir;
        while *end != 0 && *end != b':' as c_char { end = end.add(1); }
        let dlen = end.offset_from(dir) as usize;
        // An empty PATH component means the current directory (no slash).
        let need_slash = dlen > 0;
        let total = dlen + need_slash as usize + file_len;
        if total < buf.len() {
            let mut i = 0;
            while i < dlen { buf[i] = *dir.add(i); i += 1; }
            if need_slash { buf[i] = b'/' as c_char; i += 1; }
            let mut j = 0;
            while j < file_len { buf[i + j] = *file.add(j); j += 1; }
            buf[i + j] = 0;
            execve(buf.as_ptr(), argv, envp);
            match ERRNO!() {
                13 => seen_eacces = true, // EACCES
                2 | 20 => {}              // ENOENT, ENOTDIR: try next
                _ => return -1,
            }
        }
        if *end == 0 { break; }
        dir = end.add(1);
    }
    ERRNO!() = if seen_eacces { 13 } else { 2 };
    -1
}

// Locale-aware entry points. keel currently has a single global locale,
// so these are source/ABI-compatible wrappers around the normal functions.
#[no_mangle]
pub unsafe extern "C" fn strcoll_l(s1: *const c_char, s2: *const c_char, _loc: locale_t) -> c_int {
    strcoll(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn strxfrm_l(dest: *mut c_char, src: *const c_char, n: usize, _loc: locale_t) -> usize {
    strxfrm(dest, src, n)
}

#[no_mangle]
pub unsafe extern "C" fn strerror_l(errnum: c_int, _loc: locale_t) -> *mut c_char {
    strerror(errnum)
}

#[no_mangle]
pub unsafe extern "C" fn strcasecmp_l(s1: *const c_char, s2: *const c_char, _loc: locale_t) -> c_int {
    strcasecmp(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn strncasecmp_l(s1: *const c_char, s2: *const c_char, n: usize, _loc: locale_t) -> c_int {
    strncasecmp(s1, s2, n)
}

#[no_mangle]
pub unsafe extern "C" fn strftime_l(s: *mut c_char, max: usize, fmt: *const c_char, tm: *const tm, _loc: locale_t) -> usize {
    strftime(s, max, fmt, tm)
}

#[no_mangle]
pub unsafe extern "C" fn nl_langinfo_l(item: c_int, _loc: locale_t) -> *mut c_char {
    nl_langinfo(item)
}

#[no_mangle]
pub unsafe extern "C" fn wcscoll_l(s1: *const u32, s2: *const u32, _loc: locale_t) -> c_int {
    wcscoll(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn wcsxfrm_l(dst: *mut wchar_t, src: *const wchar_t, n: usize, _loc: locale_t) -> usize {
    wcsxfrm(dst, src, n)
}

#[no_mangle]
pub unsafe extern "C" fn wcsftime_l(s: *mut u32, max: usize, fmt: *const u32, tm: *const c_void, _loc: locale_t) -> usize {
    wcsftime(s, max, fmt, tm)
}

#[no_mangle]
pub unsafe extern "C" fn wctype_l(name: *const c_char, _loc: locale_t) -> wctype_t {
    wctype(name)
}

#[no_mangle]
pub unsafe extern "C" fn wctrans_l(name: *const c_char, _loc: locale_t) -> wctrans_t {
    wctrans(name)
}

#[no_mangle]
pub unsafe extern "C" fn towlower_l(c: wint_t, _loc: locale_t) -> wint_t {
    towlower(c)
}

#[no_mangle]
pub unsafe extern "C" fn towupper_l(c: wint_t, _loc: locale_t) -> wint_t {
    towupper(c)
}

#[no_mangle]
pub unsafe extern "C" fn towctrans_l(c: wint_t, desc: wctrans_t, _loc: locale_t) -> wint_t {
    towctrans(c, desc)
}

#[no_mangle] pub unsafe extern "C" fn iswalnum_l(c: wint_t, _l: locale_t) -> c_int { iswalnum(c) }
#[no_mangle] pub unsafe extern "C" fn iswalpha_l(c: wint_t, _l: locale_t) -> c_int { iswalpha(c) }
#[no_mangle] pub unsafe extern "C" fn iswblank_l(c: wint_t, _l: locale_t) -> c_int { iswblank(c) }
#[no_mangle] pub unsafe extern "C" fn iswcntrl_l(c: wint_t, _l: locale_t) -> c_int { iswcntrl(c) }
#[no_mangle] pub unsafe extern "C" fn iswdigit_l(c: wint_t, _l: locale_t) -> c_int { iswdigit(c) }
#[no_mangle] pub unsafe extern "C" fn iswgraph_l(c: wint_t, _l: locale_t) -> c_int { iswgraph(c) }
#[no_mangle] pub unsafe extern "C" fn iswlower_l(c: wint_t, _l: locale_t) -> c_int { iswlower(c) }
#[no_mangle] pub unsafe extern "C" fn iswprint_l(c: wint_t, _l: locale_t) -> c_int { iswprint(c) }
#[no_mangle] pub unsafe extern "C" fn iswpunct_l(c: wint_t, _l: locale_t) -> c_int { iswpunct(c) }
#[no_mangle] pub unsafe extern "C" fn iswspace_l(c: wint_t, _l: locale_t) -> c_int { iswspace(c) }
#[no_mangle] pub unsafe extern "C" fn iswupper_l(c: wint_t, _l: locale_t) -> c_int { iswupper(c) }
#[no_mangle] pub unsafe extern "C" fn iswxdigit_l(c: wint_t, _l: locale_t) -> c_int { iswxdigit(c) }
#[no_mangle] pub unsafe extern "C" fn iswctype_l(c: wint_t, desc: wctype_t, _l: locale_t) -> c_int { iswctype(c, desc) }

// ISO C99 scanf ABI aliases expected by musl binaries.
#[no_mangle] pub unsafe extern "C" fn __isoc99_scanf(fmt: *const c_char, args: ...) -> c_int { vfscanf(stdin, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_fscanf(f: *mut FILE, fmt: *const c_char, args: ...) -> c_int { vfscanf(f, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_sscanf(s: *const c_char, fmt: *const c_char, args: ...) -> c_int { vsscanf(s, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vscanf(fmt: *const c_char, args: VaList) -> c_int { vfscanf(stdin, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vfscanf(f: *mut FILE, fmt: *const c_char, args: VaList) -> c_int { vfscanf(f, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vsscanf(s: *const c_char, fmt: *const c_char, args: VaList) -> c_int { vsscanf(s, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_wscanf(fmt: *const wchar_t, args: ...) -> c_int { vfwscanf(stdin, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_fwscanf(f: *mut FILE, fmt: *const wchar_t, args: ...) -> c_int { vfwscanf(f, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_swscanf(s: *const wchar_t, fmt: *const wchar_t, args: ...) -> c_int { vswscanf(s, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vwscanf(fmt: *const wchar_t, args: VaList) -> c_int { vfwscanf(stdin, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vfwscanf(f: *mut FILE, fmt: *const wchar_t, args: VaList) -> c_int { vfwscanf(f, fmt, args) }
#[no_mangle] pub unsafe extern "C" fn __isoc99_vswscanf(s: *const wchar_t, fmt: *const wchar_t, args: VaList) -> c_int { vswscanf(s, fmt, args) }

// Unlocked stdio entry points.  keel's stdio does not take FILE locks
// internally (see docs/limitations.md), so these are aliases for the normal
// entry points.
#[no_mangle] pub unsafe extern "C" fn clearerr_unlocked(f: *mut FILE) { clearerr(f) }
#[no_mangle] pub unsafe extern "C" fn feof_unlocked(f: *mut FILE) -> c_int { feof(f) }
#[no_mangle] pub unsafe extern "C" fn ferror_unlocked(f: *mut FILE) -> c_int { ferror(f) }
#[no_mangle] pub unsafe extern "C" fn fileno_unlocked(f: *mut FILE) -> c_int { fileno(f) }
#[no_mangle] pub unsafe extern "C" fn fflush_unlocked(f: *mut FILE) -> c_int { fflush(f) }
#[no_mangle] pub unsafe extern "C" fn fgetc_unlocked(f: *mut FILE) -> c_int { fgetc(f) }
#[no_mangle] pub unsafe extern "C" fn fgets_unlocked(s: *mut c_char, n: c_int, f: *mut FILE) -> *mut c_char { fgets(s, n, f) }
#[no_mangle] pub unsafe extern "C" fn fputc_unlocked(c: c_int, f: *mut FILE) -> c_int { fputc(c, f) }
#[no_mangle] pub unsafe extern "C" fn fputs_unlocked(s: *const c_char, f: *mut FILE) -> c_int { fputs(s, f) }
#[no_mangle] pub unsafe extern "C" fn fread_unlocked(p: *mut c_void, size: usize, n: usize, f: *mut FILE) -> usize { fread(p, size, n, f) }
#[no_mangle] pub unsafe extern "C" fn fwrite_unlocked(p: *const c_void, size: usize, n: usize, f: *mut FILE) -> usize { fwrite(p, size, n, f) }
#[no_mangle] pub unsafe extern "C" fn fgetwc_unlocked(f: *mut FILE) -> wint_t { fgetwc(f) }
#[no_mangle] pub unsafe extern "C" fn fgetws_unlocked(s: *mut u32, n: c_int, f: *mut c_void) -> *mut u32 { fgetws(s, n, f) }
#[no_mangle] pub unsafe extern "C" fn fputwc_unlocked(c: wchar_t, f: *mut FILE) -> wint_t { fputwc(c, f) }
#[no_mangle] pub unsafe extern "C" fn fputws_unlocked(s: *const wchar_t, f: *mut FILE) -> c_int { fputws(s, f) }
#[no_mangle] pub unsafe extern "C" fn getwc_unlocked(f: *mut c_void) -> wint_t { getwc(f) }
#[no_mangle] pub unsafe extern "C" fn putwc_unlocked(c: u32, f: *mut c_void) -> wint_t { putwc(c, f) }
#[no_mangle] pub unsafe extern "C" fn getwchar_unlocked() -> wint_t { getwchar() }
#[no_mangle] pub unsafe extern "C" fn putwchar_unlocked(c: wchar_t) -> wint_t { putwchar(c) }
#[no_mangle] pub unsafe extern "C" fn fpurge(f: *mut FILE) { clearerr(f) }
#[no_mangle] pub unsafe extern "C" fn _IO_getc(f: *mut FILE) -> c_int { getc(f) }
#[no_mangle] pub unsafe extern "C" fn _IO_putc(c: c_int, f: *mut FILE) -> c_int { putc(c, f) }
#[no_mangle] pub unsafe extern "C" fn _IO_getc_unlocked(f: *mut FILE) -> c_int { getc(f) }
#[no_mangle] pub unsafe extern "C" fn _IO_putc_unlocked(c: c_int, f: *mut FILE) -> c_int { putc(c, f) }
#[no_mangle] pub unsafe extern "C" fn _IO_feof_unlocked(f: *mut FILE) -> c_int { feof(f) }
#[no_mangle] pub unsafe extern "C" fn _IO_ferror_unlocked(f: *mut FILE) -> c_int { ferror(f) }

// ABI aliases used by musl and older C libraries.
#[no_mangle] pub unsafe extern "C" fn __getauxval(t: c_ulong) -> c_ulong { getauxval(t) }
#[no_mangle] pub unsafe extern "C" fn __longjmp(env: *const c_ulong, val: c_int) -> ! { longjmp(env, val) }
#[no_mangle] pub unsafe extern "C" fn bsd_signal(sig: c_int, handler: usize) -> usize { signal(sig, handler) }
#[no_mangle] pub unsafe extern "C" fn __sysv_signal(sig: c_int, handler: usize) -> usize { signal(sig, handler) }
#[no_mangle] pub unsafe extern "C" fn eaccess(path: *const c_char, mode: c_int) -> c_int { access(path, mode) }
#[no_mangle] pub unsafe extern "C" fn __xpg_basename(path: *mut c_char) -> *mut c_char { basename(path) }
#[no_mangle] pub unsafe extern "C" fn __xpg_strerror_r(errnum: c_int, buf: *mut c_char, len: usize) -> c_int { strerror_r(errnum, buf, len) }
#[no_mangle] pub unsafe extern "C" fn __posix_getopt(argc: c_int, argv: *const *const c_char, optstring: *const c_char) -> c_int { getopt(argc, argv, optstring) }

// ============================================================
// Remaining musl ABI entry points
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn cfsetspeed(termios_p: *mut c_void, speed: u32) -> c_int {
    let r = cfsetispeed(termios_p, speed);
    if r != 0 { return r; }
    cfsetospeed(termios_p, speed)
}

#[no_mangle]
pub unsafe extern "C" fn futimesat(dirfd: c_int, path: *const c_char, times: *const timeval) -> c_int {
    if times.is_null() {
        return utimensat(dirfd, path, core::ptr::null(), 0);
    }
    let tv = times as *const [timeval; 2];
    let mut ts: [timespec; 2] = [
        timespec { tv_sec: (*tv)[0].tv_sec, tv_nsec: (*tv)[0].tv_usec * 1000 },
        timespec { tv_sec: (*tv)[1].tv_sec, tv_nsec: (*tv)[1].tv_usec * 1000 },
    ];
    for t in &mut ts {
        if t.tv_nsec < 0 || t.tv_nsec >= 1_000_000_000 {
            ERRNO!() = EINVAL;
            return -1;
        }
    }
    utimensat(dirfd, path, ts.as_ptr(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn membarrier(cmd: c_int, flags: c_int, cpu_id: c_int) -> c_int {
    <Arch as Syscalls>::syscall3(SYS_MEMBARRIER, cmd as i64, flags as i64, cpu_id as i64) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn mremap(
    old_address: *mut c_void,
    old_size: usize,
    new_size: usize,
    flags: c_int,
    mut args: ...
) -> *mut c_void {
    let new_address = if flags & 2 != 0 { args.next_arg::<*mut c_void>() } else { core::ptr::null_mut() };
    let r = <Arch as Syscalls>::syscall5(
        SYS_MREMAP,
        old_address as i64,
        old_size as i64,
        new_size as i64,
        flags as i64,
        new_address as i64,
    );
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return (-1isize) as *mut c_void;
    }
    r as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn ptsname_r(fd: c_int, buf: *mut c_char, buflen: usize) -> c_int {
    let name = ptsname(fd);
    if name.is_null() { return -1; }
    let n = crate::strlen(name);
    if n + 1 > buflen {
        ERRNO!() = ERANGE;
        return ERANGE;
    }
    core::ptr::copy_nonoverlapping(name, buf, n + 1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn posix_getdents(fd: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize {
    if flags != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    let r = <Arch as Syscalls>::syscall3(SYS_GETDENTS64, fd as i64, buf as i64, len as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

const SYS_RISCV_FLUSH_ICACHE: i64 = 259;
#[no_mangle]
pub unsafe extern "C" fn riscv_flush_icache(start: *mut c_void, end: *mut c_void, flags: c_ulong) -> c_int {
    #[cfg(target_arch = "riscv64")]
    {
        let r = <Arch as Syscalls>::syscall3(SYS_RISCV_FLUSH_ICACHE, start as i64, end as i64, flags as i64);
        syscall_ret(r) as c_int
    }
    #[cfg(not(target_arch = "riscv64"))]
    {
        let _ = (start, end, flags);
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn __riscv_flush_icache(start: *mut c_void, end: *mut c_void, flags: c_ulong) -> c_int {
    riscv_flush_icache(start, end, flags)
}

#[no_mangle]
pub unsafe extern "C" fn mkostemps(template: *mut c_char, suffixlen: c_int, flags: c_int) -> c_int {
    mkstemps_flags(template, suffixlen, flags)
}

#[no_mangle]
pub unsafe extern "C" fn statfs(path: *const c_char, buf: *mut c_void) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_STATFS, path as i64, buf as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fstatfs(fd: c_int, buf: *mut c_void) -> c_int {
    let r = <Arch as Syscalls>::syscall2(SYS_FSTATFS, fd as i64, buf as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sysinfo(info: *mut c_void) -> c_int {
    let r = <Arch as Syscalls>::syscall1(SYS_SYSINFO, info as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn pthread_tryjoin_np(thread: PthreadT, retval: *mut *mut c_void) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    if (*slot).detach_state == DT_DETACHED { return EINVAL; }
    let tid = core::ptr::read_volatile(&raw const (*slot).tid);
    if tid > 0 { return EBUSY; }
    if tid < 0 { return EINVAL; }
    pthread_join(thread, retval)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_timedjoin_np(thread: PthreadT, retval: *mut *mut c_void, abstime: *const timespec) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    if thread == pthread_self() { return EDEADLK; }
    if (*slot).detach_state == DT_DETACHED { return EINVAL; }
    loop {
        let r = pthread_tryjoin_np(thread, retval);
        if r != EBUSY { return r; }
        let tid = core::ptr::read_volatile(&raw const (*slot).tid);
        if tid <= 0 { continue; }
        let e = futex_timedwait(&raw mut (*slot).tid, tid, abstime);
        if e == ETIMEDOUT { return ETIMEDOUT; }
    }
}


// utmp/utmpx are backed by the classic /var/run/utmp file.  musl itself
// leaves these as no-ops; keel provides the real file-backed API.
#[repr(C)]
pub struct UtmpExit {
    pub e_termination: i16,
    pub e_exit: i16,
}

#[repr(C)]
pub struct Utmpx {
    pub ut_type: i16,
    pub __ut_pad1: i16,
    pub ut_pid: i32,
    pub ut_line: [c_char; 32],
    pub ut_id: [c_char; 4],
    pub ut_user: [c_char; 32],
    pub ut_host: [c_char; 256],
    pub ut_exit: UtmpExit,
    pub ut_session: i32,
    pub __ut_pad2: i32,
    pub ut_tv: timeval,
    pub ut_addr_v6: [u32; 4],
    pub __unused: [u8; 20],
}

const UTMP_SIZE: usize = 400;
const _: [(); UTMP_SIZE] = [(); core::mem::size_of::<Utmpx>()];

static mut UTMP_FD: c_int = -1;
static mut UTMP_PATH: [u8; 256] = [0; 256];
static mut UTMP_PATH_SET: bool = false;
#[repr(align(8))]
struct UtmpBuf([u8; UTMP_SIZE]);
static mut UTMP_BUF: UtmpBuf = UtmpBuf([0; UTMP_SIZE]);

unsafe fn utmp_path_ptr() -> *const c_char {
    if !UTMP_PATH_SET {
        let default = b"/var/run/utmp\0";
        let mut i = 0;
        while i < default.len() && i < UTMP_PATH.len() {
            UTMP_PATH[i] = default[i];
            i += 1;
        }
        UTMP_PATH_SET = true;
    }
    UTMP_PATH.as_ptr() as *const c_char
}

unsafe fn utmp_open_read() -> c_int {
    if UTMP_FD >= 0 {
        crate::close(UTMP_FD);
    }
    UTMP_FD = crate::open(utmp_path_ptr(), 0, 0);
    UTMP_FD
}

unsafe fn utmp_open_append() -> c_int {
    crate::open(utmp_path_ptr(), O_WRONLY | O_CREAT | O_APPEND, 0o644)
}

#[no_mangle]
pub unsafe extern "C" fn setutent() {
    let _ = utmp_open_read();
}

#[no_mangle]
pub unsafe extern "C" fn endutent() {
    if UTMP_FD >= 0 {
        crate::close(UTMP_FD);
        UTMP_FD = -1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn getutent() -> *mut c_void {
    if UTMP_FD < 0 && utmp_open_read() < 0 {
        return core::ptr::null_mut();
    }
    let n = crate::read(UTMP_FD, UTMP_BUF.0.as_mut_ptr() as *mut c_void, UTMP_SIZE);
    if n == UTMP_SIZE as isize {
        UTMP_BUF.0.as_mut_ptr() as *mut c_void
    } else {
        core::ptr::null_mut()
    }
}

unsafe fn utmp_id_eq(a: &[c_char; 4], b: &[c_char; 4]) -> bool {
    a[0] == b[0] && a[1] == b[1] && a[2] == b[2] && a[3] == b[3]
}

unsafe fn utmp_line_eq(a: &[c_char; 32], b: &[c_char; 32]) -> bool {
    let mut i = 0;
    while i < 32 {
        if a[i] != b[i] { return false; }
        if a[i] == 0 { return true; }
        i += 1;
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn getutid(ut: *const c_void) -> *mut c_void {
    if ut.is_null() { return core::ptr::null_mut(); }
    let want = &*(ut as *const Utmpx);
    loop {
        let p = getutent();
        if p.is_null() { return p; }
        let cur = &*(p as *const Utmpx);
        let matched = match want.ut_type {
            1 | 2 | 3 | 4 => cur.ut_type == want.ut_type,
            _ => utmp_id_eq(&cur.ut_id, &want.ut_id)
                && (want.ut_type == 0 || cur.ut_type == want.ut_type),
        };
        if matched { return p; }
    }
}

#[no_mangle]
pub unsafe extern "C" fn getutline(ut: *const c_void) -> *mut c_void {
    if ut.is_null() { return core::ptr::null_mut(); }
    let want = &*(ut as *const Utmpx);
    loop {
        let p = getutent();
        if p.is_null() { return p; }
        if utmp_line_eq(&(*(p as *const Utmpx)).ut_line, &want.ut_line) {
            return p;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pututline(ut: *const c_void) -> *mut c_void {
    if ut.is_null() { return core::ptr::null_mut(); }
    let fd = utmp_open_append();
    if fd < 0 { return core::ptr::null_mut(); }
    let n = crate::write(fd, ut, UTMP_SIZE);
    crate::close(fd);
    if n == UTMP_SIZE as isize { ut as *mut c_void } else { core::ptr::null_mut() }
}

#[no_mangle]
pub unsafe extern "C" fn updwtmp(file: *const c_char, ut: *const c_void) {
    if file.is_null() || ut.is_null() { return; }
    let fd = crate::open(file, O_WRONLY | O_CREAT | O_APPEND, 0o644);
    if fd >= 0 {
        let _ = crate::write(fd, ut, UTMP_SIZE);
        crate::close(fd);
    }
}

unsafe fn utmp_set_name(file: *const c_char) -> c_int {
    if file.is_null() { return -1; }
    let len = crate::strlen(file);
    if len + 1 > UTMP_PATH.len() { return -1; }
    if UTMP_FD >= 0 {
        crate::close(UTMP_FD);
        UTMP_FD = -1;
    }
    core::ptr::copy_nonoverlapping(file, UTMP_PATH.as_mut_ptr() as *mut c_char, len + 1);
    UTMP_PATH_SET = true;
    0
}

#[no_mangle]
pub unsafe extern "C" fn utmpname(file: *const c_char) -> c_int { utmp_set_name(file) }

#[no_mangle]
pub unsafe extern "C" fn utmpxname(file: *const c_char) -> c_int { utmp_set_name(file) }


// Data symbols and internal aliases still referenced by some musl binaries.
#[no_mangle] pub static mut ___environ: *mut *mut c_char = core::ptr::null_mut();
#[no_mangle] pub static mut _environ: *mut *mut c_char = core::ptr::null_mut();
#[no_mangle] pub unsafe extern "C" fn __freelocale(loc: locale_t) { freelocale(loc) }
#[no_mangle] pub unsafe extern "C" fn __getdelim(lineptr: *mut *mut c_char, n: *mut usize, delim: c_int, stream: *mut FILE) -> isize { getdelim(lineptr, n, delim, stream) }
#[no_mangle] pub unsafe extern "C" fn __strtod_l(s: *const c_char, end: *mut *mut c_char, loc: locale_t) -> f64 { strtod_l(s, end, loc) }
#[no_mangle] pub unsafe extern "C" fn __strtof_l(s: *const c_char, end: *mut *mut c_char, loc: locale_t) -> f32 { strtof_l(s, end, loc) }
#[cfg(target_arch = "x86_64")]
#[no_mangle] pub unsafe extern "C" fn __strtold_l(s: *const c_char, end: *mut *mut c_char, loc: locale_t) -> f64 { strtold_l(s, end, loc) }
#[cfg(not(target_arch = "x86_64"))]
#[no_mangle] pub unsafe extern "C" fn __strtold_l(s: *const c_char, end: *mut *mut c_char, loc: locale_t) -> f128 { strtold_l(s, end, loc) }

#[no_mangle] pub unsafe extern "C" fn __strtol_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long { strtol(s, end, base) }
#[no_mangle] pub unsafe extern "C" fn __strtoul_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_ulong { strtoul(s, end, base) }
#[no_mangle] pub unsafe extern "C" fn __strtoll_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_longlong { strtoll(s, end, base) }
#[no_mangle] pub unsafe extern "C" fn __strtoull_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_ulonglong { strtoull(s, end, base) }
#[no_mangle] pub unsafe extern "C" fn __strtoimax_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> i64 { strtoimax(s, end, base) }
#[no_mangle] pub unsafe extern "C" fn __strtoumax_internal(s: *const c_char, end: *mut *mut c_char, base: c_int) -> u64 { strtoumax(s, end, base) }

