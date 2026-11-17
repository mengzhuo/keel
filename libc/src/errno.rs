// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// errno
// ============================================================

const EILSEQ: c_int = 84;
const EINVAL: c_int = 22;
const EACCES: c_int = 13;
const ENODEV: c_int = 19;
const EMSGSIZE: c_int = 90;
const EFAULT: c_int = 14;
const E2BIG: c_int = 7;
const EIO: c_int = 5;
const ENXIO: c_int = 6;
const EBADF: c_int = 9;
const ERANGE: c_int = 34;
const ENOSYS: c_int = 38;
const EOPNOTSUPP: c_int = 95;
const ENOENT: c_int = 2;
const EEXIST: c_int = 17;
const ENOTDIR: c_int = 20;
const EMFILE: c_int = 24;
const ENOSPC: c_int = 28;
const ENAMETOOLONG: c_int = 36;
const ELOOP: c_int = 40;
const EIDRM: c_int = 43;
const ENOTSUP: c_int = 95;
const EAFNOSUPPORT: c_int = 97;

const LC_CTYPE: c_int = 0;
const LC_NUMERIC: c_int = 1;
const LC_TIME: c_int = 2;
const LC_COLLATE: c_int = 3;
const LC_MONETARY: c_int = 4;
const LC_MESSAGES: c_int = 5;
const LC_ALL: c_int = 6;
const ENOMEM: c_int = 12;
const EINTR: c_int = 4;
const EPERM: c_int = 1;
const EAGAIN: c_int = 11;
const EBUSY: c_int = 16;
const EDEADLK: c_int = 35;
const ETIMEDOUT: c_int = 110;
const ECANCELED: c_int = 125;
const EOVERFLOW: c_int = 75;
const EOWNERDEAD: c_int = 130;
const ENOTRECOVERABLE: c_int = 131;

// Static executables have no dynamic loader to build a TCB, so they keep the
// traditional process-global errno. Dynamic programs store it in the TCB, just
// below the thread pointer. The offset is inside the 256-byte TCB that
// libc.so allocates for every thread (0x00 self pointer, 0x08 generation,
// 0x28 stack canary).
static mut ERRNO_FALLBACK: c_int = 0;
static ERRNO_TLS_ACTIVE: AtomicUsize = AtomicUsize::new(0);
const ERRNO_TCB_OFFSET: usize = 0x30;

#[inline]
unsafe fn errno_ptr() -> *mut c_int {
    if ERRNO_TLS_ACTIVE.load(Ordering::Relaxed) == 0 {
        return core::ptr::addr_of_mut!(ERRNO_FALLBACK);
    }
    // tcb_slot() performs the TP read and self-pointer validation once.
    let slot = tcb_slot(ERRNO_TCB_OFFSET);
    if slot.is_null() {
        core::ptr::addr_of_mut!(ERRNO_FALLBACK)
    } else {
        slot as *mut c_int
    }
}

/// Called once from __libc_start_main. The dynamic loader has already set up
/// a TCB for dynamic programs; static programs leave %fs / TP at zero.
unsafe fn errno_init() {
    let tp = arch_get_tp();
    #[cfg(target_arch = "x86_64")]
    let tcb = tp;
    #[cfg(target_arch = "powerpc64")]
    let tcb = tp.wrapping_sub(256 + 0x7000);
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
    let tcb = tp.wrapping_sub(256);
    if tp != 0 && core::ptr::read(tcb as *const usize) == tcb {
        ERRNO_TLS_ACTIVE.store(1, Ordering::Relaxed);
    }
}

/// Normalize a raw syscall result the way musl's `__syscall_ret` does:
/// turn `-errno` into `-1` and publish the error through `errno`.
#[inline]
unsafe fn syscall_ret(r: i64) -> i64 {
    if r < 0 {
        *errno_ptr() = (-r) as c_int;
        -1
    } else {
        r
    }
}

#[no_mangle]
pub unsafe extern "C" fn __errno_location() -> *mut c_int {
    errno_ptr()
}

unsafe fn h_errno_ptr() -> *mut c_int {
    if ERRNO_TLS_ACTIVE.load(Ordering::Relaxed) == 0 {
        return core::ptr::addr_of_mut!(h_errno);
    }
    // errno occupies 0x30..0x34; h_errno is the next int in the TCB.
    errno_ptr().add(1)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    // A libc cannot unwind; looping here turns every internal panic into a
    // permanent CPU spin (and may leave locks held).  Match abort semantics
    // so the process dies with SIGABRT and the failure is observable.
    unsafe { abort() }
}

// Required by the Rust runtime; panic=abort never unwinds.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn rust_eh_personality() {}

