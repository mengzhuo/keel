// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Misc functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn __assert_fail(expr: *const core::ffi::c_char, file: *const core::ffi::c_char, line: u32, func: *const core::ffi::c_char) {
    crate::write(2, b"Assertion failed: ".as_ptr() as *const core::ffi::c_void, 18);
    if !expr.is_null() { crate::write(2, expr as *const core::ffi::c_void, crate::strlen(expr)); }
    crate::write(2, b" (".as_ptr() as *const core::ffi::c_void, 2);
    if !file.is_null() { crate::write(2, file as *const core::ffi::c_void, crate::strlen(file)); }
    crate::write(2, b": ".as_ptr() as *const core::ffi::c_void, 2);
    let mut n = line as usize;
    let mut digits: [u8; 32] = [0; 32];
    let mut di = 0;
    if n == 0 { digits[0] = 0; di = 1; }
    while n > 0 && di < 32 { digits[di] = (n % 10) as u8; n /= 10; di += 1; }
    while di > 0 { di -= 1; let c = (b'0' + digits[di]) as c_char; crate::write(2, &c as *const c_char as *const core::ffi::c_void, 1); }
    crate::write(2, b")".as_ptr() as *const core::ffi::c_void, 1);
    if !func.is_null() { crate::write(2, func as *const core::ffi::c_void, crate::strlen(func)); }
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
    crate::abort()
}

const QUICK_EXIT_MAX: usize = 32;
static mut QUICK_EXIT_FNS: [Option<unsafe extern "C" fn()>; QUICK_EXIT_MAX] =
    [None; QUICK_EXIT_MAX];
static mut QUICK_EXIT_COUNT: usize = 0;

#[no_mangle]
pub unsafe extern "C" fn at_quick_exit(func: Option<unsafe extern "C" fn()>) -> i32 {
    let Some(func) = func else { return -1; };
    if QUICK_EXIT_COUNT >= QUICK_EXIT_MAX {
        return -1;
    }
    QUICK_EXIT_FNS[QUICK_EXIT_COUNT] = Some(func);
    QUICK_EXIT_COUNT += 1;
    0
}

#[no_mangle]
pub unsafe extern "C" fn brk(addr: *mut u8) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_BRK, addr as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn copy_file_range(fd_in: i32, off_in: *mut i64, fd_out: i32, off_out: *mut i64, len: usize, flags: u32) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall6(SYS_COPY_FILE_RANGE, fd_in as i64, off_in as i64, fd_out as i64, off_out as i64, len as i64, flags as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn ctermid(s: *mut core::ffi::c_char) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 20] = [0; 20];
    let dst = if s.is_null() { BUF.as_mut_ptr() } else { s };
    let src = b"/dev/tty\0".as_ptr() as *const core::ffi::c_char;
    let mut i = 0;
    while i < 9 { *dst.add(i) = *src.add(i); i += 1; }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn delete_module(name: *const core::ffi::c_char, flags: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_DELETE_MODULE, name as i64, flags as i64)) as i32
}





#[no_mangle]
pub unsafe extern "C" fn __duplocale(loc: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    loc
}

#[no_mangle]
pub unsafe extern "C" fn euidaccess(pathname: *const core::ffi::c_char, mode: i32) -> i32 {
    faccessat(AT_FDCWD, pathname, mode, AT_EACCESS)
}

#[no_mangle]
pub unsafe extern "C" fn fgetln(f: *mut core::ffi::c_void, lenp: *mut usize) -> *mut core::ffi::c_char {
    static mut BUF: *mut core::ffi::c_char = core::ptr::null_mut();
    static mut CAP: usize = 0;
    let n = getline(&raw mut BUF, &raw mut CAP, f as *mut FILE);
    if n <= 0 {
        if !lenp.is_null() { *lenp = 0; }
        return core::ptr::null_mut();
    }
    if !lenp.is_null() { *lenp = n as usize; }
    BUF
}

#[no_mangle]
pub unsafe extern "C" fn fgetws(ws: *mut u32, n: i32, f: *mut core::ffi::c_void) -> *mut u32 {
    let mut i = 0;
    while i < n - 1 {
        let c = getwc(f as *mut core::ffi::c_void);
        if c == 0 { break; }
        *ws.add(i as usize) = c;
        i += 1;
        if c == '\n' as u32 { break; }
    }
    *ws.add(i as usize) = 0;
    if i == 0 { core::ptr::null_mut() } else { ws }
}

#[no_mangle]
pub unsafe extern "C" fn fmtmsg(_classification: i64, label: *const core::ffi::c_char, _severity: i32, text: *const core::ffi::c_char, _action: *const core::ffi::c_char, _tag: *const core::ffi::c_char) -> i32 {
    if !label.is_null() { crate::write(2, label as *const core::ffi::c_void, crate::strlen(label)); }
    crate::write(2, b": ".as_ptr() as *const core::ffi::c_void, 2);
    if !text.is_null() { crate::write(2, text as *const core::ffi::c_void, crate::strlen(text)); }
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn ftime(tp: *mut core::ffi::c_void) -> i32 {
    let mut tv: timeval = core::mem::zeroed();
    if gettimeofday(&mut tv, core::ptr::null_mut()) == 0 {
        let tb = tp as *mut Timeb;
        (*tb).time = tv.tv_sec;
        (*tb).millitm = (tv.tv_usec / 1000) as u16;
        (*tb).timezone = 0;
        (*tb).dstflag = 0;
        0
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn getdents(fd: i32, dirp: *mut u8, count: usize) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_GETDENTS64, fd as i64, dirp as i64, count as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn gethostid() -> i64 {
    // Read from /proc/sys/kernel/random/boot_id or similar
    let fd = crate::open(b"/proc/sys/kernel/random/boot_id\0".as_ptr() as *const core::ffi::c_char, 0, 0);
    if fd < 0 { return 0; }
    let mut buf: [u8; 64] = [0; 64];
    let n = crate::read(fd, buf.as_mut_ptr() as *mut core::ffi::c_void, 63);
    crate::close(fd);
    if n <= 0 { return 0; }
    buf[n as usize] = 0;
    crate::strtol(buf.as_ptr() as *const core::ffi::c_char, core::ptr::null_mut(), 16)
}

#[repr(C)]
#[derive(Copy, Clone)]
struct TermiosBuf {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,
    c_line: u8,
    c_cc: [u8; 32],
    c_ispeed: u32,
    c_ospeed: u32,
}

#[no_mangle]
pub unsafe extern "C" fn getpass(prompt: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 256] = [0; 256];
    if prompt.is_null() { return core::ptr::null_mut(); }
    const O_NOCTTY_V: i32 = 0x100;
    const ECHO_V: u32 = 0o10;
    const ISIG_V: u32 = 0o1;
    const ICANON_V: u32 = 0o2;
    const INLCR_V: u32 = 0o100;
    const IGNCR_V: u32 = 0o200;
    const ICRNL_V: u32 = 0o400;
    const TCSAFLUSH_V: i32 = 2;

    let fd = crate::open(
        b"/dev/tty\0".as_ptr() as *const core::ffi::c_char,
        O_RDWR | O_NOCTTY_V | O_CLOEXEC,
        0,
    );
    if fd < 0 { return core::ptr::null_mut(); }
    let mut t: TermiosBuf = core::mem::zeroed();
    if tcgetattr(fd, &mut t as *mut TermiosBuf as *mut core::ffi::c_void) < 0 {
        crate::close(fd);
        return core::ptr::null_mut();
    }
    let old = t;
    t.c_lflag &= !(ECHO_V | ISIG_V);
    t.c_lflag |= ICANON_V;
    t.c_iflag &= !(INLCR_V | IGNCR_V);
    t.c_iflag |= ICRNL_V;
    tcsetattr(fd, TCSAFLUSH_V, &t as *const TermiosBuf as *const core::ffi::c_void);
    tcdrain(fd);
    dprintf(fd, b"%s\0".as_ptr() as *const core::ffi::c_char, prompt);
    let n = crate::read(fd, BUF.as_mut_ptr() as *mut core::ffi::c_void, BUF.len() - 1);
    let ok = n >= 0;
    if ok {
        let mut len = n as usize;
        if (len > 0 && BUF[len - 1] == b'\n' as core::ffi::c_char) || len == BUF.len() - 1 {
            len -= 1;
        }
        BUF[len] = 0;
    }
    tcsetattr(fd, TCSAFLUSH_V, &old as *const TermiosBuf as *const core::ffi::c_void);
    dprintf(fd, b"\n\0".as_ptr() as *const core::ffi::c_char);
    crate::close(fd);
    if ok { BUF.as_mut_ptr() } else { core::ptr::null_mut() }
}

#[no_mangle]
pub unsafe extern "C" fn getsubopt(optionp: *mut *mut core::ffi::c_char, tokens: *const *const core::ffi::c_char, valuep: *mut *mut core::ffi::c_char) -> i32 {
    let s = *optionp;
    if s.is_null() { return -1; }
    // Find end of suboption
    let mut end = s;
    while *end != 0 && *end != ',' as c_char && *end != '=' as c_char { end = end.add(1); }
    let eq = *end == '=' as c_char;
    if eq { *end = 0; *valuep = end.add(1); }
    // Find matching token
    let mut i = 0;
    loop {
        let tok = *tokens.add(i);
        if tok.is_null() { break; }
        if crate::strcmp(s as *const u8, tok as *const u8) == 0 {
            // Skip to next option
    while *end != 0 && *end != ',' as c_char { end = end.add(1); }
            if *end == ',' as c_char { end = end.add(1); }
            *optionp = end;
            return i as i32;
        }
        i += 1;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn getw(f: *mut core::ffi::c_void) -> i32 {
    let c0 = getc(f as *mut FILE);
    if c0 < 0 { return -1; }
    let c1 = getc(f as *mut FILE);
    if c1 < 0 { return -1; }
    let c2 = getc(f as *mut FILE);
    if c2 < 0 { return -1; }
    let c3 = getc(f as *mut FILE);
    if c3 < 0 { return -1; }
    (c0 | (c1 << 8) | (c2 << 16) | (c3 << 24)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn getwc(f: *mut core::ffi::c_void) -> u32 {
    getc(f as *mut FILE) as u32
}

#[no_mangle]
pub unsafe extern "C" fn init_module(module_image: *mut u8, len: usize, param_values: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_INIT_MODULE, module_image as i64, len as i64, param_values as i64)) as i32
}



#[no_mangle]
pub unsafe extern "C" fn issetugid() -> i32 {
    let ruid = crate::getuid();
    let euid = crate::geteuid();
    let rgid = crate::getgid();
    let egid = crate::getegid();
    (ruid != euid || rgid != egid) as i32
}

// ============================================================
// poll functions
// ============================================================

#[repr(C)]
struct PollTimespec {
    tv_sec: i64,
    tv_nsec: i64,
}

#[no_mangle]
pub unsafe extern "C" fn poll(fds: *mut core::ffi::c_void, nfds: u32, timeout: i32) -> i32 {
    #[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
    { syscall_ret(<Arch as Syscalls>::syscall3(SYS_POLL, fds as i64, nfds as i64, timeout as i64)) as i32 }
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
    {
        // poll(2) does not exist on these architectures.  ppoll(2) takes a
        // struct timespec, not a millisecond count; passing the integer
        // directly made poll(..., 0) wait forever.
        let mut ts = PollTimespec { tv_sec: 0, tv_nsec: 0 };
        let ts_ptr = if timeout < 0 {
            core::ptr::null()
        } else {
            ts.tv_sec = (timeout / 1000) as i64;
            ts.tv_nsec = ((timeout % 1000) as i64) * 1_000_000;
            &ts as *const PollTimespec as *const core::ffi::c_void
        };
        syscall_ret(<Arch as Syscalls>::syscall5(
            SYS_PPOLL,
            fds as i64,
            nfds as i64,
            ts_ptr as i64,
            0,
            8,
        )) as i32
    }
}

#[no_mangle]
pub unsafe extern "C" fn ppoll(fds: *mut core::ffi::c_void, nfds: u32, timeout: *const core::ffi::c_void, sigmask: *const u64) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_PPOLL, fds as i64, nfds as i64, timeout as i64, sigmask as i64, 8)) as i32
}

#[repr(C)]
struct PselectSigmask {
    ss: *const u64,
    ss_len: usize,
}

#[no_mangle]
pub unsafe extern "C" fn pselect(nfds: i32, readfds: *mut u8, writefds: *mut u8, exceptfds: *mut u8, timeout: *const core::ffi::c_void, sigmask: *const u64) -> i32 {
    // pselect6 takes a { sigset_t *, size_t } pair, not the sigset pointer.
    // GNU make blocks SIGCHLD and depends on this temporary mask, so passing
    // the pointer directly deadlocks parallel builds.
    let arg = PselectSigmask {
        ss: sigmask,
        ss_len: core::mem::size_of::<SigSetT>(),
    };
    let arg_ptr = if sigmask.is_null() {
        0
    } else {
        &arg as *const PselectSigmask as i64
    };
    syscall_ret(<Arch as Syscalls>::syscall6(SYS_PSELECT6, nfds as i64, readfds as i64, writefds as i64, exceptfds as i64, timeout as i64, arg_ptr)) as i32
}

// ============================================================
// realpath function
// ============================================================

// ============================================================
// quick_exit function
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn quick_exit(status: i32) -> ! {
    // Run handlers in reverse registration order, like atexit.
    while QUICK_EXIT_COUNT > 0 {
        QUICK_EXIT_COUNT -= 1;
        if let Some(func) = QUICK_EXIT_FNS[QUICK_EXIT_COUNT] {
            func();
        }
    }
    crate::_exit(status)
}

// ============================================================
// stat functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn lstat(path: *const core::ffi::c_char, buf: *mut core::ffi::c_void) -> i32 {
    let r = <Arch as Syscalls>::syscall4(SYS_NEWFSTATAT, AT_FDCWD as i64, path as i64, buf as i64, AT_SYMLINK_NOFOLLOW as i64);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn __xstat(_ver: i32, path: *const core::ffi::c_char, buf: *mut core::ffi::c_void) -> i32 {
    crate::stat(path, buf as *mut Stat)
}

#[no_mangle]
pub unsafe extern "C" fn __lxstat(_ver: i32, path: *const core::ffi::c_char, buf: *mut core::ffi::c_void) -> i32 {
    crate::lstat(path, buf)
}

#[no_mangle]
pub unsafe extern "C" fn __fxstat(_ver: i32, fd: i32, buf: *mut core::ffi::c_void) -> i32 {
    crate::fstat(fd, buf as *mut Stat)
}

#[no_mangle]
pub unsafe extern "C" fn __fxstatat(_ver: i32, dirfd: i32, path: *const core::ffi::c_char, buf: *mut core::ffi::c_void, flags: i32) -> i32 {
    crate::fstatat(dirfd, path, buf as *mut Stat, flags)
}

// ============================================================
// mempcpy function
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn mempcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    core::ptr::copy_nonoverlapping(src, dest, n);
    dest.add(n)
}

// ============================================================
// mktemp functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn mkostemp(template: *mut core::ffi::c_char, flags: i32) -> i32 {
    mkstemps_flags(template, 0, flags)
}

#[no_mangle]
pub unsafe extern "C" fn mkstemps(template: *mut core::ffi::c_char, suffixlen: i32) -> i32 {
    mkstemps_impl(template, suffixlen)
}

// ============================================================
// system functions
// ============================================================

const PTRACE_PEEKTEXT: i32 = 1;
const PTRACE_PEEKDATA: i32 = 2;
const PTRACE_PEEKUSER: i32 = 3;

#[no_mangle]
pub unsafe extern "C" fn ptrace(request: i32, mut args: ...) -> i64 {
    let pid = args.next_arg::<c_long>();
    let addr = args.next_arg::<c_long>();
    let data = args.next_arg::<c_long>();
    // PEEKTEXT/PEEKDATA/PEEKUSER write the result through the data pointer
    // (the kernel returns EFAULT for a NULL pointer), so supply a local slot.
    if (PTRACE_PEEKTEXT..=PTRACE_PEEKUSER).contains(&request) {
        let mut result: c_long = 0;
        let r = <Arch as Syscalls>::syscall4(
            SYS_PTRACE,
            request as i64,
            pid,
            addr,
            &mut result as *mut c_long as i64,
        );
        if r < 0 {
            ERRNO!() = (-r) as c_int;
            return -1;
        }
        return result;
    }
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_PTRACE, request as i64, pid, addr, data))
}

#[no_mangle]
pub unsafe extern "C" fn personality(persona: u64) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_PERSONALITY, persona as i64)
}

const LINUX_REBOOT_MAGIC1: i64 = 0xfee1dead;
const LINUX_REBOOT_MAGIC2: i64 = 0x28121969;

#[no_mangle]
pub unsafe extern "C" fn reboot(cmd: c_int) -> i32 {
    // musl's public API is reboot(cmd), not the four-argument syscall.
    syscall_ret(<Arch as Syscalls>::syscall3(
        SYS_REBOOT,
        LINUX_REBOOT_MAGIC1,
        LINUX_REBOOT_MAGIC2,
        cmd as i64,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn pivot_root(new_root: *const core::ffi::c_char, put_old: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_PIVOT_ROOT, new_root as i64, put_old as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mount(source: *const core::ffi::c_char, target: *const core::ffi::c_char, filesystemtype: *const core::ffi::c_char, mountflags: u64, data: *const core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_MOUNT, source as i64, target as i64, filesystemtype as i64, mountflags as i64, data as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn umount(target: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_UMOUNT2, target as i64, 0)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn umount2(target: *const core::ffi::c_char, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_UMOUNT2, target as i64, flags as i64)) as i32
}

// ============================================================
// resolver functions
// ============================================================


// ============================================================
// signal/error functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn psiginfo(_pinfo: *const core::ffi::c_void, s: *const core::ffi::c_char) {
    if !s.is_null() { crate::write(2, s as *const core::ffi::c_void, crate::strlen(s)); }
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
}

#[no_mangle]
pub unsafe extern "C" fn psignal(sig: i32, s: *const core::ffi::c_char) {
    if !s.is_null() { crate::write(2, s as *const core::ffi::c_void, crate::strlen(s)); }
    crate::write(2, b": ".as_ptr() as *const core::ffi::c_void, 2);
    let msg = strsignal(sig);
    crate::write(2, msg as *const core::ffi::c_void, crate::strlen(msg));
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
}

#[no_mangle]
pub unsafe extern "C" fn strsignal(sig: c_int) -> *const c_char {
    static NAMES: [&[u8]; 32] = [
        b"Unknown signal\0",
        b"Hangup\0",
        b"Interrupt\0",
        b"Quit\0",
        b"Illegal instruction\0",
        b"Trace/breakpoint trap\0",
        b"Aborted\0",
        b"Bus error\0",
        b"Floating point exception\0",
        b"Killed\0",
        b"User defined signal 1\0",
        b"Segmentation fault\0",
        b"User defined signal 2\0",
        b"Broken pipe\0",
        b"Alarm clock\0",
        b"Terminated\0",
        b"Stack fault\0",
        b"Child process status\0",
        b"Continued\0",
        b"Stopped (signal)\0",
        b"Stopped\0",
        b"Stopped (tty input)\0",
        b"Stopped (tty output)\0",
        b"Urgent I/O condition\0",
        b"CPU time limit exceeded\0",
        b"File size limit exceeded\0",
        b"Virtual timer expired\0",
        b"Profiling timer expired\0",
        b"Window changed\0",
        b"I/O possible\0",
        b"Power failure\0",
        b"Bad system call\0",
    ];
    static mut RT_BUF: [c_char; 32] = [0; 32];
    if sig > 0 && sig < 32 {
        return NAMES[sig as usize].as_ptr() as *const c_char;
    }
    if sig >= 32 && sig < 65 {
        let mut o = 0usize;
        for &c in b"Real-time signal \0" {
            if c == 0 { break; }
            RT_BUF[o] = c as c_char;
            o += 1;
        }
        let mut v = (sig - 32) as u32;
        let mut digits = [0u8; 10];
        let mut d = 0usize;
        if v == 0 { digits[0] = b'0'; d = 1; }
        while v > 0 { digits[d] = b'0' + (v % 10) as u8; v /= 10; d += 1; }
        while d > 0 { d -= 1; RT_BUF[o] = digits[d] as c_char; o += 1; }
        RT_BUF[o] = 0;
        return RT_BUF.as_ptr();
    }
    b"Unknown signal\0".as_ptr() as *const c_char
}

#[no_mangle]
pub unsafe extern "C" fn __strerror_l(errnum: i32, _loc: *mut core::ffi::c_void) -> *const core::ffi::c_char {
    crate::strerror(errnum)
}

// ============================================================
// preadv functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn preadv(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32, offset: i64) -> isize {
    // preadv takes a 64-bit offset split into pos_l/pos_h on x86_64 and the
    // generic ABI; passing only pos_l leaves the high half undefined.
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_PREADV,
        fd as i64,
        iov as i64,
        iovcnt as i64,
        offset,
        offset >> 32,
    )) as isize
}

#[no_mangle]
pub unsafe extern "C" fn pwritev(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32, offset: i64) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_PWRITEV,
        fd as i64,
        iov as i64,
        iovcnt as i64,
        offset,
        offset >> 32,
    )) as isize
}

#[no_mangle]
pub unsafe extern "C" fn preadv2(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32, offset: i64, flags: u32) -> isize {
    // x86_64/generic preadv2 split the 64-bit offset into pos_l/pos_h and
    // take the flags as the sixth argument.
    syscall_ret(<Arch as Syscalls>::syscall6(
        SYS_PREADV2,
        fd as i64,
        iov as i64,
        iovcnt as i64,
        offset,
        offset >> 32,
        flags as i64,
    )) as isize
}

#[no_mangle]
pub unsafe extern "C" fn pwritev2(fd: i32, iov: *const core::ffi::c_void, iovcnt: i32, offset: i64, flags: u32) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall6(
        SYS_PWRITEV2,
        fd as i64,
        iov as i64,
        iovcnt as i64,
        offset,
        offset >> 32,
        flags as i64,
    )) as isize
}

// ============================================================
// nice function
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn nice(inc: i32) -> i32 {
    let mut prio = inc;
    if inc > -40 && inc < 40 {
        prio += getpriority(0, 0);
    }
    if prio > 19 { prio = 19; }
    if prio < -20 { prio = -20; }
    if setpriority(0, 0, prio) != 0 {
        if ERRNO!() == EACCES { ERRNO!() = EPERM; } // EACCES -> EPERM, as musl does
        return -1;
    }
    prio
}

// ============================================================
// misc functions
// ============================================================

const A64L_DIGITS: &[u8; 64] =
    b"./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[no_mangle]
pub unsafe extern "C" fn l64a(value: i64) -> *mut core::ffi::c_char {
    static mut BUF: [core::ffi::c_char; 7] = [0; 7];
    let mut v = value as u32;
    let mut i = 0;
    while v > 0 && i < 6 {
        BUF[i] = A64L_DIGITS[(v & 0x3f) as usize] as core::ffi::c_char;
        v >>= 6;
        i += 1;
    }
    BUF[i] = 0;
    BUF.as_mut_ptr()
}

#[no_mangle]
pub unsafe extern "C" fn a64l(s: *const core::ffi::c_char) -> i64 {
    let mut x: u32 = 0;
    let mut e = 0u32;
    while e < 36 {
        let c = *s.add((e / 6) as usize) as u8;
        if c == 0 { break; }
        let d = match A64L_DIGITS.iter().position(|&d| d == c) {
            Some(d) => d as u32,
            None => break,
        };
        x |= d << e;
        e += 6;
    }
    x as i32 as i64
}

#[no_mangle]
pub unsafe extern "C" fn lchmod(path: *const core::ffi::c_char, mode: u32) -> i32 {
    fchmodat(AT_FDCWD, path, mode, AT_SYMLINK_NOFOLLOW)
}

#[no_mangle]
pub unsafe extern "C" fn lutimes(filename: *const core::ffi::c_char, times: *const core::ffi::c_void) -> i32 {
    utimensat(AT_FDCWD, filename, times as *const timespec, AT_SYMLINK_NOFOLLOW)
}

#[no_mangle]
pub unsafe extern "C" fn mknod(pathname: *const core::ffi::c_char, mode: u32, dev: u64) -> i32 {
    #[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
    { syscall_ret(<Arch as Syscalls>::syscall3(SYS_MKNOD, pathname as i64, mode as i64, dev as i64)) as i32 }
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
    {
        syscall_ret(<Arch as Syscalls>::syscall4(SYS_MKNODAT, AT_FDCWD as i64, pathname as i64, mode as i64, dev as i64)) as i32
    }
}

#[no_mangle]
pub unsafe extern "C" fn memfd_create(name: *const core::ffi::c_char, flags: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_MEMFD_CREATE, name as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mlock2(addr: *const core::ffi::c_void, len: usize, flags: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MLOCK2, addr as i64, len as i64, flags as i64)) as i32
}



#[no_mangle]
pub unsafe extern "C" fn remap_file_pages(addr: *mut core::ffi::c_void, size: usize, prot: i32, pgoff: usize, flags: u32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_REMAP_FILE_PAGES, addr as i64, size as i64, prot as i64, pgoff as i64, flags as i64)) as i32
}

// ============================================================
// nan functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn nan(_tagp: *const core::ffi::c_char) -> f64 {
    f64::NAN
}

#[no_mangle]
pub unsafe extern "C" fn nanf(_tagp: *const core::ffi::c_char) -> f32 {
    f32::NAN
}

include!("compat_symbols.rs");
// ============================================================
// Final missing symbols
// ============================================================



#[no_mangle]
pub unsafe extern "C" fn strtod_l(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, _loc: *mut core::ffi::c_void) -> f64 {
    crate::strtod(nptr, endptr)
}

#[no_mangle]
pub unsafe extern "C" fn strtof_l(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, _loc: *mut core::ffi::c_void) -> f32 {
    crate::strtof(nptr, endptr)
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
pub unsafe extern "C" fn strtold_l(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, _loc: *mut core::ffi::c_void) -> f64 {
    crate::strtod(nptr, endptr)
}

#[cfg(not(target_arch = "x86_64"))]
#[no_mangle]
pub unsafe extern "C" fn strtold_l(nptr: *const core::ffi::c_char, endptr: *mut *mut core::ffi::c_char, _loc: *mut core::ffi::c_void) -> f128 {
    f64_to_f128(crate::strtod(nptr, endptr))
}

// ============================================================
// Directory operations - real implementations
// ============================================================

#[repr(C)]
struct linux_dirent64 {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    d_name: [u8; 0], // flexible array
}

// Directory streams used to be a process-wide singleton, which broke any
// nested opendir/readdir sequence (git, find, glob, ...). Each DIR now owns
// its fd and read buffer, like musl's __dirstream.
const DIR_BUF_SIZE: usize = 4096;

#[repr(C)]
struct DirStream {
    fd: i32,
    pos: usize,
    len: usize,
    last_off: i64,
    buf: [u8; DIR_BUF_SIZE],
}

unsafe fn dir_stream_new(fd: i32) -> *mut core::ffi::c_void {
    let ds = malloc(core::mem::size_of::<DirStream>()) as *mut DirStream;
    if ds.is_null() {
        crate::close(fd);
        ERRNO!() = ENOMEM;
        return core::ptr::null_mut();
    }
    core::ptr::write_bytes(ds, 0, 1);
    (*ds).fd = fd;
    ds as *mut core::ffi::c_void
}

#[no_mangle]
pub unsafe extern "C" fn opendir(dirname: *const core::ffi::c_char) -> *mut core::ffi::c_void {
    let fd = openat(AT_FDCWD, dirname, O_DIRECTORY, 0); // O_RDONLY | O_DIRECTORY
    if fd < 0 { return core::ptr::null_mut(); }
    dir_stream_new(fd)
}

#[no_mangle]
pub unsafe extern "C" fn closedir(dirp: *mut core::ffi::c_void) -> i32 {
    if dirp.is_null() { ERRNO!() = EINVAL; return -1; }
    let ds = dirp as *mut DirStream;
    let r = crate::close((*ds).fd);
    free(dirp);
    r
}

#[no_mangle]
pub unsafe extern "C" fn readdir(dirp: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    if dirp.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let ds = &mut *(dirp as *mut DirStream);

    loop {
        if ds.pos >= ds.len {
            let nread = syscall_ret(<Arch as Syscalls>::syscall3(
                SYS_GETDENTS64,
                ds.fd as i64,
                ds.buf.as_mut_ptr() as i64,
                DIR_BUF_SIZE as i64,
            )) as isize;
            if nread <= 0 { return core::ptr::null_mut(); }
            ds.len = nread as usize;
            ds.pos = 0;
        }

        let ent = ds.buf.as_ptr().add(ds.pos) as *const linux_dirent64;
        let ent_ref = &*ent;
        if ent_ref.d_reclen as usize == 0 {
            ds.len = 0;
            return core::ptr::null_mut();
        }
        ds.pos += ent_ref.d_reclen as usize;
        ds.last_off = ent_ref.d_off;

        // Return a pointer into this stream's buffer (valid until next call).
        return ent as *mut core::ffi::c_void;
    }
}

#[repr(C)]
struct DirentCopy {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
    d_name: [core::ffi::c_char; 256],
}

#[no_mangle]
pub unsafe extern "C" fn readdir_r(dirp: *mut core::ffi::c_void, entry: *mut core::ffi::c_void, result: *mut *mut core::ffi::c_void) -> i32 {
    if dirp.is_null() || entry.is_null() || result.is_null() {
        return EINVAL;
    }
    let ent = readdir(dirp);
    if ent.is_null() {
        *result = core::ptr::null_mut();
        return 0;
    }
    let reclen = (* (ent as *const linux_dirent64)).d_reclen as usize;
    let copy = reclen.min(core::mem::size_of::<DirentCopy>());
    core::ptr::copy_nonoverlapping(ent as *const u8, entry as *mut u8, copy);
    *result = entry;
    0
}

#[no_mangle]
pub unsafe extern "C" fn seekdir(dirp: *mut core::ffi::c_void, loc: i64) {
    if dirp.is_null() { return; }
    let ds = &mut *(dirp as *mut DirStream);
    <Arch as Syscalls>::syscall3(SYS_LSEEK, ds.fd as i64, loc, 0);
    ds.pos = 0;
    ds.len = 0;
}

#[no_mangle]
pub unsafe extern "C" fn telldir(dirp: *mut core::ffi::c_void) -> i64 {
    if dirp.is_null() { return -1; }
    (*(dirp as *mut DirStream)).last_off
}

#[no_mangle]
pub unsafe extern "C" fn rewinddir(dirp: *mut core::ffi::c_void) {
    if dirp.is_null() { return; }
    let ds = &mut *(dirp as *mut DirStream);
    <Arch as Syscalls>::syscall3(SYS_LSEEK, ds.fd as i64, 0, 0);
    ds.pos = 0;
    ds.len = 0;
    ds.last_off = 0;
}

#[no_mangle]
pub unsafe extern "C" fn dirfd(dirp: *mut core::ffi::c_void) -> i32 {
    if dirp.is_null() { ERRNO!() = EINVAL; return -1; }
    (*(dirp as *mut DirStream)).fd
}

// ============================================================
// Memory management - real implementations
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MPROTECT, addr as i64, len as i64, prot as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn msync(addr: *mut u8, len: usize, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MSYNC, addr as i64, len as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mlock(addr: *const u8, len: usize) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_MLOCK, addr as i64, len as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn munlock(addr: *const u8, len: usize) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_MUNLOCK, addr as i64, len as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mlockall(flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_MLOCKALL, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn munlockall() -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall0(SYS_MUNLOCKALL)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn madvise(addr: *mut u8, len: usize, advice: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MADVISE, addr as i64, len as i64, advice as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mincore(addr: *mut u8, len: usize, vec: *mut u8) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_MINCORE, addr as i64, len as i64, vec as i64)) as i32
}

// ============================================================

// ============================================================
// Missing math functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn drem(x: f64, y: f64) -> f64 {
    remainder(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn dremf(x: f32, y: f32) -> f32 {
    remainderf(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn pow10(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn pow10f(x: f32) -> f32 {
    libm::powf(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn pow10l(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn isless(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn lgamma_r(x: f64, signgamp: *mut i32) -> f64 {
    lgamma_r_impl(x, signgamp)
}

#[no_mangle]
pub unsafe extern "C" fn lgammaf_r(x: f32, signgamp: *mut i32) -> f32 {
    if x.is_nan() {
        if !signgamp.is_null() { *signgamp = 1; }
        return x;
    }
    if x == 0.0 {
        if !signgamp.is_null() { *signgamp = if x.is_sign_negative() { -1 } else { 1 }; }
        feraiseexcept(FE_DIVBYZERO);
        return f32::INFINITY;
    }
    let (r, sign) = libm::lgammaf_r(x);
    if !signgamp.is_null() { *signgamp = sign; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn lgammal_r(x: f64, signgamp: *mut i32) -> f64 {
    lgamma_r(x, signgamp)
}

// ============================================================
// Comparison macros
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn islessequal(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x <= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessequalf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x <= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessgreater(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y || x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn islessgreaterf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x < y || x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreater(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x > y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterequal(x: f64, y: f64) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x >= y) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn isgreaterequalf(x: f32, y: f32) -> i32 {
    if x.is_nan() || y.is_nan() { 0 } else { (x >= y) as i32 }
}

include!("compat_abi.rs");
include!("resolver.rs");
include!("legacy_compat.rs");
include!("shadow.rs");
