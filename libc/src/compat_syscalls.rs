// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// timer/clock functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn timer_create(clockid: i32, sevp: *mut core::ffi::c_void, timerid: *mut i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_TIMER_CREATE, clockid as i64, sevp as i64, timerid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timer_delete(timerid: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_TIMER_DELETE, timerid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timer_settime(timerid: i32, flags: i32, new_value: *const core::ffi::c_void, old_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_TIMER_SETTIME, timerid as i64, flags as i64, new_value as i64, old_value as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timer_gettime(timerid: i32, curr_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_TIMER_GETTIME, timerid as i64, curr_value as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timer_getoverrun(timerid: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_TIMER_GETOVERRUN, timerid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn clock_adjtime(_clockid: i32, buf: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_ADJTIMEX, buf as i64, 0)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn clock_getcpuclockid(pid: i32, clockid: *mut i32) -> i32 {
    if clockid.is_null() { return EINVAL; }
    let id: i32 = ((-(pid as i64) - 1) * 8 + 2) as i32;
    let mut ts: timespec = core::mem::zeroed();
    let r = <Arch as Syscalls>::syscall2(SYS_CLOCK_GETRES, id as i64, &mut ts as *mut timespec as i64);
    if r < 0 {
        let e = (-r) as c_int;
        return if e == EINVAL { 3 } else { e };
    }
    *clockid = id;
    0
}

// ============================================================
// String functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn strcoll(s1: *const core::ffi::c_char, s2: *const core::ffi::c_char) -> i32 {
    // musl's C locale compares byte strings directly (strcmp semantics).
    let mut i = 0;
    loop {
        let c1 = *s1.add(i) as u8;
        let c2 = *s2.add(i) as u8;
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        if c1 != c2 { return (c1 as i32) - (c2 as i32); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strxfrm(dest: *mut core::ffi::c_char, src: *const core::ffi::c_char, n: usize) -> usize {
    let mut i = 0;
    while i < n {
        let c = *src.add(i);
        *dest.add(i) = c;
        if c == 0 { break; }
        i += 1;
    }
    i
}

// ============================================================
// Directory functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn scandir(
    dirp: *const core::ffi::c_char,
    namelist: *mut *mut *mut core::ffi::c_void,
    filter: Option<unsafe extern "C" fn(*const core::ffi::c_void) -> i32>,
    compar: Option<unsafe extern "C" fn(*const core::ffi::c_void, *const core::ffi::c_void) -> i32>,
) -> c_int {
    if dirp.is_null() || namelist.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    *namelist = core::ptr::null_mut();
    let dir = opendir(dirp);
    if dir.is_null() { return -1; }

    let mut cap = 16usize;
    let mut count = 0usize;
    let mut list = crate::malloc(cap * core::mem::size_of::<*mut c_void>()) as *mut *mut c_void;
    if list.is_null() { closedir(dir); ERRNO!() = ENOMEM; return -1; }

    loop {
        let ent = readdir(dir);
        if ent.is_null() { break; }
        if let Some(filter) = filter {
            if filter(ent as *const c_void) == 0 { continue; }
        }
        if count == cap {
            let new_cap = match cap.checked_mul(2) {
                Some(n) => n,
                None => {
                    for i in 0..count { crate::free(*list.add(i)); }
                    crate::free(list as *mut c_void);
                    closedir(dir);
                    ERRNO!() = ENOMEM;
                    return -1;
                }
            };
            let p = crate::realloc(
                list as *mut c_void,
                new_cap * core::mem::size_of::<*mut c_void>(),
            ) as *mut *mut c_void;
            if p.is_null() {
                for i in 0..count { crate::free(*list.add(i)); }
                crate::free(list as *mut c_void);
                closedir(dir);
                ERRNO!() = ENOMEM;
                return -1;
            }
            list = p;
            cap = new_cap;
        }
        let name = (ent as *const u8).add(core::mem::offset_of!(linux_dirent64, d_name))
            as *const core::ffi::c_char;
        let copy = crate::strdup(name);
        if copy.is_null() {
            for i in 0..count { crate::free(*list.add(i)); }
            crate::free(list as *mut c_void);
            closedir(dir);
            ERRNO!() = ENOMEM;
            return -1;
        }
        *list.add(count) = copy as *mut c_void;
        count += 1;
    }
    closedir(dir);

    if let Some(compar) = compar {
        if count > 1 {
            qsort(
                list as *mut c_void,
                count,
                core::mem::size_of::<*mut c_void>(),
                compar,
            );
        }
    }
    *namelist = list;
    count as c_int
}

#[no_mangle]
pub unsafe extern "C" fn alphasort(a: *const *const core::ffi::c_void, b: *const *const core::ffi::c_void) -> i32 {
    let s1 = *a as *const core::ffi::c_char;
    let s2 = *b as *const core::ffi::c_char;
    crate::strcmp(s1 as *const u8, s2 as *const u8)
}

#[no_mangle]
pub unsafe extern "C" fn versionsort(a: *const *const core::ffi::c_void, b: *const *const core::ffi::c_void) -> i32 {
    let s1 = *a as *const core::ffi::c_char;
    let s2 = *b as *const core::ffi::c_char;
    crate::strcmp(s1 as *const u8, s2 as *const u8)
}

// ============================================================


// ============================================================
// err functions
// ============================================================

unsafe fn vwarn_impl(fmt: *const c_char, ap: VaList, with_errno: bool) {
    let mut buf = [0 as c_char; 1024];
    let n = vsnprintf(buf.as_mut_ptr(), buf.len(), fmt, ap);
    if n > 0 {
        let l = core::cmp::min(n as usize, buf.len() - 1);
        crate::write(2, buf.as_ptr() as *const c_void, l);
    }
    if with_errno {
        let e = ERRNO!();
        crate::write(2, b": ".as_ptr() as *const c_void, 2);
        let msg = strerror(e);
        crate::write(2, msg as *const c_void, crate::strlen(msg));
    }
    crate::write(2, b"\n".as_ptr() as *const c_void, 1);
}

#[no_mangle]
pub unsafe extern "C" fn verr(eval: c_int, fmt: *const c_char, ap: VaList) {
    vwarn_impl(fmt, ap, true);
    crate::exit(eval);
}

#[no_mangle]
pub unsafe extern "C" fn verrx(eval: c_int, fmt: *const c_char, ap: VaList) {
    vwarn_impl(fmt, ap, false);
    crate::exit(eval);
}

#[no_mangle]
pub unsafe extern "C" fn vwarn(fmt: *const c_char, ap: VaList) {
    vwarn_impl(fmt, ap, true);
}

#[no_mangle]
pub unsafe extern "C" fn vwarnx(fmt: *const c_char, ap: VaList) {
    vwarn_impl(fmt, ap, false);
}

#[no_mangle]
pub unsafe extern "C" fn err(eval: c_int, fmt: *const c_char, args: ...) {
    verr(eval, fmt, args);
}

#[no_mangle]
pub unsafe extern "C" fn errx(eval: c_int, fmt: *const c_char, args: ...) {
    verrx(eval, fmt, args);
}

#[no_mangle]
pub unsafe extern "C" fn warn(fmt: *const c_char, args: ...) {
    vwarn(fmt, args);
}

#[no_mangle]
pub unsafe extern "C" fn warnx(fmt: *const c_char, args: ...) {
    vwarnx(fmt, args);
}

// ============================================================
// ecvt functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn ecvt(value: f64, ndigit: i32, decpt: *mut i32, sign: *mut i32) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 80] = [0; 80];
    if value < 0.0 { *sign = 1; } else { *sign = 0; }
    let v = if value < 0.0 { -value } else { value };
    if v == 0.0 { *decpt = 0; BUF[0] = 0; return BUF.as_mut_ptr(); }
    let mut d = 0i32;
    let mut tmp = v;
    while tmp >= 10.0 { tmp /= 10.0; d += 1; }
    while tmp < 1.0 { tmp *= 10.0; d -= 1; }
    *decpt = d + 1;
    let mut i = 0;
    let mut val = tmp;
    while i < ndigit as usize && i < 79 {
        let digit = val as i32;
        BUF[i] = (b'0' + digit as u8) as c_char;
        val = (val - digit as f64) * 10.0;
        i += 1;
    }
    BUF[i] = 0;
    BUF.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn fcvt(value: f64, ndigit: i32, decpt: *mut i32, sign: *mut i32) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 80] = [0; 80];
    if value < 0.0 { *sign = 1; } else { *sign = 0; }
    let v = if value < 0.0 { -value } else { value };
    if v == 0.0 { *decpt = 0; BUF[0] = 0; return BUF.as_mut_ptr(); }
    let mut d = 0i32;
    let mut tmp = v;
    while tmp >= 10.0 { tmp /= 10.0; d += 1; }
    while tmp < 1.0 { tmp *= 10.0; d -= 1; }
    *decpt = d + 1;
    let mut val = tmp;
    let mut i = 0;
    while i < (*decpt) as usize && i < 79 {
        let digit = val as i32;
        BUF[i] = (b'0' + digit as u8) as c_char;
        val = (val - digit as f64) * 10.0;
        i += 1;
    }
    if ndigit > 0 && i < 79 {
        BUF[i] = '.' as c_char;
        i += 1;
        let mut frac = val;
        let mut j = 0;
        while j < ndigit as usize && i < 79 {
            let digit = frac as i32;
            BUF[i] = (b'0' + digit as u8) as c_char;
            frac = (frac - digit as f64) * 10.0;
            i += 1;
            j += 1;
        }
    }
    BUF[i] = 0;
    BUF.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn gcvt(value: f64, ndigit: i32, buf: *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    if buf.is_null() {
        return buf;
    }
    // musl implements gcvt as sprintf("%.*g"); cap the write at a size that
    // always covers %g's output for the requested precision.
    let cap = if ndigit > 0 {
        (ndigit as usize).saturating_mul(2).saturating_add(32).min(4096)
    } else {
        32
    };
    crate::snprintf(buf, cap, b"%.*g\0".as_ptr() as *const core::ffi::c_char, ndigit, value);
    buf
}

// ============================================================
// usershell functions
// ============================================================

static mut SHELLS: [*const core::ffi::c_char; 3] = [
    b"/bin/sh\0".as_ptr() as *const core::ffi::c_char,
    b"/bin/bash\0".as_ptr() as *const core::ffi::c_char,
    core::ptr::null(),
];
static mut SHELL_IDX: usize = 0;

#[no_mangle]
pub unsafe extern "C" fn getusershell() -> *mut core::ffi::c_char {
    let s = SHELLS[SHELL_IDX];
    if s.is_null() {
        SHELL_IDX = 0;
        return core::ptr::null_mut();
    }
    SHELL_IDX += 1;
    s as *mut core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn setusershell() {
    SHELL_IDX = 0;
}

#[no_mangle]
pub unsafe extern "C" fn endusershell() {
    SHELL_IDX = 0;
}

// ============================================================
// ether functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn ether_aton(s: *const core::ffi::c_char) -> *mut u8 {
    static mut EA: [u8; 6] = [0; 6];
    if ether_aton_r(s, EA.as_mut_ptr()).is_null() { core::ptr::null_mut() } else { EA.as_mut_ptr() }
}

#[no_mangle]
pub unsafe extern "C" fn ether_aton_r(s: *const core::ffi::c_char, ea: *mut u8) -> *mut u8 {
    if ea.is_null() { return core::ptr::null_mut(); }
    let mut i = 0;
    let mut j = 0;
    let mut val: u32 = 0;
    let mut digits = 0;
    while *s.add(i) != 0 && j < 6 {
        let c = *s.add(i);
        if c == ':' as c_char {
            if digits == 0 { return core::ptr::null_mut(); }
            *ea.add(j) = val as u8;
            j += 1;
            val = 0;
            digits = 0;
        } else if (c >= '0' as c_char && c <= '9' as c_char) || (c >= 'a' as c_char && c <= 'f' as c_char) || (c >= 'A' as c_char && c <= 'F' as c_char) {
            let d = if c >= '0' as c_char && c <= '9' as c_char { (c - '0' as c_char) as u32 }
                   else if c >= 'a' as c_char { (c - 'a' as c_char + 10) as u32 }
                   else { (c - 'A' as c_char + 10) as u32 };
            val = val * 16 + d;
            if val > 255 { return core::ptr::null_mut(); }
            digits += 1;
            if digits > 2 { return core::ptr::null_mut(); }
        } else {
            return core::ptr::null_mut();
        }
        i += 1;
    }
    if digits == 0 || j != 5 { return core::ptr::null_mut(); }
    *ea.add(j) = val as u8;
    ea
}

#[no_mangle]
pub unsafe extern "C" fn ether_ntoa(ea: *const u8) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 18] = [0; 18];
    ether_ntoa_r(ea, BUF.as_mut_ptr());
    BUF.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn ether_ntoa_r(ea: *const u8, buf: *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    let a = *ea.add(0); let b = *ea.add(1); let c = *ea.add(2);
    let d = *ea.add(3); let e = *ea.add(4); let f = *ea.add(5);
    let hex = b"0123456789abcdef\0".as_ptr() as *const u8;
    let mut i = 0;
    *buf.add(i) = *hex.add((a >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((a & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = ':' as c_char; i += 1;
    *buf.add(i) = *hex.add((b >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((b & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = ':' as c_char; i += 1;
    *buf.add(i) = *hex.add((c >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((c & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = ':' as c_char; i += 1;
    *buf.add(i) = *hex.add((d >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((d & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = ':' as c_char; i += 1;
    *buf.add(i) = *hex.add((e >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((e & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = ':' as c_char; i += 1;
    *buf.add(i) = *hex.add((f >> 4) as usize) as c_char; i += 1;
    *buf.add(i) = *hex.add((f & 0xf) as usize) as c_char; i += 1;
    *buf.add(i) = 0;
    buf
}

// ============================================================
// utmpx functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getutxent() -> *mut core::ffi::c_void {
    getutent()
}

#[no_mangle]
pub unsafe extern "C" fn getutxid(ut: *const core::ffi::c_void) -> *mut core::ffi::c_void {
    getutid(ut)
}

#[no_mangle]
pub unsafe extern "C" fn getutxline(ut: *const core::ffi::c_void) -> *mut core::ffi::c_void {
    getutline(ut)
}

#[no_mangle]
pub unsafe extern "C" fn pututxline(ut: *const core::ffi::c_void) -> *mut core::ffi::c_void {
    pututline(ut)
}

#[no_mangle]
pub unsafe extern "C" fn setutxent() {
    setutent()
}

#[no_mangle]
pub unsafe extern "C" fn endutxent() {
    endutent()
}

// ============================================================
// exec functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn execv(path: *const core::ffi::c_char, argv: *const *const core::ffi::c_char) -> i32 {
    crate::execve(path, argv, __environ as *const *const core::ffi::c_char)
}

#[no_mangle]
pub unsafe extern "C" fn execlp(file: *const core::ffi::c_char, _args: ...) -> i32 {
    let mut cap = 16usize;
    let mut argv = malloc(cap * core::mem::size_of::<*const core::ffi::c_char>()) as *mut *const core::ffi::c_char;
    if argv.is_null() { ERRNO!() = ENOMEM; return -1; }
    let mut i = 0usize;
    let mut args = _args;
    loop {
        let arg = args.next_arg::<*const core::ffi::c_char>();
        if arg.is_null() { break; }
        if i + 1 >= cap {
            cap = match cap.checked_mul(2) { Some(v) => v, None => { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; } };
            let grown = realloc(argv as *mut c_void, cap * core::mem::size_of::<*const core::ffi::c_char>()) as *mut *const core::ffi::c_char;
            if grown.is_null() { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            argv = grown;
        }
        *argv.add(i) = arg;
        i += 1;
    }
    *argv.add(i) = core::ptr::null();
    let r = execvp(file, argv);
    free(argv as *mut c_void);
    r
}

#[no_mangle]
pub unsafe extern "C" fn execvp(file: *const core::ffi::c_char, argv: *const *const core::ffi::c_char) -> i32 {
    execvpe(file, argv, __environ as *const *const core::ffi::c_char)
}

unsafe fn write_decimal(dst: *mut c_char, mut value: usize) -> usize {
    let mut tmp = [0u8; 20];
    let mut n = 0usize;
    if value == 0 {
        tmp[0] = b'0';
        n = 1;
    } else {
        while value > 0 {
            tmp[n] = b'0' + (value % 10) as u8;
            value /= 10;
            n += 1;
        }
    }
    for i in 0..n {
        *dst.add(i) = tmp[n - 1 - i] as c_char;
    }
    n
}

#[no_mangle]
pub unsafe extern "C" fn fexecve(fd: i32, argv: *const *const core::ffi::c_char, envp: *const *const core::ffi::c_char) -> i32 {
    // Read /proc/self/fd/N to get the path
    let mut path: [core::ffi::c_char; 64] = [0; 64];
    let mut i = 0;
    let src = b"/proc/self/fd/\0".as_ptr() as *const core::ffi::c_char;
    while i < 63 && *src.add(i) != 0 { path[i] = *src.add(i); i += 1; }
    if fd < 0 { ERRNO!() = EBADF; return -1; }
    i += write_decimal(path.as_mut_ptr().add(i), fd as usize);
    path[i] = 0;
    crate::execve(path.as_ptr(), argv, envp)
}

// ============================================================
// conf functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn confstr(name: i32, buf: *mut core::ffi::c_char, len: usize) -> usize {
    // _CS_PATH is the only value with content; the POSIX_V6/V7 option
    // strings are empty on Linux. Truncation follows snprintf semantics.
    let value: &[u8] = if name == 0 {
        b"/bin:/usr/bin"
    } else if (1..=37).contains(&name) {
        b""
    } else {
        ERRNO!() = EINVAL;
        return 0;
    };
    let need = value.len() + 1;
    if !buf.is_null() && len > 0 {
        let n = core::cmp::min(value.len(), len - 1);
        core::ptr::copy_nonoverlapping(value.as_ptr(), buf as *mut u8, n);
        *buf.add(n) = 0;
    }
    need
}

#[no_mangle]
pub unsafe extern "C" fn pathconf(_path: *const core::ffi::c_char, name: i32) -> i64 {
    fpathconf(-1, name)
}

#[no_mangle]
pub unsafe extern "C" fn fpathconf(_fd: i32, name: i32) -> i64 {
    // Same policy as musl's conf/fpathconf.c: the values are filesystem
    // independent defaults, so the path/fd argument is not consulted.
    match name {
        0 => 8,       // _PC_LINK_MAX (_POSIX_LINK_MAX)
        1 => 255,     // _PC_MAX_CANON
        2 => 255,     // _PC_MAX_INPUT
        3 => 255,     // _PC_NAME_MAX
        4 => 4096,    // _PC_PATH_MAX
        5 => 4096,    // _PC_PIPE_BUF
        6 => 1,       // _PC_CHOWN_RESTRICTED
        7 => 1,       // _PC_NO_TRUNC
        8 => 0,       // _PC_VDISABLE
        9 => 1,       // _PC_SYNC_IO
        10 => -1,     // _PC_ASYNC_IO
        11 => -1,     // _PC_PRIO_IO
        12 => -1,     // _PC_SOCK_MAXBUF
        13 => 64,     // _PC_FILESIZEBITS
        14..=17 => 4096, // _PC_REC_*_XFER_SIZE/_ALIGN
        18 => 4096,   // _PC_ALLOC_SIZE_MIN
        19 => -1,     // _PC_SYMLINK_MAX
        20 => 1,      // _PC_2_SYMLINKS
        _ => {
            ERRNO!() = EINVAL;
            -1
        }
    }
}

// ============================================================
// memory functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn explicit_bzero(s: *mut u8, n: usize) {
    core::ptr::write_bytes(s, 0, n);
    core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[no_mangle]
pub unsafe extern "C" fn memccpy(dest: *mut u8, src: *const u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        let byte = *src.add(i);
        *dest.add(i) = byte;
        if byte == c as u8 {
            return dest.add(i + 1);
        }
        i += 1;
    }
    core::ptr::null_mut()
}

// ============================================================
// *at functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn faccessat(dirfd: i32, pathname: *const core::ffi::c_char, mode: i32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_FACCESSAT, dirfd as i64, pathname as i64, mode as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fchmodat(dirfd: i32, pathname: *const core::ffi::c_char, mode: u32, flags: i32) -> i32 {
    if flags == 0 {
        return syscall_ret(<Arch as Syscalls>::syscall4(SYS_FCHMODAT, dirfd as i64, pathname as i64, mode as i64, 0)) as i32;
    }
    let r = <Arch as Syscalls>::syscall4(SYS_FCHMODAT2, dirfd as i64, pathname as i64, mode as i64, flags as i64);
    if r != -(ENOSYS as i64) {
        return syscall_ret(r) as i32;
    }
    if flags != AT_SYMLINK_NOFOLLOW {
        // Only AT_SYMLINK_NOFOLLOW is defined for fchmodat.
        ERRNO!() = EINVAL;
        return -1;
    }
    // Kernel without fchmodat2 (or an emulator that does not translate it):
    // chmod the /proc/self/fd handle instead, as musl does.
    let mut st: Stat = core::mem::zeroed();
    if fstatat(dirfd, pathname, &mut st, AT_SYMLINK_NOFOLLOW) != 0 {
        return -1;
    }
    if st.st_mode & S_IFMT == S_IFLNK {
        ERRNO!() = EOPNOTSUPP;
        return -1;
    }
    let fd2 = openat(
        dirfd,
        pathname,
        O_RDONLY | O_PATH | O_NOFOLLOW | O_NOCTTY | O_CLOEXEC,
        0,
    );
    if fd2 < 0 {
        if ERRNO!() == ELOOP {
            ERRNO!() = EOPNOTSUPP;
        }
        return -1;
    }
    const PREFIX: &[u8] = b"/proc/self/fd/";
    let mut proc = [0u8; 32];
    proc[..PREFIX.len()].copy_from_slice(PREFIX);
    let n = PREFIX.len()
        + write_decimal(
            proc.as_mut_ptr().add(PREFIX.len()) as *mut core::ffi::c_char,
            fd2 as usize,
        );
    proc[n] = 0;
    let mut st2: Stat = core::mem::zeroed();
    let mut res: i32 = -1;
    if stat(proc.as_ptr() as *const core::ffi::c_char, &mut st2) == 0 {
        if st2.st_mode & S_IFMT == S_IFLNK {
            ERRNO!() = EOPNOTSUPP;
        } else {
            res = syscall_ret(<Arch as Syscalls>::syscall4(
                SYS_FCHMODAT,
                AT_FDCWD as i64,
                proc.as_ptr() as i64,
                mode as i64,
                0,
            )) as i32;
        }
    }
    close(fd2);
    res
}

#[no_mangle]
pub unsafe extern "C" fn fchownat(dirfd: i32, pathname: *const core::ffi::c_char, uid: u32, gid: u32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_FCHOWNAT, dirfd as i64, pathname as i64, uid as i64, gid as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn linkat(olddirfd: i32, oldpath: *const core::ffi::c_char, newdirfd: i32, newpath: *const core::ffi::c_char, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_LINKAT, olddirfd as i64, oldpath as i64, newdirfd as i64, newpath as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mkdirat(dirfd: i32, pathname: *const core::ffi::c_char, mode: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MKDIRAT, dirfd as i64, pathname as i64, mode as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mkfifoat(dirfd: i32, pathname: *const core::ffi::c_char, mode: u32) -> i32 {
    mknodat(dirfd, pathname, mode | 0o010000, 0)
}

#[no_mangle]
pub unsafe extern "C" fn mknodat(dirfd: i32, pathname: *const core::ffi::c_char, mode: u32, dev: u64) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_MKNODAT, dirfd as i64, pathname as i64, mode as i64, dev as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn readlinkat(dirfd: i32, pathname: *const core::ffi::c_char, buf: *mut core::ffi::c_char, bufsiz: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_READLINKAT, dirfd as i64, pathname as i64, buf as i64, bufsiz as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn renameat(olddirfd: i32, oldpath: *const core::ffi::c_char, newdirfd: i32, newpath: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_RENAMEAT2, olddirfd as i64, oldpath as i64, newdirfd as i64, newpath as i64, 0)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn renameat2(olddirfd: i32, oldpath: *const core::ffi::c_char, newdirfd: i32, newpath: *const core::ffi::c_char, flags: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_RENAMEAT2, olddirfd as i64, oldpath as i64, newdirfd as i64, newpath as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn symlinkat(target: *const core::ffi::c_char, newdirfd: i32, linkpath: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SYMLINKAT, target as i64, newdirfd as i64, linkpath as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn unlinkat(dirfd: i32, pathname: *const core::ffi::c_char, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_UNLINKAT, dirfd as i64, pathname as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn utimes(filename: *const core::ffi::c_char, times: *const core::ffi::c_void) -> i32 {
    utimensat(AT_FDCWD, filename, times as *const timespec, 0)
}

#[no_mangle]
pub unsafe extern "C" fn futimes(fd: i32, times: *const core::ffi::c_void) -> i32 {
    utimensat(fd, core::ptr::null(), times as *const timespec, 0)
}

// ============================================================
// inotify functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn inotify_init() -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_INOTIFY_INIT, 0)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn inotify_init1(flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_INOTIFY_INIT1, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn inotify_add_watch(fd: i32, pathname: *const core::ffi::c_char, mask: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_INOTIFY_ADD_WATCH, fd as i64, pathname as i64, mask as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn inotify_rm_watch(fd: i32, wd: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_INOTIFY_RM_WATCH, fd as i64, wd as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fanotify_init(flags: i32, event_f_flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_FANOTIFY_INIT, flags as i64, event_f_flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fanotify_mark(fd: i32, flags: i32, mask: u64, dirfd: i32, pathname: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_FANOTIFY_MARK, fd as i64, flags as i64, mask as i64, dirfd as i64, pathname as i64)) as i32
}

// ============================================================
// ffs functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn ffs(x: i32) -> i32 {
    if x == 0 { return 0; }
    x.trailing_zeros() as i32 + 1
}

#[no_mangle]
pub unsafe extern "C" fn ffsl(x: i64) -> i32 {
    if x == 0 { return 0; }
    x.trailing_zeros() as i32 + 1
}

#[no_mangle]
pub unsafe extern "C" fn ffsll(x: i64) -> i32 {
    if x == 0 { return 0; }
    x.trailing_zeros() as i32 + 1
}

// ============================================================
// More system functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fchdir(fd: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_FCHDIR, fd as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fchown(fd: i32, uid: u32, gid: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_FCHOWN, fd as i64, uid as i64, gid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fdopendir(fd: i32) -> *mut core::ffi::c_void {
    if fd < 0 { ERRNO!() = EBADF; return core::ptr::null_mut(); }
    dir_stream_new(fd)
}

#[no_mangle]
pub unsafe extern "C" fn getdtablesize() -> i32 {
    let mut rlim: Rlimit = core::mem::zeroed();
    if getrlimit(RLIMIT_NOFILE, &mut rlim) == 0 {
        rlim.rlim_cur as i32
    } else {
        1024
    }
}

#[no_mangle]
pub unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    if buflen > 256 {
        ERRNO!() = EIO;
        return -1;
    }
    let mut pos = buf;
    let mut left = buflen;
    while left > 0 {
        let r = getrandom(pos, left, 0);
        if r < 0 {
            if ERRNO!() == EINTR {
                continue;
            }
            return -1;
        }
        pos = pos.add(r as usize);
        left -= r as usize;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, flags: u32) -> isize {
    if let Some(r) = vgetrandom(buf, buflen, flags) {
        return if r < 0 {
            ERRNO!() = (-r) as c_int;
            -1
        } else {
            r
        };
    }
    let r = <Arch as Syscalls>::syscall3(SYS_GETRANDOM, buf as i64, buflen as i64, flags as i64);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        -1
    } else {
        r as isize
    }
}

#[no_mangle]
pub unsafe extern "C" fn killpg(pgrp: i32, sig: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_KILL, -pgrp as i64, sig as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mkfifo(pathname: *const core::ffi::c_char, mode: u32) -> i32 {
    mknod(pathname, 0o010000 | mode, 0)
}

// ============================================================
// Memory allocation functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn aligned_alloc(alignment: usize, size: usize) -> *mut core::ffi::c_void {
    alloc_aligned_impl(alignment, size) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn memalign(alignment: usize, size: usize) -> *mut core::ffi::c_void {
    alloc_aligned_impl(alignment, size) as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn posix_memalign(memptr: *mut *mut core::ffi::c_void, alignment: usize, size: usize) -> i32 {
    if memptr.is_null() { return EINVAL; }
    if alignment < core::mem::size_of::<*mut core::ffi::c_void>() || !alignment.is_power_of_two() {
        return EINVAL;
    }
    let p = alloc_aligned_impl(alignment, size);
    if p.is_null() { return ENOMEM; }
    *memptr = p as *mut core::ffi::c_void;
    0
}

#[no_mangle]
pub unsafe extern "C" fn valloc(size: usize) -> *mut core::ffi::c_void {
    alloc_aligned_impl(PAGE, size) as *mut core::ffi::c_void
}

// ============================================================
// glob functions
// ============================================================

// ============================================================
// wordexp functions
// ============================================================

// ============================================================
// gettext functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn gettext(msgid: *const core::ffi::c_char) -> *const core::ffi::c_char {
    msgid
}

#[no_mangle]
pub unsafe extern "C" fn dgettext(_domainname: *const core::ffi::c_char, msgid: *const core::ffi::c_char) -> *const core::ffi::c_char {
    msgid
}

#[no_mangle]
pub unsafe extern "C" fn dcgettext(_domainname: *const core::ffi::c_char, msgid: *const core::ffi::c_char, _category: i32) -> *const core::ffi::c_char {
    msgid
}

#[no_mangle]
pub unsafe extern "C" fn ngettext(msgid1: *const core::ffi::c_char, msgid2: *const core::ffi::c_char, n: u64) -> *const core::ffi::c_char {
    if n == 1 { msgid1 } else { msgid2 }
}

#[no_mangle]
pub unsafe extern "C" fn dngettext(_domainname: *const core::ffi::c_char, msgid1: *const core::ffi::c_char, msgid2: *const core::ffi::c_char, n: u64) -> *const core::ffi::c_char {
    if n == 1 { msgid1 } else { msgid2 }
}

#[no_mangle]
pub unsafe extern "C" fn dcngettext(_domainname: *const core::ffi::c_char, msgid1: *const core::ffi::c_char, msgid2: *const core::ffi::c_char, n: u64, _category: i32) -> *const core::ffi::c_char {
    if n == 1 { msgid1 } else { msgid2 }
}

#[no_mangle]
pub unsafe extern "C" fn textdomain(domainname: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    static mut DOMAIN: [core::ffi::c_char; 256] = [0; 256];
    if domainname.is_null() { return DOMAIN.as_mut_ptr(); }
    let mut i = 0;
    while i < 255 && *domainname.add(i) != 0 {
        DOMAIN[i] = *domainname.add(i);
        i += 1;
    }
    DOMAIN[i] = 0;
    DOMAIN.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn bindtextdomain(_domainname: *const core::ffi::c_char, dirname: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    dirname as *mut core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn bind_textdomain_codeset(_domainname: *const core::ffi::c_char, codeset: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    codeset as *mut core::ffi::c_char
}

// ============================================================
// Wide string functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn wmemcpy(dest: *mut u32, src: *const u32, n: usize) -> *mut u32 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn wmemmove(dest: *mut u32, src: *const u32, n: usize) -> *mut u32 {
    if dest as usize <= src as usize {
        let mut i = 0;
        while i < n {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn wmemset(dest: *mut u32, c: u32, n: usize) -> *mut u32 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = c;
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn wmemcmp(s1: *const u32, s2: *const u32, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 != c2 { return (c1 as i32) - (c2 as i32); }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn wmemchr(s: *const u32, c: u32, n: usize) -> *mut u32 {
    let mut i = 0;
    while i < n {
        if *s.add(i) == c { return s.add(i) as *mut u32; }
        i += 1;
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wcpcpy(dest: *mut u32, src: *const u32) -> *mut u32 {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dest.add(i) = c;
        if c == 0 { return dest.add(i); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcpncpy(dest: *mut u32, src: *const u32, n: usize) -> *mut u32 {
    let mut i = 0;
    while i < n {
        let c = *src.add(i);
        *dest.add(i) = c;
        if c == 0 { return dest.add(i); }
        i += 1;
    }
    dest.add(n)
}

#[no_mangle]
pub unsafe extern "C" fn wprintf(fmt: *const wchar_t, args: ...) -> c_int {
    vfwprintf(stdout, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vwprintf(fmt: *const wchar_t, ap: core::ffi::VaList) -> c_int {
    vfwprintf(stdout, fmt, ap)
}

// ============================================================
// C32 functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn c16rtomb(s: *mut u8, c16: u16, ps: *mut core::ffi::c_void) -> usize {
    // UTF-16 to UTF-8, with surrogate pairs tracked in the mbstate.
    let state = ps as *mut u32;
    let saved: u32 = if state.is_null() { 0 } else { *state };
    if s.is_null() {
        if !state.is_null() { *state = 0; }
        return 1;
    }
    let cp: u32;
    if saved != 0 {
        if (c16 as u32) < UNICODE_LOW_SURROGATE_START || (c16 as u32) > UNICODE_SURROGATE_END {
            if !state.is_null() { *state = 0; }
            ERRNO!() = EILSEQ;
            return !0usize;
        }
        cp = UNICODE_SUPPLEMENTARY_BASE
            + (((saved - UNICODE_SURROGATE_START) << 10) | ((c16 as u32) - UNICODE_LOW_SURROGATE_START));
        if !state.is_null() { *state = 0; }
    } else if (c16 as u32) >= UNICODE_SURROGATE_START && (c16 as u32) <= UNICODE_HIGH_SURROGATE_END {
        if !state.is_null() { *state = c16 as u32; }
        return 0;
    } else if c16 >= 0xdc00 && c16 <= 0xdfff {
        ERRNO!() = EILSEQ;
        return !0usize;
    } else {
        cp = c16 as u32;
    }
    if cp < 0x80 {
        *s = cp as u8;
        1
    } else if cp < 0x800 {
        *s = (0xc0 | (cp >> 6)) as u8;
        *s.add(1) = (0x80 | (cp & 0x3f)) as u8;
        2
    } else if cp < 0x10000 {
        *s = (0xe0 | (cp >> 12)) as u8;
        *s.add(1) = (0x80 | ((cp >> 6) & 0x3f)) as u8;
        *s.add(2) = (0x80 | (cp & 0x3f)) as u8;
        3
    } else {
        *s = (0xf0 | (cp >> 18)) as u8;
        *s.add(1) = (0x80 | ((cp >> 12) & 0x3f)) as u8;
        *s.add(2) = (0x80 | ((cp >> 6) & 0x3f)) as u8;
        *s.add(3) = (0x80 | (cp & 0x3f)) as u8;
        4
    }
}

#[no_mangle]
pub unsafe extern "C" fn c32rtomb(s: *mut u8, c32: u32, _ps: *mut core::ffi::c_void) -> usize {
    if s.is_null() { return 1; }
    if c32 > 0x10ffff || (c32 >= 0xd800 && c32 <= 0xdfff) {
        ERRNO!() = EILSEQ;
        return !0usize;
    }
    if c32 < 0x80 {
        *s = c32 as u8;
        1
    } else if c32 < 0x800 {
        *s = (0xc0 | (c32 >> 6)) as u8;
        *s.add(1) = (0x80 | (c32 & 0x3f)) as u8;
        2
    } else if c32 < 0x10000 {
        *s = (0xe0 | (c32 >> 12)) as u8;
        *s.add(1) = (0x80 | ((c32 >> 6) & 0x3f)) as u8;
        *s.add(2) = (0x80 | (c32 & 0x3f)) as u8;
        3
    } else {
        *s = (0xf0 | (c32 >> 18)) as u8;
        *s.add(1) = (0x80 | ((c32 >> 12) & 0x3f)) as u8;
        *s.add(2) = (0x80 | ((c32 >> 6) & 0x3f)) as u8;
        *s.add(3) = (0x80 | (c32 & 0x3f)) as u8;
        4
    }
}

#[no_mangle]
pub unsafe extern "C" fn mbrtoc16(pc16: *mut u16, s: *const u8, n: usize, _ps: *mut core::ffi::c_void) -> usize {
    if s.is_null() { return 0; }
    if n == 0 { return 1; }
    let c = *s;
    if c == 0 { return 0; }
    if c < 0x80 {
        if !pc16.is_null() { *pc16 = c as u16; }
        return 1;
    }
    if c < 0xC2 { return 1; } // Invalid
    if c < 0xE0 {
        if n < 2 { return 1; }
        let c2 = *s.add(1);
        if (c2 & 0xC0) != 0x80 { return 1; }
        if !pc16.is_null() { *pc16 = ((((c & 0x1F) as u32) << 6) | ((c2 & 0x3F) as u32)) as u16; }
        return 2;
    }
    if c < 0xF0 {
        if n < 3 { return 1; }
        let c2 = *s.add(1);
        let c3 = *s.add(2);
        if (c2 & 0xC0) != 0x80 || (c3 & 0xC0) != 0x80 { return 1; }
        if !pc16.is_null() { *pc16 = ((((c & 0x0F) as u32) << 12) | (((c2 & 0x3F) as u32) << 6) | ((c3 & 0x3F) as u32)) as u16; }
        return 3;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn mbrtoc32(pc32: *mut u32, s: *const u8, n: usize, _ps: *mut core::ffi::c_void) -> usize {
    if s.is_null() { return 0; }
    if n == 0 { return 1; }
    let c = *s;
    if c == 0 { return 0; }
    if c < 0x80 {
        if !pc32.is_null() { *pc32 = c as u32; }
        return 1;
    }
    if c < 0xC2 { return 1; } // Invalid
    if c < 0xE0 {
        if n < 2 { return 1; }
        let c2 = *s.add(1);
        if (c2 & 0xC0) != 0x80 { return 1; }
        if !pc32.is_null() { *pc32 = (((c & 0x1F) as u32) << 6) | ((c2 & 0x3F) as u32); }
        return 2;
    }
    if c < 0xF0 {
        if n < 3 { return 1; }
        let c2 = *s.add(1);
        let c3 = *s.add(2);
        if (c2 & 0xC0) != 0x80 || (c3 & 0xC0) != 0x80 { return 1; }
        if !pc32.is_null() { *pc32 = (((c & 0x0F) as u32) << 12) | (((c2 & 0x3F) as u32) << 6) | ((c3 & 0x3F) as u32); }
        return 3;
    }
    if c < 0xF5 {
        if n < 4 { return 1; }
        let c2 = *s.add(1);
        let c3 = *s.add(2);
        let c4 = *s.add(3);
        if (c2 & 0xC0) != 0x80 || (c3 & 0xC0) != 0x80 || (c4 & 0xC0) != 0x80 { return 1; }
        if !pc32.is_null() { *pc32 = (((c & 0x07) as u32) << 18) | (((c2 & 0x3F) as u32) << 12) | (((c3 & 0x3F) as u32) << 6) | ((c4 & 0x3F) as u32); }
        return 4;
    }
    1
}

// ============================================================
// Shadow functions
// ============================================================




// ============================================================
// chown functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn chown(path: *const core::ffi::c_char, uid: u32, gid: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_FCHOWNAT,
        AT_FDCWD as i64,
        path as i64,
        uid as i64,
        gid as i64,
        0,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn lchown(path: *const core::ffi::c_char, uid: u32, gid: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_FCHOWNAT,
        AT_FDCWD as i64,
        path as i64,
        uid as i64,
        gid as i64,
        AT_SYMLINK_NOFOLLOW as i64,
    )) as i32
}

// ============================================================
// chroot functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn chroot(path: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_CHROOT, path as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn get_current_dir_name() -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; PATH_MAX_V] = [0; PATH_MAX_V];
    let r = getcwd(BUF.as_mut_ptr(), PATH_MAX_V);
    if r.is_null() { core::ptr::null_mut() } else { BUF.as_mut_ptr() }
}

// ============================================================
// I/O functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn creat(pathname: *const core::ffi::c_char, mode: u32) -> i32 {
    // O_WRONLY | O_CREAT | O_TRUNC. The previous flags included O_EXCL.
    open(pathname, O_WRONLY | O_CREAT | O_TRUNC, mode as i32)
}

#[no_mangle]
pub unsafe extern "C" fn openat(dirfd: i32, pathname: *const core::ffi::c_char, flags: i32, mode: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_OPENAT, dirfd as i64, pathname as i64, flags as i64, mode as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pwrite(fd: i32, buf: *const u8, count: usize, offset: i64) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_PWRITE64, fd as i64, buf as i64, count as i64, offset)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn readv(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_READV, fd as i64, iov as i64, iovcnt as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn writev(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_WRITEV, fd as i64, iov as i64, iovcnt as i64)) as isize
}

// ============================================================
// Lock functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn flock(fd: i32, operation: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_FLOCK, fd as i64, operation as i64)) as i32
}

#[repr(C)]
struct FlockArg {
    l_type: i16,
    l_whence: i16,
    l_start: i64,
    l_len: i64,
    l_pid: i32,
}

#[no_mangle]
pub unsafe extern "C" fn lockf(fd: i32, cmd: i32, len: i64) -> i32 {
    // POSIX lockf commands map onto fcntl record locks relative to the
    // current file offset.
    const F_RDLCK: i16 = 0;
    const F_WRLCK: i16 = 1;
    const F_UNLCK: i16 = 2;
    const F_SETLK: i64 = 6;
    const F_SETLKW: i64 = 7;
    const SEEK_CUR: i16 = 1;
    let (l_type, wait) = match cmd {
        0 => (F_UNLCK, false), // F_ULOCK
        1 => (F_WRLCK, true),  // F_LOCK
        2 => (F_WRLCK, false), // F_TLOCK
        3 => (F_RDLCK, false), // F_TEST
        _ => {
            ERRNO!() = EINVAL;
            return -1;
        }
    };
    let fl = FlockArg { l_type, l_whence: SEEK_CUR, l_start: 0, l_len: len, l_pid: 0 };
    let r = <Arch as Syscalls>::syscall3(
        SYS_FCNTL,
        fd as i64,
        if wait { F_SETLKW } else { F_SETLK },
        &fl as *const FlockArg as i64,
    );
    syscall_ret(r) as c_int
}

// ============================================================
// Hostname/login functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getdomainname(name: *mut core::ffi::c_char, len: usize) -> i32 {
    if name.is_null() || len == 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    let mut uts: UtsName = core::mem::zeroed();
    let r = <Arch as Syscalls>::syscall1(SYS_UNAME, &mut uts as *mut UtsName as i64);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    let mut n = 0usize;
    while n < uts.domainname.len() && uts.domainname[n] != 0 { n += 1; }
    if n >= len {
        ERRNO!() = EINVAL;
        return -1;
    }
    core::ptr::copy_nonoverlapping(uts.domainname.as_ptr(), name as *mut u8, n);
    *name.add(n) = 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn setdomainname(name: *const core::ffi::c_char, len: usize) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SETDOMAINNAME, name as i64, len as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sethostname(name: *const core::ffi::c_char, len: usize) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SETHOSTNAME, name as i64, len as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getlogin_r(name: *mut core::ffi::c_char, bufsize: usize) -> i32 {
    let user = getlogin();
    if user.is_null() { return ENXIO; }
    let len = crate::strlen(user);
    if len >= bufsize { return 34; } // ERANGE
    core::ptr::copy_nonoverlapping(user, name, len + 1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn cuserid(s: *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    let user = crate::getenv(b"USER\0".as_ptr() as *const core::ffi::c_char);
    if user.is_null() { return core::ptr::null_mut(); }
    if s.is_null() {
        static mut BUF: [core::ffi::c_char; 32] = [0; 32];
        let mut i = 0;
        while i < 31 && *user.add(i) != 0 { BUF[i] = *user.add(i); i += 1; }
        BUF[i] = 0;
        BUF.as_mut_ptr()
    } else {
        let mut i = 0;
        while i < 31 && *user.add(i) != 0 { *s.add(i) = *user.add(i); i += 1; }
        *s.add(i) = 0;
        s
    }
}

include!("passwd_group.rs");
// ============================================================
// User/group functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn setgroups(ngroups: usize, gidset: *const u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SETGROUPS, ngroups as i64, gidset as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn setresuid(ruid: u32, euid: u32, suid: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SETRESUID, ruid as i64, euid as i64, suid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn setresgid(rgid: u32, egid: u32, sgid: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SETRESGID, rgid as i64, egid as i64, sgid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getresuid(ruid: *mut u32, euid: *mut u32, suid: *mut u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_GETRESUID, ruid as i64, euid as i64, suid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getresgid(rgid: *mut u32, egid: *mut u32, sgid: *mut u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_GETRESGID, rgid as i64, egid as i64, sgid as i64)) as i32
}

// ============================================================
// Socket functions
// ============================================================

// musl's public struct msghdr uses 32-bit msg_iovlen/msg_controllen followed
// by padding on 64-bit little-endian. The kernel reads 64-bit fields, so the
// padding must be zero; callers (nginx, for example) do not initialize it.
#[repr(C)]
struct KernelMsgHdr {
    msg_name: *mut c_void,
    msg_namelen: u32,
    __pad0: u32,
    msg_iov: *mut c_void,
    msg_iovlen: u64,
    msg_control: *mut c_void,
    msg_controllen: u64,
    msg_flags: u32,
    __pad1: u32,
}

#[repr(C)]
struct UserMsgHdr {
    msg_name: *mut c_void,
    msg_namelen: u32,
    msg_iov: *mut c_void,
    msg_iovlen: i32,
    __pad1: i32,
    msg_control: *mut c_void,
    msg_controllen: u32,
    __pad2: i32,
    msg_flags: i32,
}

unsafe fn msg_to_kernel(msg: *const UserMsgHdr, out: &mut KernelMsgHdr) {
    *out = KernelMsgHdr {
        msg_name: (*msg).msg_name,
        msg_namelen: (*msg).msg_namelen,
        __pad0: 0,
        msg_iov: (*msg).msg_iov,
        msg_iovlen: (*msg).msg_iovlen as u32 as u64,
        msg_control: (*msg).msg_control,
        msg_controllen: (*msg).msg_controllen as u64,
        msg_flags: (*msg).msg_flags as u32,
        __pad1: 0,
    };
}

#[no_mangle]
pub unsafe extern "C" fn sendmsg(fd: i32, msg: *const core::ffi::c_void, flags: i32) -> isize {
    if msg.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let mut k: KernelMsgHdr = core::mem::zeroed();
    msg_to_kernel(msg as *const UserMsgHdr, &mut k);
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SENDMSG, fd as i64, &k as *const KernelMsgHdr as i64, flags as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn recvmsg(fd: i32, msg: *mut core::ffi::c_void, flags: i32) -> isize {
    if msg.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let user = msg as *mut UserMsgHdr;
    let mut k: KernelMsgHdr = core::mem::zeroed();
    msg_to_kernel(user, &mut k);
    let r = syscall_ret(<Arch as Syscalls>::syscall3(SYS_RECVMSG, fd as i64, &mut k as *mut KernelMsgHdr as i64, flags as i64));
    (*user).msg_flags = k.msg_flags as i32;
    (*user).msg_controllen = k.msg_controllen as u32;
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn sendmmsg(fd: i32, msgvec: *mut core::ffi::c_void, vlen: u32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_SENDMMSG, fd as i64, msgvec as i64, vlen as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn recvmmsg(fd: i32, msgvec: *mut core::ffi::c_void, vlen: u32, flags: i32, timeout: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_RECVMMSG, fd as i64, msgvec as i64, vlen as i64, flags as i64, timeout as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getpeername(fd: i32, addr: *mut core::ffi::c_void, addrlen: *mut u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_GETPEERNAME, fd as i64, addr as i64, addrlen as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getsockopt(fd: i32, level: i32, optname: i32, optval: *mut core::ffi::c_void, optlen: *mut u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_GETSOCKOPT, fd as i64, level as i64, optname as i64, optval as i64, optlen as i64)) as i32
}

// ============================================================
// stdio unlocked functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn ftrylockfile(file: *mut FILE) -> i32 {
    if file.is_null() { return EINVAL; }
    if (*file).lock < 0 { return 0; }
    let tid = sys_gettid() as c_int;
    let v = a_load(&raw const (*file).lock);
    if v == tid {
        (*file).lockcount += 1;
        return 0;
    }
    if v == 0 && a_cas(&raw mut (*file).lock, 0, tid) == 0 {
        (*file).lockcount = 1;
        return 0;
    }
    EBUSY
}

#[no_mangle]
pub unsafe extern "C" fn getc_unlocked(file: *mut core::ffi::c_char) -> i32 {
    getc(file as *mut FILE)
}

#[no_mangle]
pub unsafe extern "C" fn getchar_unlocked() -> i32 {
    getchar()
}

#[no_mangle]
pub unsafe extern "C" fn putc_unlocked(c: i32, file: *mut core::ffi::c_char) -> i32 {
    putc(c, file as *mut FILE)
}

#[no_mangle]
pub unsafe extern "C" fn putchar_unlocked(c: i32) -> i32 {
    putchar(c)
}

// ============================================================
// getopt functions
// ============================================================

#[no_mangle] pub static mut optarg: *mut core::ffi::c_char = core::ptr::null_mut();
#[no_mangle] pub static mut optind: i32 = 1;
#[no_mangle] pub static mut opterr: i32 = 1;
#[no_mangle] pub static mut optopt: i32 = '?' as i32;
#[no_mangle] pub static mut optreset: i32 = 0;

// ============================================================
// Timer functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getitimer(which: i32, curr_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_GETITIMER, which as i64, curr_value as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn setitimer(which: i32, new_value: *const core::ffi::c_void, old_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SETITIMER, which as i64, new_value as i64, old_value as i64)) as i32
}

// ============================================================
// System info functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getloadavg(loadavg: *mut f64, nelem: i32) -> i32 {
    let fd = crate::open(b"/proc/loadavg\0".as_ptr() as *const core::ffi::c_char, 0, 0);
    if fd < 0 { return -1; }
    let mut buf: [u8; 64] = [0; 64];
    let n = crate::read(fd, buf.as_mut_ptr() as *mut core::ffi::c_void, 63);
    crate::close(fd);
    if n <= 0 { return -1; }
    buf[n as usize] = 0;
    if loadavg.is_null() || nelem <= 0 { return -1; }
    let mut p = buf.as_ptr() as *const core::ffi::c_char;
    let mut i = 0usize;
    while i < nelem as usize && i < 3 {
        let mut end: *mut core::ffi::c_char = core::ptr::null_mut();
        let val = crate::strtod(p, &mut end);
        if end as *const core::ffi::c_char == p { break; }
        *loadavg.add(i) = val;
        i += 1;
        p = end;
        while *p == b' ' as c_char || *p == b'\t' as c_char || *p == b'\n' as c_char {
            p = p.add(1);
        }
    }
    i as i32
}

#[no_mangle]
pub unsafe extern "C" fn get_nprocs() -> i32 {
    sysconf(84) as i32 // _SC_NPROCESSORS_ONLN
}

#[no_mangle]
pub unsafe extern "C" fn get_nprocs_conf() -> i32 {
    sysconf(83) as i32 // _SC_NPROCESSORS_CONF
}

#[no_mangle]
pub unsafe extern "C" fn get_avphys_pages() -> i64 {
    sysconf(86) // _SC_AVPHYS_PAGES
}

#[no_mangle]
pub unsafe extern "C" fn get_phys_pages() -> i64 {
    sysconf(85) // _SC_PHYS_PAGES
}

// ============================================================
// Capability functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn capget(header: *mut core::ffi::c_void, data: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_CAPGET, header as i64, data as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn capset(header: *const core::ffi::c_void, data: *const core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_CAPSET, header as i64, data as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn arch_prctl(code: i32, addr: u64) -> i32 {
    #[cfg(target_arch = "x86_64")]
    { syscall_ret(<Arch as Syscalls>::syscall2(crate::SYS_ARCH_PRCTL, code as i64, addr as i64)) as i32 }
    #[cfg(not(target_arch = "x86_64"))]
    { let _ = (code, addr); ERRNO!() = ENOSYS; -1 }
}

// ============================================================
// termios functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn cfgetospeed(termios_p: *const core::ffi::c_void) -> u32 {
    unsafe { *(termios_p as *const u32).add(1) }
}

#[no_mangle]
pub unsafe extern "C" fn cfgetispeed(termios_p: *const core::ffi::c_void) -> u32 {
    unsafe { *(termios_p as *const u32).add(2) }
}

#[no_mangle]
pub unsafe extern "C" fn cfsetospeed(termios_p: *mut core::ffi::c_void, speed: u32) -> i32 {
    unsafe { *(termios_p as *mut u32).add(1) = speed; }
    0
}

#[no_mangle]
pub unsafe extern "C" fn cfsetispeed(termios_p: *mut core::ffi::c_void, speed: u32) -> i32 {
    unsafe { *(termios_p as *mut u32).add(2) = speed; }
    0
}

#[no_mangle]
pub unsafe extern "C" fn cfmakeraw(termios_p: *mut core::ffi::c_void) {
    unsafe {
        let p = termios_p as *mut u32;
        *p.add(0) &= !0o37;
        *p.add(0) &= !(0o4000 | 0o2000 | 0o10000 | 0o20);
        *p.add(0) |= 0o60;
        *p.add(3) &= !0o17;
        *p.add(3) &= !(0o1000 | 0o40000000);
        *p.add(1) = 1;
        *p.add(2) = 0;
    }
}

// ============================================================
// Resource functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getrusage(who: i32, usage: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_GETRUSAGE, who as i64, usage as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getpriority(which: i32, who: u32) -> i32 {
    // The kernel encodes nice as 20 - nice; musl converts it back.
    let r = <Arch as Syscalls>::syscall2(SYS_GETPRIORITY, which as i64, who as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    20 - r as i32
}

#[no_mangle]
pub unsafe extern "C" fn setpriority(which: i32, who: u32, prio: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SETPRIORITY, which as i64, who as i64, prio as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn prlimit(pid: u32, resource: i32, new_limit: *const core::ffi::c_void, old_limit: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_PRLIMIT64, pid as i64, resource as i64, new_limit as i64, old_limit as i64)) as i32
}

// ============================================================
// fork/clone functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn clone(
    func: usize,
    stack: *mut u8,
    flags: c_int,
    arg: *mut c_void,
    mut args: ...
) -> c_int {
    // musl-compatible signature: clone(fn, stack, flags, arg, ptid, tls, ctid).
    const CLONE_PIDFD_C: c_int = 0x00001000;
    const CLONE_VM_C: c_int = 0x00000100;
    const CLONE_THREAD_C: c_int = 0x00010000;
    const CLONE_SETTLS_C: c_int = 0x00080000;
    const CLONE_PARENT_SETTID_C: c_int = 0x00100000;
    const CLONE_CHILD_SETTID_C: c_int = 0x01000000;
    const CLONE_CHILD_CLEARTID_C: c_int = 0x00200000;
    let _ = CLONE_VM_C;
    if flags & (CLONE_THREAD_C | CLONE_SETTLS_C | CLONE_CHILD_CLEARTID_C) != 0
        || stack.is_null()
    {
        ERRNO!() = EINVAL;
        return -1;
    }
    let mut ptid: *mut c_int = core::ptr::null_mut();
    let mut tls: c_ulong = 0;
    let mut ctid: *mut c_int = core::ptr::null_mut();
    if flags & (CLONE_PIDFD_C | CLONE_PARENT_SETTID_C | CLONE_CHILD_SETTID_C) != 0 {
        ptid = args.next_arg::<*mut c_int>();
    }
    if flags & CLONE_CHILD_SETTID_C != 0 {
        tls = args.next_arg::<c_ulong>();
        ctid = args.next_arg::<*mut c_int>();
    }
    let r = __keel_clone(func, stack, flags as c_ulong, arg, ptid, tls, ctid);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn _Fork() -> i32 {
    crate::fork()
}

#[no_mangle]
pub unsafe extern "C" fn forkpty(amaster: *mut i32, name: *mut core::ffi::c_char, termp: *const core::ffi::c_void, winp: *const core::ffi::c_void) -> i32 {
    let mut master: i32 = 0;
    let mut slave: i32 = 0;
    if openpty(&mut master, &mut slave, name, termp, winp) < 0 { return -1; }
    let pid = crate::fork();
    if pid < 0 { crate::close(master); crate::close(slave); return -1; }
    if pid == 0 {
        crate::close(master);
        crate::setsid();
        crate::dup2(slave, 0);
        crate::dup2(slave, 1);
        crate::dup2(slave, 2);
    } else {
        crate::close(slave);
        if !amaster.is_null() { *amaster = master; }
    }
    pid
}

#[no_mangle]
pub unsafe extern "C" fn openpty(amaster: *mut i32, aslave: *mut i32, name: *mut core::ffi::c_char, _termp: *const core::ffi::c_void, _winp: *const core::ffi::c_void) -> i32 {
    let master = crate::open(b"/dev/ptmx\0".as_ptr() as *const core::ffi::c_char, 0o2000, 0);
    if master < 0 { return -1; }
    grantpt(master);
    unlockpt(master);
    let slave_name = ptsname(master);
    if slave_name.is_null() { crate::close(master); return -1; }
    let slave = crate::open(slave_name, 0o2000, 0);
    if slave < 0 { crate::close(master); return -1; }
    if !amaster.is_null() { *amaster = master; }
    if !aslave.is_null() { *aslave = slave; }
    if !name.is_null() { crate::strcpy(name as *mut u8, slave_name as *const u8); }
    0
}

#[no_mangle]
pub unsafe extern "C" fn login_tty(fd: i32) -> i32 {
    crate::setsid();
    crate::dup2(fd, 0);
    crate::dup2(fd, 1);
    crate::dup2(fd, 2);
    0
}

// ============================================================
// dl functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn __dls2b() {}
#[no_mangle]
pub unsafe extern "C" fn __dls3() {}
#[no_mangle]
pub unsafe extern "C" fn _dlstart() {}

// ============================================================
// string functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn index(s: *const core::ffi::c_char, c: i32) -> *mut core::ffi::c_char {
    let mut i = 0;
    loop {
        let ch = *s.add(i);
        if ch == c as core::ffi::c_char { return s.add(i) as *mut core::ffi::c_char; }
        if ch == 0 { return core::ptr::null_mut(); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn rindex(s: *const core::ffi::c_char, c: i32) -> *mut core::ffi::c_char {
    let mut last = core::ptr::null_mut();
    let mut i = 0;
    loop {
        let ch = *s.add(i);
        if ch == 0 { return last; }
        if ch == c as core::ffi::c_char { last = s.add(i) as *mut core::ffi::c_char; }
        i += 1;
    }
}

// ============================================================
// intmax functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn imaxabs(j: i64) -> i64 {
    j.abs()
}

#[no_mangle]
pub unsafe extern "C" fn imaxdiv(numer: i64, denom: i64) -> (i64, i64) {
    (numer / denom, numer % denom)
}

// ============================================================
// herror functions
// ============================================================

#[no_mangle]
pub static mut h_errno: i32 = 0;

#[no_mangle]
pub unsafe extern "C" fn __h_errno_location() -> *mut i32 {
    h_errno_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn herror(s: *const core::ffi::c_char) {
    if !s.is_null() { crate::write(2, s as *const core::ffi::c_void, crate::strlen(s)); }
    crate::write(2, b": ".as_ptr() as *const core::ffi::c_void, 2);
    let msg = hstrerror(*__h_errno_location());
    crate::write(2, msg as *const core::ffi::c_void, crate::strlen(msg));
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
}

#[no_mangle]
pub unsafe extern "C" fn hstrerror(err: i32) -> *const core::ffi::c_char {
    let msg: &[u8] = match err {
        1 => b"Host not found\0",
        2 => b"Try again\0",
        3 => b"Non-recoverable error\0",
        4 => b"Address not available\0",
        _ => b"Unknown error\0",
    };
    msg.as_ptr() as *const core::ffi::c_char
}

// ============================================================
// gettid function
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn gettid() -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall0(SYS_GETTID)) as i32
}

// ============================================================
// ioctl functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn ioctl(fd: i32, request: u32, mut args: ...) -> i32 {
    let arg = args.next_arg::<i64>();
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, request as i64, arg)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn ioperm(from: u64, num: u64, turn_on: i32) -> i32 {
    #[cfg(target_arch = "x86_64")]
    { syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOPERM, from as i64, num as i64, turn_on as i64)) as i32 }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (from, num, turn_on);
        ERRNO!() = ENOSYS;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn iopl(level: i32) -> i32 {
    #[cfg(target_arch = "x86_64")]
    { syscall_ret(<Arch as Syscalls>::syscall1(SYS_IOPL, level as i64)) as i32 }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = level;
        ERRNO!() = ENOSYS;
        -1
    }
}

// ============================================================
// xattr functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn getxattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char, value: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_GETXATTR, path as i64, name as i64, value as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn lgetxattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char, value: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_LGETXATTR, path as i64, name as i64, value as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn fgetxattr(fd: i32, name: *const core::ffi::c_char, value: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_FGETXATTR, fd as i64, name as i64, value as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn listxattr(path: *const core::ffi::c_char, list: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_LISTXATTR, path as i64, list as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn llistxattr(path: *const core::ffi::c_char, list: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_LLISTXATTR, path as i64, list as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn flistxattr(fd: i32, list: *mut u8, size: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_FLISTXATTR, fd as i64, list as i64, size as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn setxattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char, value: *const u8, size: usize, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_SETXATTR, path as i64, name as i64, value as i64, size as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn lsetxattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char, value: *const u8, size: usize, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_LSETXATTR, path as i64, name as i64, value as i64, size as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fsetxattr(fd: i32, name: *const core::ffi::c_char, value: *const u8, size: usize, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_FSETXATTR, fd as i64, name as i64, value as i64, size as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn removexattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_REMOVEXATTR, path as i64, name as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn lremovexattr(path: *const core::ffi::c_char, name: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_LREMOVEXATTR, path as i64, name as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn fremovexattr(fd: i32, name: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_FREMOVEXATTR, fd as i64, name as i64)) as i32
}

// ============================================================
// adjtime functions
// ============================================================

#[repr(C)]
struct TimexBuf {
    modes: c_uint,
    offset: c_long,
    freq: c_long,
    maxerror: c_long,
    esterror: c_long,
    status: c_int,
    constant: c_long,
    precision: c_long,
    tolerance: c_long,
    time: timeval,
    tick: c_long,
    ppsfreq: c_long,
    jitter: c_long,
    shift: c_int,
    stabil: c_long,
    jitcnt: c_long,
    calcnt: c_long,
    errcnt: c_long,
    stbcnt: c_long,
    tai: c_int,
    padding: [c_int; 11],
}

#[no_mangle]
pub unsafe extern "C" fn adjtime(delta: *const timeval, olddelta: *mut timeval) -> i32 {
    // Port of musl's linux/adjtime.c.
    let mut tx: TimexBuf = core::mem::zeroed();
    if !delta.is_null() {
        if (*delta).tv_sec > 1000 || (*delta).tv_usec > 1_000_000_000 {
            ERRNO!() = EINVAL;
            return -1;
        }
        tx.offset = (*delta).tv_sec * 1_000_000 + (*delta).tv_usec;
        tx.modes = 0x8001; // ADJ_OFFSET_SINGLESHOT
    }
    let r = <Arch as Syscalls>::syscall1(SYS_ADJTIMEX, &mut tx as *mut TimexBuf as i64);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    if !olddelta.is_null() {
        let mut sec = tx.offset / 1_000_000;
        let mut usec = tx.offset % 1_000_000;
        if usec < 0 {
            sec -= 1;
            usec += 1_000_000;
        }
        (*olddelta).tv_sec = sec;
        (*olddelta).tv_usec = usec;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn adjtimex(buf: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_ADJTIMEX, buf as i64)) as i32
}

// ============================================================
// pty functions
// ============================================================

// Linux devpts ioctls (asm-generic/ioctls.h): TIOCGPTN = _IOR('T', 0x30, int),
// TIOCSPTLCK = _IOW('T', 0x31, int). The old 0x541x values are the window-size
// and process-group requests, which both return the wrong data and write more
// bytes than the int the caller supplied.
#[cfg(target_arch = "powerpc64")]
const TIOCGPTN: i64 = 0x40045430;
#[cfg(not(target_arch = "powerpc64"))]
const TIOCGPTN: i64 = 0x80045430;
#[cfg(target_arch = "powerpc64")]
const TIOCSPTLCK: i64 = 0x80045431;
#[cfg(not(target_arch = "powerpc64"))]
const TIOCSPTLCK: i64 = 0x40045431;

#[no_mangle]
pub unsafe extern "C" fn grantpt(_fd: i32) -> i32 {
    // devpts creates the slave already owned by the caller.
    0
}

#[no_mangle]
pub unsafe extern "C" fn unlockpt(fd: i32) -> i32 {
    let mut unlock: i32 = 0;
    let r = <Arch as Syscalls>::syscall3(
        SYS_IOCTL,
        fd as i64,
        TIOCSPTLCK,
        &mut unlock as *mut i32 as i64,
    );
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn ptsname(fd: i32) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 32] = [0; 32];
    let mut pty_nr: i32 = 0;
    if <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, TIOCGPTN, &mut pty_nr as *mut i32 as i64) < 0 {
        return core::ptr::null_mut();
    }
    // Format /dev/pts/N
    BUF[0] = '/' as c_char; BUF[1] = 'd' as c_char; BUF[2] = 'e' as c_char; BUF[3] = 'v' as c_char;
    BUF[4] = '/' as c_char; BUF[5] = 'p' as c_char; BUF[6] = 't' as c_char; BUF[7] = 's' as c_char;
    BUF[8] = '/' as c_char;
    let i = 9 + write_decimal(BUF.as_mut_ptr().add(9), pty_nr as usize);
    if i >= BUF.len() { return core::ptr::null_mut(); }
    BUF[i] = 0;
    BUF.as_mut_ptr()
}

// ============================================================
// ftw functions
// ============================================================

// ============================================================
// ctype locale functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn __isalnum_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as i32 && c <= '9' as i32 || c >= 'A' as i32 && c <= 'Z' as i32 || c >= 'a' as i32 && c <= 'z' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isalpha_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'A' as i32 && c <= 'Z' as i32 || c >= 'a' as i32 && c <= 'z' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isblank_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c == ' ' as i32 || c == '\t' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iscntrl_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c < 0x20 || c == 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isdigit_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as i32 && c <= '9' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isgraph_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c > 0x20 && c < 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __islower_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'a' as i32 && c <= 'z' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isprint_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 0x20 && c < 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __ispunct_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 0x21 && c <= 0x2f || c >= 0x3a && c <= 0x40 || c >= 0x5b && c <= 0x60 || c >= 0x7b && c <= 0x7e) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isspace_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c == ' ' as i32 || c >= 0x09 && c <= 0x0d) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isupper_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'A' as i32 && c <= 'Z' as i32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __isxdigit_l(c: i32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as i32 && c <= '9' as i32 || c >= 'A' as i32 && c <= 'F' as i32 || c >= 'a' as i32 && c <= 'f' as i32) as i32
}

// ============================================================
// wide char locale functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn __iswalnum_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as u32 && c <= '9' as u32 || c >= 'A' as u32 && c <= 'Z' as u32 || c >= 'a' as u32 && c <= 'z' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswalpha_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'A' as u32 && c <= 'Z' as u32 || c >= 'a' as u32 && c <= 'z' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswblank_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c == ' ' as u32 || c == '\t' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswcntrl_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c < 0x20 || c == 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswdigit_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as u32 && c <= '9' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswgraph_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c > 0x20 && c < 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswlower_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'a' as u32 && c <= 'z' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswprint_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 0x20 && c < 0x7f) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswpunct_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 0x21 && c <= 0x2f || c >= 0x3a && c <= 0x40 || c >= 0x5b && c <= 0x60 || c >= 0x7b && c <= 0x7e) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswspace_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c == ' ' as u32 || c >= 0x09 && c <= 0x0d) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswupper_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= 'A' as u32 && c <= 'Z' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswxdigit_l(c: u32, _loc: *mut core::ffi::c_void) -> i32 {
    (c >= '0' as u32 && c <= '9' as u32 || c >= 'A' as u32 && c <= 'F' as u32 || c >= 'a' as u32 && c <= 'f' as u32) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __iswctype_l(c: u32, ctype: u32, _loc: *mut core::ffi::c_void) -> i32 {
    crate::iswctype(c as wint_t, ctype as wctype_t)
}

#[no_mangle]
pub unsafe extern "C" fn __towlower_l(c: u32, _loc: *mut core::ffi::c_void) -> u32 {
    crate::towlower(c)
}

#[no_mangle]
pub unsafe extern "C" fn __towupper_l(c: u32, _loc: *mut core::ffi::c_void) -> u32 {
    crate::towupper(c)
}

#[no_mangle]
pub unsafe extern "C" fn __towctrans_l(c: u32, _trans: u32, _loc: *mut core::ffi::c_void) -> u32 {
    c
}

// ============================================================
// _r functions
// ============================================================

// ============================================================
// stdio internal functions
// ============================================================

include!("compat_misc.rs");
