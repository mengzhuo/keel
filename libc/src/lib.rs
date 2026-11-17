// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

#![cfg_attr(not(test), no_std)]
#![allow(dead_code, non_camel_case_types)]
// A libc necessarily keeps mutable global state (stdio tables, passwd/group
// buffers, resolver state).  References to it are deliberate and confined to
// the C ABI entry points.
#![allow(static_mut_refs)]
// C `_Complex` values are passed as two floating-point registers on every
// supported target; Rust tuples use the same SysV/AAPCS classification.
#![allow(improper_ctypes_definitions)]

use core::ffi::{c_char, c_int, c_long, c_longlong, c_uint, c_ulong, c_ulonglong, c_void, VaList};

// Stable-Rust representation of C `long double` on the 128-bit targets.
// AArch64 passes binary128 in one SIMD register; RISC-V passes it in a pair
// of FP registers, which a two-f64 struct reproduces.
#[cfg(target_arch = "aarch64")]
pub type f128 = core::arch::aarch64::float64x2_t;

#[cfg(any(target_arch = "riscv64", target_arch = "loongarch64"))]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct f128 {
    lo: u64,
    hi: u64,
}

// powerpc64 uses the IBM double-double format: two doubles, high part first.
// Passing it as a two-f64 aggregate reproduces the ELFv2 register pair.
#[cfg(target_arch = "powerpc64")]
#[repr(C)]
#[derive(Copy, Clone)]
pub struct f128 {
    hi: f64,
    lo: f64,
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
#[inline]
fn f128_from_bits(bits: u128) -> f128 {
    unsafe { core::mem::transmute::<u128, f128>(bits) }
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
#[inline]
fn f128_to_bits(x: f128) -> u128 {
    unsafe { core::mem::transmute::<f128, u128>(x) }
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
fn f128_is_nan(x: f128) -> bool {
    let bits = f128_to_bits(x);
    ((bits >> 112) & 0x7fff) == 0x7fff && (bits & ((1u128 << 112) - 1)) != 0
}

#[cfg(target_arch = "powerpc64")]
fn f128_is_nan(x: f128) -> bool {
    x.hi.is_nan() || x.lo.is_nan()
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
fn f128_to_f64(x: f128) -> f64 {
    let bits = f128_to_bits(x);
    let sign = (bits >> 127) as u64;
    let exp = ((bits >> 112) & 0x7fff) as i32;
    let frac = bits & ((1u128 << 112) - 1);
    let signbit = sign << 63;
    if exp == 0x7fff {
        let m = if frac == 0 { 0x7ff0_0000_0000_0000 } else { 0x7ff8_0000_0000_0000 };
        return f64::from_bits(signbit | m);
    }
    if exp == 0 { return f64::from_bits(signbit); }
    let e = exp - 16383;
    if e > 1023 { return f64::from_bits(signbit | 0x7ff0_0000_0000_0000); }
    if e < -1074 { return f64::from_bits(signbit); }
    let sig = (1u128 << 112) | frac;
    if e >= -1022 {
        let mut m = (sig >> 60) as u64;
        let rem = sig & ((1u128 << 60) - 1);
        let half = 1u128 << 59;
        if rem > half || (rem == half && (m & 1) != 0) { m += 1; }
        let mut ef = (e + 1023) as u64;
        if m == (1u64 << 53) { m >>= 1; ef += 1; }
        if ef >= 0x7ff { return f64::from_bits(signbit | 0x7ff0_0000_0000_0000); }
        return f64::from_bits(signbit | (ef << 52) | (m & ((1u64 << 52) - 1)));
    }
    let rshift = (-(e + 962)) as u32;
    let mut m = (sig >> rshift) as u64;
    let rem = sig & ((1u128 << rshift) - 1);
    let half = 1u128 << (rshift - 1);
    if rem > half || (rem == half && (m & 1) != 0) { m += 1; }
    if m >= (1u64 << 52) {
        return f64::from_bits(signbit | (1u64 << 52));
    }
    f64::from_bits(signbit | m)
}

#[cfg(target_arch = "powerpc64")]
fn f128_to_f64(x: f128) -> f64 {
    x.hi + x.lo
}

#[cfg(target_arch = "powerpc64")]
unsafe fn valist_read_f128(args: &mut VaList<'_>) -> f128 {
    // IBM double-double occupies two consecutive FPR save slots.
    let hi = args.next_arg::<f64>();
    let lo = args.next_arg::<f64>();
    f128 { hi, lo }
}

#[cfg(target_arch = "riscv64")]
unsafe fn valist_read_f128(args: &mut VaList<'_>) -> f128 {
    // RISC-V va_list is an opaque pointer to the next argument. A 16-byte
    // variable argument is 16-byte aligned and 16 bytes long.
    let pp = args as *mut VaList<'_> as *mut *const u8;
    let p = ((*pp as usize) + 15) & !15usize;
    let bits = core::ptr::read_unaligned(p as *const u128);
    *pp = (p + 16) as *const u8;
    f128_from_bits(bits)
}

#[cfg(target_arch = "aarch64")]
#[repr(C)]
struct Aarch64VaList {
    stack: *const u8,
    gr_top: *const u8,
    vr_top: *const u8,
    gr_offs: i32,
    vr_offs: i32,
}

#[cfg(target_arch = "aarch64")]
unsafe fn valist_read_f128(args: &mut VaList<'_>) -> f128 {
    let p = args as *mut VaList<'_> as *mut Aarch64VaList;
    let off = (*p).vr_offs;
    let addr = if off == 0 {
        let a = (*p).stack;
        (*p).stack = a.add(16);
        a
    } else {
        let a = (*p).vr_top.offset(off as isize);
        (*p).vr_offs = off + 16;
        a
    };
    f128_from_bits(core::ptr::read_unaligned(addr as *const u128))
}

#[cfg(target_arch = "powerpc64")]
fn f64_to_f128(v: f64) -> f128 {
    f128 { hi: v, lo: 0.0 }
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
fn f64_to_f128(v: f64) -> f128 {
    let bits = v.to_bits();
    let sign = (bits >> 63) as u128;
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    if exp == 0x7ff {
        let m = if frac == 0 { 0 } else { ((frac as u128) << 60) | (1u128 << 111) };
        return f128_from_bits((sign << 127) | (0x7fff_u128 << 112) | m);
    }
    if exp == 0 {
        if frac == 0 { return f128_from_bits(sign << 127); }
        let msb = 63 - frac.leading_zeros() as i32; // 0..51
        let e = msb - 1074;
        let f = ((frac as u128) << (112 - msb as u32)) & ((1u128 << 112) - 1);
        return f128_from_bits((sign << 127) | (((e + 16383) as u128) << 112) | f);
    }
    let e = exp - 1023;
    let f = (frac as u128) << 60;
    f128_from_bits((sign << 127) | (((e + 16383) as u128) << 112) | f)
}

use core::ptr::null_mut;
use core::sync::atomic::{AtomicBool, AtomicI32, AtomicPtr, AtomicU32, AtomicUsize, Ordering};

macro_rules! ERRNO {
    () => { *errno_ptr() };
}

// h_errno shares the TCB with errno: 0x30 is errno, 0x34 is the free slot
// before the vDSO getrandom state at 0x38.  The exported `h_errno` symbol is
// the fallback for static programs and threads without a TCB, matching musl.
macro_rules! H_ERRNO {
    () => { *h_errno_ptr() };
}

include!("encoding_tables.rs");
include!("math_helpers.rs");
include!("math_bitmanip.rs");
include!("math_sqrtfmod.rs");
include!("math_trig.rs");
include!("math_exp.rs");
include!("math_lgamma.rs");
include!("math_tgamma.rs");
include!("math_bessel.rs");
include!("alloc.rs");
include!("math_log.rs");
include!("math_pow.rs");
include!("math_hypot.rs");
include!("math_hyperbolic.rs");
include!("math_invtrig.rs");
include!("math_lrint.rs");
include!("math_long_double.rs");
include!("string_fast.rs");
include!("vdso.rs");
include!("iconv_iso8859.rs");
include!("arch.rs");

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

// ============================================================
// Constants
// ============================================================

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_SHARED: i32 = 0x01;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

// ============================================================
// Syscall wrappers (raw, no_std)
// ============================================================

#[inline]
unsafe fn sys_write(fd: i64, buf: *const u8, count: usize) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_WRITE, fd as i64, buf as i64, count as i64)
}

#[inline]
unsafe fn sys_read(fd: i64, buf: *mut u8, count: usize) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_READ, fd as i64, buf as i64, count as i64)
}

#[inline]
unsafe fn sys_open(path: *const u8, flags: i64, mode: i64) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_OPENAT, AT_FDCWD as i64, path as i64, flags as i64, mode as i64)
}

#[inline]
unsafe fn sys_close(fd: i64) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_CLOSE, fd as i64)
}

#[inline]
unsafe fn sys_lseek(fd: i64, offset: i64, whence: i64) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_LSEEK, fd as i64, offset as i64, whence as i64)
}

unsafe fn sys_mmap(
    addr: *mut u8,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> *mut u8 {
    let result = <Arch as Syscalls>::syscall6(SYS_MMAP, addr as i64, length as i64, prot as i64, flags as i64, fd as i64, offset);
    if result < 0 && result > -crate::SYSCALL_ERR_MAX {
        ERRNO!() = (-result) as c_int;
        return MMAP_FAILED;
    }
    result as *mut u8
}

unsafe fn sys_munmap(addr: *mut u8, length: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_MUNMAP, addr as i64, length as i64)
}

// ============================================================
// String/memory functions
// ============================================================

// strlen, memcpy, memset, memmove and memcmp live in string_fast.rs.

#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int {
    memcmp(s1, s2, n)
}

// strcmp lives in string_fast.rs.

#[no_mangle]
pub unsafe extern "C" fn strncmp(s1: *const u8, s2: *const u8, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as c_int - b as c_int;
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n && *src.add(i) != 0 {
        *dst.add(i) = *src.add(i);
        i += 1;
    }
    while i < n {
        *dst.add(i) = 0;
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strcat(dst: *mut u8, src: *const u8) -> *mut u8 {
    let mut i = 0;
    while *dst.add(i) != 0 {
        i += 1;
    }
    let mut j = 0;
    loop {
        let c = *src.add(j);
        *dst.add(i + j) = c;
        if c == 0 {
            break;
        }
        j += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncat(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while *dst.add(i) != 0 {
        i += 1;
    }
    let mut j = 0;
    while j < n && *src.add(j) != 0 {
        *dst.add(i + j) = *src.add(j);
        j += 1;
    }
    *dst.add(i + j) = 0;
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strlcpy(dst: *mut u8, src: *const u8, n: usize) -> usize {
    let l = strlen(src as *const c_char);
    if n > 0 {
        let c = if l < n { l } else { n - 1 };
        core::ptr::copy_nonoverlapping(src, dst, c);
        *dst.add(c) = 0;
    }
    l
}

#[no_mangle]
pub unsafe extern "C" fn strlcat(dst: *mut u8, src: *const u8, n: usize) -> usize {
    let dl = strlen(dst as *const c_char);
    let sl = strlen(src as *const c_char);
    if dl >= n { return n + sl; }
    let c = if sl < n - dl { sl } else { n - dl - 1 };
    core::ptr::copy_nonoverlapping(src, dst.add(dl), c);
    *dst.add(dl + c) = 0;
    dl + sl
}

#[no_mangle]
pub unsafe extern "C" fn bcopy(src: *const c_void, dst: *mut c_void, n: usize) {
    core::ptr::copy(src as *const u8, dst as *mut u8, n);
}

#[no_mangle]
pub unsafe extern "C" fn bzero(s: *mut c_void, n: usize) {
    core::ptr::write_bytes(s as *mut u8, 0, n);
}

#[no_mangle]
pub unsafe extern "C" fn strndup(s: *const c_char, n: usize) -> *mut c_char {
    let mut len = 0;
    while len < n && *s.add(len) != 0 {
        len += 1;
    }
    let p = malloc(len + 1) as *mut c_char;
    if p.is_null() {
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(s, p, len);
    *p.add(len) = 0;
    p
}

#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const u8, c: c_int) -> *mut u8 {
    let target = c as u8;
    let mut i = 0;
    loop {
        let ch = *s.add(i);
        if ch == target {
            return s.add(i) as *mut u8;
        }
        if ch == 0 {
            return null_mut();
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strrchr(s: *const u8, c: c_int) -> *mut u8 {
    let target = c as u8;
    let mut i = 0;
    let mut last: *mut u8 = null_mut();
    loop {
        let ch = *s.add(i);
        if ch == target {
            last = s.add(i) as *mut u8;
        }
        if ch == 0 {
            return last;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strnlen(s: *const u8, maxlen: usize) -> usize {
    let mut i = 0;
    while i < maxlen && *s.add(i) != 0 {
        i += 1;
    }
    i
}

#[no_mangle]
pub unsafe extern "C" fn strcspn(s: *const u8, reject: *const u8) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 {
            return i;
        }
        let mut j = 0;
        loop {
            let r = *reject.add(j);
            if r == 0 {
                break;
            }
            if c == r {
                return i;
            }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strspn(s: *const u8, accept: *const u8) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 {
            return i;
        }
        let mut found = false;
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 {
                break;
            }
            if c == a {
                found = true;
                break;
            }
            j += 1;
        }
        if !found {
            return i;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strpbrk(s: *const u8, accept: *const u8) -> *mut u8 {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 {
            return null_mut();
        }
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 {
                break;
            }
            if c == a {
                return s.add(i) as *mut u8;
            }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strstr(haystack: *const u8, needle: *const u8) -> *mut u8 {
    if *needle == 0 {
        return haystack as *mut u8;
    }
    let mut i = 0;
    loop {
        let h = *haystack.add(i);
        if h == 0 {
            return null_mut();
        }
        if h == *needle {
            let mut j = 0;
            loop {
                let n = *needle.add(j);
                if n == 0 {
                    return haystack.add(i) as *mut u8;
                }
                if *haystack.add(i + j) != n {
                    break;
                }
                j += 1;
            }
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn memchr(s: *const u8, c: c_int, n: usize) -> *mut u8 {
    let target = c as u8;
    let mut i = 0;
    while i < n {
        if *s.add(i) == target {
            return s.add(i) as *mut u8;
        }
        i += 1;
    }
    null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn memrchr(s: *const u8, c: c_int, n: usize) -> *mut u8 {
    let target = c as u8;
    let mut i = n;
    while i > 0 {
        i -= 1;
        if *s.add(i) == target {
            return s.add(i) as *mut u8;
        }
    }
    null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn memmem(
    haystack: *const c_void, haystacklen: usize,
    needle: *const c_void, needlelen: usize,
) -> *mut c_void {
    if needlelen == 0 { return haystack as *mut c_void; }
    if haystacklen < needlelen { return null_mut(); }
    let h = haystack as *const u8;
    let n = needle as *const u8;
    let last = haystacklen - needlelen;
    for i in 0..=last {
        if *h.add(i) == *n && memcmp(h.add(i) as *const c_void, n as *const c_void, needlelen) == 0 {
            return h.add(i) as *mut c_void;
        }
    }
    null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn strtok(s: *mut u8, delim: *const u8) -> *mut u8 {
    static mut STATE: *mut u8 = core::ptr::null_mut();
    let mut p = if s.is_null() { STATE } else { s };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    loop {
        let c = *p;
        if c == 0 {
            STATE = core::ptr::null_mut();
            return core::ptr::null_mut();
        }
        let mut is_delim = false;
        let mut i = 0;
        loop {
            let d = *delim.add(i);
            if d == 0 {
                break;
            }
            if c == d {
                is_delim = true;
                break;
            }
            i += 1;
        }
        if !is_delim {
            break;
        }
        p = p.add(1);
    }
    let start = p;
    loop {
        let c = *p;
        if c == 0 {
            STATE = core::ptr::null_mut();
            return start;
        }
        let mut is_delim = false;
        let mut i = 0;
        loop {
            let d = *delim.add(i);
            if d == 0 {
                break;
            }
            if c == d {
                is_delim = true;
                break;
            }
            i += 1;
        }
        if is_delim {
            *p = 0;
            STATE = p.add(1);
            return start;
        }
        p = p.add(1);
    }
}

// ============================================================
// C type aliases
// ============================================================

type SizeT = usize;
type SSizeT = isize;
type mode_t = c_uint;
type TimeT = c_long;
type ClockT = c_long;
type wchar_t = c_int;
type wint_t = c_uint;
type wctype_t = *const c_int;
type wctrans_t = *const c_int;

const WEOF: wint_t = 0xffffffff;

#[repr(C)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

#[repr(C)]
pub struct timeval {
    pub tv_sec: TimeT,
    pub tv_usec: c_long,
}

#[repr(C)]
pub struct Timeb {
    pub time: TimeT,
    pub millitm: u16,
    pub timezone: i16,
    pub dstflag: u16,
}

// ============================================================
// ctype.h
// ============================================================

const CT_UPPER: u8 = 1 << 0;
const CT_LOWER: u8 = 1 << 1;
const CT_DIGIT: u8 = 1 << 2;
const CT_SPACE: u8 = 1 << 3;
const CT_PUNCT: u8 = 1 << 4;
const CT_CNTRL: u8 = 1 << 5;
const CT_BLANK: u8 = 1 << 6;
const CT_XDIGIT: u8 = 1 << 7;

const CT_TABLE: [u8; 128] = {
    let mut t = [0u8; 128];
    let mut i = 0;
    while i < 128 {
        let c = i as u8;
        let mut flags = 0u8;
        if c >= b'A' && c <= b'Z' {
            flags |= CT_UPPER;
        }
        if c >= b'a' && c <= b'z' {
            flags |= CT_LOWER;
        }
        if c >= b'0' && c <= b'9' {
            flags |= CT_DIGIT;
        }
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'\x0c' || c == b'\x0b' {
            flags |= CT_SPACE;
        }
        if (c >= b'!' && c <= b'/')
            || (c >= b':' && c <= b'@')
            || (c >= b'[' && c <= b'`')
            || (c >= b'{' && c <= b'~')
        {
            flags |= CT_PUNCT;
        }
        if c < 0x20 || c == 0x7f {
            flags |= CT_CNTRL;
        }
        if c == b' ' || c == b'\t' {
            flags |= CT_BLANK;
        }
        if (c >= b'0' && c <= b'9') || (c >= b'A' && c <= b'F') || (c >= b'a' && c <= b'f') {
            flags |= CT_XDIGIT;
        }
        t[i] = flags;
        i += 1;
    }
    t
};

#[inline]
fn ct_flags(c: c_int) -> u8 {
    if c < 0 || c > 127 {
        0
    } else {
        CT_TABLE[c as usize]
    }
}

#[no_mangle]
pub extern "C" fn isalnum(c: c_int) -> c_int {
    (ct_flags(c) & (CT_UPPER | CT_LOWER | CT_DIGIT) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isalpha(c: c_int) -> c_int {
    (ct_flags(c) & (CT_UPPER | CT_LOWER) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isascii(c: c_int) -> c_int {
    ((c as u32) <= 127) as c_int
}

#[no_mangle]
pub extern "C" fn isblank(c: c_int) -> c_int {
    (ct_flags(c) & CT_BLANK != 0) as c_int
}

#[no_mangle]
pub extern "C" fn iscntrl(c: c_int) -> c_int {
    (ct_flags(c) & CT_CNTRL != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isdigit(c: c_int) -> c_int {
    (ct_flags(c) & CT_DIGIT != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isgraph(c: c_int) -> c_int {
    (ct_flags(c) & (CT_UPPER | CT_LOWER | CT_DIGIT | CT_PUNCT) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn islower(c: c_int) -> c_int {
    (ct_flags(c) & CT_LOWER != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isprint(c: c_int) -> c_int {
    let flags = ct_flags(c);
    (flags & (CT_UPPER | CT_LOWER | CT_DIGIT | CT_PUNCT | CT_BLANK) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn ispunct(c: c_int) -> c_int {
    (ct_flags(c) & CT_PUNCT != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isspace(c: c_int) -> c_int {
    (ct_flags(c) & CT_SPACE != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isupper(c: c_int) -> c_int {
    (ct_flags(c) & CT_UPPER != 0) as c_int
}

#[no_mangle]
pub extern "C" fn isxdigit(c: c_int) -> c_int {
    (ct_flags(c) & CT_XDIGIT != 0) as c_int
}

#[no_mangle]
pub extern "C" fn tolower(c: c_int) -> c_int {
    if (ct_flags(c) & CT_UPPER) != 0 {
        c + (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

#[no_mangle]
pub extern "C" fn toupper(c: c_int) -> c_int {
    if (ct_flags(c) & CT_LOWER) != 0 {
        c - (b'a' as c_int - b'A' as c_int)
    } else {
        c
    }
}

// ============================================================
// stdlib.h: integer conversion, abs, rand
// ============================================================

unsafe fn parse_digit(c: u8, base: c_int) -> Option<u8> {
    if c == 0 { return None; }
    let d = if c >= b'0' && c <= b'9' {
        c - b'0'
    } else if c >= b'a' && c <= b'z' {
        c - b'a' + 10
    } else if c >= b'A' && c <= b'Z' {
        c - b'A' + 10
    } else {
        return None;
    };
    if (d as c_int) < base {
        Some(d)
    } else {
        None
    }
}

unsafe fn parse_prefix(s: *const u8, base: *mut c_int) -> *const u8 {
    let mut p = s;
    if *base == 0 {
        if *p == b'0' {
            if (*p.add(1) == b'x' || *p.add(1) == b'X') && parse_digit(*p.add(2), 16).is_some() {
                *base = 16;
                p = p.add(2);
            } else {
                *base = 8;
            }
        } else {
            *base = 10;
        }
    } else if *base == 16 {
        if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') && parse_digit(*p.add(2), 16).is_some() {
            p = p.add(2);
        }
    }
    p
}

unsafe fn strtox(
    s: *const c_char,
    endptr: *mut *mut c_char,
    mut base: c_int,
    pos_limit: u64,
    neg_limit: u64,
) -> (u64, bool, bool) {
    let s0 = s as *const u8;
    let mut p = s0;
    while isspace(*p as c_int) != 0 {
        p = p.add(1);
    }

    if base < 0 || base == 1 || base > 36 {
        if !endptr.is_null() {
            *endptr = s as *mut c_char;
        }
        ERRNO!() = EINVAL;
        return (0, false, false);
    }

    let mut neg = false;
    match *p {
        b'-' => { neg = true; p = p.add(1); }
        b'+' => p = p.add(1),
        _ => {}
    }

    p = parse_prefix(p, &mut base);

    let limit = if neg { neg_limit } else { pos_limit };
    let mut val: u64 = 0;
    let mut any = false;
    while let Some(d) = parse_digit(*p, base) {
        any = true;
        if val > (limit - d as u64) / base as u64 {
            while parse_digit(*p, base).is_some() { p = p.add(1); }
            if !endptr.is_null() { *endptr = p as *mut c_char; }
            ERRNO!() = ERANGE;
            return (limit, true, neg);
        }
        val = val * base as u64 + d as u64;
        p = p.add(1);
    }

    if !endptr.is_null() {
        *endptr = if any { p as *mut c_char } else { s as *mut c_char };
    }
    (val, false, neg)
}

#[no_mangle]
pub unsafe extern "C" fn strtol(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_long {
    let pos_limit = c_long::MAX as u64;
    let neg_limit = pos_limit.wrapping_add(1);
    let (val, overflow, neg) = strtox(s, endptr, base, pos_limit, neg_limit);
    if overflow {
        return if neg { c_long::MIN } else { c_long::MAX };
    }
    if neg { val.wrapping_neg() as c_long } else { val as c_long }
}

#[no_mangle]
pub unsafe extern "C" fn strtoul(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulong {
    let limit = c_ulong::MAX as u64;
    let (val, overflow, neg) = strtox(s, endptr, base, limit, limit);
    if overflow {
        return c_ulong::MAX;
    }
    if neg { val.wrapping_neg() as c_ulong } else { val as c_ulong }
}

#[no_mangle]
pub unsafe extern "C" fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_longlong {
    let pos_limit = c_longlong::MAX as u64;
    let neg_limit = pos_limit.wrapping_add(1);
    let (val, overflow, neg) = strtox(s, endptr, base, pos_limit, neg_limit);
    if overflow {
        return if neg { c_longlong::MIN } else { c_longlong::MAX };
    }
    if neg { val.wrapping_neg() as c_longlong } else { val as c_longlong }
}

#[no_mangle]
pub unsafe extern "C" fn strtoull(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> c_ulonglong {
    let limit = c_ulonglong::MAX as u64;
    let (val, overflow, neg) = strtox(s, endptr, base, limit, limit);
    if overflow {
        return c_ulonglong::MAX;
    }
    if neg { val.wrapping_neg() as c_ulonglong } else { val as c_ulonglong }
}

#[no_mangle]
pub unsafe extern "C" fn atoi(s: *const c_char) -> c_int {
    strtol(s, core::ptr::null_mut(), 10) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn atol(s: *const c_char) -> c_long {
    strtol(s, core::ptr::null_mut(), 10)
}

#[no_mangle]
pub unsafe extern "C" fn atoll(s: *const c_char) -> c_longlong {
    strtoll(s, core::ptr::null_mut(), 10)
}

#[no_mangle]
pub extern "C" fn abs(n: c_int) -> c_int {
    if n < 0 {
        -n
    } else {
        n
    }
}

#[no_mangle]
pub extern "C" fn labs(n: c_long) -> c_long {
    if n < 0 {
        -n
    } else {
        n
    }
}

#[no_mangle]
pub extern "C" fn llabs(n: c_longlong) -> c_longlong {
    if n < 0 {
        -n
    } else {
        n
    }
}

static mut RAND_STATE: c_uint = 1;

#[no_mangle]
pub extern "C" fn srand(seed: c_uint) {
    unsafe {
        RAND_STATE = seed;
    }
}

#[no_mangle]
pub extern "C" fn rand() -> c_int {
    unsafe {
        RAND_STATE = RAND_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        ((RAND_STATE >> 16) & 0x7fff) as c_int
    }
}

// ============================================================
// stdlib.h: div, ldiv, lldiv
// ============================================================

#[repr(C)]
pub struct div_t { pub quot: c_int, pub rem: c_int }
#[repr(C)]
pub struct ldiv_t { pub quot: c_long, pub rem: c_long }
#[repr(C)]
pub struct lldiv_t { pub quot: c_longlong, pub rem: c_longlong }

#[no_mangle]
pub extern "C" fn div(num: c_int, den: c_int) -> div_t {
    div_t { quot: num / den, rem: num % den }
}

#[no_mangle]
pub extern "C" fn ldiv(num: c_long, den: c_long) -> ldiv_t {
    ldiv_t { quot: num / den, rem: num % den }
}

#[no_mangle]
pub extern "C" fn lldiv(num: c_longlong, den: c_longlong) -> lldiv_t {
    lldiv_t { quot: num / den, rem: num % den }
}

// ============================================================
// bsearch
// ============================================================

type CmpFn = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;

#[no_mangle]
pub unsafe extern "C" fn bsearch(
    key: *const c_void,
    base: *const c_void,
    nel: usize,
    width: usize,
    cmp: CmpFn,
) -> *mut c_void {
    let mut base = base as *const u8;
    let mut nel = nel;
    while nel > 0 {
        let trial = base.add(width * (nel / 2));
        let sign = cmp(key, trial as *const c_void);
        if sign < 0 {
            nel /= 2;
        } else if sign > 0 {
            base = trial.add(width);
            nel -= nel / 2 + 1;
        } else {
            return trial as *mut c_void;
        }
    }
    null_mut()
}

include!("signal.rs");
include!("process.rs");
// ============================================================
// Syscall wrappers as public C ABI
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn write(fd: c_int, buf: *const c_void, count: SizeT) -> SSizeT {
    syscall_ret(sys_write(fd as i64, buf as *const u8, count)) as SSizeT
}

#[no_mangle]
pub unsafe extern "C" fn read(fd: c_int, buf: *mut c_void, count: SizeT) -> SSizeT {
    syscall_ret(sys_read(fd as i64, buf as *mut u8, count)) as SSizeT
}

#[no_mangle]
pub unsafe extern "C" fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int {
    syscall_ret(sys_open(path as *const u8, flags as i64, mode as i64)) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn close(fd: c_int) -> c_int {
    syscall_ret(sys_close(fd as i64)) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64 {
    syscall_ret(sys_lseek(fd as i64, offset, whence as i64))
}

// ============================================================
// Additional syscall wrappers for stdio
// ============================================================

const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR: i32 = 2;
const O_CREAT: i32 = 64;
const O_TRUNC: i32 = 512;
const O_APPEND: i32 = 1024;
const O_EXCL: i32 = 128;
const O_NONBLOCK: i32 = 2048;
const O_NOFOLLOW: i32 = if cfg!(any(target_arch = "aarch64", target_arch = "powerpc64")) {
    0o100000
} else {
    0o400000
};
const O_DIRECTORY: i32 = if cfg!(any(target_arch = "aarch64", target_arch = "powerpc64")) {
    0o40000
} else {
    0o200000
};
const O_NOCTTY: i32 = 0o400;
const O_PATH: i32 = 0o10000000;
const O_CLOEXEC: i32 = 0x80000;

const F_GETFD: i32 = 1;
const F_GETFL: i32 = 3;
const F_GETOWN: i32 = 9;
const F_GETSIG: i32 = 11;
const F_GETLEASE: i32 = 1025;
const F_GETPIPE_SZ: i32 = 1032;
const F_GET_SEALS: i32 = 1034;
const F_SETFD: i32 = 2;
const F_SETFL: i32 = 4;
const F_GETLK: i32 = 5;
const F_SETLK: i32 = 6;
const F_SETLKW: i32 = 7;
const F_DUPFD: i32 = 0;
const F_DUPFD_CLOEXEC: i32 = 1030;
const FD_CLOEXEC: i32 = 1;

const TIOCGWINSZ: u32 = 0x5413;

#[repr(C)]
struct winsize {
    ws_row: u16,
    ws_col: u16,
    ws_xpixel: u16,
    ws_ypixel: u16,
}

#[inline]
unsafe fn sys_pipe2(fds: *mut c_int, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_PIPE2, fds as i64, flags as i64)
}

#[inline]
unsafe fn sys_dup3(oldfd: i32, newfd: i32, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_DUP3, oldfd as i64, newfd as i64, flags as i64)
}

#[inline]
unsafe fn sys_fcntl(fd: i32, cmd: i32, arg: i64) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_FCNTL, fd as i64, cmd as i64, arg as i64)
}

#[inline]
unsafe fn sys_ioctl(fd: c_int, request: u32, arg: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_IOCTL, fd as i64, request as i64, arg as i64)
}

#[inline]
unsafe fn sys_unlinkat(dirfd: i32, path: *const u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_UNLINKAT, dirfd as i64, path as i64, flags as i64)
}

#[inline]
unsafe fn sys_renameat2(olddirfd: i32, oldpath: *const u8, newdirfd: i32, newpath: *const u8, flags: u32) -> i64 {
    <Arch as Syscalls>::syscall5(SYS_RENAMEAT2, olddirfd as i64, oldpath as i64, newdirfd as i64, newpath as i64, flags as i64)
}

include!("stdio.rs");
include!("printf.rs");
include!("scanf.rs");
include!("stdio_misc.rs");
// ============================================================
// Allocator basics
// ============================================================

const MMAP_FAILED: *mut u8 = !0usize as *mut u8;
const PAGE: usize = 4096;

// ============================================================
// Process: exit / _exit / _Exit
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn _exit(code: c_int) -> ! {
    sys_exit_group(code)
}

#[inline]
unsafe fn sys_exit_group(code: c_int) -> ! {
    <Arch as Syscalls>::syscall_noreturn1(SYS_EXIT_GROUP, code as i64)
}

#[inline]
unsafe fn sys_exit_thread(code: c_int) -> ! {
    <Arch as Syscalls>::syscall_noreturn1(SYS_EXIT, code as i64)
}

#[no_mangle]
pub unsafe extern "C" fn _Exit(code: c_int) -> ! {
    _exit(code);
}

#[no_mangle]
pub unsafe extern "C" fn exit(code: c_int) -> ! {
    fflush(core::ptr::null_mut());
    __funcs_on_exit();
    _exit(code);
}

#[no_mangle]
pub unsafe extern "C" fn abort() -> ! {
    raise(6); // SIGABRT
    _exit(128 + 6);
}

// The 128-bit targets and powerpc64 access the stack canary through the
// global __stack_chk_guard symbol (GOT- or TOC-based for PIE).  Export it so
// the dynamic linker can resolve it, and seed it from AT_RANDOM at startup.
#[no_mangle]
pub static mut __stack_chk_guard: usize = 0;

#[no_mangle]
pub unsafe extern "C" fn __stack_chk_fail() -> ! {
    const MSG: &[u8] = b"*** stack smashing detected ***: terminated\n";
    let _ = sys_write(2, MSG.as_ptr(), MSG.len());
    abort();
}

#[no_mangle]
pub unsafe extern "C" fn __stack_chk_fail_local() -> ! {
    __stack_chk_fail()
}

// ============================================================
// qsort / qsort_r (smoothsort, O(1) aux, adaptive)
// ============================================================

type CmpRfn = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int;

const AR_LEN: usize = 16 * core::mem::size_of::<usize>();
const AR_MASK: usize = AR_LEN - 1;

unsafe fn qsort_pntz(p: &[usize; 2]) -> i32 {
    if p[0] != 1 {
        p[0].wrapping_sub(1).trailing_zeros() as i32
    } else if p[1] != 0 {
        (8 * core::mem::size_of::<usize>()) as i32 + p[1].trailing_zeros() as i32
    } else {
        0
    }
}

unsafe fn qsort_cycle(width: usize, ar: &mut [*mut u8; AR_LEN], n: usize) {
    if n < 2 { return; }
    let mut tmp = [0u8; 256];
    ar[n] = tmp.as_mut_ptr();
    let mut w = width;
    while w > 0 {
        let l = if 256 < w { 256 } else { w };
        core::ptr::copy_nonoverlapping(ar[0], ar[n], l);
        for i in 0..n {
            core::ptr::copy_nonoverlapping(ar[i + 1], ar[i], l);
            ar[i] = ar[i].add(l);
        }
        w -= l;
    }
}

unsafe fn qsort_shl(p: &mut [usize; 2], n: i32) {
    let bits = (8 * core::mem::size_of::<usize>()) as i32;
    if n >= bits {
        p[1] = p[0];
        p[0] = 0;
        let n = n - bits;
        if n == 0 { return; }
    }
    p[1] = (p[1] << n) | (p[0] >> (bits - n));
    p[0] <<= n;
}

unsafe fn qsort_shr(p: &mut [usize; 2], n: i32) {
    let bits = (8 * core::mem::size_of::<usize>()) as i32;
    if n >= bits {
        p[0] = p[1];
        p[1] = 0;
        let n = n - bits;
        if n == 0 { return; }
    }
    p[0] = (p[0] >> n) | (p[1] << (bits - n));
    p[1] >>= n;
}

unsafe fn qsort_sift(
    head: *mut u8, width: usize,
    cmp: CmpRfn, arg: *mut c_void,
    pshift: i32, lp: &[usize],
) {
    let mut ar: [*mut u8; AR_LEN] = [core::ptr::null_mut(); AR_LEN];
    ar[0] = head;
    let mut i = 1usize;
    let mut h = head;
    let mut ps = pshift;

    while ps > 1 {
        let rt = h.sub(width);
        let lf = h.sub(width + lp[(ps - 2) as usize]);

        if cmp(ar[0] as *const c_void, lf as *const c_void, arg) >= 0
            && cmp(ar[0] as *const c_void, rt as *const c_void, arg) >= 0
        {
            break;
        }
        if cmp(lf as *const c_void, rt as *const c_void, arg) >= 0 {
            ar[i & AR_MASK] = lf;
            i += 1;
            h = lf;
            ps -= 1;
        } else {
            ar[i & AR_MASK] = rt;
            i += 1;
            h = rt;
            ps -= 2;
        }
    }
    qsort_cycle(width, &mut ar, i & AR_MASK);
}

unsafe fn qsort_trinkle(
    head: *mut u8, width: usize,
    cmp: CmpRfn, arg: *mut c_void,
    pp: &[usize; 2], pshift: i32, trusty: i32,
    lp: &[usize],
) {
    let mut ar: [*mut u8; AR_LEN] = [core::ptr::null_mut(); AR_LEN];
    let mut p = *pp;
    ar[0] = head;
    let mut i = 1usize;
    let mut h = head;
    let mut ps = pshift;
    let mut t = trusty;

    while p[0] != 1 || p[1] != 0 {
        let stepson = h.sub(lp[ps as usize]);
        if cmp(stepson as *const c_void, ar[0] as *const c_void, arg) <= 0 {
            break;
        }
        if t == 0 && ps > 1 {
            let rt = h.sub(width);
            let lf = h.sub(width + lp[(ps - 2) as usize]);
            if cmp(rt as *const c_void, stepson as *const c_void, arg) >= 0
                || cmp(lf as *const c_void, stepson as *const c_void, arg) >= 0
            {
                break;
            }
        }

        ar[i & AR_MASK] = stepson;
        i += 1;
        h = stepson;
        let trail = qsort_pntz(&p);
        qsort_shr(&mut p, trail);
        ps += trail;
        t = 0;
    }
    if t == 0 {
        qsort_cycle(width, &mut ar, i & AR_MASK);
        qsort_sift(h, width, cmp, arg, ps, lp);
    }
}

#[no_mangle]
pub unsafe extern "C" fn __qsort_r(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: CmpRfn,
    arg: *mut c_void,
) {
    let size = match width.checked_mul(nel) {
        Some(v) => v,
        None => return,
    };
    if size == 0 { return; }

    let mut lp = [0usize; 96];
    lp[0] = width;
    lp[1] = width;
    let mut li = 2;
    while li + 1 < lp.len() {
        lp[li] = match lp[li - 2]
            .checked_add(lp[li - 1])
            .and_then(|v| v.checked_add(width))
        {
            Some(v) => v,
            None => break,
        };
        if lp[li] >= size { break; }
        li += 1;
    }

    let mut head = base as *mut u8;
    let high = head.add(size - width);
    let mut p: [usize; 2] = [1, 0];
    let mut pshift: i32 = 1;

    while head < high {
        if (p[0] & 3) == 3 {
            qsort_sift(head, width, cmp, arg, pshift, &lp);
            qsort_shr(&mut p, 2);
            pshift += 2;
        } else {
            if lp[(pshift - 1) as usize] >= (high as usize) - (head as usize) {
                qsort_trinkle(head, width, cmp, arg, &p, pshift, 0, &lp);
            } else {
                qsort_sift(head, width, cmp, arg, pshift, &lp);
            }

            if pshift == 1 {
                qsort_shl(&mut p, 1);
                pshift = 0;
            } else {
                qsort_shl(&mut p, pshift - 1);
                pshift = 1;
            }
        }

        p[0] |= 1;
        head = head.add(width);
    }

    qsort_trinkle(head, width, cmp, arg, &p, pshift, 0, &lp);

    while pshift != 1 || p[0] != 1 || p[1] != 0 {
        if pshift <= 1 {
            let trail = qsort_pntz(&p);
            qsort_shr(&mut p, trail);
            pshift += trail;
        } else {
            qsort_shl(&mut p, 2);
            pshift -= 2;
            p[0] ^= 7;
            qsort_shr(&mut p, 1);
            qsort_trinkle(
                head.sub(lp[pshift as usize] + width),
                width, cmp, arg, &p, pshift + 1, 1, &lp,
            );
            qsort_shl(&mut p, 1);
            p[0] |= 1;
            qsort_trinkle(head.sub(width), width, cmp, arg, &p, pshift, 1, &lp);
        }
        head = head.sub(width);
    }
}

#[no_mangle]
pub unsafe extern "C" fn qsort_r(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: CmpRfn,
    arg: *mut c_void,
) {
    __qsort_r(base, nel, width, cmp, arg);
}

unsafe extern "C" fn qsort_wrap_cmp(
    a: *const c_void,
    b: *const c_void,
    cmp_ctx: *mut c_void,
) -> c_int {
    let cmp: CmpFn = core::mem::transmute(cmp_ctx);
    cmp(a, b)
}

#[no_mangle]
pub unsafe extern "C" fn qsort(
    base: *mut c_void,
    nel: usize,
    width: usize,
    cmp: CmpFn,
) {
    __qsort_r(base, nel, width, qsort_wrap_cmp, cmp as *mut c_void);
}

// ============================================================
// atexit / __cxa_atexit / __funcs_on_exit
// ============================================================

const ATEXIT_COUNT: usize = 32;

#[repr(C)]
struct AtExitBlock {
    next: *mut AtExitBlock,
    f: [Option<unsafe extern "C" fn(*mut c_void)>; ATEXIT_COUNT],
    a: [*mut c_void; ATEXIT_COUNT],
}

static mut ATEXIT_BUILTIN: AtExitBlock = AtExitBlock {
    next: core::ptr::null_mut(),
    f: [None; ATEXIT_COUNT],
    a: [core::ptr::null_mut(); ATEXIT_COUNT],
};
static mut ATEXIT_HEAD: *mut AtExitBlock = core::ptr::null_mut();
static mut ATEXIT_SLOT: usize = 0;
static mut ATEXIT_FINISHED: bool = false;

#[no_mangle]
pub unsafe extern "C" fn __funcs_on_exit() {
    while !ATEXIT_HEAD.is_null() {
        let head = ATEXIT_HEAD;
        while ATEXIT_SLOT > 0 {
            ATEXIT_SLOT -= 1;
            if let Some(f) = (*head).f[ATEXIT_SLOT] {
                let a = (*head).a[ATEXIT_SLOT];
                f(a);
            }
        }
        ATEXIT_HEAD = (*head).next;
        ATEXIT_SLOT = ATEXIT_COUNT;
    }
    ATEXIT_FINISHED = true;
}

#[no_mangle]
pub unsafe extern "C" fn __cxa_atexit(
    func: unsafe extern "C" fn(*mut c_void),
    arg: *mut c_void,
    _dso: *mut c_void,
) -> c_int {
    if ATEXIT_FINISHED {
        return -1;
    }
    if ATEXIT_HEAD.is_null() {
        ATEXIT_HEAD = core::ptr::addr_of_mut!(ATEXIT_BUILTIN);
    }
    if ATEXIT_SLOT == ATEXIT_COUNT {
        let new = calloc(1, core::mem::size_of::<AtExitBlock>()) as *mut AtExitBlock;
        if new.is_null() {
            return -1;
        }
        (*new).next = ATEXIT_HEAD;
        ATEXIT_HEAD = new;
        ATEXIT_SLOT = 0;
    }
    (*ATEXIT_HEAD).f[ATEXIT_SLOT] = Some(func);
    (*ATEXIT_HEAD).a[ATEXIT_SLOT] = arg;
    ATEXIT_SLOT += 1;
    0
}

unsafe extern "C" fn atexit_caller(arg: *mut c_void) {
    let func: unsafe extern "C" fn() = core::mem::transmute(arg);
    func();
}

#[no_mangle]
pub unsafe extern "C" fn atexit(func: unsafe extern "C" fn()) -> c_int {
    __cxa_atexit(atexit_caller, func as *mut c_void, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn __cxa_finalize(_dso: *mut c_void) {
    // Modules are never unloaded (dlclose is a no-op), so there is nothing
    // to finalize here.
}

// ============================================================
// getenv / setenv / putenv / unsetenv / clearenv
// ============================================================

#[no_mangle]
pub static mut __environ: *mut *mut c_char = core::ptr::null_mut();

// Growable tracking of strings allocated by setenv/putenv.  Realloc keeps
// this unbounded; if it fails the new string is simply left untracked.
static mut ENV_ALLOCED: *mut *mut c_char = core::ptr::null_mut();
static mut ENV_ALLOCED_N: usize = 0;
static mut ENV_ALLOCED_CAP: usize = 0;

unsafe fn env_rm_add(old: *mut c_char, mut new: *mut c_char) {
    for i in 0..ENV_ALLOCED_N {
        if (*ENV_ALLOCED.add(i)) == old {
            *ENV_ALLOCED.add(i) = new;
            if !old.is_null() {
                free(old as *mut c_void);
            }
            return;
        } else if (*ENV_ALLOCED.add(i)).is_null() && !new.is_null() {
            *ENV_ALLOCED.add(i) = new;
            new = core::ptr::null_mut();
        }
    }
    if !new.is_null() {
        if ENV_ALLOCED_N == ENV_ALLOCED_CAP {
            let new_cap = if ENV_ALLOCED_CAP == 0 { 16 } else { ENV_ALLOCED_CAP * 2 };
            let grown = realloc(
                ENV_ALLOCED as *mut c_void,
                new_cap * core::mem::size_of::<*mut c_char>(),
            ) as *mut *mut c_char;
            if grown.is_null() {
                return;
            }
            ENV_ALLOCED = grown;
            ENV_ALLOCED_CAP = new_cap;
        }
        *ENV_ALLOCED.add(ENV_ALLOCED_N) = new;
        ENV_ALLOCED_N += 1;
    }
}

unsafe fn strchrnul_u8(s: *const u8, c: u8) -> *const u8 {
    let mut p = s;
    while *p != 0 && *p != c {
        p = p.add(1);
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn getenv(name: *const c_char) -> *mut c_char {
    if __environ.is_null() || name.is_null() {
        return core::ptr::null_mut();
    }
    let name = name as *const u8;
    let l = strchrnul_u8(name, b'=') as usize - name as usize;
    if l == 0 || *name.add(l) != 0 {
        return core::ptr::null_mut();
    }
    let mut e = __environ;
    while !(*e).is_null() {
        let entry = *e as *const u8;
        if strncmp(entry, name, l) == 0 && *entry.add(l) == b'=' {
            return entry.add(l + 1) as *mut c_char;
        }
        e = e.add(1);
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn setenv(
    var: *const c_char,
    value: *const c_char,
    overwrite: c_int,
) -> c_int {
    if var.is_null() || value.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let var = var as *const u8;
    let l1 = strchrnul_u8(var, b'=') as usize - var as usize;
    if l1 == 0 || *var.add(l1) != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    if overwrite == 0 && !getenv(var as *const c_char).is_null() {
        return 0;
    }
    let l2 = strlen(value as *const c_char);
    let s = malloc(l1 + l2 + 2) as *mut u8;
    if s.is_null() {
        return -1;
    }
    core::ptr::copy_nonoverlapping(var, s, l1);
    *s.add(l1) = b'=';
    core::ptr::copy_nonoverlapping(value as *const u8, s.add(l1 + 1), l2 + 1);
    putenv_internal(s as *mut c_char, l1, s as *mut c_char)
}

unsafe fn putenv_internal(s: *mut c_char, l: usize, r: *mut c_char) -> c_int {
    let mut i = 0;
    if !__environ.is_null() {
        let mut e = __environ;
        while !(*e).is_null() {
            if strncmp(s as *const u8, *e as *const u8, l + 1) == 0 {
                let tmp = *e;
                *e = s;
                env_rm_add(tmp, r);
                return 0;
            }
            e = e.add(1);
            i += 1;
        }
    }
    let newenv = malloc(core::mem::size_of::<*mut c_char>() * (i + 2)) as *mut *mut c_char;
    if newenv.is_null() {
        if !r.is_null() { free(r as *mut c_void); }
        return -1;
    }
    if i > 0 && !__environ.is_null() {
        core::ptr::copy_nonoverlapping(__environ, newenv, i);
    }
    *newenv.add(i) = s;
    *newenv.add(i + 1) = core::ptr::null_mut();
    // don't free old __environ (may not be from our malloc)
    __environ = newenv;
    environ = __environ;
    if !r.is_null() {
        env_rm_add(core::ptr::null_mut(), r);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn putenv(s: *mut c_char) -> c_int {
    if s.is_null() { return -1; }
    let l = strchrnul_u8(s as *const u8, b'=') as usize - s as *const u8 as usize;
    if l == 0 || *s.add(l) as u8 == 0 {
        return unsetenv(s);
    }
    putenv_internal(s, l, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn unsetenv(name: *const c_char) -> c_int {
    if name.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let name = name as *const u8;
    let l = strchrnul_u8(name, b'=') as usize - name as usize;
    if l == 0 || *name.add(l) != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    if __environ.is_null() {
        return 0;
    }
    let mut e = __environ;
    let mut eo = e;
    while !(*e).is_null() {
        if strncmp(name, *e as *const u8, l) == 0 && *(*e as *const u8).add(l) == b'=' {
            env_rm_add(*e, core::ptr::null_mut());
        } else {
            if eo != e {
                *eo = *e;
            }
            eo = eo.add(1);
        }
        e = e.add(1);
    }
    if eo != e {
        *eo = core::ptr::null_mut();
    }
    environ = __environ;
    0
}

#[no_mangle]
pub unsafe extern "C" fn clearenv() -> c_int {
    let e = __environ;
    __environ = core::ptr::null_mut();
    environ = core::ptr::null_mut();
    if !e.is_null() {
        let mut p = e;
        while !(*p).is_null() {
            env_rm_add(*p, core::ptr::null_mut());
            p = p.add(1);
        }
    }
    0
}

// ============================================================
// random / srandom / initstate / setstate (BSD lagged fibonacci)
// musl serializes the process-global state with a lock; keel currently
// does not, so concurrent callers must provide their own serialization.
// ============================================================

static mut RANDOM_INIT: [u32; 32] = [
    0x00000000,0x5851f42d,0xc0b18ccf,0xcbb5f646,
    0xc7033129,0x30705b04,0x20fd5db4,0x9a8b7f78,
    0x502959d8,0xab894868,0x6c0356a7,0x88cdb7ff,
    0xb477d43f,0x70a3a52b,0xa8e4baf1,0xfd8341fc,
    0x8ae16fd9,0x742d2f7a,0x0d1f0796,0x76035e09,
    0x40f7702c,0x6fa72ca5,0xaaa84157,0x58a0df74,
    0xc74a0364,0xae533cc4,0x04185faf,0x6de3b115,
    0x0cab8628,0xf043bfa4,0x398150e9,0x37521657,
];

static mut RANDOM_N: i32 = 31;
static mut RANDOM_I: i32 = 3;
static mut RANDOM_J: i32 = 0;
static mut RANDOM_X: *mut u32 = unsafe { core::ptr::addr_of_mut!(RANDOM_INIT).cast::<u32>().add(1) };

unsafe fn random_lcg31(x: u32) -> u32 {
    (1103515245u32.wrapping_mul(x).wrapping_add(12345)) & 0x7fffffff
}

unsafe fn random_lcg64(x: u64) -> u64 {
    6364136223846793005u64.wrapping_mul(x).wrapping_add(1)
}

unsafe fn random_savestate() -> *mut u8 {
    (*RANDOM_X.offset(-1)) = ((RANDOM_N as u32) << 16) | ((RANDOM_I as u32) << 8) | (RANDOM_J as u32);
    RANDOM_X.offset(-1) as *mut u8
}

unsafe fn random_loadstate(state: *mut u32) {
    RANDOM_X = state.add(1);
    RANDOM_N = ((*RANDOM_X.offset(-1)) >> 16) as i32;
    RANDOM_I = (((*RANDOM_X.offset(-1)) >> 8) & 0xff) as i32;
    RANDOM_J = ((*RANDOM_X.offset(-1)) & 0xff) as i32;
}

unsafe fn random_srandom_inner(seed: u32) {
    let mut s = seed as u64;
    if RANDOM_N == 0 {
        *RANDOM_X = s as u32;
        return;
    }
    RANDOM_I = if RANDOM_N == 31 || RANDOM_N == 7 { 3 } else { 1 };
    RANDOM_J = 0;
    for k in 0..RANDOM_N as usize {
        s = random_lcg64(s);
        *RANDOM_X.add(k) = (s >> 32) as u32;
    }
    *RANDOM_X |= 1;
}

#[no_mangle]
pub unsafe extern "C" fn srandom(seed: c_uint) {
    random_srandom_inner(seed);
}

#[no_mangle]
pub unsafe extern "C" fn initstate(seed: c_uint, state: *mut c_char, size: usize) -> *mut c_char {
    if size < 8 { return core::ptr::null_mut(); }
    let old = random_savestate();
    RANDOM_N = if size < 32 { 0 }
        else if size < 64 { 7 }
        else if size < 128 { 15 }
        else if size < 256 { 31 }
        else { 63 };
    RANDOM_X = (state as *mut u32).add(1);
    random_srandom_inner(seed);
    random_savestate();
    old as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn setstate(state: *mut c_char) -> *mut c_char {
    let old = random_savestate();
    random_loadstate(state as *mut u32);
    old as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn random() -> c_long {
    let k: c_long;
    if RANDOM_N == 0 {
        *RANDOM_X = random_lcg31(*RANDOM_X);
        k = *RANDOM_X as c_long;
    } else {
        *RANDOM_X.add(RANDOM_I as usize) = (*RANDOM_X.add(RANDOM_I as usize)).wrapping_add(*RANDOM_X.add(RANDOM_J as usize));
        k = (*RANDOM_X.add(RANDOM_I as usize) >> 1) as c_long;
        RANDOM_I += 1;
        if RANDOM_I == RANDOM_N { RANDOM_I = 0; }
        RANDOM_J += 1;
        if RANDOM_J == RANDOM_N { RANDOM_J = 0; }
    }
    k
}

include!("mbyte.rs");
include!("wchar.rs");
include!("wstdio.rs");
include!("time.rs");
// ============================================================
// system()
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn system(cmd: *const c_char) -> c_int {
    if cmd.is_null() {
        // test shell availability via fork+exec
        let pid = sys_fork();
        if pid == 0 {
            let sh = b"/bin/sh\0".as_ptr() as *const c_char;
            let dash_c = b"-c\0".as_ptr() as *const c_char;
            let exit0 = b"exit 0\0".as_ptr() as *const c_char;
            let argv = [b"sh\0".as_ptr() as *const c_char, dash_c, exit0, core::ptr::null()];
            sys_execve(sh, argv.as_ptr(), __environ as *const *const c_char);
            _exit(127);
        }
        if pid < 0 { return -1; }
        let mut status: c_int = 0;
        loop {
            let r = sys_wait4(pid as c_int, &mut status, 0, core::ptr::null_mut());
            if r >= 0 { return status; }
            if r != -4 { return -1; } // not EINTR
        }
    }

    let sa_ignore = KSigAction {
        sa_handler: SIG_IGN,
        sa_flags: SA_RESTORER,
        sa_restorer: sig_restorer as *const () as usize,
        sa_mask: [0],
    };
    let mut oldint: KSigAction = core::mem::zeroed();
    let mut oldquit: KSigAction = core::mem::zeroed();

    sys_rt_sigaction(SIGINT, &sa_ignore, &mut oldint, 8);
    sys_rt_sigaction(SIGQUIT, &sa_ignore, &mut oldquit, 8);

    let mut block_set: SigSetT = 0;
    block_set |= 1u64 << (SIGCHLD - 1);
    let mut oldmask: SigSetT = 0;
    sys_rt_sigprocmask(SIG_BLOCK, &block_set, &mut oldmask, 8);

    let pid = sys_fork();

    if pid == 0 {
        sys_rt_sigaction(SIGINT, &oldint, core::ptr::null_mut(), 8);
        sys_rt_sigaction(SIGQUIT, &oldquit, core::ptr::null_mut(), 8);
        sys_rt_sigprocmask(SIG_SETMASK, &oldmask, core::ptr::null_mut(), 8);

        let sh = b"/bin/sh\0".as_ptr() as *const c_char;
        let argv = [
            b"sh\0".as_ptr() as *const c_char,
            b"-c\0".as_ptr() as *const c_char,
            cmd,
            core::ptr::null(),
        ];
        sys_execve(sh, argv.as_ptr(), __environ as *const *const c_char);
        _exit(127);
    }

    let mut status: c_int = -1;
    if pid > 0 {
        loop {
            let r = sys_wait4(pid as c_int, &mut status, 0, core::ptr::null_mut());
            if r >= 0 { break; }
            if r != -4 { break; } // not EINTR
        }
    }

    sys_rt_sigaction(SIGINT, &oldint, core::ptr::null_mut(), 8);
    sys_rt_sigaction(SIGQUIT, &oldquit, core::ptr::null_mut(), 8);
    sys_rt_sigprocmask(SIG_SETMASK, &oldmask, core::ptr::null_mut(), 8);

    status
}

// ============================================================
// errno: strerror / strerror_r / environ alias
// ============================================================

// static error string table indexed by errno
static ERR_STRS: [&[u8]; 134] = [
    b"Success\0",                     // 0
    b"Operation not permitted\0",     // 1
    b"No such file or directory\0",   // 2
    b"No such process\0",             // 3
    b"Interrupted system call\0",     // 4
    b"Input/output error\0",          // 5
    b"No such device or address\0",   // 6
    b"Argument list too long\0",      // 7
    b"Exec format error\0",           // 8
    b"Bad file descriptor\0",         // 9
    b"No child processes\0",          // 10
    b"Resource temporarily unavailable\0", // 11
    b"Cannot allocate memory\0",      // 12
    b"Permission denied\0",           // 13
    b"Bad address\0",                 // 14
    b"Block device required\0",       // 15
    b"Device or resource busy\0",     // 16
    b"File exists\0",                 // 17
    b"Invalid cross-device link\0",   // 18
    b"No such device\0",              // 19
    b"Not a directory\0",             // 20
    b"Is a directory\0",              // 21
    b"Invalid argument\0",            // 22
    b"Too many open files in system\0", // 23
    b"Too many open files\0",         // 24
    b"Inappropriate ioctl for device\0", // 25
    b"Text file busy\0",              // 26
    b"File too large\0",              // 27
    b"No space left on device\0",     // 28
    b"Illegal seek\0",                // 29
    b"Read-only file system\0",       // 30
    b"Too many links\0",              // 31
    b"Broken pipe\0",                 // 32
    b"Numerical argument out of domain\0", // 33
    b"Numerical result out of range\0", // 34
    b"Resource deadlock avoided\0",   // 35
    b"File name too long\0",          // 36
    b"No locks available\0",          // 37
    b"Function not implemented\0",    // 38
    b"Directory not empty\0",         // 39
    b"Too many levels of symbolic links\0", // 40
    b"Unknown error 41\0",            // 41
    b"No message of desired type\0",  // 42
    b"Identifier removed\0",          // 43
    b"Channel number out of range\0", // 44
    b"Level 2 not synchronized\0",    // 45
    b"Level 3 halted\0",              // 46
    b"Level 3 reset\0",               // 47
    b"Link number out of range\0",    // 48
    b"Protocol driver not attached\0", // 49
    b"No CSI structure available\0",  // 50
    b"Level 2 halted\0",              // 51
    b"Invalid exchange\0",            // 52
    b"Invalid request descriptor\0",  // 53
    b"Exchange full\0",               // 54
    b"No anode\0",                    // 55
    b"Invalid request code\0",        // 56
    b"Invalid slot\0",                // 57
    b"Unknown error 58\0",            // 58
    b"Bad font file format\0",        // 59
    b"Device not a stream\0",         // 60
    b"No data available\0",           // 61
    b"Timer expired\0",               // 62
    b"Out of streams resources\0",    // 63
    b"Machine is not on the network\0", // 64
    b"Package not installed\0",       // 65
    b"Object is remote\0",            // 66
    b"Link has been severed\0",       // 67
    b"Advertise error\0",             // 68
    b"Srmount error\0",               // 69
    b"Communication error on send\0", // 70
    b"Protocol error\0",              // 71
    b"Multihop attempted\0",          // 72
    b"RFS specific error\0",          // 73
    b"Bad message\0",                 // 74
    b"Value too large for defined data type\0", // 75
    b"Name not unique on network\0",  // 76
    b"File descriptor in bad state\0", // 77
    b"Remote address changed\0",      // 78
    b"Can not access a needed shared library\0", // 79
    b"Accessing a corrupted shared library\0", // 80
    b".lib section in a.out corrupted\0", // 81
    b"Attempting to link in too many shared libraries\0", // 82
    b"Cannot exec a shared library directly\0", // 83
    b"Invalid or incomplete multibyte or wide character\0", // 84
    b"Interrupted system call should be restarted\0", // 85
    b"Streams pipe error\0",          // 86
    b"Too many users\0",              // 87
    b"Socket operation on non-socket\0", // 88
    b"Destination address required\0", // 89
    b"Message too long\0",            // 90
    b"Protocol wrong type for socket\0", // 91
    b"Protocol not available\0",      // 92
    b"Protocol not supported\0",      // 93
    b"Socket type not supported\0",   // 94
    b"Operation not supported\0",     // 95
    b"Protocol family not supported\0", // 96
    b"Address family not supported by protocol\0", // 97
    b"Address already in use\0",      // 98
    b"Cannot assign requested address\0", // 99
    b"Network is down\0",             // 100
    b"Network is unreachable\0",      // 101
    b"Network dropped connection on reset\0", // 102
    b"Software caused connection abort\0", // 103
    b"Connection reset by peer\0",    // 104
    b"No buffer space available\0",   // 105
    b"Transport endpoint is already connected\0", // 106
    b"Transport endpoint is not connected\0", // 107
    b"Cannot send after transport endpoint shutdown\0", // 108
    b"Too many references: cannot splice\0", // 109
    b"Connection timed out\0",        // 110
    b"Connection refused\0",          // 111
    b"Host is down\0",                // 112
    b"No route to host\0",            // 113
    b"Operation already in progress\0", // 114
    b"Operation now in progress\0",   // 115
    b"Stale file handle\0",           // 116
    b"Structure needs cleaning\0",    // 117
    b"Not a XENIX named type file\0", // 118
    b"No XENIX semaphores available\0", // 119
    b"Is a named type file\0",        // 120
    b"Remote I/O error\0",            // 121
    b"Quota exceeded\0",              // 122
    b"No medium found\0",             // 123
    b"Wrong medium type\0",           // 124
    b"Operation canceled\0",          // 125
    b"Required key not available\0",  // 126
    b"Key has expired\0",             // 127
    b"Key has been revoked\0",        // 128
    b"Key was rejected by service\0", // 129
    b"Owner died\0",                  // 130
    b"State not recoverable\0",       // 131
    b"Operation not possible due to RF-kill\0", // 132
    b"Memory page has hardware error\0", // 133
];

#[no_mangle]
pub unsafe extern "C" fn strerror(errnum: c_int) -> *mut c_char {
    if errnum < 0 || errnum as usize >= ERR_STRS.len() {
        // static buf for unknown errors
        static mut UNKNOWN_BUF: [c_char; 64] = [0; 64];
        let prefix = b"Unknown error \0";
        let mut i = 0;
        while prefix[i] != 0 {
            UNKNOWN_BUF[i] = prefix[i] as c_char;
            i += 1;
        }
        let (buf, len) = format_u64(errnum.unsigned_abs() as u64);
        for j in 0..len {
            UNKNOWN_BUF[i] = buf[20 - len + j] as c_char;
            i += 1;
        }
        UNKNOWN_BUF[i] = 0;
        return core::ptr::addr_of_mut!(UNKNOWN_BUF).cast::<c_char>();
    }
    ERR_STRS[errnum as usize].as_ptr() as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: usize) -> c_int {
    let s = strerror(errnum);
    let slen = strlen(s as *const c_char);
    if slen >= buflen {
        if buflen > 0 {
            core::ptr::copy_nonoverlapping(s as *const u8, buf as *mut u8, buflen - 1);
            *buf.add(buflen - 1) = 0;
        }
        return 34; // ERANGE
    }
    core::ptr::copy_nonoverlapping(s as *const u8, buf as *mut u8, slen + 1);
    0
}

// Alias for programs that declare `extern char **environ`.
#[no_mangle]
pub static mut environ: *mut *mut c_char = core::ptr::null_mut();

// keep environ in sync with __environ
unsafe fn sync_environ() {
    environ = __environ;
}

include!("unistd_ext.rs");
include!("libgen.rs");
include!("inet.rs");
include!("strtod.rs");
include!("stdio_mem.rs");
// ============================================================
// Stat (for ftok)
// ============================================================

#[repr(C)]
struct kernel_stat64 {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: c_int,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
    __unused: [i64; 3],
}

// sys_stat = 4 on x86_64
#[inline]
unsafe fn sys_stat(path: *const u8, statbuf: *mut kernel_stat64) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_NEWFSTATAT, AT_FDCWD as i64, path as i64, statbuf as i64, 0)
}

include!("search.rs");
include!("mntent.rs");
include!("sysvipc.rs");
// ============================================================
// regex.h
// ============================================================

// musl defines regoff_t as _Addr (long on every Linux target), not int.
type regoff_t = c_long;

#[repr(C)]
pub struct regex_t {
    pub re_nsub: usize,
    pub __opaque: *mut c_void,
    pub __padding: [*mut c_void; 4],
    pub __nsub2: usize,
    pub __padding2: c_char,
}

#[repr(C)]
pub struct regmatch_t {
    pub rm_so: regoff_t,
    pub rm_eo: regoff_t,
}

include!("regex.rs");
include!("tz.rs");

// The dynamic loader is part of libc, mirroring musl's single-file design.
// It is compiled as a module so its private syscall/string helpers do not
// collide with the libc namespace; its exported entry points remain global.
#[allow(dead_code, deref_nullptr)]
mod ldso;

include!("math_complex.rs");
// ============================================================
// Wide char functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn wcscasecmp(s1: *const u32, s2: *const u32) -> i32 {
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        let l1 = if c1 >= 'A' as u32 && c1 <= 'Z' as u32 { c1 + 32 } else { c1 };
        let l2 = if c2 >= 'A' as u32 && c2 <= 'Z' as u32 { c2 + 32 } else { c2 };
        if l1 != l2 { return (l1 as i32) - (l2 as i32); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscasecmp_l(s1: *const u32, s2: *const u32, _loc: *mut core::ffi::c_void) -> i32 {
    wcscasecmp(s1, s2)
}

#[no_mangle]
pub unsafe extern "C" fn wcsncasecmp(s1: *const u32, s2: *const u32, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        let l1 = if c1 >= 'A' as u32 && c1 <= 'Z' as u32 { c1 + 32 } else { c1 };
        let l2 = if c2 >= 'A' as u32 && c2 <= 'Z' as u32 { c2 + 32 } else { c2 };
        if l1 != l2 { return (l1 as i32) - (l2 as i32); }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn wcsncasecmp_l(s1: *const u32, s2: *const u32, n: usize, _loc: *mut core::ffi::c_void) -> i32 {
    wcsncasecmp(s1, s2, n)
}

#[no_mangle]
pub unsafe extern "C" fn wcscoll(s1: *const u32, s2: *const u32) -> i32 {
    // musl's C locale compares wide characters by codepoint.
    let mut i = 0;
    loop {
        let c1 = *s1.add(i);
        let c2 = *s2.add(i);
        if c1 == 0 || c2 == 0 { return (c1 as i32) - (c2 as i32); }
        if c1 != c2 { return (c1 as i32) - (c2 as i32); }
        i += 1;
    }
}

unsafe fn wc_in_set(c: u32, set: *const u32) -> bool {
    let mut i = 0usize;
    while *set.add(i) != 0 {
        if *set.add(i) == c { return true; }
        i += 1;
    }
    false
}

#[no_mangle]
pub unsafe extern "C" fn wcstok(s: *mut u32, sep: *const u32, p: *mut *mut u32) -> *mut u32 {
    if sep.is_null() || p.is_null() { return core::ptr::null_mut(); }
    let mut src = if s.is_null() {
        if (*p).is_null() { return core::ptr::null_mut(); }
        *p
    } else { s };
    while *src != 0 && wc_in_set(*src, sep) { src = src.add(1); }
    if *src == 0 { return core::ptr::null_mut(); }
    let start = src;
    while *src != 0 && !wc_in_set(*src, sep) { src = src.add(1); }
    if *src != 0 {
        *src = 0;
        *p = src.add(1);
    } else {
        *p = src;
    }
    start
}

#[no_mangle]
pub unsafe extern "C" fn wcswcs(haystack: *const u32, needle: *const u32) -> *mut u32 {
    if *needle == 0 { return haystack as *mut u32; }
    let mut i = 0;
    while *haystack.add(i) != 0 {
        let mut j = 0;
        while *needle.add(j) != 0 && *haystack.add(i + j) == *needle.add(j) {
            j += 1;
        }
        if *needle.add(j) == 0 { return haystack.add(i) as *mut u32; }
        i += 1;
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn wcswidth(s: *const u32, n: usize) -> i32 {
    let mut width = 0i32;
    let mut i = 0usize;
    while i < n {
        let c = *s.add(i);
        if c == 0 { break; }
        let w = wcwidth(c);
        if w < 0 { return w; }
        width += w;
        i += 1;
    }
    width
}

#[no_mangle]
pub unsafe extern "C" fn wcsftime(s: *mut u32, maxsize: usize, fmt: *const u32, tm: *const c_void) -> usize {
    if s.is_null() || fmt.is_null() || maxsize == 0 { return 0; }
    // Convert the wide format string to multibyte, let strftime do the work,
    // then convert the result back. Both buffers scale with the inputs.
    let wlen = wcslen(fmt as *const wchar_t);
    let cfmt_cap = match wlen.checked_mul(4).and_then(|v| v.checked_add(1)) {
        Some(v) => v,
        None => return 0,
    };
    let cfmt = crate::malloc(cfmt_cap) as *mut u8;
    if cfmt.is_null() { ERRNO!() = ENOMEM; return 0; }
    let mut o = 0usize;
    let mut i = 0usize;
    let mut st: c_uint = 0;
    while *fmt.add(i) != 0 {
        let n = wcrtomb(
            cfmt.add(o) as *mut c_char,
            *fmt.add(i) as c_int,
            &mut st as *mut c_uint,
        );
        if n == !0usize || o + n >= cfmt_cap {
            crate::free(cfmt as *mut c_void);
            return 0;
        }
        o += n;
        i += 1;
    }
    *cfmt.add(o) = 0;
    let out_cap = match maxsize.checked_mul(4).and_then(|v| v.checked_add(1)) {
        Some(v) => v,
        None => { crate::free(cfmt as *mut c_void); return 0; }
    };
    let out = crate::malloc(out_cap) as *mut u8;
    if out.is_null() {
        crate::free(cfmt as *mut c_void);
        ERRNO!() = ENOMEM;
        return 0;
    }
    let n = strftime(out as *mut c_char, out_cap, cfmt as *const c_char, tm as *const tm);
    crate::free(cfmt as *mut c_void);
    if n == 0 {
        crate::free(out as *mut c_void);
        return 0;
    }
    let mut ws = 0usize;
    let mut p = 0usize;
    while p < n && ws + 1 < maxsize {
        let mut wc: c_int = 0;
        let mut st2: c_uint = 0;
        let r = mbrtowc(&mut wc, out.add(p) as *const c_char, n - p, &mut st2);
        if r == !0usize {
            crate::free(out as *mut c_void);
            return 0;
        }
        if r == 0 || r == !1usize { break; }
        *s.add(ws) = wc as u32;
        ws += 1;
        p += r;
    }
    *s.add(ws) = 0;
    crate::free(out as *mut c_void);
    ws
}

#[no_mangle]
pub unsafe extern "C" fn mbsnrtowcs(
    dst: *mut u32,
    src: *mut *const u8,
    nms: usize,
    len: usize,
    st: *mut core::ffi::c_void,
) -> usize {
    if src.is_null() { return 0; }
    let mut s = *src;
    if s.is_null() { return 0; }
    let state = st as *mut c_uint;
    let limit = if dst.is_null() { usize::MAX } else { len };
    let mut count = 0usize;
    let mut n = nms;
    while count < limit && n > 0 {
        let c = *s;
        if c == 0 {
            if !dst.is_null() {
                if count < len { *dst.add(count) = 0; }
                *src = core::ptr::null();
            }
            return count;
        }
        let mut wc: c_int = 0;
        let r = mbrtowc(&mut wc, s as *const c_char, n, state);
        if r == !0usize {
            *src = s;
            return !0usize;
        }
        if r == !1usize { break; }
        if !dst.is_null() { *dst.add(count) = wc as u32; }
        count += 1;
        s = s.add(r);
        n -= r;
    }
    *src = s;
    count
}

#[no_mangle]
pub unsafe extern "C" fn wcsnrtombs(dst: *mut u8, src: *mut *const u32, nwc: usize, len: usize, _st: *mut core::ffi::c_void) -> usize {
    // Port of musl's wcsnrtombs: nwc counts wide characters, len counts
    // output bytes, and a converted NUL terminates the source pointer.
    let mut ws = *src;
    let mut wn = nwc;
    let mut cnt = 0usize;
    let mut n = len;
    let mut d = dst;
    while !ws.is_null() && wn > 0 {
        let c = *ws;
        let mut tmp = [0u8; 8];
        let mut state: c_uint = 0;
        let l = wcrtomb(tmp.as_mut_ptr() as *mut c_char, c as c_int, &mut state);
        if l == !0usize {
            return !0usize;
        }
        if !d.is_null() {
            if l > n {
                break;
            }
            core::ptr::copy_nonoverlapping(tmp.as_ptr(), d, l);
            d = d.add(l);
            n -= l;
        }
        if c == 0 {
            ws = core::ptr::null();
            break;
        }
        ws = ws.add(1);
        wn -= 1;
        cnt += l;
    }
    if !dst.is_null() {
        *src = ws;
    }
    cnt
}

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
// exp10 functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn exp10(x: f64) -> f64 {
    libm::pow(10.0, x)
}

#[no_mangle]
pub unsafe extern "C" fn exp10f(x: f32) -> f32 {
    libm::powf(10.0, x)
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
// fpclassify functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn finite(x: f64) -> i32 {
    (!x.is_nan() && !x.is_infinite()) as i32
}

#[no_mangle]
pub unsafe extern "C" fn finitef(x: f32) -> i32 {
    (!x.is_nan() && !x.is_infinite()) as i32
}

#[no_mangle]
pub unsafe extern "C" fn __fpclassifyl(x: f64) -> i32 {
    __fpclassify(x)
}

#[no_mangle]
pub unsafe extern "C" fn __signbitl(x: f64) -> i32 {
    __signbit(x)
}

// ============================================================
// ftw functions
// ============================================================

// ============================================================
// Bessel functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn j0(x: f64) -> f64 {
    libm::j0(x)
}

#[no_mangle]
pub unsafe extern "C" fn j0f(x: f32) -> f32 {
    libm::j0f(x)
}

#[no_mangle]
pub unsafe extern "C" fn j1(x: f64) -> f64 {
    libm::j1(x)
}

#[no_mangle]
pub unsafe extern "C" fn j1f(x: f32) -> f32 {
    libm::j1f(x)
}

#[no_mangle]
pub unsafe extern "C" fn jn(n: i32, x: f64) -> f64 {
    libm::jn(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn jnf(n: i32, x: f32) -> f32 {
    libm::jnf(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn y0(x: f64) -> f64 {
    y0_impl(x)
}

#[no_mangle]
pub unsafe extern "C" fn y0f(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f32::NEG_INFINITY; }
    libm::y0f(x)
}

#[no_mangle]
pub unsafe extern "C" fn y1(x: f64) -> f64 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalid(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f64::NEG_INFINITY; }
    libm::y1(x)
}

#[no_mangle]
pub unsafe extern "C" fn y1f(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 { feraiseexcept(FE_DIVBYZERO); return f32::NEG_INFINITY; }
    libm::y1f(x)
}

#[no_mangle]
pub unsafe extern "C" fn yn(n: i32, x: f64) -> f64 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalid(x); }
    if x == 0.0 {
        feraiseexcept(FE_DIVBYZERO);
        if n >= 0 { return f64::NEG_INFINITY; }
    }
    libm::yn(n, x)
}

#[no_mangle]
pub unsafe extern "C" fn ynf(n: i32, x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x < 0.0 { return __math_invalidf(x); }
    if x == 0.0 {
        feraiseexcept(FE_DIVBYZERO);
        if n >= 0 { return f32::NEG_INFINITY; }
    }
    libm::ynf(n, x)
}

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
