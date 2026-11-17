// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Thread sync functions (C11 threads)
// ============================================================


// C11 thread types
pub type thrd_start_t = Option<unsafe extern "C" fn(arg: *mut core::ffi::c_void) -> i32>;

// C11 threads: thin wrappers around the pthread implementation.
#[no_mangle]
pub unsafe extern "C" fn thrd_create(thr: *mut usize, func: thrd_start_t, arg: *mut core::ffi::c_void) -> i32 {
    let mut thread: u64 = 0;
    let r = pthread_create(&mut thread, core::ptr::null(), core::mem::transmute(func), arg);
    if r == 0 {
        *thr = thread as usize;
        0 // thrd_success
    } else {
        -1 // thrd_nomem
    }
}

#[no_mangle]
pub unsafe extern "C" fn thrd_join(thr: usize, res: *mut i32) -> i32 {
    let mut retval: *mut core::ffi::c_void = core::ptr::null_mut();
    let r = pthread_join(thr as u64, &mut retval);
    if r == 0 {
        if !res.is_null() { *res = retval as i32; }
        0
    } else {
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn thrd_exit(res: i32) {
    pthread_exit(res as *mut core::ffi::c_void);
}

#[no_mangle]
pub unsafe extern "C" fn thrd_detach(thr: usize) -> i32 {
    let r = pthread_detach(thr as u64);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn thrd_current() -> usize {
    pthread_self() as usize
}

#[no_mangle]
pub unsafe extern "C" fn thrd_equal(thr1: usize, thr2: usize) -> i32 {
    (thr1 == thr2) as i32
}

#[no_mangle]
pub unsafe extern "C" fn thrd_sleep(duration: *const core::ffi::c_void, remaining: *mut core::ffi::c_void) -> i32 {
    nanosleep(duration as *const timespec, remaining as *mut timespec)
}

#[no_mangle]
pub unsafe extern "C" fn thrd_yield() {
    sched_yield();
}

// Mutex
#[no_mangle]
pub unsafe extern "C" fn mtx_init(mtx: *mut usize, _mtx_type: i32) -> i32 {
    let r = pthread_mutex_init(mtx as *mut pthread_mutex_t, core::ptr::null());
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn mtx_lock(mtx: *mut usize) -> i32 {
    let r = pthread_mutex_lock(mtx as *mut pthread_mutex_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn mtx_trylock(mtx: *mut usize) -> i32 {
    let r = pthread_mutex_trylock(mtx as *mut pthread_mutex_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn mtx_unlock(mtx: *mut usize) -> i32 {
    let r = pthread_mutex_unlock(mtx as *mut pthread_mutex_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn mtx_destroy(mtx: *mut usize) {
    pthread_mutex_destroy(mtx as *mut pthread_mutex_t);
}

#[no_mangle]
pub unsafe extern "C" fn mtx_timedlock(mtx: *mut usize, ts: *const core::ffi::c_void) -> i32 {
    let r = pthread_mutex_timedlock(mtx as *mut pthread_mutex_t, ts as *const timespec);
    if r == 0 { 0 } else { -1 }
}

// Condition variable
#[no_mangle]
pub unsafe extern "C" fn cnd_init(cnd: *mut usize) -> i32 {
    let r = pthread_cond_init(cnd as *mut pthread_cond_t, core::ptr::null());
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn cnd_signal(cnd: *mut usize) -> i32 {
    let r = pthread_cond_signal(cnd as *mut pthread_cond_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn cnd_broadcast(cnd: *mut usize) -> i32 {
    let r = pthread_cond_broadcast(cnd as *mut pthread_cond_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn cnd_wait(cnd: *mut usize, mtx: *mut usize) -> i32 {
    let r = pthread_cond_wait(cnd as *mut pthread_cond_t, mtx as *mut pthread_mutex_t);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn cnd_timedwait(cnd: *mut usize, mtx: *mut usize, ts: *const core::ffi::c_void) -> i32 {
    let r = pthread_cond_timedwait(cnd as *mut pthread_cond_t, mtx as *mut pthread_mutex_t, ts as *const timespec);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn cnd_destroy(cnd: *mut usize) {
    pthread_cond_destroy(cnd as *mut pthread_cond_t);
}

// Thread-specific storage
#[no_mangle]
pub unsafe extern "C" fn tss_create(key: *mut u32, dtor: Option<unsafe extern "C" fn(*mut core::ffi::c_void)>) -> i32 {
    let r = pthread_key_create(key, dtor);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn tss_delete(key: u32) {
    pthread_key_delete(key);
}

#[no_mangle]
pub unsafe extern "C" fn tss_set(key: u32, val: *mut core::ffi::c_void) -> i32 {
    let r = pthread_setspecific(key, val);
    if r == 0 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn tss_get(key: u32) -> *mut core::ffi::c_void {
    pthread_getspecific(key)
}

// call_once
#[no_mangle]
pub unsafe extern "C" fn call_once(flag: *mut u32, func: Option<unsafe extern "C" fn()>) {
    if flag.is_null() { return; }
    let state = &*(flag as *const AtomicU32);
    if state.load(Ordering::Acquire) == 2 { return; }
    if state.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire).is_ok() {
        if let Some(f) = func { f(); }
        state.store(2, Ordering::Release);
    } else {
        while state.load(Ordering::Acquire) != 2 {
            core::hint::spin_loop();
        }
    }
}

// ============================================================
// Network functions
// ============================================================

// accept4 - Linux-specific extension
#[no_mangle]
pub unsafe extern "C" fn accept4(fd: i32, addr: *mut core::ffi::c_void, addrlen: *mut u32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_ACCEPT4, fd as i64, addr as i64, addrlen as i64, flags as i64)) as i32
}

// getifaddrs / freeifaddrs
// ============================================================
// epoll/eventfd/signalfd/timerfd
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn epoll_create(size: i32) -> i32 {
    #[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
    { syscall_ret(<Arch as Syscalls>::syscall1(SYS_EPOLL_CREATE, size as i64)) as i32 }
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
    {
        if size < 1 {
            ERRNO!() = EINVAL;
            return -1;
        }
        syscall_ret(<Arch as Syscalls>::syscall1(SYS_EPOLL_CREATE1, 0)) as i32
    }
}

#[no_mangle]
pub unsafe extern "C" fn epoll_create1(flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_EPOLL_CREATE1, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_EPOLL_CTL, epfd as i64, op as i64, fd as i64, event as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn epoll_wait(epfd: i32, events: *mut core::ffi::c_void, maxevents: i32, timeout: i32) -> i32 {
    #[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
    {
        syscall_ret(<Arch as Syscalls>::syscall4(
            SYS_EPOLL_WAIT,
            epfd as i64,
            events as i64,
            maxevents as i64,
            timeout as i64,
        )) as i32
    }
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
    {
        epoll_pwait(epfd, events, maxevents, timeout, core::ptr::null())
    }
}

#[no_mangle]
pub unsafe extern "C" fn epoll_pwait(
    epfd: i32,
    events: *mut core::ffi::c_void,
    maxevents: i32,
    timeout: i32,
    sigmask: *const u64,
) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall6(
        SYS_EPOLL_PWAIT,
        epfd as i64,
        events as i64,
        maxevents as i64,
        timeout as i64,
        sigmask as i64,
        8,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn eventfd(initval: u32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_EVENTFD2, initval as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn eventfd_read(fd: i32, value: *mut u64) -> i32 {
    let n = crate::read(fd, value as *mut core::ffi::c_void, 8);
    if n == 8 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn eventfd_write(fd: i32, value: u64) -> i32 {
    let n = crate::write(fd, &value as *const u64 as *const core::ffi::c_void, 8);
    if n == 8 { 0 } else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn signalfd(fd: i32, mask: *const u64, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_SIGNALFD4, fd as i64, mask as i64, 8, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timerfd_create(clockid: i32, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_TIMERFD_CREATE, clockid as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timerfd_settime(fd: i32, flags: i32, new_value: *const core::ffi::c_void, old_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_TIMERFD_SETTIME, fd as i64, flags as i64, new_value as i64, old_value as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn timerfd_gettime(fd: i32, curr_value: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_TIMERFD_GETTIME, fd as i64, curr_value as i64)) as i32
}

// ============================================================
// System functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn prctl(option: i32, mut args: ...) -> i32 {
    let a1 = args.next_arg::<i64>();
    let a2 = args.next_arg::<i64>();
    let a3 = args.next_arg::<i64>();
    let a4 = args.next_arg::<i64>();
    syscall_ret(<Arch as Syscalls>::syscall5(SYS_PRCTL, option as i64, a1, a2, a3, a4)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn quotactl(cmd: i32, special: *const core::ffi::c_char, id: i32, addr: *mut core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_QUOTACTL, cmd as i64, special as i64, id as i64, addr as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn klogctl(type_: i32, bufp: *mut core::ffi::c_char, len: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SYSLOG, type_ as i64, bufp as i64, len as i64)) as i32
}

static mut SYSLOG_IDENT: [core::ffi::c_char; 64] = [0; 64];
static mut SYSLOG_OPTIONS: i32 = 0;
static mut SYSLOG_FACILITY: i32 = 1;
const SYSLOG_FACILITY_MASK: i32 = 0x3f8;

#[no_mangle]
pub unsafe extern "C" fn openlog(ident: *const core::ffi::c_char, option: i32, facility: i32) {
    SYSLOG_OPTIONS = option;
    SYSLOG_FACILITY = facility & SYSLOG_FACILITY_MASK;
    SYSLOG_IDENT[0] = 0;
    if !ident.is_null() {
        let mut i = 0;
        while i < 63 && *ident.add(i) != 0 {
            SYSLOG_IDENT[i] = *ident.add(i);
            i += 1;
        }
        SYSLOG_IDENT[i] = 0;
    }
}

#[repr(C)]
struct SockAddrUn {
    sun_family: u16,
    sun_path: [core::ffi::c_char; 108],
}

unsafe fn syslog_emit(priority: i32, msg: *const core::ffi::c_char, len: usize) {
    let mut out = [0u8; 1200];
    let mut o = 0usize;
    let pri = (SYSLOG_FACILITY | (priority & 7)) as u32;
    out[o] = b'<'; o += 1;
    if pri >= 100 { out[o] = b'0' + (pri / 100) as u8; o += 1; }
    if pri >= 10 { out[o] = b'0' + ((pri / 10) % 10) as u8; o += 1; }
    out[o] = b'0' + (pri % 10) as u8; o += 1;
    out[o] = b'>'; o += 1;
    let mut i = 0;
    while i < 63 && SYSLOG_IDENT[i] != 0 && o + 1 < out.len() {
        out[o] = SYSLOG_IDENT[i] as u8; o += 1; i += 1;
    }
    if SYSLOG_OPTIONS & 1 != 0 {
        out[o] = b'['; o += 1;
        let mut pid = crate::getpid() as u32;
        let mut digits = [0u8; 10];
        let mut d = 0;
        if pid == 0 { digits[0] = b'0'; d = 1; }
        while pid > 0 { digits[d] = b'0' + (pid % 10) as u8; pid /= 10; d += 1; }
        while d > 0 { d -= 1; out[o] = digits[d]; o += 1; }
        out[o] = b']'; o += 1;
    }
    out[o] = b':'; o += 1; out[o] = b' '; o += 1;
    let copy = core::cmp::min(len, out.len() - o - 1);
    core::ptr::copy_nonoverlapping(msg as *const u8, out.as_mut_ptr().add(o), copy);
    o += copy;
    let fd = crate::socket(AF_UNIX, SOCK_DGRAM, 0);
    if fd >= 0 {
        let mut sa: SockAddrUn = core::mem::zeroed();
        sa.sun_family = AF_UNIX as u16;
        let path = b"/dev/log";
        for i in 0..path.len() { sa.sun_path[i] = path[i] as core::ffi::c_char; }
        let r = crate::sendto(fd, out.as_ptr() as *const core::ffi::c_void, o, 0, &sa as *const SockAddrUn as *const crate::sockaddr, core::mem::size_of::<SockAddrUn>() as u32);
        crate::close(fd);
        if r >= 0 { return; }
    }
    crate::write(2, out.as_ptr() as *const core::ffi::c_void, o);
    crate::write(2, b"\n".as_ptr() as *const core::ffi::c_void, 1);
}

#[no_mangle]
pub unsafe extern "C" fn vsyslog(priority: c_int, fmt: *const c_char, ap: VaList) {
    let mut msg = [0 as c_char; 1024];
    let n = vsnprintf(msg.as_mut_ptr(), msg.len(), fmt, ap);
    if n > 0 { syslog_emit(priority, msg.as_ptr(), core::cmp::min(n as usize, msg.len() - 1)); }
}

#[no_mangle]
pub unsafe extern "C" fn syslog(priority: c_int, fmt: *const c_char, args: ...) {
    vsyslog(priority, fmt, args);
}

#[no_mangle]
pub unsafe extern "C" fn closelog() {
    // Nothing to do
}

#[no_mangle]
pub unsafe extern "C" fn setlogmask(mask: i32) -> i32 {
    static mut LOG_MASK: i32 = 0x01 | 0x02 | 0x04 | 0x08 | 0x10 | 0x20 | 0x40 | 0x80;
    let old = LOG_MASK;
    if mask != 0 { LOG_MASK = mask; }
    old
}

#[no_mangle]
pub unsafe extern "C" fn acct(filename: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_ACCT, filename as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn swapon(path: *const core::ffi::c_char, flags: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SWAPON, path as i64, flags as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn swapoff(path: *const core::ffi::c_char) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_SWAPOFF, path as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn syncfs(fd: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_SYNCFS, fd as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sync_file_range(fd: i32, offset: i64, nbytes: u64, flags: u32) -> i32 {
    #[cfg(target_arch = "powerpc64")]
    { syscall_ret(<Arch as Syscalls>::syscall4(SYS_SYNC_FILE_RANGE2, fd as i64, flags as i64, offset, nbytes as i64)) as i32 }
    #[cfg(not(target_arch = "powerpc64"))]
    { syscall_ret(<Arch as Syscalls>::syscall4(SYS_SYNC_FILE_RANGE, fd as i64, offset, nbytes as i64, flags as i64)) as i32 }
}

#[no_mangle]
pub unsafe extern "C" fn posix_fadvise(fd: i32, offset: i64, len: u64, advice: i32) -> i32 {
    // POSIX returns the error number directly, not -1/errno.
    let r = <Arch as Syscalls>::syscall4(SYS_FADVISE64, fd as i64, offset, len as i64, advice as i64);
    if r < 0 { (-r) as i32 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn posix_fallocate(fd: i32, offset: i64, len: u64) -> i32 {
    let r = <Arch as Syscalls>::syscall3(SYS_FALLOCATE, fd as i64, offset, len as i64);
    if r < 0 { (-r) as i32 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn readahead(fd: i32, offset: i64, count: usize) -> isize {
    // musl declares readahead() as ssize_t; returning int leaves the upper
    // half of rax undefined for callers that expect a 64-bit result.
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_READAHEAD, fd as i64, offset, count as i64)) as isize
}

#[no_mangle]
pub unsafe extern "C" fn fallocate(fd: i32, mode: i32, offset: i64, len: u64) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall4(SYS_FALLOCATE, fd as i64, mode as i64, offset, len as i64)) as i32
}

// ============================================================
// Terminal functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn tcgetattr(fd: i32, termios_p: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5401, termios_p as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const core::ffi::c_void) -> i32 {
    let cmd = match optional_actions {
        0 => 0x5402,
        1 => 0x5403,
        2 => 0x5404,
        _ => 0x5402,
    };
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, cmd, termios_p as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcdrain(fd: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5409, 1)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcflush(fd: i32, queue_selector: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x540B, queue_selector as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcflow(fd: i32, action: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x540A, action as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcsendbreak(fd: i32, duration: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5425, duration as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn tcgetsid(fd: i32) -> i32 {
    let mut sid: i32 = 0;
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5429, &mut sid as *mut i32 as i64);
    if r < 0 { -1 } else { sid }
}

#[no_mangle]
pub unsafe extern "C" fn tcgetpgrp(fd: i32) -> i32 {
    let mut pgrp: i32 = 0;
    let r = <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x540F, &mut pgrp as *mut i32 as i64);
    if r < 0 { -1 } else { pgrp }
}

#[no_mangle]
pub unsafe extern "C" fn tcsetpgrp(fd: i32, pgrp: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, 0x5410, &pgrp as *const i32 as i64)) as i32
}

// ============================================================
// Memory locking functions
// ============================================================

// ============================================================

// ============================================================
// POSIX message queue functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn mq_open(name: *const core::ffi::c_char, oflag: i32, mut args: ...) -> i32 {
    if name.is_null() { ERRNO!() = EINVAL; return -1; }
    let mut p = name as *const u8;
    if *p == b'/' { p = p.add(1); }
    let mut mode: u32 = 0;
    let mut attr: *const core::ffi::c_void = core::ptr::null();
    if oflag & 0o100 != 0 {
        mode = args.next_arg::<u32>();
        attr = args.next_arg::<*const core::ffi::c_void>();
    }
    syscall_ret(<Arch as Syscalls>::syscall4(
        SYS_MQ_OPEN,
        p as i64,
        oflag as i64,
        mode as i64,
        attr as i64,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mq_close(mqdes: i32) -> i32 {
    crate::close(mqdes as i32)
}

#[no_mangle]
pub unsafe extern "C" fn mq_unlink(name: *const core::ffi::c_char) -> i32 {
    if name.is_null() { ERRNO!() = EINVAL; return -1; }
    let mut p = name as *const u8;
    if *p == b'/' { p = p.add(1); }
    let r = <Arch as Syscalls>::syscall1(SYS_MQ_UNLINK, p as i64);
    if r < 0 {
        // The kernel reports EPERM for an unlink while the queue is open.
        let e = if (-r) as c_int == 1 { 13 } else { (-r) as c_int };
        ERRNO!() = e;
        -1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn mq_send(mqdes: i32, msg_ptr: *const core::ffi::c_char, msg_len: usize, msg_prio: u32) -> i32 {
    mq_timedsend(mqdes, msg_ptr, msg_len, msg_prio, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn mq_receive(mqdes: i32, msg_ptr: *mut core::ffi::c_char, msg_len: usize, msg_prio: *mut u32) -> isize {
    mq_timedreceive(mqdes, msg_ptr, msg_len, msg_prio, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn mq_timedsend(
    mqdes: i32,
    msg_ptr: *const core::ffi::c_char,
    msg_len: usize,
    msg_prio: u32,
    abs_timeout: *const core::ffi::c_void,
) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_MQ_TIMEDSEND,
        mqdes as i64,
        msg_ptr as i64,
        msg_len as i64,
        msg_prio as i64,
        abs_timeout as i64,
    )) as i32
}

#[no_mangle]
pub unsafe extern "C" fn mq_timedreceive(
    mqdes: i32,
    msg_ptr: *mut core::ffi::c_char,
    msg_len: usize,
    msg_prio: *mut u32,
    abs_timeout: *const core::ffi::c_void,
) -> isize {
    syscall_ret(<Arch as Syscalls>::syscall5(
        SYS_MQ_TIMEDRECEIVE,
        mqdes as i64,
        msg_ptr as i64,
        msg_len as i64,
        msg_prio as i64,
        abs_timeout as i64,
    )) as isize
}

#[repr(C)]
pub struct SigEventFull {
    sigev_value: usize,
    sigev_signo: i32,
    sigev_notify: i32,
    sigev_function: usize,
    sigev_attributes: usize,
    sigev_pad: [u8; 32],
}

struct MqNotifyArgs {
    sem: sem_t,
    sock: i32,
    mqd: i32,
    err: i32,
    func: usize,
    value: usize,
}

unsafe extern "C" fn mq_notify_thread(arg: *mut c_void) -> *mut c_void {
    let a = &mut *(arg as *mut MqNotifyArgs);
    let sock = a.sock;
    let mqd = a.mqd;
    let func = a.func;
    let value = a.value;
    let zeros = [0u8; 32];
    let sev2 = SigEventFull {
        sigev_value: zeros.as_ptr() as usize,
        sigev_signo: sock,
        sigev_notify: 2, // SIGEV_THREAD, consumed by the kernel as a netlink fd
        sigev_function: 0,
        sigev_attributes: 0,
        sigev_pad: [0; 32],
    };
    let r = <Arch as Syscalls>::syscall2(SYS_MQ_NOTIFY, mqd as i64, &sev2 as *const SigEventFull as i64);
    a.err = if r < 0 { (-r) as i32 } else { 0 };
    sem_post(&mut a.sem);
    if a.err != 0 { return core::ptr::null_mut(); }
    pthread_detach(pthread_self());
    let mut buf = [0u8; 32];
    let n = recv(sock, buf.as_mut_ptr() as *mut c_void, buf.len(), 0x4000 | 0x100);
    crate::close(sock);
    if n as usize == buf.len() && buf[buf.len() - 1] == 1 && func != 0 {
        let f: extern "C" fn(usize) = core::mem::transmute(func);
        f(value);
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn mq_notify(mqdes: i32, sev: *const core::ffi::c_void) -> i32 {
    if sev.is_null() {
        return syscall_ret(<Arch as Syscalls>::syscall2(SYS_MQ_NOTIFY, mqdes as i64, 0)) as i32;
    }
    let ev = &*(sev as *const SigEventFull);
    if ev.sigev_notify != 2 {
        return syscall_ret(<Arch as Syscalls>::syscall2(SYS_MQ_NOTIFY, mqdes as i64, sev as i64)) as i32;
    }

    // SIGEV_THREAD is emulated with a netlink socket and a helper thread,
    // matching musl. The kernel only deals with the socket.
    let sock = socket(16 /* AF_NETLINK */, 3 | 0x80000 /* SOCK_RAW|SOCK_CLOEXEC */, 0);
    if sock < 0 { return -1; }
    let mut a = MqNotifyArgs {
        sem: core::mem::zeroed(),
        sock,
        mqd: mqdes,
        err: 0,
        func: ev.sigev_function,
        value: ev.sigev_value,
    };
    sem_init(&mut a.sem, 0, 0);

    let mut attr: pthread_attr_t = core::mem::zeroed();
    if ev.sigev_attributes != 0 {
        attr = core::ptr::read(ev.sigev_attributes as *const pthread_attr_t);
    } else {
        pthread_attr_init(&mut attr);
    }
    pthread_attr_setdetachstate(&mut attr, PTHREAD_CREATE_JOINABLE);

    let mut all: SigSetT = 0;
    sigfillset(&mut all);
    let mut orig: SigSetT = 0;
    pthread_sigmask(0 /* SIG_BLOCK */, &all, &mut orig);

    let mut td: PthreadT = 0;
    let r = pthread_create(&mut td, &attr, mq_notify_thread as *const () as usize, &mut a as *mut MqNotifyArgs as *mut c_void);
    pthread_sigmask(2 /* SIG_SETMASK */, &orig, core::ptr::null_mut());
    if r != 0 {
        crate::close(sock);
        ERRNO!() = EAGAIN;
        return -1;
    }
    sem_wait(&mut a.sem);
    sem_destroy(&mut a.sem);
    if a.err != 0 {
        crate::close(sock);
        pthread_join(td, core::ptr::null_mut());
        ERRNO!() = a.err;
        return -1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn mq_getattr(mqdes: i32, attr: *mut core::ffi::c_void) -> i32 {
    mq_setattr(mqdes, core::ptr::null(), attr)
}

#[no_mangle]
pub unsafe extern "C" fn mq_setattr(
    mqdes: i32,
    new: *const core::ffi::c_void,
    old: *mut core::ffi::c_void,
) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(
        SYS_MQ_GETSETATTR,
        mqdes as i64,
        new as i64,
        old as i64,
    )) as i32
}

// ============================================================
// pthread functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn pthread_getaffinity_np(thread: usize, cpusetsize: usize, cpuset: *mut u8) -> i32 {
    sched_getaffinity(thread as i32, cpusetsize, cpuset)
}

#[no_mangle]
pub unsafe extern "C" fn pthread_setaffinity_np(thread: usize, cpusetsize: usize, cpuset: *const u8) -> i32 {
    sched_setaffinity(thread as i32, cpusetsize, cpuset)
}

// ============================================================
// sched functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn sched_yield() -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall0(SYS_SCHED_YIELD)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_get_priority_max(policy: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_SCHED_GET_PRIORITY_MAX, policy as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_get_priority_min(policy: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_SCHED_GET_PRIORITY_MIN, policy as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_getparam(pid: i32, param: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SCHED_GETPARAM, pid as i64, param as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_setparam(pid: i32, param: *const core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SCHED_SETPARAM, pid as i64, param as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_getscheduler(pid: i32) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall1(SYS_SCHED_GETSCHEDULER, pid as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_setscheduler(pid: i32, policy: i32, param: *const core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SCHED_SETSCHEDULER, pid as i64, policy as i64, param as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_setaffinity(pid: i32, cpusetsize: usize, cpuset: *const u8) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall3(SYS_SCHED_SETAFFINITY, pid as i64, cpusetsize as i64, cpuset as i64)) as i32
}

#[no_mangle]
pub unsafe extern "C" fn sched_getaffinity(pid: i32, cpusetsize: usize, cpuset: *mut u8) -> i32 {
    let r = <Arch as Syscalls>::syscall3(SYS_SCHED_GETAFFINITY, pid as i64, cpusetsize as i64, cpuset as i64);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    // The kernel returns the number of bytes copied, not 0.
    0
}

#[cold]
unsafe fn sched_getcpu_syscall(cpu: *mut u32) -> i32 {
    let r = <Arch as Syscalls>::syscall3(SYS_GETCPU, cpu as i64, 0, 0);
    if r == 0 {
        core::ptr::read_unaligned(cpu) as i32
    } else {
        syscall_ret(r) as i32
    }
}

#[no_mangle]
pub unsafe extern "C" fn sched_getcpu() -> i32 {
    // Both the vDSO and the syscall fill the slot on success.  A 16-byte
    // scratch area keeps the write away from the return address; a 4-byte
    // slot overlaps the compiler's push and triggers a store-forwarding
    // stall on the vDSO write (measured ~2.5 ns).
    let mut cpu = core::mem::MaybeUninit::<[u64; 2]>::uninit();
    let cpu_ptr = cpu.as_mut_ptr() as *mut u32;
    if vdso_getcpu(cpu_ptr) {
        return core::ptr::read_unaligned(cpu_ptr) as i32;
    }
    sched_getcpu_syscall(cpu_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn sched_rr_get_interval(pid: i32, tp: *mut core::ffi::c_void) -> i32 {
    syscall_ret(<Arch as Syscalls>::syscall2(SYS_SCHED_RR_GET_INTERVAL, pid as i64, tp as i64)) as i32
}

