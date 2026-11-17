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

// ============================================================
// signal.h
// ============================================================

pub const SA_RESTORER: c_ulong = 0x04000000;

pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;
pub const SIG_ERR: usize = !0usize;

pub const SIGHUP: c_int = 1;
pub const SIGINT: c_int = 2;
pub const SIGQUIT: c_int = 3;
pub const SIGILL: c_int = 4;
pub const SIGTRAP: c_int = 5;
pub const SIGABRT: c_int = 6;
pub const SIGIOT: c_int = 6;
pub const SIGBUS: c_int = 7;
pub const SIGFPE: c_int = 8;
pub const SIGKILL: c_int = 9;
pub const SIGUSR1: c_int = 10;
pub const SIGSEGV: c_int = 11;
pub const SIGUSR2: c_int = 12;
pub const SIGPIPE: c_int = 13;
pub const SIGALRM: c_int = 14;
pub const SIGTERM: c_int = 15;
pub const SIGSTKFLT: c_int = 16;
pub const SIGCHLD: c_int = 17;
pub const SIGCONT: c_int = 18;
pub const SIGSTOP: c_int = 19;
pub const SIGTSTP: c_int = 20;
pub const SIGTTIN: c_int = 21;
pub const SIGTTOU: c_int = 22;
pub const SIGURG: c_int = 23;
pub const SIGXCPU: c_int = 24;
pub const SIGXFSZ: c_int = 25;
pub const SIGVTALRM: c_int = 26;
pub const SIGPROF: c_int = 27;
pub const SIGWINCH: c_int = 28;
pub const SIGIO: c_int = 29;
pub const SIGPOLL: c_int = 29;
pub const SIGPWR: c_int = 30;
pub const SIGSYS: c_int = 31;
pub const SIGUNUSED: c_int = 31;
pub const _NSIG: c_int = 65;

pub const SIG_BLOCK: c_int = 0;
pub const SIG_UNBLOCK: c_int = 1;
pub const SIG_SETMASK: c_int = 2;

pub const SA_NOCLDSTOP: c_ulong = 1;
pub const SA_NOCLDWAIT: c_ulong = 2;
pub const SA_SIGINFO: c_ulong = 4;
pub const SA_ONSTACK: c_ulong = 0x08000000;
pub const SA_RESTART: c_ulong = 0x10000000;
pub const SA_NODEFER: c_ulong = 0x40000000;
pub const SA_RESETHAND: c_ulong = 0x80000000;

pub const SS_ONSTACK: c_int = 1;
pub const SS_DISABLE: c_int = 2;
#[cfg(target_arch = "aarch64")]
pub const MINSIGSTKSZ: usize = 6144;
#[cfg(target_arch = "aarch64")]
pub const SIGSTKSZ: usize = 12288;
#[cfg(not(target_arch = "aarch64"))]
pub const MINSIGSTKSZ: usize = 2048;
#[cfg(not(target_arch = "aarch64"))]
pub const SIGSTKSZ: usize = 8192;

pub const SI_USER: c_int = 0;
pub const SI_TKILL: c_int = -6;

#[repr(C)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

#[repr(C)]
pub struct siginfo_t {
    pub si_signo: c_int,
    pub si_errno: c_int,
    pub si_code: c_int,
    // The remaining bytes of the 128-byte siginfo_t are not accessed.
    _pad: [u8; 128 - 3 * core::mem::size_of::<c_int>()],
}

#[repr(C)]
pub struct stack_t {
    pub ss_sp: *mut c_void,
    pub ss_flags: c_int,
    pub ss_size: usize,
}

// Kernel ABI for rt_sigaction.  All supported architectures use the same
// handler/flags/restorer/8-byte-mask layout.
#[repr(C)]
pub struct KSigAction {
    pub sa_handler: usize,
    pub sa_flags: c_ulong,
    pub sa_restorer: usize,
    pub sa_mask: [c_ulong; 1],
}

// musl's public layout: handler, 128-byte sigset_t, flags, restorer.
#[repr(C)]
pub struct sigaction {
    pub sa_handler: usize,
    pub sa_mask: [u64; 16],
    pub sa_flags: c_int,
    pub sa_restorer: usize,
}

pub type SigSetT = c_ulong;

#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "mov eax, 15",
    "syscall",
);

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "mov x8, #139",
    "svc #0",
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".global sig_restorer",
    ".type sig_restorer, @function",
    "sig_restorer:",
    "li a7, 139",
    "ecall",
);

extern "C" {
    fn sig_restorer();
}

#[inline]
unsafe fn sys_rt_sigaction(sig: c_int,
    act: *const KSigAction,
    oldact: *mut KSigAction,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGACTION, sig as i64, act as i64, oldact as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigprocmask(how: c_int,
    set: *const SigSetT,
    oldset: *mut SigSetT,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGPROCMASK, how as i64, set as i64, oldset as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigpending(set: *mut SigSetT, sigsetsize: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_RT_SIGPENDING, set as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigsuspend(mask: *const SigSetT, sigsetsize: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_RT_SIGSUSPEND, mask as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_rt_sigtimedwait(set: *const SigSetT,
    info: *mut siginfo_t,
    timeout: *const timespec,
    sigsetsize: usize,) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_RT_SIGTIMEDWAIT, set as i64, info as i64, timeout as i64, sigsetsize as i64)
}

#[inline]
unsafe fn sys_sigaltstack(ss: *const stack_t, old_ss: *mut stack_t) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SIGALTSTACK, ss as i64, old_ss as i64)
}

#[inline]
unsafe fn sys_tgkill(tgid: c_int, tid: c_int, sig: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_TGKILL, tgid as i64, tid as i64, sig as i64)
}

const CLOCK_REALTIME: c_int = 0;
const CLOCK_MONOTONIC: c_int = 1;

#[inline]
unsafe fn sys_clock_gettime(clockid: c_int, ts: *mut timespec) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_CLOCK_GETTIME, clockid as i64, ts as i64)
}

// clock_gettime lives in vdso.rs: it prefers the kernel's vDSO entry point
// and only falls back to sys_clock_gettime() there.

#[no_mangle]
pub unsafe extern "C" fn sigprocmask(
    how: c_int,
    set: *const SigSetT,
    oldset: *mut SigSetT,
) -> c_int {
    // mask out internal rt signals (32, 33) before touching the kernel
    let internal_mask: SigSetT = (1u64 << 31) | (1u64 << 32);
    let filtered: SigSetT;
    let set_ptr: *const SigSetT = if set.is_null() {
        set
    } else {
        filtered = *set & !internal_mask;
        &filtered
    };
    let r = sys_rt_sigprocmask(how, set_ptr, oldset, core::mem::size_of::<SigSetT>());
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    if !oldset.is_null() {
        *oldset &= !internal_mask;
    }
    0
}

#[inline]
unsafe fn sys_kill(pid: c_int, sig: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_KILL, pid as i64, sig as i64)
}

#[inline]
unsafe fn sys_getpid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETPID)
}

#[no_mangle]
pub unsafe extern "C" fn sigaction(
    signum: c_int,
    act: *const sigaction,
    oldact: *mut sigaction,
) -> c_int {
    let mut kact: KSigAction = core::mem::zeroed();
    let act_ptr = if act.is_null() {
        core::ptr::null()
    } else {
        kact.sa_handler = (*act).sa_handler;
        kact.sa_flags = ((*act).sa_flags as c_ulong) | SA_RESTORER;
        kact.sa_restorer = if (*act).sa_flags as c_ulong & SA_RESTORER != 0
            && (*act).sa_restorer != 0
        {
            (*act).sa_restorer
        } else {
            sig_restorer as *const () as usize
        };
        kact.sa_mask[0] = (*act).sa_mask[0];
        &kact
    };
    let mut kold: KSigAction = core::mem::zeroed();
    let old_ptr = if oldact.is_null() { core::ptr::null_mut() } else { &mut kold };
    let r = sys_rt_sigaction(signum, act_ptr, old_ptr, 8);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    if !oldact.is_null() {
        (*oldact).sa_handler = kold.sa_handler;
        (*oldact).sa_flags = kold.sa_flags as c_int;
        (*oldact).sa_restorer = kold.sa_restorer;
        (*oldact).sa_mask = [0; 16];
        (*oldact).sa_mask[0] = kold.sa_mask[0];
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn signal(signum: c_int, handler: usize) -> usize {
    // musl's signal() installs the handler with SA_RESTART, so interrupted
    // syscalls (waitpid, read, ...) are restarted automatically.
    let act = sigaction {
        sa_handler: handler,
        sa_mask: [0; 16],
        sa_flags: SA_RESTART as c_int,
        sa_restorer: 0,
    };
    let mut old: sigaction = core::mem::zeroed();
    if sigaction(signum, &act, &mut old) == -1 {
        SIG_ERR
    } else {
        old.sa_handler
    }
}

#[no_mangle]
pub unsafe extern "C" fn kill(pid: c_int, sig: c_int) -> c_int {
    let r = sys_kill(pid, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn tgkill(tgid: c_int, tid: c_int, sig: c_int) -> c_int {
    let r = sys_tgkill(tgid, tid, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getpid() -> c_int {
    sys_getpid() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn raise(sig: c_int) -> c_int {
    let r = sys_tgkill(sys_getpid() as c_int, sys_gettid() as c_int, sig);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigemptyset(set: *mut SigSetT) -> c_int {
    *set = 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigfillset(set: *mut SigSetT) -> c_int {
    // musl: all signals except 32 (SIGCANCEL) and 33 (SIGSYNCCALL)
    *set = 0xFFFF_FFFC_7FFF_FFFFu64;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigaddset(set: *mut SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 {
        ERRNO!() = EINVAL;
        return -1;
    }
    // internal rt signals (32, 33) cannot be manipulated
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    *set |= 1u64 << s;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigdelset(set: *mut SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 {
        ERRNO!() = EINVAL;
        return -1;
    }
    // internal rt signals (32, 33) are never members
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    *set &= !(1u64 << s);
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigismember(set: *const SigSetT, signum: c_int) -> c_int {
    let s = (signum as c_uint).wrapping_sub(1);
    if s as usize >= 64 { return 0; }
    // internal rt signals (32, 33) are never members
    if signum >= 32 && signum < __libc_current_sigrtmin() {
        return 0;
    }
    ((*set & (1u64 << s)) != 0) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigpending(set: *mut SigSetT) -> c_int {
    let r = sys_rt_sigpending(set, core::mem::size_of::<SigSetT>());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sigsuspend(mask: *const SigSetT) -> c_int {
    let r = sys_rt_sigsuspend(mask, core::mem::size_of::<SigSetT>());
    if r < 0 {
        ERRNO!() = (-r) as c_int;
    }
    -1
}

#[no_mangle]
pub unsafe extern "C" fn sigtimedwait(
    mask: *const SigSetT,
    info: *mut siginfo_t,
    timeout: *const timespec,
) -> c_int {
    loop {
        let r = sys_rt_sigtimedwait(mask, info, timeout, core::mem::size_of::<SigSetT>());
        if r >= 0 {
            return r as c_int;
        }
        let e = (-r) as c_int;
        if e != EINTR {
            ERRNO!() = e;
            return -1;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sigwaitinfo(
    mask: *const SigSetT,
    info: *mut siginfo_t,
) -> c_int {
    sigtimedwait(mask, info, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn sigwait(
    mask: *const SigSetT,
    sig: *mut c_int,
) -> c_int {
    let mut info: siginfo_t = core::mem::zeroed();
    if sigtimedwait(mask, &mut info, core::ptr::null()) < 0 {
        return ERRNO!();
    }
    *sig = info.si_signo;
    0
}

#[no_mangle]
pub unsafe extern "C" fn sigaltstack(ss: *const stack_t, old_ss: *mut stack_t) -> c_int {
    if !ss.is_null() {
        if (*ss).ss_flags & SS_ONSTACK != 0 {
            ERRNO!() = EINVAL;
            return -1;
        }
        if (*ss).ss_flags & SS_DISABLE == 0 && (*ss).ss_size < MINSIGSTKSZ {
            ERRNO!() = ENOMEM;
            return -1;
        }
    }
    let r = sys_sigaltstack(ss, old_ss);
    syscall_ret(r) as c_int
}

include!("setjmp.rs");
// ============================================================
// syscall wrappers: fstat/newfstatat/getrlimit/setrlimit/utimensat
// ============================================================

#[inline]
unsafe fn sys_newfstatat(dirfd: i32, path: *const c_char, buf: *mut u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_NEWFSTATAT, dirfd as i64, path as i64, buf as i64, flags as i64)
}

#[inline]
unsafe fn sys_fstat(fd: i32, buf: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_FSTAT, fd as i64, buf as i64)
}

// Linux removed the legacy getrlimit/setrlimit entry points on some
// architectures (qemu-ppc64le reports syscall 76 as unknown); prlimit64
// with pid 0 provides the same semantics.
#[cfg(any(target_arch = "powerpc64", target_arch = "loongarch64"))]
#[inline]
unsafe fn sys_getrlimit(resource: i32, rlim: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_PRLIMIT64, 0, resource as i64, 0, rlim as i64)
}

#[cfg(any(target_arch = "powerpc64", target_arch = "loongarch64"))]
#[inline]
unsafe fn sys_setrlimit(resource: i32, rlim: *const u8) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_PRLIMIT64, 0, resource as i64, rlim as i64, 0)
}

#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
#[inline]
unsafe fn sys_getrlimit(resource: i32, rlim: *mut u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_GETRLIMIT, resource as i64, rlim as i64)
}

#[cfg(not(any(target_arch = "powerpc64", target_arch = "loongarch64")))]
#[inline]
unsafe fn sys_setrlimit(resource: i32, rlim: *const u8) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SETRLIMIT, resource as i64, rlim as i64)
}

#[inline]
unsafe fn sys_utimensat(dirfd: i32, path: *const c_char, times: *const u8, flags: i32) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_UTIMENSAT, dirfd as i64, path as i64, times as i64, flags as i64)
}

// ============================================================
// unistd.h: process primitives
// ============================================================

#[inline]
unsafe fn sys_fork() -> i64 {
    #[cfg(target_arch = "x86_64")]
    { <Arch as Syscalls>::syscall0(SYS_FORK) }
    #[cfg(target_arch = "aarch64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
    #[cfg(target_arch = "riscv64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
    #[cfg(target_arch = "powerpc64")]
    { <Arch as Syscalls>::syscall0(SYS_FORK) }
    #[cfg(target_arch = "loongarch64")]
    { <Arch as Syscalls>::syscall2(SYS_CLONE, 17, 0) }
}


#[inline]
unsafe fn sys_execve(path: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_EXECVE, path as i64, argv as i64, envp as i64)
}

#[inline]
unsafe fn sys_wait4(pid: c_int, status: *mut c_int, options: c_int, rusage: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_WAIT4, pid as i64, status as i64, options as i64, rusage as i64)
}

#[inline]
unsafe fn sys_getppid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETPPID)
}

#[inline]
unsafe fn sys_getuid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETUID)
}

#[inline]
unsafe fn sys_getgid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETGID)
}

#[inline]
unsafe fn sys_geteuid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETEUID)
}

#[inline]
unsafe fn sys_getegid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETEGID)
}

#[no_mangle]
pub unsafe extern "C" fn fork() -> c_int {
    __fork_handler(-1);
    let ret = sys_fork();
    // Run the atfork handlers before publishing errno, exactly like musl's
    // __syscall_ret: a failing fork must report the kernel's errno even if a
    // handler changes errno. RLIMIT_NPROC is enforced by the kernel and does
    // not apply to root, so do not second-guess it here.
    if ret < 0 {
        let err = (-ret) as c_int;
        __fork_handler(0);
        ERRNO!() = err;
        return -1;
    }
    if ret == 0 {
        allocator_after_fork();
        thread_allocator_after_fork();
        __fork_handler(1);
    } else {
        __fork_handler(0);
    }
    ret as c_int
}

#[no_mangle]
pub unsafe extern "C" fn execve(
    path: *const c_char,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let r = sys_execve(path, argv, envp);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn wait(status: *mut c_int) -> c_int {
    let r = sys_wait4(-1, status, 0, core::ptr::null_mut());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int {
    let r = sys_wait4(pid, status, options, core::ptr::null_mut());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getppid() -> c_int {
    sys_getppid() as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getuid() -> c_uint {
    sys_getuid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn getgid() -> c_uint {
    sys_getgid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn geteuid() -> c_uint {
    sys_geteuid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn getegid() -> c_uint {
    sys_getegid() as c_uint
}

#[no_mangle]
pub unsafe extern "C" fn vfork() -> c_int {
    // keel uses the portable fork semantics; fork() also runs the atfork
    // handlers and resets the allocator/thread caches in the child.
    fork()
}

#[no_mangle]
pub unsafe extern "C" fn execl(path: *const c_char, arg: *const c_char, mut args: ...) -> c_int {
    let mut cap = 16usize;
    let mut argv = malloc(cap * core::mem::size_of::<*const c_char>()) as *mut *const c_char;
    if argv.is_null() { ERRNO!() = ENOMEM; return -1; }
    *argv.add(0) = arg;
    let mut n: usize = 1;
    loop {
        let a: *const c_char = args.next_arg();
        if a.is_null() { break; }
        if n + 1 >= cap {
            cap = match cap.checked_mul(2) { Some(v) => v, None => { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; } };
            let grown = realloc(argv as *mut c_void, cap * core::mem::size_of::<*const c_char>()) as *mut *const c_char;
            if grown.is_null() { free(argv as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            argv = grown;
        }
        *argv.add(n) = a;
        n += 1;
    }
    *argv.add(n) = core::ptr::null();
    let r = sys_execve(path, argv, __environ as *const *const c_char);
    free(argv as *mut c_void);
    syscall_ret(r) as c_int
}

// ============================================================
// spawn.h: posix_spawn family
// ============================================================

// Matches musl's posix_spawn_file_actions_t layout.  Actions are a growable
// linked list; PATH search uses a stack buffer and an O(n) scan per entry.
unsafe fn spawn_execvp(file: *const c_char, argv: *const *const c_char, envp: *const *const c_char) -> ! {
    let flen = strlen(file as *const c_char);
    let f = file as *const u8;
    let mut has_slash = false;
    for i in 0..flen {
        if *f.add(i) == b'/' { has_slash = true; break; }
    }
    if has_slash {
        sys_execve(file, argv, envp);
        _exit(127);
    }
    let path = getenv(b"PATH\0".as_ptr() as *const c_char);
    let default_path = b"/usr/local/bin:/usr/bin:/bin\0";
    let p = if path.is_null() { default_path.as_ptr() } else { path as *const u8 };
    let plen = strlen(p as *const c_char);
    let mut buf = [0u8; PATH_MAX_V];
    let mut i: usize = 0;
    while i <= plen {
        let start = i;
        while i < plen && *p.add(i) != b':' { i += 1; }
        let dlen = i - start;
        if dlen > 0 && dlen + 1 + flen < PATH_MAX_V {
            core::ptr::copy_nonoverlapping(p.add(start), buf.as_mut_ptr(), dlen);
            *buf.as_mut_ptr().add(dlen) = b'/';
            core::ptr::copy_nonoverlapping(f, buf.as_mut_ptr().add(dlen + 1), flen + 1);
            sys_execve(buf.as_ptr() as *const c_char, argv, envp);
            // execve failed, try next
        }
        i += 1; // skip ':'
    }
    _exit(127);
}

#[no_mangle]
pub unsafe extern "C" fn posix_spawnp(
    pid: *mut c_int,
    file: *const c_char,
    fa: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    let child = sys_fork();
    if child < 0 {
        ERRNO!() = (-child) as c_int;
        return -1;
    }
    if child == 0 {
        // child: apply attributes and file actions, then exec
        spawn_apply_attr(attrp);
        spawn_apply_actions(fa);
        let actual_envp = if envp.is_null() { __environ as *const *const c_char } else { envp };
        spawn_execvp(file, argv, actual_envp);
    }
    *pid = child as c_int;
    0
}

#[no_mangle]
pub unsafe extern "C" fn posix_spawn(
    pid: *mut c_int,
    path: *const c_char,
    fa: *const posix_spawn_file_actions_t,
    attrp: *const posix_spawnattr_t,
    argv: *const *const c_char,
    envp: *const *const c_char,
) -> c_int {
    // posix_spawnp() skips PATH search when the path contains '/', so it is
    // also the correct implementation of posix_spawn().
    posix_spawnp(pid, path, fa, attrp, argv, envp)
}

// ============================================================
// sys/stat.h: stat / fstat / utimensat / futimens
// ============================================================

#[cfg(any(target_arch = "x86_64", target_arch = "powerpc64"))]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    _pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    _unused: [i64; 3],
}

#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "loongarch64"))]
#[repr(C)]
pub struct Stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_mode: u32,
    pub st_nlink: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    __pad1: u64,
    pub st_size: i64,
    pub st_blksize: i32,
    __pad2: i32,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    __unused4: u32,
    __unused5: u32,
}

pub const S_IFMT: u32 = 0o170000;
pub const S_IFDIR: u32 = 0o040000;
pub const S_IFCHR: u32 = 0o020000;
pub const S_IFBLK: u32 = 0o060000;
pub const S_IFREG: u32 = 0o100000;
pub const S_IFLNK: u32 = 0o120000;
pub const S_IFIFO: u32 = 0o010000;
pub const S_IFSOCK: u32 = 0o140000;

pub const AT_FDCWD: i32 = -100;

const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
const AT_EACCESS: c_int = 0x200;
const AT_REMOVEDIR: c_int = 0x200;

#[no_mangle]
pub unsafe extern "C" fn stat(path: *const c_char, buf: *mut Stat) -> c_int {
    let r = sys_newfstatat(AT_FDCWD, path, buf as *mut u8, 0);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fstat(fd: c_int, buf: *mut Stat) -> c_int {
    let r = sys_fstat(fd, buf as *mut u8);
    syscall_ret(r) as c_int
}

// ============================================================
// sys/resource.h: getrlimit / setrlimit
// ============================================================

#[repr(C)]
pub struct Rlimit {
    pub rlim_cur: u64,
    pub rlim_max: u64,
}

pub const RLIMIT_NOFILE: c_int = 7;
pub const RLIMIT_STACK: c_int = 3;
pub const RLIMIT_DATA: c_int = 2;
pub const RLIMIT_AS: c_int = 9;
pub const RLIMIT_CPU: c_int = 0;
pub const RLIMIT_FSIZE: c_int = 1;
pub const RLIMIT_CORE: c_int = 4;
pub const RLIMIT_RSS: c_int = 5;
pub const RLIMIT_NPROC: c_int = 6;
pub const RLIMIT_MEMLOCK: c_int = 8;
pub const RLIMIT_LOCKS: c_int = 10;
pub const RLIMIT_SIGPENDING: c_int = 11;
pub const RLIMIT_MSGQUEUE: c_int = 12;
pub const RLIMIT_NICE: c_int = 13;
pub const RLIMIT_RTPRIO: c_int = 14;
pub const RLIMIT_RTTIME: c_int = 15;
pub const RLIM_INFINITY: u64 = !0u64;

#[no_mangle]
pub unsafe extern "C" fn getrlimit(resource: c_int, rlim: *mut Rlimit) -> c_int {
    let r = sys_getrlimit(resource, rlim as *mut u8);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setrlimit(resource: c_int, rlim: *const Rlimit) -> c_int {
    let r = sys_setrlimit(resource, rlim as *const u8);
    syscall_ret(r) as c_int
}

// ============================================================
// utimensat / futimens
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn utimensat(dirfd: c_int, path: *const c_char, times: *const timespec, flags: c_int) -> c_int {
    let r = sys_utimensat(dirfd, path, times as *const u8, flags);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn futimens(fd: c_int, times: *const timespec) -> c_int {
    let r = sys_utimensat(fd, core::ptr::null(), times as *const u8, 0);
    syscall_ret(r) as c_int
}

include!("socket.rs");
include!("shm_sem.rs");
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

// ============================================================
// multibyte (UTF-8, musl state machine)
// ============================================================

const SA: u8 = 0xC2;
const SB: u8 = 0xF4;

const BITTAB: [u32; 51] = [
    0xC000_0002, 0xC000_0003, 0xC000_0004, 0xC000_0005, 0xC000_0006, 0xC000_0007,
    0xC000_0008, 0xC000_0009, 0xC000_000A, 0xC000_000B, 0xC000_000C, 0xC000_000D,
    0xC000_000E, 0xC000_000F, 0xC000_0010, 0xC000_0011, 0xC000_0012, 0xC000_0013,
    0xC000_0014, 0xC000_0015, 0xC000_0016, 0xC000_0017, 0xC000_0018, 0xC000_0019,
    0xC000_001A, 0xC000_001B, 0xC000_001C, 0xC000_001D, 0xC000_001E, 0xC000_001F,
    0xB300_0000, 0xC300_0001, 0xC300_0002, 0xC300_0003, 0xC300_0004, 0xC300_0005,
    0xC300_0006, 0xC300_0007, 0xC300_0008, 0xC300_0009, 0xC300_000A, 0xC300_000B,
    0xC300_000C, 0xD300_000D, 0xC300_000E, 0xC300_000F, 0xBB0C_0000, 0xC30C_0001,
    0xC30C_0002, 0xC30C_0003, 0xDB0C_0004,
];

#[inline]
fn mb_oob(c: u32, b: u8) -> u32 {
    let b3 = (b >> 3) as u32;
    ((b3.wrapping_sub(0x10)) | (b3.wrapping_add((c as i32 >> 26) as u32))) & !7
}

static mut MB_STATE: c_uint = 0;

/// Select the caller's multibyte state or the process-global fallback used
/// when `st` is null.
#[inline]
unsafe fn mb_state_or(st: *mut c_uint) -> &'static mut c_uint {
    if st.is_null() { &mut *(&raw mut MB_STATE) } else { &mut *st }
}

const MB_CUR_MAX_VAL_UTF8: usize = 4;
const MB_CUR_MAX_VAL_C: usize = 1;

#[no_mangle]
pub extern "C" fn __ctype_get_mb_cur_max() -> usize {
    unsafe { if LOCALE_CTYPE_UTF8 { MB_CUR_MAX_VAL_UTF8 } else { MB_CUR_MAX_VAL_C } }
}

#[no_mangle]
pub unsafe extern "C" fn mbrtowc(
    wc: *mut c_int,
    src: *const c_char,
    n: usize,
    st: *mut c_uint,
) -> usize {
    let s = src as *const u8;
    let n0 = n;
    let mut dummy: c_int = 0;
    let wc = if wc.is_null() { &mut dummy } else { &mut *wc };
    let st = mb_state_or(st);

    let mut c: u32 = *st;
    let mut s = s;
    let mut n = n;

    if s.is_null() {
        if c != 0 { *st = 0; ERRNO!() = EILSEQ; return !0usize; }
        return 0;
    }

    if n == 0 { return !1usize; }

    if unsafe { !LOCALE_CTYPE_UTF8 } {
        let b = *s;
        *wc = c_locale_codeunit(b) as c_int;
        return if b == 0 { 0 } else { 1 };
    }

    if c == 0 {
        if *s < 0x80 {
            let r = if *s == 0 { 0 } else { 1 };
            *wc = *s as c_int;
            return r;
        }
        if (*s).wrapping_sub(SA) as u32 > (SB - SA) as u32 {
            *st = 0; ERRNO!() = EILSEQ; return !0usize;
        }
        c = BITTAB[*s as usize - SA as usize];
        s = s.add(1);
        n -= 1;
    }

    if n > 0 {
        if mb_oob(c, *s) != 0 {
            *st = 0; ERRNO!() = EILSEQ; return !0usize;
        }
        loop {
            c = (c << 6) | (*s as u32).wrapping_sub(0x80);
            s = s.add(1);
            n -= 1;
            if c & (1u32 << 31) == 0 {
                *st = 0;
                *wc = c as c_int;
                return n0 - n;
            }
            if n == 0 { break; }
            if (*s).wrapping_sub(0x80) >= 0x40 {
                *st = 0; ERRNO!() = EILSEQ; return !0usize;
            }
        }
    }

    *st = c;
    !1usize
}

#[no_mangle]
pub unsafe extern "C" fn wcrtomb(
    s: *mut c_char,
    wc: c_int,
    _st: *mut c_uint,
) -> usize {
    if s.is_null() { return 1; }
    let wc_u = wc as u32;
    if unsafe { !LOCALE_CTYPE_UTF8 } {
        if wc_u < 0x80 {
            *s = wc as c_char;
            return 1;
        }
        if c_locale_is_codeunit(wc_u) {
            *s = (wc_u & 0xFF) as c_char;
            return 1;
        }
        ERRNO!() = EILSEQ;
        return !0usize;
    }
    if wc_u < 0x80 {
        *s = wc as c_char;
        return 1;
    }
    if wc_u < 0x800 {
        *s = (0xC0 | (wc_u >> 6)) as c_char;
        *s.add(1) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 2;
    }
    if wc_u < UNICODE_SURROGATE_START || wc_u.wrapping_sub(UNICODE_SURROGATE_END_EXCL) < 0x2000 {
        *s = (0xE0 | (wc_u >> 12)) as c_char;
        *s.add(1) = (0x80 | ((wc_u >> 6) & 0x3F)) as c_char;
        *s.add(2) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 3;
    }
    if wc_u.wrapping_sub(UNICODE_SUPPLEMENTARY_BASE) < UNICODE_SUPPLEMENTARY_SIZE {
        *s = (0xF0 | (wc_u >> 18)) as c_char;
        *s.add(1) = (0x80 | ((wc_u >> 12) & 0x3F)) as c_char;
        *s.add(2) = (0x80 | ((wc_u >> 6) & 0x3F)) as c_char;
        *s.add(3) = (0x80 | (wc_u & 0x3F)) as c_char;
        return 4;
    }
    ERRNO!() = EILSEQ;
    !0usize
}

#[no_mangle]
pub unsafe extern "C" fn mblen(s: *const c_char, n: usize) -> c_int {
    if s.is_null() { return 0; }
    mbtowc(core::ptr::null_mut(), s, n)
}

#[no_mangle]
pub unsafe extern "C" fn mbtowc(
    wc: *mut c_int,
    src: *const c_char,
    n: usize,
) -> c_int {
    if src.is_null() { return 0; }
    if n == 0 { ERRNO!() = EILSEQ; return -1; }
    let mut state: c_uint = 0;
    let mut w: c_int = 0;
    let r = mbrtowc(&mut w, src, n, &mut state);
    if r == !0usize { return -1; }
    if !wc.is_null() { *wc = w; }
    r as c_int
}

#[no_mangle]
pub unsafe extern "C" fn wctomb(s: *mut c_char, wc: c_int) -> c_int {
    if s.is_null() { return 0; }
    wcrtomb(s, wc, core::ptr::null_mut()) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn mbsrtowcs(
    ws: *mut c_int,
    src: *mut *const c_char,
    wn: usize,
    st: *mut c_uint,
) -> usize {
    let st = mb_state_or(st);
    let mut s = *src as *const u8;
    let wn0 = wn;
    let mut wn = wn;

    if ws.is_null() {
        while *s != 0 {
            if *s < 0x80 {
                s = s.add(1);
                wn = wn.wrapping_sub(1);
                continue;
            }
            let mut wc: c_int = 0;
            let r = mbrtowc(&mut wc, s as *const c_char, 4, st);
            if r == !0usize { return !0usize; }
            if r == !1usize { return wn0 - wn; }
            s = s.add(r);
            wn = wn.wrapping_sub(1);
        }
        wn0 - wn
    } else {
        let mut w = ws;
        while wn > 0 {
            if *s < 0x80 {
                let c = *s as c_int;
                *w = c;
                if c == 0 {
                    *src = core::ptr::null();
                    return wn0 - wn;
                }
                w = w.add(1);
                s = s.add(1);
                wn -= 1;
                continue;
            }
            let mut wc: c_int = 0;
            let r = mbrtowc(&mut wc, s as *const c_char, 4, st);
            if r == !0usize {
                *src = s as *const c_char;
                return !0usize;
            }
            if r == !1usize {
                *src = s as *const c_char;
                return wn0 - wn;
            }
            *w = wc;
            w = w.add(1);
            s = s.add(r);
            wn -= 1;
        }
        *src = s as *const c_char;
        wn0 - wn
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsrtombs(
    s: *mut c_char,
    ws: *mut *const c_int,
    n: usize,
    _st: *mut c_uint,
) -> usize {
    let mut w = *ws;
    let n0 = n;
    let mut n = n;

    if s.is_null() {
        let mut count = 0usize;
        while *w != 0 {
            if (*w as u32) < 0x80 {
                count += 1;
            } else {
                let mut buf = [0u8; 4];
                let r = wcrtomb(buf.as_mut_ptr() as *mut c_char, *w, core::ptr::null_mut());
                if r == !0usize { return !0usize; }
                count += r;
            }
            w = w.add(1);
        }
        return count;
    }

    let mut dst = s;
    while n > 0 {
        if (*w as u32) < 0x80 {
            *dst = *w as c_char;
            if *w == 0 {
                *ws = core::ptr::null();
                return n0 - n;
            }
            dst = dst.add(1);
            n -= 1;
        } else {
            let r = wcrtomb(dst, *w, core::ptr::null_mut());
            if r == !0usize {
                *ws = w;
                return !0usize;
            }
            if r > n {
                *ws = w;
                return n0 - n;
            }
            dst = dst.add(r);
            n -= r;
        }
        w = w.add(1);
    }
    *ws = w;
    n0
}

#[no_mangle]
pub unsafe extern "C" fn mbstowcs(
    ws: *mut c_int,
    src: *const c_char,
    wn: usize,
) -> usize {
    let mut src_ptr = src;
    mbsrtowcs(ws, &mut src_ptr, wn, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn wcstombs(
    s: *mut c_char,
    ws: *const c_int,
    n: usize,
) -> usize {
    let mut ws_ptr = ws;
    wcsrtombs(s, &mut ws_ptr, n, core::ptr::null_mut())
}

// ============================================================
// wchar: btowc / wctob / mbsinit / mbrlen
// ============================================================

// musl's C locale represents bytes >= 0x80 as the single-byte "code units"
// 0xDF80..0xDFFF so they are distinct from Unicode code points.
#[inline]
fn c_locale_codeunit(b: u8) -> u32 {
    0xdfff & (b as i8 as i32) as u32
}

#[inline]
fn c_locale_is_codeunit(c: u32) -> bool {
    c.wrapping_sub(0xDF80) < 0x80
}

#[no_mangle]
pub extern "C" fn btowc(c: c_int) -> wint_t {
    if c == -1 { return WEOF; }
    unsafe {
        if LOCALE_CTYPE_UTF8 {
            if (c as u32) < 128 { c as wint_t } else { WEOF }
        } else {
            c_locale_codeunit(c as u8) as wint_t
        }
    }
}

#[no_mangle]
pub extern "C" fn wctob(c: wint_t) -> c_int {
    unsafe {
        if LOCALE_CTYPE_UTF8 {
            if c < 128 { c as c_int } else { -1 }
        } else if c < 128 {
            c as c_int
        } else if c_locale_is_codeunit(c) {
            (c & 0xFF) as c_int
        } else {
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn mbsinit(s: *const c_uint) -> c_int {
    if s.is_null() || *s == 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn mbrlen(s: *const c_char, n: usize, ps: *mut c_uint) -> usize {
    mbrtowc(core::ptr::null_mut(), s, n, ps)
}

// ============================================================
// wchar: wide string functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn wcslen(s: *const wchar_t) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 { len += 1; }
    len
}

#[no_mangle]
pub unsafe extern "C" fn wcschr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let ch = *s.add(i);
        if ch == c { return s.add(i) as *mut wchar_t; }
        if ch == 0 { return null_mut(); }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsrchr(s: *const wchar_t, c: wchar_t) -> *mut wchar_t {
    let mut i = 0;
    let mut last: *mut wchar_t = null_mut();
    loop {
        let ch = *s.add(i);
        if ch == c { last = s.add(i) as *mut wchar_t; }
        if ch == 0 { return last; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsstr(haystack: *const wchar_t, needle: *const wchar_t) -> *mut wchar_t {
    if *needle == 0 { return haystack as *mut wchar_t; }
    let mut i = 0;
    loop {
        let h = *haystack.add(i);
        if h == 0 { return null_mut(); }
        if h == *needle {
            let mut j = 0;
            loop {
                let n = *needle.add(j);
                if n == 0 { return haystack.add(i) as *mut wchar_t; }
                if *haystack.add(i + j) != n { break; }
                j += 1;
            }
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscmp(s1: *const wchar_t, s2: *const wchar_t) -> c_int {
    let mut i = 0;
    loop {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b { return a - b; }
        if a == 0 { return 0; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsncmp(s1: *const wchar_t, s2: *const wchar_t, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b { return a - b; }
        if a == 0 { return 0; }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn wcscpy(dst: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let c = *src.add(i);
        *dst.add(i) = c;
        if c == 0 { break; }
        i += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsncpy(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t {
    let mut i = 0;
    while i < n && *src.add(i) != 0 { *dst.add(i) = *src.add(i); i += 1; }
    while i < n { *dst.add(i) = 0; i += 1; }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcscat(dst: *mut wchar_t, src: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    while *dst.add(i) != 0 { i += 1; }
    let mut j = 0;
    loop {
        let c = *src.add(j);
        *dst.add(i + j) = c;
        if c == 0 { break; }
        j += 1;
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsncat(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> *mut wchar_t {
    let mut i = 0;
    while *dst.add(i) != 0 { i += 1; }
    let mut j = 0;
    while j < n && *src.add(j) != 0 { *dst.add(i + j) = *src.add(j); j += 1; }
    *dst.add(i + j) = 0;
    dst
}

#[no_mangle]
pub unsafe extern "C" fn wcsdup(s: *const wchar_t) -> *mut wchar_t {
    let len = wcslen(s);
    let p = malloc((len + 1) * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
    if p.is_null() { return null_mut(); }
    core::ptr::copy_nonoverlapping(s, p, len + 1);
    p
}

#[no_mangle]
pub unsafe extern "C" fn wcsnlen(s: *const wchar_t, maxlen: usize) -> usize {
    let mut i = 0;
    while i < maxlen && *s.add(i) != 0 { i += 1; }
    i
}

#[no_mangle]
pub unsafe extern "C" fn wcsspn(s: *const wchar_t, accept: *const wchar_t) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return i; }
        let mut found = false;
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 { break; }
            if c == a { found = true; break; }
            j += 1;
        }
        if !found { return i; }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcscspn(s: *const wchar_t, reject: *const wchar_t) -> usize {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return i; }
        let mut j = 0;
        loop {
            let r = *reject.add(j);
            if r == 0 { break; }
            if c == r { return i; }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcspbrk(s: *const wchar_t, accept: *const wchar_t) -> *mut wchar_t {
    let mut i = 0;
    loop {
        let c = *s.add(i);
        if c == 0 { return null_mut(); }
        let mut j = 0;
        loop {
            let a = *accept.add(j);
            if a == 0 { break; }
            if c == a { return s.add(i) as *mut wchar_t; }
            j += 1;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn wcsxfrm(dst: *mut wchar_t, src: *const wchar_t, n: usize) -> usize {
    let len = wcslen(src);
    if !dst.is_null() {
        let copy = if len < n { len } else { if n > 0 { n - 1 } else { 0 } };
        for i in 0..copy { *dst.add(i) = *src.add(i); }
        if n > 0 { *dst.add(copy) = 0; }
    }
    len
}

// ============================================================
// wchar: wcsto* number conversions
// ============================================================

unsafe fn wcs_to_bytes(ws: *const wchar_t, buf: *mut u8, bufsz: usize) -> usize {
    let mut i = 0;
    while i < bufsz - 1 {
        let c = *ws.add(i);
        if c == 0 { break; }
        if (c as u32) > 127 { break; }
        *buf.add(i) = c as u8;
        i += 1;
    }
    *buf.add(i) = 0;
    i
}

macro_rules! wcsto_int {
    ($name:ident, $ret:ty, $conv:ident) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            s: *const wchar_t,
            endptr: *mut *mut wchar_t,
            base: c_int,
        ) -> $ret {
            let cap = wcslen(s).saturating_add(1);
            let heap = cap > 256;
            let mut stack = [0u8; 256];
            let buf = if heap {
                crate::malloc(cap) as *mut u8
            } else {
                stack.as_mut_ptr()
            };
            if buf.is_null() {
                ERRNO!() = ENOMEM;
                return 0;
            }
            wcs_to_bytes(s, buf, cap);
            let mut end: *mut c_char = core::ptr::null_mut();
            let r = $conv(buf as *const c_char, &mut end, base);
            if !endptr.is_null() {
                let offset = end as usize - buf as usize;
                *endptr = s.add(offset) as *mut wchar_t;
            }
            if heap {
                crate::free(buf as *mut c_void);
            }
            r
        }
    };
}

wcsto_int!(wcstol, c_long, strtol);
wcsto_int!(wcstoul, c_ulong, strtoul);
wcsto_int!(wcstoll, c_longlong, strtoll);
wcsto_int!(wcstoull, c_ulonglong, strtoull);

#[no_mangle]
pub unsafe extern "C" fn wcstod(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 {
    let cap = wcslen(s).saturating_add(1);
    let heap = cap > 256;
    let mut stack = [0u8; 256];
    let buf = if heap {
        crate::malloc(cap) as *mut u8
    } else {
        stack.as_mut_ptr()
    };
    if buf.is_null() {
        ERRNO!() = ENOMEM;
        return 0.0;
    }
    wcs_to_bytes(s, buf, cap);
    let mut end: *mut c_char = core::ptr::null_mut();
    let r = strtod(buf as *const c_char, &mut end);
    if !endptr.is_null() {
        let offset = end as usize - buf as usize;
        *endptr = s.add(offset) as *mut wchar_t;
    }
    if heap {
        crate::free(buf as *mut c_void);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn wcstof(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f32 { wcstod(s, endptr) as f32 }

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f64 { wcstod(s, endptr) }

#[no_mangle]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f128 { f64_to_f128(wcstod(s, endptr)) }

#[no_mangle]
#[cfg(any(target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
pub unsafe extern "C" fn wcstold(s: *const wchar_t, endptr: *mut *mut wchar_t) -> f128 { f64_to_f128(wcstod(s, endptr)) }

#[no_mangle]
pub unsafe extern "C" fn wcstoimax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> c_longlong { wcstoll(s, endptr, base) }

#[no_mangle]
pub unsafe extern "C" fn wcstoumax(s: *const wchar_t, endptr: *mut *mut wchar_t, base: c_int) -> c_ulonglong { wcstoull(s, endptr, base) }

// ============================================================
// wctype: classification functions
// ============================================================

const WC_ALPHA: c_int = 1;
const WC_DIGIT: c_int = 2;
const WC_SPACE: c_int = 4;
const WC_UPPER: c_int = 8;
const WC_LOWER: c_int = 16;
const WC_ALNUM: c_int = WC_ALPHA | WC_DIGIT;
const WC_BLANK: c_int = 32;
const WC_CNTRL: c_int = 64;
const WC_PUNCT: c_int = 128;
const WC_GRAPH: c_int = WC_ALNUM | WC_PUNCT;
const WC_PRINT: c_int = WC_GRAPH | WC_BLANK;
const WC_XDIGIT: c_int = 256;

#[inline]
fn wctable_bit(table: &[u8], wc: wint_t) -> bool {
    let wc = wc as u32;
    let page = table[(wc >> 8) as usize] as usize;
    (table[page * 32 + ((wc & 255) >> 3) as usize] >> (wc & 7)) & 1 != 0
}

/// Port of musl's iswalpha: table lookup below U+20000, then a broad
/// alphabetic range up to U+2FFFD.
#[no_mangle]
pub extern "C" fn iswalpha(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0x20000 {
        return wctable_bit(&WCTYPE_ALPHA_TABLE, c) as c_int;
    }
    if wc < 0x2fffe { 1 } else { 0 }
}

/// Port of musl's iswpunct.
#[no_mangle]
pub extern "C" fn iswpunct(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0x20000 {
        wctable_bit(&WCTYPE_PUNCT_TABLE, c) as c_int
    } else {
        0
    }
}

/// Port of musl's iswprint.
#[no_mangle]
pub extern "C" fn iswprint(c: wint_t) -> c_int {
    let wc = c as u32;
    if wc < 0xff {
        return (((wc + 1) & 0x7f) >= 0x21) as c_int;
    }
    if wc < 0x2028
        || wc.wrapping_sub(0x202a) < 0xd800 - 0x202a
        || wc.wrapping_sub(0xe000) < 0xfff9 - 0xe000
    {
        return 1;
    }
    if wc.wrapping_sub(0xfffc) > 0x10ffff - 0xfffc || (wc & 0xfffe) == 0xfffe {
        return 0;
    }
    1
}

/// Port of musl's iswspace: Unicode White_Space minus U+00A0/U+2007/U+202F,
/// plus U+1680 and U+180E are excluded (they have non-blank glyphs).
#[no_mangle]
pub extern "C" fn iswspace(c: wint_t) -> c_int {
    matches!(
        c as u32,
        0x20 | 0x09 | 0x0a | 0x0d | 0x0b | 0x0c | 0x85
            | 0x2000..=0x2006 | 0x2008..=0x200a
            | 0x2028 | 0x2029 | 0x205f | 0x3000
    ) as c_int
}

/// Port of musl's iswcntrl.
#[no_mangle]
pub extern "C" fn iswcntrl(c: wint_t) -> c_int {
    let wc = c as u32;
    ((wc < 32)
        || wc.wrapping_sub(0x7f) < 33
        || wc.wrapping_sub(0x2028) < 2
        || wc.wrapping_sub(0xfff9) < 3) as c_int
}

#[no_mangle]
pub extern "C" fn iswdigit(c: wint_t) -> c_int {
    ((c as u32).wrapping_sub(b'0' as u32) < 10) as c_int
}

#[no_mangle]
pub extern "C" fn iswxdigit(c: wint_t) -> c_int {
    let wc = c as u32;
    ((wc.wrapping_sub(b'0' as u32) < 10) || ((wc | 32).wrapping_sub(b'a' as u32) < 6)) as c_int
}

#[no_mangle]
pub extern "C" fn iswalnum(c: wint_t) -> c_int {
    (iswdigit(c) != 0 || iswalpha(c) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn iswblank(c: wint_t) -> c_int {
    (c == b' ' as wint_t || c == b'\t' as wint_t) as c_int
}

#[no_mangle]
pub extern "C" fn iswgraph(c: wint_t) -> c_int {
    (iswspace(c) == 0 && iswprint(c) != 0) as c_int
}

#[no_mangle]
pub extern "C" fn iswlower(c: wint_t) -> c_int {
    (unsafe { towupper(c) } != c) as c_int
}

#[no_mangle]
pub extern "C" fn iswupper(c: wint_t) -> c_int {
    (unsafe { towlower(c) } != c) as c_int
}

static WCTYPE_ALNUM: c_int = WC_ALNUM;
static WCTYPE_ALPHA: c_int = WC_ALPHA;
static WCTYPE_BLANK: c_int = WC_BLANK;
static WCTYPE_CNTRL: c_int = WC_CNTRL;
static WCTYPE_DIGIT: c_int = WC_DIGIT;
static WCTYPE_GRAPH: c_int = WC_GRAPH;
static WCTYPE_LOWER: c_int = WC_LOWER;
static WCTYPE_PRINT: c_int = WC_PRINT;
static WCTYPE_PUNCT: c_int = WC_PUNCT;
static WCTYPE_SPACE: c_int = WC_SPACE;
static WCTYPE_UPPER: c_int = WC_UPPER;
static WCTYPE_XDIGIT: c_int = WC_XDIGIT;

#[no_mangle]
pub unsafe extern "C" fn wctype(name: *const c_char) -> wctype_t {
    if name.is_null() { return core::ptr::null(); }
    let n = name as *const u8;
    if strcmp(n, b"alnum\0".as_ptr()) == 0 { return &WCTYPE_ALNUM; }
    if strcmp(n, b"alpha\0".as_ptr()) == 0 { return &WCTYPE_ALPHA; }
    if strcmp(n, b"blank\0".as_ptr()) == 0 { return &WCTYPE_BLANK; }
    if strcmp(n, b"cntrl\0".as_ptr()) == 0 { return &WCTYPE_CNTRL; }
    if strcmp(n, b"digit\0".as_ptr()) == 0 { return &WCTYPE_DIGIT; }
    if strcmp(n, b"graph\0".as_ptr()) == 0 { return &WCTYPE_GRAPH; }
    if strcmp(n, b"lower\0".as_ptr()) == 0 { return &WCTYPE_LOWER; }
    if strcmp(n, b"print\0".as_ptr()) == 0 { return &WCTYPE_PRINT; }
    if strcmp(n, b"punct\0".as_ptr()) == 0 { return &WCTYPE_PUNCT; }
    if strcmp(n, b"space\0".as_ptr()) == 0 { return &WCTYPE_SPACE; }
    if strcmp(n, b"upper\0".as_ptr()) == 0 { return &WCTYPE_UPPER; }
    if strcmp(n, b"xdigit\0".as_ptr()) == 0 { return &WCTYPE_XDIGIT; }
    core::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn iswctype(c: wint_t, desc: wctype_t) -> c_int {
    if desc.is_null() { return 0; }
    if desc == core::ptr::addr_of!(WCTYPE_ALNUM) { return iswalnum(c); }
    if desc == core::ptr::addr_of!(WCTYPE_ALPHA) { return iswalpha(c); }
    if desc == core::ptr::addr_of!(WCTYPE_BLANK) { return iswblank(c); }
    if desc == core::ptr::addr_of!(WCTYPE_CNTRL) { return iswcntrl(c); }
    if desc == core::ptr::addr_of!(WCTYPE_DIGIT) { return iswdigit(c); }
    if desc == core::ptr::addr_of!(WCTYPE_GRAPH) { return iswgraph(c); }
    if desc == core::ptr::addr_of!(WCTYPE_LOWER) { return iswlower(c); }
    if desc == core::ptr::addr_of!(WCTYPE_PRINT) { return iswprint(c); }
    if desc == core::ptr::addr_of!(WCTYPE_PUNCT) { return iswpunct(c); }
    if desc == core::ptr::addr_of!(WCTYPE_SPACE) { return iswspace(c); }
    if desc == core::ptr::addr_of!(WCTYPE_UPPER) { return iswupper(c); }
    if desc == core::ptr::addr_of!(WCTYPE_XDIGIT) { return iswxdigit(c); }
    0
}

static WCTRANS_LOWER: c_int = 1;
static WCTRANS_UPPER: c_int = 2;

/// Port of musl's casemap() from ctype/towctrans.c.
fn casemap(c: wint_t, dir: i32) -> wint_t {
    let c0 = c;
    if c0 >= 0x20000 {
        return c0;
    }
    let b = (c0 >> 8) as usize;
    let cc = c0 & 255;
    let x = (cc / 3) as usize;
    let y = (cc % 3) as usize;
    const MT: [u32; 3] = [2048, 342, 57];
    let v = WCTYPE_CASEMAP_TAB[WCTYPE_CASEMAP_TAB[b] as usize * 86 + x] as u32;
    let v = (v * MT[y] >> 11) % 6;
    let r = WCTYPE_CASEMAP_RULES[(WCTYPE_CASEMAP_RULEBASES[b] + v as u8) as usize];
    let rt = (r & 255) as i32;
    let rd = r >> 8;
    if rt < 2 {
        return (c0 as i32 + (rd & -((rt) ^ dir))) as wint_t;
    }
    let mut xn = (rd & 0xff) as u32;
    let mut xb = (rd as u32) >> 8;
    while xn != 0 {
        let idx = (xb + xn / 2) as usize;
        let try_ = WCTYPE_CASEMAP_EXCEPTIONS[idx][0] as u32;
        if try_ == cc {
            let r2 = WCTYPE_CASEMAP_RULES[WCTYPE_CASEMAP_EXCEPTIONS[idx][1] as usize];
            let rt2 = (r2 & 255) as i32;
            let rd2 = r2 >> 8;
            if rt2 < 2 {
                return (c0 as i32 + (rd2 & -((rt2) ^ dir))) as wint_t;
            }
            return (c0 as i32 + if dir != 0 { -1 } else { 1 }) as wint_t;
        } else if try_ > cc {
            xn /= 2;
        } else {
            xb += xn / 2;
            xn -= xn / 2;
        }
    }
    c0
}

#[no_mangle]
pub unsafe extern "C" fn towlower(c: wint_t) -> wint_t {
    casemap(c, 0)
}

#[no_mangle]
pub unsafe extern "C" fn towupper(c: wint_t) -> wint_t {
    casemap(c, 1)
}

#[no_mangle]
pub unsafe extern "C" fn wctrans(name: *const c_char) -> wctrans_t {
    if name.is_null() { return core::ptr::null(); }
    let n = name as *const u8;
    if strcmp(n, b"tolower\0".as_ptr()) == 0 { return &WCTRANS_LOWER; }
    if strcmp(n, b"toupper\0".as_ptr()) == 0 { return &WCTRANS_UPPER; }
    core::ptr::null()
}

#[no_mangle]
pub unsafe extern "C" fn towctrans(c: wint_t, desc: wctrans_t) -> wint_t {
    if desc.is_null() { return c; }
    match *desc {
        1 => towlower(c),
        2 => towupper(c),
        _ => c,
    }
}

// ============================================================
// wchar: wide stdio
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fgetwc(f: *mut FILE) -> wint_t {
    let mut state: c_uint = 0;
    let mut wc: wchar_t = 0;
    let mut buf = [0u8; 4];
    let c = fgetc(f);
    if c == -1 { return WEOF; }
    buf[0] = c as u8;
    if buf[0] < 0x80 { return buf[0] as wint_t; }
    let len = if buf[0] < 0xE0 { 2 } else if buf[0] < 0xF0 { 3 } else { 4 };
    let mut n = 1usize;
    while n < len {
        let c2 = fgetc(f);
        if c2 == -1 {
            // Put the partial sequence back so a later read sees it.
            while n > 1 {
                n -= 1;
                ungetc(buf[n] as c_int, f);
            }
            return WEOF;
        }
        buf[n] = c2 as u8;
        n += 1;
    }
    let r = mbrtowc(&mut wc, buf.as_ptr() as *const c_char, n, &mut state);
    if r == !0usize {
        ERRNO!() = EILSEQ;
        WEOF
    } else if r == !1usize {
        while n > 0 {
            n -= 1;
            ungetc(buf[n] as c_int, f);
        }
        WEOF
    } else {
        wc as wint_t
    }
}

#[no_mangle]
pub unsafe extern "C" fn fputwc(c: wchar_t, f: *mut FILE) -> wint_t {
    let mut buf = [0u8; 4];
    let n = wcrtomb(buf.as_mut_ptr() as *mut c_char, c, core::ptr::null_mut());
    if n == !0usize || n == 0 { return WEOF; }
    if fwrite(buf.as_ptr() as *const c_void, 1, n, f) != n { return WEOF; }
    c as wint_t
}

#[no_mangle]
pub unsafe extern "C" fn fputws(s: *const wchar_t, f: *mut FILE) -> c_int {
    let mut i = 0;
    while *s.add(i) != 0 {
        if fputwc(*s.add(i), f) == WEOF { return -1; }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t {
    if c == WEOF { return WEOF; }
    let mut buf = [0u8; 4];
    let n = wcrtomb(buf.as_mut_ptr() as *mut c_char, c as wchar_t, core::ptr::null_mut());
    if n == !0usize || n == 0 { return WEOF; }
    let mut i = n;
    while i > 0 {
        i -= 1;
        if ungetc(buf[i] as c_int, f) == -1 { return WEOF; }
    }
    c
}

#[no_mangle]
pub unsafe extern "C" fn getwchar() -> wint_t { fgetwc(stdin) }

#[no_mangle]
pub unsafe extern "C" fn putwchar(c: wchar_t) -> wint_t { fputwc(c, stdout) }

// ============================================================
// wchar: swprintf / vswprintf / fwprintf / vfwprintf
// ============================================================

unsafe fn wfmt_write_str(dst: *mut wchar_t, pos: usize, cap: usize, s: *const u8, len: usize) -> usize {
    let mut i = 0;
    while i < len {
        if pos + i < cap { *dst.add(pos + i) = *s.add(i) as wchar_t; }
        i += 1;
    }
    pos + len
}

struct HeapGuard(*mut u8);

impl Drop for HeapGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { crate::free(self.0 as *mut c_void); }
        }
    }
}

#[no_mangle]
unsafe fn vswprintf_impl(
    s: *mut *mut wchar_t,
    n: usize,
    fmt: *const wchar_t,
    mut args: VaList,
    grow: bool,
) -> c_int {
    if !grow && n == 0 { return -1; }
    let mut buf = *s;
    let mut cap = if grow { n } else { n - 1 };
    let mut pos = 0usize;
    // Format specifiers are ASCII. Keep one byte per wide character so a
    // non-ASCII literal can still be emitted from the original string.
    let fmt_len = wcslen(fmt);
    let fmt_cap = fmt_len.saturating_add(1);
    let fmt_guard = HeapGuard(crate::malloc(fmt_cap) as *mut u8);
    let fmt_narrow = fmt_guard.0;
    if fmt_narrow.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(fmt_narrow, fmt_cap, fmt);
    let mut fi = 0usize;
    macro_rules! wc {
        ($c:expr) => {
            if grow && pos + 1 >= cap {
                let newcap = cap.saturating_mul(2).max(pos + 2);
                let grown = crate::realloc(buf as *mut c_void, newcap * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
                if grown.is_null() { return -1; }
                buf = grown;
                cap = newcap;
            }
            if pos < cap { *buf.add(pos) = $c as wchar_t; }
            pos += 1;
        }
    }
    macro_rules! wemit {
        ($buf:expr, $len:expr) => {
            for k in 0..$len { wc!($buf[k]); }
        };
    }
    loop {
        let c = *fmt_narrow.add(fi);
        if c == 0 { break; }
        if c != b'%' {
            wc!(*fmt.add(fi) as u32);
            fi += 1;
            continue;
        }
        fi += 1;
        let fs = parse_format_spec(fmt_narrow, fi, &mut args);
        let flags = fs.flags;
        let width = fs.width;
        let precision = fs.precision;
        let len_mod = fs.len1;
        fi = fs.next;
        if fs.len2 != 0 {
            let spec = fs.len2;
            let spec2 = fs.spec;
            match (len_mod, spec, spec2) {
                    (b'h', b'h', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut u8) = pos as u8; } }
                    (b'l', b'l', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_longlong) = pos as c_longlong; } }
                    (b'h', b'h', b'd') | (b'h', b'h', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i8 as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'd') | (b'l', b'l', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_longlong>() as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    _ => { wc!(b'%'); wc!(len_mod); wc!(spec); wc!(spec2); }
                }
            } else if len_mod != 0 {
                let spec = fs.spec;
                let lm = match len_mod {
                        b'z' | b't' if spec != b'n' => b'l',
                        // intmax_t is long on every target keel supports.
                        b'j' if spec != b'n' => b'l',
                        _ => len_mod,
                    };
                match (lm, spec) {
                    (b'h', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut u16) = pos as u16; } }
                    (b'l', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_long) = pos as c_long; } }
                    (b'z', b'n') | (b't', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut usize) = pos; } }
                    (b'j', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_ulonglong) = pos as c_ulonglong; } }
                    (b'h', b'd') | (b'h', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i16 as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'd') | (b'l', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_long>() as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'c') => {
                        let ch = args.next_arg::<wint_t>();
                        let pad = width.saturating_sub(1);
                        if flags & FLAG_MINUS != 0 {
                            wc!(ch);
                            for _ in 0..pad { wc!(b' '); }
                        } else {
                            for _ in 0..pad { wc!(b' '); }
                            wc!(ch);
                        }
                    }
                    (b'l', b's') => {
                        let ws_arg = args.next_arg::<*const wchar_t>();
                        if ws_arg.is_null() {
                            let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                            if flags & FLAG_MINUS != 0 {
                                for k in 0..slen { wc!(b"(null)"[k]); }
                                for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                            } else {
                                for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                                for k in 0..slen { wc!(b"(null)"[k]); }
                            }
                        } else {
                            let mut wlen = 0usize;
                            loop {
                                let ch = *ws_arg.add(wlen);
                                if ch == 0 { break; }
                                if precision >= 0 && wlen >= precision as usize { break; }
                                wlen += 1;
                            }
                            if flags & FLAG_MINUS != 0 {
                                for k in 0..wlen { wc!(*ws_arg.add(k)); }
                                for _ in 0..width.saturating_sub(wlen) { wc!(b' '); }
                            } else {
                                for _ in 0..width.saturating_sub(wlen) { wc!(b' '); }
                                for k in 0..wlen { wc!(*ws_arg.add(k)); }
                            }
                        }
                    }
                    (b'l', b'f') | (b'l', b'F') | (b'l', b'e') | (b'l', b'E')
                    | (b'l', b'g') | (b'l', b'G') | (b'l', b'a') | (b'l', b'A') => {
                        let mut wbuf = [0u8; FMT_FLOAT_BUF];
                        let wlen = format_float_into(wbuf.as_mut_ptr(), args.next_arg::<f64>(), spec, precision, width, flags);
                        wemit!(wbuf, wlen);
                    },
                    (b'L', b'f') | (b'L', b'F') | (b'L', b'e') | (b'L', b'E')
                    | (b'L', b'g') | (b'L', b'G') | (b'L', b'a') | (b'L', b'A') => {
                        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "powerpc64"))]
                        let val = f128_to_f64(valist_read_f128(&mut args));
                        #[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
                        let val = args.next_arg::<f64>();
                        let mut wbuf = [0u8; FMT_FLOAT_BUF];
                        let wlen = format_float_into(wbuf.as_mut_ptr(), val, spec, precision, width, flags);
                        wemit!(wbuf, wlen);
                    },
                    _ => { wc!(b'%'); wc!(len_mod); wc!(spec); }
                }
            } else {
            let spec = fs.spec;
            match spec {
                b'd' | b'i' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'u' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'x' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, false, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'X' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, true, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'o' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                    let val = args.next_arg::<f64>();
                    let ucase = spec == b'F' || spec == b'E' || spec == b'G' || spec == b'A';
                    let ftype = match spec | 0x20 { b'f' => FMT_F, b'e' => FMT_E, b'g' => FMT_G, b'a' => FMT_A, _ => FMT_F };
                    let mut fbuf = [0u8; FMT_FLOAT_BUF];
                    let flen = format_f64_full(fbuf.as_mut_ptr(), val, ftype, precision, flags, ucase);
                    let mut wbuf = [0u8; FMT_FLOAT_BUF];
                    let wlen = apply_width_flags(wbuf.as_mut_ptr(), fbuf.as_ptr(), flen, width, flags);
                    for k in 0..wlen { wc!(wbuf[k]); }
                }
                b'c' => {
                    let ch = args.next_arg::<c_int>();
                    wc!(ch as u8);
                }
                b's' => {
                    let p = args.next_arg::<*const c_char>();
                    if p.is_null() {
                        let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                        if flags & FLAG_MINUS != 0 {
                            for k in 0..slen { wc!(b"(null)"[k]); }
                            for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                        } else {
                            for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                            for k in 0..slen { wc!(b"(null)"[k]); }
                        }
                    } else {
                        let mut wbuf_local = [0i32; 256];
                        let mut wcount = 0usize;
                        let mut byte_pos = 0usize;
                        let pbytes = p as *const u8;
                        loop {
                            if precision >= 0 && wcount >= precision as usize { break; }
                            if wcount >= 256 { break; }
                            let mut wc_val: c_int = 0;
                            let consumed = mbrtowc(&mut wc_val as *mut c_int, p.add(byte_pos) as *const c_char, 256, core::ptr::null_mut());
                            if consumed == !0usize || consumed == !1usize {
                                wbuf_local[wcount] = *pbytes.add(byte_pos) as c_int;
                                byte_pos += 1;
                                wcount += 1;
                            } else if consumed == 0 {
                                break;
                            } else {
                                wbuf_local[wcount] = wc_val;
                                byte_pos += consumed;
                                wcount += 1;
                            }
                        }
                        if flags & FLAG_MINUS != 0 {
                            for k in 0..wcount { wc!(wbuf_local[k]); }
                            for _ in 0..width.saturating_sub(wcount) { wc!(b' '); }
                        } else {
                            for _ in 0..width.saturating_sub(wcount) { wc!(b' '); }
                            for k in 0..wcount { wc!(wbuf_local[k]); }
                        }
                    }
                }
                b'p' => {
                    wc!(b'0'); wc!(b'x');
                    let p = args.next_arg::<*const c_void>();
                    let b = format_hex(p as u64, false);
                    for k in 0..b.1 { wc!(b.0[k]); }
                }
                b'n' => {
                    let p = args.next_arg::<*mut c_void>();
                    if !p.is_null() { *(p as *mut c_int) = pos as c_int; }
                }
                b'%' => { wc!(b'%'); }
                _ => { wc!(b'%'); wc!(spec); }
            }
        }
        fi += 1;
    }
    let null_pos = if pos < cap { pos } else { cap };
    *buf.add(null_pos) = 0;
    if grow { *s = buf; }
    if pos > cap { -1 } else { pos as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn vswprintf(s: *mut wchar_t, n: usize, fmt: *const wchar_t, args: VaList) -> c_int {
    let mut buf = s;
    vswprintf_impl(&mut buf, n, fmt, args, false)
}

#[no_mangle]
pub unsafe extern "C" fn swprintf(s: *mut wchar_t, n: usize, fmt: *const wchar_t, args: ...) -> c_int {
    vswprintf(s, n, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vfwprintf(f: *mut FILE, fmt: *const wchar_t, args: VaList) -> c_int {
    // Grow the wide staging buffer instead of failing when the formatted
    // output exceeds a fixed size.
    let cap = 512usize;
    let mut buf = crate::malloc(cap * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
    if buf.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    let r = vswprintf_impl(&mut buf, cap, fmt, args, true);
    if r < 0 {
        crate::free(buf as *mut c_void);
        return r;
    }
    let mut i = 0;
    while i < r as usize {
        let mut mb = [0u8; 4];
        let n = wcrtomb(mb.as_mut_ptr() as *mut c_char, *buf.add(i), core::ptr::null_mut());
        if n == !0usize {
            crate::free(buf as *mut c_void);
            return -1;
        }
        if fwrite(mb.as_ptr() as *const c_void, 1, n, f) != n {
            crate::free(buf as *mut c_void);
            return -1;
        }
        i += 1;
    }
    crate::free(buf as *mut c_void);
    r
}

#[no_mangle]
pub unsafe extern "C" fn fwprintf(f: *mut FILE, fmt: *const wchar_t, args: ...) -> c_int {
    vfwprintf(f, fmt, args)
}

// ============================================================
// wchar: vswscanf / swscanf / vfwscanf / fwscanf / vwscanf / wscanf
// ============================================================

// Convert wchar_t format to byte format (ASCII cast, format specifiers are ASCII)
unsafe fn wcsfmt_to_mbs(dst: *mut u8, dst_len: usize, src: *const wchar_t) {
    let mut i = 0usize;
    loop {
        let ch = *src.add(i) as u32;
        if ch == 0 || i >= dst_len - 1 { break; }
        *dst.add(i) = if ch < 128 { ch as u8 } else { b'?' };
        i += 1;
    }
    *dst.add(i) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn vswscanf(s: *const wchar_t, fmt: *const wchar_t, mut args: VaList) -> c_int {
    if s.is_null() || fmt.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let wlen = wcslen(s);
    let mut cap = wlen.saturating_mul(4).saturating_add(1);
    let mut mbs_buf = crate::malloc(cap) as *mut u8;
    if mbs_buf.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    let mut i = 0usize;
    let mut mb_pos = 0usize;
    loop {
        let wc = *s.add(i) as c_int;
        if wc == 0 { break; }
        let mut mb = [0u8; 4];
        let n = wcrtomb(mb.as_mut_ptr() as *mut c_char, wc, core::ptr::null_mut());
        if n == !0usize || n == 0 { break; }
        if mb_pos + n >= cap {
            let next = match cap.checked_mul(2) {
                Some(v) => v,
                None => { crate::free(mbs_buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            };
            let grown = crate::realloc(mbs_buf as *mut c_void, next) as *mut u8;
            if grown.is_null() { crate::free(mbs_buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            mbs_buf = grown;
            cap = next;
        }
        core::ptr::copy_nonoverlapping(mb.as_ptr(), mbs_buf.add(mb_pos), n);
        mb_pos += n;
        i += 1;
    }
    *mbs_buf.add(mb_pos) = 0;
    let flen = wcslen(fmt);
    let fcap = flen.saturating_add(1);
    let mbs_fmt = crate::malloc(fcap) as *mut u8;
    if mbs_fmt.is_null() {
        crate::free(mbs_buf as *mut c_void);
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(mbs_fmt, fcap, fmt);
    let r = do_vsscanf(mbs_buf, mb_pos, mbs_fmt as *const c_char, &mut args, core::ptr::null_mut());
    crate::free(mbs_fmt as *mut c_void);
    crate::free(mbs_buf as *mut c_void);
    r
}

#[no_mangle]
pub unsafe extern "C" fn swscanf(s: *const wchar_t, fmt: *const wchar_t, args: ...) -> c_int {
    vswscanf(s, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vfwscanf(f: *mut FILE, fmt: *const wchar_t, mut args: VaList) -> c_int {
    if f.is_null() || fmt.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let start_pos = ftello(f);
    if start_pos < 0 { return -1; }
    let mut cap = SCAN_BUF_INIT;
    let mut buf = crate::malloc(cap) as *mut u8;
    if buf.is_null() { ERRNO!() = ENOMEM; return -1; }
    let mut n = 0usize;
    loop {
        if n + 1 >= cap {
            let next = match cap.checked_mul(2) {
                Some(v) => v,
                None => { crate::free(buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            };
            let grown = crate::realloc(buf as *mut c_void, next) as *mut u8;
            if grown.is_null() { crate::free(buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            buf = grown;
            cap = next;
        }
        let c = fgetc(f);
        if c == -1 { break; }
        *buf.add(n) = c as u8;
        n += 1;
    }
    *buf.add(n) = 0;
    let flen = wcslen(fmt);
    let fcap = flen.saturating_add(1);
    let mbs_fmt = crate::malloc(fcap) as *mut u8;
    if mbs_fmt.is_null() {
        crate::free(buf as *mut c_void);
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(mbs_fmt, fcap, fmt);
    let mut consumed = 0usize;
    let hit_eof = (*f)._eof != 0;
    let result = do_vsscanf(buf, n, mbs_fmt as *const c_char, &mut args, &mut consumed);
    crate::free(mbs_fmt as *mut c_void);
    crate::free(buf as *mut c_void);
    let _ = fseeko(f, start_pos + consumed as i64, SEEK_SET);
    if hit_eof && consumed == n {
        (*f)._eof = 1;
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn fwscanf(f: *mut FILE, fmt: *const wchar_t, args: ...) -> c_int {
    vfwscanf(f, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vwscanf(fmt: *const wchar_t, args: VaList) -> c_int {
    vfwscanf(stdin, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn wscanf(fmt: *const wchar_t, args: ...) -> c_int {
    vfwscanf(stdin, fmt, args)
}

// ============================================================
// Time: internal helpers
// ============================================================

const LEAPOCH: i64 = 946684800 + 86400 * (31 + 29);
const DAYS_PER_400Y: i64 = 365 * 400 + 97;
const DAYS_PER_100Y: i64 = 365 * 100 + 24;
const DAYS_PER_4Y: i64 = 365 * 4 + 1;

const SECS_THROUGH_MONTH: [i32; 12] = [
    0, 31*86400, 59*86400, 90*86400, 120*86400, 151*86400,
    181*86400, 212*86400, 243*86400, 273*86400, 304*86400, 334*86400,
];

const DAYS_IN_MONTH: [i32; 12] = [31, 30, 31, 30, 31, 31, 30, 31, 30, 31, 31, 29];

unsafe fn year_to_secs(year: i64, is_leap: &mut bool) -> i64 {
    // musl compares (year-2) as unsigned: the small-year fast path is only
    // valid for 1902..2038.  A signed comparison miscomputes negative years.
    if (year.wrapping_sub(2) as u64) <= 136 {
        let y = year as i32;
        let mut leaps = (y - 68) >> 2;
        if (y - 68) & 3 == 0 { leaps -= 1; *is_leap = true; } else { *is_leap = false; }
        return 31536000 * (y as i64 - 70) + 86400 * leaps as i64;
    }
    let mut cycles = ((year - 100) / 400) as i32;
    let mut rem = ((year - 100) % 400) as i32;
    if rem < 0 { cycles -= 1; rem += 400; }
    let (centuries, leaps);
    if rem == 0 { *is_leap = true; centuries = 0; leaps = 0; }
    else {
        let c;
        if rem >= 200 { if rem >= 300 { c = 3; rem -= 300; } else { c = 2; rem -= 200; } }
        else { if rem >= 100 { c = 1; rem -= 100; } else { c = 0; } }
        centuries = c;
        if rem == 0 { *is_leap = false; leaps = 0; }
        else { leaps = rem / 4; rem %= 4; *is_leap = rem == 0; }
    }
    let total_leaps = leaps + 97 * cycles + 24 * centuries - (*is_leap as i32);
    (year - 100) * 31536000 + total_leaps as i64 * 86400 + 946684800 + 86400
}

fn month_to_secs(month: i32, is_leap: bool) -> i32 {
    let mut t = SECS_THROUGH_MONTH[month as usize];
    if is_leap && month >= 2 { t += 86400; }
    t
}

unsafe fn secs_to_tm(t: i64, tm: &mut tm) -> bool {
    if t < (i32::MIN as i64) * 31622400 || t > (i32::MAX as i64) * 31622400 { return false; }
    let secs = t - LEAPOCH;
    let mut days = secs / 86400;
    let mut remsecs = (secs % 86400) as i32;
    if remsecs < 0 { remsecs += 86400; days -= 1; }
    let mut wday = ((3 + days) % 7) as i32;
    if wday < 0 { wday += 7; }
    let mut qc_cycles = (days / DAYS_PER_400Y) as i32;
    let mut remdays = (days % DAYS_PER_400Y) as i32;
    if remdays < 0 { remdays += DAYS_PER_400Y as i32; qc_cycles -= 1; }
    let mut c_cycles = remdays / DAYS_PER_100Y as i32;
    if c_cycles == 4 { c_cycles -= 1; }
    remdays -= c_cycles * DAYS_PER_100Y as i32;
    let mut q_cycles = remdays / DAYS_PER_4Y as i32;
    if q_cycles == 25 { q_cycles -= 1; }
    remdays -= q_cycles * DAYS_PER_4Y as i32;
    let mut remyears = remdays / 365;
    if remyears == 4 { remyears -= 1; }
    remdays -= remyears * 365;
    let leap = remyears == 0 && (q_cycles != 0 || c_cycles == 0);
    let mut yday = remdays + 31 + 28 + leap as i32;
    if yday >= 365 + leap as i32 { yday -= 365 + leap as i32; }
    let years = remyears as i64 + 4 * q_cycles as i64 + 100 * c_cycles as i64 + 400 * qc_cycles as i64;
    let mut months = 0i32;
    let mut rd = remdays;
    for m in 0..12 {
        if DAYS_IN_MONTH[m] <= rd { rd -= DAYS_IN_MONTH[m]; } else { months = m as i32; break; }
    }
    let mut adj_years = years;
    if months >= 10 { months -= 12; adj_years += 1; }
    if adj_years + 100 > i32::MAX as i64 || adj_years + 100 < i32::MIN as i64 { return false; }
    tm.tm_year = (adj_years + 100) as c_int;
    tm.tm_mon = months + 2;
    tm.tm_mday = rd + 1;
    tm.tm_wday = wday;
    tm.tm_yday = yday;
    tm.tm_hour = remsecs / 3600;
    tm.tm_min = remsecs / 60 % 60;
    tm.tm_sec = remsecs % 60;
    true
}

unsafe fn tm_to_secs(tm: *const tm) -> i64 {
    let mut year = (*tm).tm_year as i64;
    let mut month = (*tm).tm_mon;
    if month >= 12 || month < 0 {
        let adj = month / 12;
        month %= 12;
        if month < 0 { month += 12; }
        year += adj as i64;
    }
    let mut is_leap = false;
    let mut t = year_to_secs(year, &mut is_leap);
    t += month_to_secs(month, is_leap) as i64;
    t += 86400 * ((*tm).tm_mday as i64 - 1);
    t += 3600 * (*tm).tm_hour as i64;
    t += 60 * (*tm).tm_min as i64;
    t += (*tm).tm_sec as i64;
    t
}

static mut TM_BUF: tm = tm {
    tm_sec: 0, tm_min: 0, tm_hour: 0, tm_mday: 0, tm_mon: 0,
    tm_year: 0, tm_wday: 0, tm_yday: 0, tm_isdst: 0,
    tm_gmtoff: 0, tm_zone: core::ptr::null(),
};

static mut UTC_STR: [c_char; 4] = [b'U' as c_char, b'T' as c_char, b'C' as c_char, 0];

const CLOCK_PROCESS_CPUTIME_ID: c_int = 2;

// ============================================================
// Time: core functions
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn time(t: *mut TimeT) -> TimeT {
    let mut ts: timespec = core::mem::zeroed();
    let _ = clock_gettime(CLOCK_REALTIME, &mut ts);
    if !t.is_null() { *t = ts.tv_sec; }
    ts.tv_sec
}

#[no_mangle]
pub unsafe extern "C" fn difftime(t1: TimeT, t0: TimeT) -> f64 {
    (t1 as i64 - t0 as i64) as f64
}

#[no_mangle]
pub unsafe extern "C" fn clock() -> ClockT {
    let mut ts: timespec = core::mem::zeroed();
    let _ = sys_clock_gettime(CLOCK_PROCESS_CPUTIME_ID, &mut ts);
    ts.tv_sec * 1000000 + ts.tv_nsec / 1000
}

#[no_mangle]
pub unsafe extern "C" fn gmtime(t: *const TimeT) -> *mut tm {
    gmtime_r(t, &raw mut TM_BUF)
}

unsafe fn tm_set_utc(tm: *mut tm) {
    (*tm).tm_isdst = 0;
    (*tm).tm_gmtoff = 0;
    (*tm).tm_zone = core::ptr::addr_of!(UTC_STR).cast::<c_char>();
}

#[no_mangle]
pub unsafe extern "C" fn gmtime_r(t: *const TimeT, tm: *mut tm) -> *mut tm {
    if !secs_to_tm(*t as i64, &mut *tm) { ERRNO!() = EOVERFLOW; return core::ptr::null_mut(); }
    tm_set_utc(tm);
    tm
}

#[no_mangle]
pub unsafe extern "C" fn localtime(t: *const TimeT) -> *mut tm {
    tz_localtime_r(t, &raw mut TM_BUF)
}

#[no_mangle]
pub unsafe extern "C" fn localtime_r(t: *const TimeT, tm: *mut tm) -> *mut tm {
    tz_localtime_r(t, tm)
}

#[no_mangle]
pub unsafe extern "C" fn mktime(tm: *mut tm) -> TimeT {
    tz_mktime(tm)
}

#[no_mangle]
pub unsafe extern "C" fn timegm(tm: *mut tm) -> TimeT {
    let t = tm_to_secs(tm as *const tm);
    if !secs_to_tm(t, &mut *tm) { ERRNO!() = EOVERFLOW; return -1; }
    tm_set_utc(tm);
    t as TimeT
}

static mut ASCTIME_BUF: [c_char; 26] = [0; 26];

const DAY_NAMES: [&[u8]; 7] = [b"Sun\0", b"Mon\0", b"Tue\0", b"Wed\0", b"Thu\0", b"Fri\0", b"Sat\0"];
const MON_NAMES: [&[u8]; 12] = [b"Jan\0", b"Feb\0", b"Mar\0", b"Apr\0", b"May\0", b"Jun\0", b"Jul\0", b"Aug\0", b"Sep\0", b"Oct\0", b"Nov\0", b"Dec\0"];

#[no_mangle]
pub unsafe extern "C" fn asctime(tm: *const tm) -> *mut c_char {
    asctime_r(tm, core::ptr::addr_of_mut!(ASCTIME_BUF).cast::<c_char>())
}

#[no_mangle]
pub unsafe extern "C" fn asctime_r(tm: *const tm, buf: *mut c_char) -> *mut c_char {
    let wday = if (*tm).tm_wday >= 0 && (*tm).tm_wday < 7 { (*tm).tm_wday as usize } else { 0 };
    let mon = if (*tm).tm_mon >= 0 && (*tm).tm_mon < 12 { (*tm).tm_mon as usize } else { 0 };
    let dn = DAY_NAMES[wday];
    let mn = MON_NAMES[mon];
    let mut p = 0;
    for k in 0..3 { *buf.add(p) = *dn.as_ptr().add(k) as c_char; p += 1; }
    *buf.add(p) = b' ' as c_char; p += 1;
    for k in 0..3 { *buf.add(p) = *mn.as_ptr().add(k) as c_char; p += 1; }
    *buf.add(p) = b' ' as c_char; p += 1;
    // %.3s %.3s%3d: the day is right-aligned in a three-column field.
    let v = (*tm).tm_mday;
    if v < 10 {
        *buf.add(p) = b' ' as c_char; p += 1;
        *buf.add(p) = b' ' as c_char; p += 1;
        *buf.add(p) = (b'0' + v as u8) as c_char; p += 1;
    } else if v < 100 {
        *buf.add(p) = b' ' as c_char; p += 1;
        *buf.add(p) = (b'0' + (v / 10) as u8) as c_char; p += 1;
        *buf.add(p) = (b'0' + (v % 10) as u8) as c_char; p += 1;
    } else {
        *buf.add(p) = (b'0' + (v / 100) as u8) as c_char; p += 1;
        *buf.add(p) = (b'0' + ((v / 10) % 10) as u8) as c_char; p += 1;
        *buf.add(p) = (b'0' + (v % 10) as u8) as c_char; p += 1;
    }
    *buf.add(p) = b' ' as c_char; p += 1;
    let hh = (*tm).tm_hour;
    *buf.add(p) = (b'0' + (hh / 10) as u8) as c_char; p += 1;
    *buf.add(p) = (b'0' + (hh % 10) as u8) as c_char; p += 1;
    *buf.add(p) = b':' as c_char; p += 1;
    let mm = (*tm).tm_min;
    *buf.add(p) = (b'0' + (mm / 10) as u8) as c_char; p += 1;
    *buf.add(p) = (b'0' + (mm % 10) as u8) as c_char; p += 1;
    *buf.add(p) = b':' as c_char; p += 1;
    let ss = (*tm).tm_sec;
    *buf.add(p) = (b'0' + (ss / 10) as u8) as c_char; p += 1;
    *buf.add(p) = (b'0' + (ss % 10) as u8) as c_char; p += 1;
    *buf.add(p) = b' ' as c_char; p += 1;
    // %d: negative and >4-digit years are valid in musl asctime.
    let yr = (*tm).tm_year as i64 + 1900;
    let mut tmp = [0u8; 16];
    let mut n = 0usize;
    let mut v2 = if yr < 0 { -yr } else { yr };
    if v2 == 0 {
        tmp[n] = b'0';
        n += 1;
    } else {
        while v2 > 0 && n < tmp.len() {
            tmp[n] = b'0' + (v2 % 10) as u8;
            v2 /= 10;
            n += 1;
        }
    }
    if yr < 0 {
        *buf.add(p) = b'-' as c_char;
        p += 1;
    }
    for k in (0..n).rev() {
        *buf.add(p) = tmp[k] as c_char;
        p += 1;
    }
    *buf.add(p) = b'\n' as c_char; p += 1;
    *buf.add(p) = 0;
    buf
}

#[no_mangle]
pub unsafe extern "C" fn ctime(t: *const TimeT) -> *mut c_char {
    let tm = localtime(t);
    if tm.is_null() { core::ptr::null_mut() } else { asctime(tm) }
}

#[no_mangle]
pub unsafe extern "C" fn ctime_r(t: *const TimeT, buf: *mut c_char) -> *mut c_char {
    let mut tmp: tm = core::mem::zeroed();
    if localtime_r(t, &mut tmp).is_null() { core::ptr::null_mut() } else { asctime_r(&tmp, buf) }
}

// ============================================================
// Time: clock_getres / clock_settime / clock_nanosleep / gettimeofday
// ============================================================

unsafe fn sys_clock_getres(clockid: c_int, ts: *mut timespec) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_CLOCK_GETRES, clockid as i64, ts as i64)
}

unsafe fn sys_clock_settime(clockid: c_int, ts: *const timespec) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_CLOCK_SETTIME, clockid as i64, ts as i64)
}

unsafe fn sys_clock_nanosleep(clockid: c_int, flags: c_int, req: *const timespec, rem: *mut timespec) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_CLOCK_NANOSLEEP, clockid as i64, flags as i64, req as i64, rem as i64)
}

#[no_mangle]
pub unsafe extern "C" fn clock_getres(clockid: c_int, ts: *mut timespec) -> c_int {
    let r = sys_clock_getres(clockid, ts);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn clock_settime(clockid: c_int, ts: *const timespec) -> c_int {
    let r = sys_clock_settime(clockid, ts);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn clock_nanosleep(clockid: c_int, flags: c_int, req: *const timespec, rem: *mut timespec) -> c_int {
    let r = sys_clock_nanosleep(clockid, flags, req, rem);
    if r < 0 {
        let e = (-r) as c_int;
        // The kernel returns EOPNOTSUPP for CLOCK_THREAD_CPUTIME_ID;
        // POSIX/musl report EINVAL instead.
        if clockid == 3 && e == 95 { return 22; }
        e
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn gettimeofday(tv: *mut timeval, _tz: *mut c_void) -> c_int {
    if tv.is_null() { return 0; }
    let mut ts: timespec = core::mem::zeroed();
    let _ = clock_gettime(CLOCK_REALTIME, &mut ts);
    (*tv).tv_sec = ts.tv_sec;
    (*tv).tv_usec = ts.tv_nsec / 1000;
    0
}

// ============================================================
// Time: strftime
// ============================================================

fn is_leap_year(y: i32) -> bool {
    // Widen before adding 1900 so years near i32::MAX cannot wrap; 2000 is
    // not congruent to 1900 modulo 400, so the old shortcut was wrong.
    let yr = (y as i64) + 1900;
    yr % 4 == 0 && (yr % 100 != 0 || yr % 400 == 0)
}

fn week_num(tm: &tm) -> i32 {
    let mut val = (tm.tm_yday + 7 - ((tm.tm_wday + 6) % 7)) / 7;
    if (tm.tm_wday + 371 - tm.tm_yday - 2).rem_euclid(7) <= 2 { val += 1; }
    if val == 0 {
        val = 52;
        let dec31 = (tm.tm_wday + 7 - tm.tm_yday - 1).rem_euclid(7);
        if dec31 == 4 || (dec31 == 5 && is_leap_year(tm.tm_year % 400 - 1)) { val += 1; }
    } else if val == 53 {
        let jan1 = (tm.tm_wday + 371 - tm.tm_yday).rem_euclid(7);
        if jan1 != 4 && (jan1 != 3 || !is_leap_year(tm.tm_year)) { val = 1; }
    }
    val
}

// Langinfo constants for strftime
const ABDAY_1: c_int = 0x20000;
const DAY_1: c_int = 0x20007;
const ABMON_1: c_int = 0x2000E;
const MON_1: c_int = 0x2001A;
const AM_STR_LI: c_int = 0x20026;
const PM_STR_LI: c_int = 0x20027;
const D_T_FMT: c_int = 0x20028;
const D_FMT: c_int = 0x20029;
const T_FMT: c_int = 0x2002A;
const T_FMT_AMPM: c_int = 0x2002B;

// Format an i64 into buf as decimal with zero/sign padding. Returns (ptr_to_start, length).
// pad: '-' = no pad, '_' = space pad, '0' or 0 = zero pad. width: minimum digits (excluding sign).
unsafe fn num_fmt(buf: &mut [u8; 100], val: i64, pad: u8, width: usize) -> (*const u8, usize) {
    let negative = val < 0;
    let mut v = if negative { -val as u64 } else { val as u64 };
    let mut tmp = [0u8; 24];
    let mut pos = 24usize;
    if v == 0 { pos -= 1; tmp[pos] = b'0'; }
    while v > 0 { pos -= 1; tmp[pos] = b'0' + (v % 10) as u8; v /= 10; }
    let digits = 24 - pos;
    let sign_chars = if negative { 1 } else { 0 };
    let needed = sign_chars + digits;
    let fill = if width > needed { width - needed } else { 0 };
    let total = fill + needed;
    let mut p = 0usize;
    match pad {
        b'-' => { /* no fill */ }
        b'_' => {
            for _ in 0..fill { buf[p] = b' '; p += 1; }
            if negative { buf[p] = b'-'; p += 1; }
        }
        _ => {
            if negative { buf[p] = b'-'; p += 1; }
            for _ in 0..fill { buf[p] = b'0'; p += 1; }
        }
    }
    for i in 0..digits { buf[p] = tmp[pos + i]; p += 1; }
    (buf.as_ptr(), total)
}

// Copy a null-terminated C string into buf. Returns (ptr, len).
unsafe fn str_to_buf(buf: &mut [u8; 100], s: *const c_char) -> (*const u8, usize) {
    let mut k = 0usize;
    // tm_zone and nl_langinfo strings are caller/file controlled; never walk
    // past the fixed buffer (the format result is truncated instead).
    while k + 1 < buf.len() && *s.add(k) != 0 {
        buf[k] = *s.add(k) as u8;
        k += 1;
    }
    (buf.as_ptr(), k)
}

// Core format dispatcher. Returns (ptr, len) where ptr points into buf or a static string.
// pad=0 means use default; returns (null, 0) for unknown specifiers.
unsafe fn __strftime_fmt_1(buf: &mut [u8; 100], f: u8, tm: *const tm, pad: u8) -> (*const u8, usize) {
    let mut val: i64;
    let mut width: usize = 2;
    let mut def_pad: u8 = b'0';
    let fmt: *const c_char;

    match f {
        b'a' => {
            if (*tm).tm_wday as u32 > 6 { return (b"\0".as_ptr(), 0); }
            let p = nl_langinfo(ABDAY_1 + (*tm).tm_wday);
            return str_to_buf(buf, p);
        }
        b'A' => {
            if (*tm).tm_wday as u32 > 6 { return (b"\0".as_ptr(), 0); }
            let p = nl_langinfo(DAY_1 + (*tm).tm_wday);
            return str_to_buf(buf, p);
        }
        b'h' | b'b' => {
            if (*tm).tm_mon as u32 > 11 { return (b"\0".as_ptr(), 0); }
            let p = nl_langinfo(ABMON_1 + (*tm).tm_mon);
            return str_to_buf(buf, p);
        }
        b'B' => {
            if (*tm).tm_mon as u32 > 11 { return (b"\0".as_ptr(), 0); }
            let p = nl_langinfo(MON_1 + (*tm).tm_mon);
            return str_to_buf(buf, p);
        }
        b'c' => {
            let p = nl_langinfo(D_T_FMT);
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, p, tm);
            return (buf.as_ptr(), l);
        }
        b'C' => {
            val = (1900i64 + (*tm).tm_year as i64) / 100;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'e' => { def_pad = b'_'; }
        b'd' => {
            val = (*tm).tm_mday as i64;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'D' => {
            fmt = b"%m/%d/%y\0".as_ptr() as *const c_char;
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, fmt, tm);
            return (buf.as_ptr(), l);
        }
        b'F' => {
            fmt = b"%Y-%m-%d\0".as_ptr() as *const c_char;
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, fmt, tm);
            return (buf.as_ptr(), l);
        }
        b'g' | b'G' => {
            val = (*tm).tm_year as i64 + 1900;
            if (*tm).tm_yday < 3 && week_num(&*tm) != 1 { val -= 1; }
            else if (*tm).tm_yday > 360 && week_num(&*tm) == 1 { val += 1; }
            if f == b'g' { val %= 100; }
            else { width = 4; }
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'H' => {
            val = (*tm).tm_hour as i64;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'I' => {
            val = (*tm).tm_hour as i64;
            if val == 0 { val = 12; }
            else if val > 12 { val -= 12; }
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'j' => {
            val = (*tm).tm_yday as i64 + 1;
            width = 3;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'm' => {
            val = (*tm).tm_mon as i64 + 1;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'M' => {
            val = (*tm).tm_min as i64;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'n' => { buf[0] = b'\n'; return (buf.as_ptr(), 1); }
        b'p' => {
            let p = if (*tm).tm_hour >= 12 { nl_langinfo(PM_STR_LI) } else { nl_langinfo(AM_STR_LI) };
            return str_to_buf(buf, p);
        }
        b'r' => {
            let p = nl_langinfo(T_FMT_AMPM);
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, p, tm);
            return (buf.as_ptr(), l);
        }
        b'R' => {
            fmt = b"%H:%M\0".as_ptr() as *const c_char;
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, fmt, tm);
            return (buf.as_ptr(), l);
        }
        b's' => {
            val = tm_to_secs(tm) - (*tm).tm_gmtoff as i64;
            width = 1;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'S' => {
            val = (*tm).tm_sec as i64;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b't' => { buf[0] = b'\t'; return (buf.as_ptr(), 1); }
        b'T' => {
            fmt = b"%H:%M:%S\0".as_ptr() as *const c_char;
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, fmt, tm);
            return (buf.as_ptr(), l);
        }
        b'u' => {
            val = if (*tm).tm_wday != 0 { (*tm).tm_wday as i64 } else { 7 };
            width = 1;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'U' => {
            val = ((*tm).tm_yday as u64 + 7 - (*tm).tm_wday as u64) as i64 / 7;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'W' => {
            val = ((*tm).tm_yday as u64 + 7 - ((*tm).tm_wday as u64 + 6) % 7) as i64 / 7;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'V' => {
            val = week_num(&*tm) as i64;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'w' => {
            val = (*tm).tm_wday as i64;
            width = 1;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'x' => {
            let p = nl_langinfo(D_FMT);
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, p, tm);
            return (buf.as_ptr(), l);
        }
        b'X' => {
            let p = nl_langinfo(T_FMT);
            let l = __strftime_l(buf.as_mut_ptr() as *mut c_char, 100, p, tm);
            return (buf.as_ptr(), l);
        }
        b'y' => {
            val = (1900i64 + (*tm).tm_year as i64) % 100;
            if val < 0 { val = -val; }
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'Y' => {
            val = (*tm).tm_year as i64 + 1900;
            if val >= 10000 {
                let mut tmp = [0u8; 100];
                let (_, k) = num_fmt(&mut tmp, val, b'-', 1);
                buf[0] = b'+';
                for i in 0..k { buf[1 + i] = tmp[i]; }
                return (buf.as_ptr(), 1 + k);
            }
            width = 4;
            return num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width);
        }
        b'z' => {
            if (*tm).tm_isdst < 0 { return (b"\0".as_ptr(), 0); }
            let gmtoff = (*tm).tm_gmtoff as i64;
            let sign = if gmtoff >= 0 { b'+' } else { b'-' };
            let off = if gmtoff < 0 { -gmtoff } else { gmtoff };
            let hh = (off / 3600) as i32;
            let mm = ((off % 3600) / 60) as i32;
            buf[0] = sign;
            buf[1] = b'0' + (hh / 10) as u8;
            buf[2] = b'0' + (hh % 10) as u8;
            buf[3] = b'0' + (mm / 10) as u8;
            buf[4] = b'0' + (mm % 10) as u8;
            return (buf.as_ptr(), 5);
        }
        b'Z' => {
            if (*tm).tm_isdst < 0 || (*tm).tm_zone.is_null() { return (b"\0".as_ptr(), 0); }
            return str_to_buf(buf, (*tm).tm_zone);
        }
        b'%' => { buf[0] = b'%'; return (buf.as_ptr(), 1); }
        _ => { return (core::ptr::null(), 0); }
    }
    // fallthrough for 'e' (def_pad='_', same logic as 'd')
    val = (*tm).tm_mday as i64;
    num_fmt(buf, val, if pad != 0 { pad } else { def_pad }, width)
}

unsafe fn __strftime_l(s: *mut c_char, n: usize, f: *const c_char, tm: *const tm) -> usize {
    let mut l: usize = 0;
    let mut fi: usize = 0;
    while l < n {
        let ch = *f.add(fi) as u8;
        if ch == 0 {
            *s.add(l) = 0;
            return l;
        }
        if ch != b'%' {
            *s.add(l) = ch as c_char;
            l += 1;
            fi += 1;
            continue;
        }
        fi += 1;
        // Parse flags
        let mut pad: u8 = 0;
        let fc = *f.add(fi) as u8;
        if fc == b'-' || fc == b'_' || fc == b'0' { pad = fc; fi += 1; }
        let plus = *f.add(fi) as u8 == b'+';
        if plus { fi += 1; }
        // Parse width
        let mut width: usize = 0;
        let mut p_idx = fi;
        {
            let fc2 = *f.add(fi) as u8;
            if fc2 >= b'0' && fc2 <= b'9' {
                let mut w: usize = 0;
                while { let c = *f.add(p_idx) as u8; c >= b'0' && c <= b'9' } {
                    w = w * 10 + (*f.add(p_idx) as usize - b'0' as usize);
                    p_idx += 1;
                }
                width = w;
            }
        }
        let spec_ch = *f.add(p_idx) as u8;
        if spec_ch == b'C' || spec_ch == b'F' || spec_ch == b'G' || spec_ch == b'Y' {
            if width == 0 && p_idx != fi { width = 1; }
        } else {
            width = 0;
        }
        fi = p_idx;
        // Skip E/O modifier
        let fc3 = *f.add(fi) as u8;
        if fc3 == b'E' || fc3 == b'O' { fi += 1; }
        let fch = *f.add(fi) as u8;
        // Call formatter
        let mut buf = [0u8; 100];
        let (t, k) = __strftime_fmt_1(&mut buf, fch, tm, pad);
        if t.is_null() { break; }
        let mut k = k;
        let mut t = t;
        if width != 0 {
            // Strip sign and leading zeros, count remaining digits
            if *t == b'+' || *t == b'-' { t = t.add(1); k -= 1; }
            while k > 0 && *t == b'0' && k > 1 && *t.add(1) >= b'0' && *t.add(1) <= b'9' {
                t = t.add(1); k -= 1;
            }
            if width < k { width = k; }
            let mut d: usize = 0;
            while d < k && *t.add(d) >= b'0' && *t.add(d) <= b'9' { d += 1; }
            if (*tm).tm_year < -1900 {
                if l < n { *s.add(l) = b'-' as c_char; }
                l += 1;
                width -= 1;
            } else if plus && d + (width - k) >= (if spec_ch == b'C' { 3 } else { 5 }) {
                if l < n { *s.add(l) = b'+' as c_char; }
                l += 1;
                width -= 1;
            }
            while width > k && l < n {
                *s.add(l) = b'0' as c_char;
                l += 1;
                width -= 1;
            }
        }
        let copy = if k > n - l { n - l } else { k };
        core::ptr::copy_nonoverlapping(t, s.add(l) as *mut u8, copy);
        l += copy;
        fi += 1;
    }
    if n > 0 {
        if l == n { l = n - 1; }
        *s.add(l) = 0;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strftime(s: *mut c_char, maxsize: usize, fmt: *const c_char, tm: *const tm) -> usize {
    __strftime_l(s, maxsize, fmt, tm)
}

// ============================================================
// Time: strptime
// ============================================================

unsafe fn match_prefix(s: *const u8, prefix: &[u8]) -> bool {
    for i in 0..prefix.len() {
        if *s.add(i) == 0 || *s.add(i) != prefix[i] { return false; }
    }
    true
}

unsafe fn match_ci_prefix(s: *const u8, prefix: &[u8]) -> bool {
    for i in 0..prefix.len() {
        let c = *s.add(i);
        if c == 0 { return false; }
        if c.to_ascii_lowercase() != prefix[i].to_ascii_lowercase() { return false; }
    }
    true
}

unsafe fn parse_int(p: *const u8, max_digits: usize) -> (i32, *const u8) {
    let mut val = 0i32;
    let mut np = p;
    let mut count = 0;
    while *np >= b'0' && *np <= b'9' && count < max_digits {
        val = val * 10 + (*np - b'0') as i32;
        np = np.add(1);
        count += 1;
    }
    if count == 0 { (0, p) } else { (val, np) }
}

unsafe fn parse_range(p: *const u8, min: i32, max: i32) -> (i32, *const u8) {
    let (val, np) = parse_int(p, 4);
    if np == p || val < min || val > max { (0, p) } else { (val, np) }
}

fn is_ascii_space(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0c || c == 0x0b
}

const WDAY_FULL: [&[u8]; 7] = [
    b"Sunday", b"Monday", b"Tuesday", b"Wednesday", b"Thursday", b"Friday", b"Saturday",
];
const WDAY_ABBR: [&[u8]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];
const MON_FULL: [&[u8]; 12] = [
    b"January", b"February", b"March", b"April", b"May", b"June",
    b"July", b"August", b"September", b"October", b"November", b"December",
];
const MON_ABBR: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun",
    b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

/// Parse a `%z` offset (`+hhmm`, `+hh`, or `+hhmmss` is not accepted) and
/// return the seconds east of UTC plus the first unconsumed byte.
unsafe fn parse_tz_offset(mut p: *const u8) -> Option<(i64, *const u8)> {
    if *p != b'+' && *p != b'-' { return None; }
    let neg = *p == b'-';
    p = p.add(1);
    let mut off = 0i64;
    let mut digits = 0;
    while digits < 4 && *p >= b'0' && *p <= b'9' {
        off = off * 10 + (*p - b'0') as i64;
        p = p.add(1);
        digits += 1;
    }
    if digits == 0 { return None; }
    let (hh, mm) = if digits <= 2 { (off, 0i64) } else { (off / 100, off % 100) };
    Some(((hh * 3600 + mm * 60) * if neg { -1 } else { 1 }, p))
}

#[no_mangle]
pub unsafe extern "C" fn strptime(s: *const c_char, fmt: *const c_char, tm: *mut tm) -> *mut c_char {
    let mut p = s as *const u8;
    let mut f = fmt as *const u8;
    let mut want_century = 0i32;
    let mut century = 0i32;
    let mut relyear = 0i32;
    while *f != 0 {
        if *f != b'%' {
            if is_ascii_space(*f) {
                while *p != 0 && is_ascii_space(*p) { p = p.add(1); }
            } else {
                if *p != *f { return core::ptr::null_mut(); }
                p = p.add(1);
            }
            f = f.add(1);
            continue;
        }
        f = f.add(1);
        if *f == b'+' { f = f.add(1); }
        while *f >= b'0' && *f <= b'9' { f = f.add(1); }
        if *f == b'E' || *f == b'O' { f = f.add(1); }
        match *f {
            b'a' | b'A' => {
                let mut found = false;
                for i in 0..7 {
                    if match_prefix(p, WDAY_FULL[i]) { (*tm).tm_wday = i as c_int; p = p.add(WDAY_FULL[i].len()); found = true; break; }
                    if match_prefix(p, WDAY_ABBR[i]) { (*tm).tm_wday = i as c_int; p = p.add(WDAY_ABBR[i].len()); found = true; break; }
                }
                if !found { return core::ptr::null_mut(); }
            }
            b'b' | b'B' | b'h' => {
                let mut found = false;
                for i in 0..12 {
                    if match_prefix(p, MON_FULL[i]) { (*tm).tm_mon = i as c_int; p = p.add(MON_FULL[i].len()); found = true; break; }
                    if match_prefix(p, MON_ABBR[i]) { (*tm).tm_mon = i as c_int; p = p.add(MON_ABBR[i].len()); found = true; break; }
                }
                if !found { return core::ptr::null_mut(); }
            }
            b'C' => {
                let (v, np) = parse_int(p, 2);
                if np == p { return core::ptr::null_mut(); }
                century = v;
                want_century |= 2;
                p = np;
            }
            b'd' | b'e' => {
                let (v, np) = parse_range(p, 1, 31);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_mday = v;
                p = np;
            }
            b'D' => {
                let r = strptime(p as *const c_char, b"%m/%d/%y\0".as_ptr() as *const c_char, tm);
                if r.is_null() { return core::ptr::null_mut(); }
                p = r as *const u8;
            }
            b'F' => {
                let r = strptime(p as *const c_char, b"%Y-%m-%d\0".as_ptr() as *const c_char, tm);
                if r.is_null() { return core::ptr::null_mut(); }
                p = r as *const u8;
            }
            b'H' | b'I' => {
                let (v, np) = parse_range(p, 0, 23);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_hour = v;
                p = np;
            }
            b'j' => {
                let (v, np) = parse_range(p, 1, 366);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_yday = v - 1;
                p = np;
            }
            b'm' => {
                let (v, np) = parse_range(p, 1, 12);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_mon = v - 1;
                p = np;
            }
            b'M' => {
                let (v, np) = parse_range(p, 0, 59);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_min = v;
                p = np;
            }
            b'n' | b't' => {
                while *p != 0 && is_ascii_space(*p) { p = p.add(1); }
            }
            b'p' => {
                if match_ci_prefix(p, b"AM") {
                    if (*tm).tm_hour == 12 { (*tm).tm_hour = 0; }
                    p = p.add(2);
                } else if match_ci_prefix(p, b"PM") {
                    if (*tm).tm_hour < 12 { (*tm).tm_hour += 12; }
                    p = p.add(2);
                } else { return core::ptr::null_mut(); }
            }
            b'r' => {
                let r = strptime(p as *const c_char, b"%I:%M:%S %p\0".as_ptr() as *const c_char, tm);
                if r.is_null() { return core::ptr::null_mut(); }
                p = r as *const u8;
            }
            b'R' => {
                let r = strptime(p as *const c_char, b"%H:%M\0".as_ptr() as *const c_char, tm);
                if r.is_null() { return core::ptr::null_mut(); }
                p = r as *const u8;
            }
            b's' => {
                let neg = if *p == b'-' { p = p.add(1); true } else { false };
                if *p < b'0' || *p > b'9' { return core::ptr::null_mut(); }
                let mut val = 0i64;
                while *p >= b'0' && *p <= b'9' {
                    val = val * 10 + (*p - b'0') as i64;
                    p = p.add(1);
                }
                if neg { val = -val; }
                secs_to_tm(val, &mut *tm);
            }
            b'S' => {
                let (v, np) = parse_range(p, 0, 60);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_sec = v;
                p = np;
            }
            b'T' => {
                let r = strptime(p as *const c_char, b"%H:%M:%S\0".as_ptr() as *const c_char, tm);
                if r.is_null() { return core::ptr::null_mut(); }
                p = r as *const u8;
            }
            b'U' | b'W' => {
                let (_, np) = parse_range(p, 0, 53);
                if np == p { return core::ptr::null_mut(); }
                p = np;
            }
            b'V' => {
                let (_, np) = parse_range(p, 1, 53);
                if np == p { return core::ptr::null_mut(); }
                p = np;
            }
            b'w' => {
                let (v, np) = parse_range(p, 0, 6);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_wday = v;
                p = np;
            }
            b'y' => {
                let (v, np) = parse_int(p, 2);
                if np == p { return core::ptr::null_mut(); }
                relyear = v;
                want_century |= 1;
                p = np;
            }
            b'Y' => {
                let neg = if *p == b'-' { p = p.add(1); true } else { false };
                let (v, np) = parse_int(p, 4);
                if np == p { return core::ptr::null_mut(); }
                (*tm).tm_year = if neg { -(v + 1900) } else { v - 1900 };
                want_century = 0;
                p = np;
            }
            b'z' => {
                let (off, np) = match parse_tz_offset(p) {
                    Some(v) => v,
                    None => return core::ptr::null_mut(),
                };
                (*tm).tm_gmtoff = off;
                p = np;
            }
            b'Z' => {
                let mut matched = false;
                for i in 0..2 {
                    let name = crate::tzname[i];
                    if name.is_null() { continue; }
                    let len = strlen(name);
                    if strncmp(p, name as *const u8, len) == 0 {
                        (*tm).tm_isdst = i as c_int;
                        p = p.add(len);
                        matched = true;
                        break;
                    }
                }
                if !matched {
                    // No match: skip an alphabetic zone abbreviation.
                    while *p != 0 && (*p | 32).wrapping_sub(b'a') <= 25 { p = p.add(1); }
                }
            }
            b'%' => {
                if *p != b'%' { return core::ptr::null_mut(); }
                p = p.add(1);
            }
            _ => { return core::ptr::null_mut(); }
        }
        f = f.add(1);
    }
    if want_century != 0 {
        (*tm).tm_year = relyear;
        if want_century & 2 != 0 { (*tm).tm_year += century * 100 - 1900; }
        else if (*tm).tm_year <= 68 { (*tm).tm_year += 100; }
    }
    p as *mut c_char
}

// ============================================================
// Time: tzset / daylight / timezone / tzname
// ============================================================

#[no_mangle]
pub static mut daylight: c_int = 0;
#[no_mangle]
pub static mut timezone: c_long = 0;
#[no_mangle]
pub static mut tzname: [*mut c_char; 2] = [b"UTC\0".as_ptr() as *mut c_char, b"UTC\0".as_ptr() as *mut c_char];

#[no_mangle]
pub unsafe extern "C" fn tzset() {
    tz_set();
}

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
    let mut ts: timespec = core::mem::zeroed();
    let _ = sys_clock_gettime(CLOCK_REALTIME, &mut ts);
    let mut ctr: u32 = ts.tv_nsec as u32;
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

// ============================================================
// libgen: basename / dirname
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn dirname(s: *mut c_char) -> *mut c_char {
    if s.is_null() || *s == 0 {
        return b".\0".as_ptr() as *mut c_char;
    }
    let mut i = strlen(s as *const c_char);
    i = i.wrapping_sub(1);
    // skip trailing slashes
    while i > 0 && *s.add(i) == b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 && *s.add(0) == b'/' as c_char {
        return b"/\0".as_ptr() as *mut c_char;
    }
    // skip non-slashes
    while i > 0 && *s.add(i) != b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 {
        if *s.add(0) == b'/' as c_char {
            return b"/\0".as_ptr() as *mut c_char;
        }
        return b".\0".as_ptr() as *mut c_char;
    }
    // skip slashes
    while i > 0 && *s.add(i - 1) == b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    if i == 0 {
        return b"/\0".as_ptr() as *mut c_char;
    }
    *s.add(i) = 0;
    s
}

#[no_mangle]
pub unsafe extern "C" fn basename(s: *mut c_char) -> *mut c_char {
    if s.is_null() || *s == 0 {
        return b".\0".as_ptr() as *mut c_char;
    }
    let mut i = strlen(s as *const c_char);
    i = i.wrapping_sub(1);
    // strip trailing slashes
    while i > 0 && *s.add(i) == b'/' as c_char {
        *s.add(i) = 0;
        i = i.wrapping_sub(1);
    }
    // find last slash
    while i > 0 && *s.add(i - 1) != b'/' as c_char {
        i = i.wrapping_sub(1);
    }
    s.add(i)
}

// ============================================================
// arpa/inet: inet_pton / inet_ntop / inet_addr / inet_ntoa / inet_aton
// ============================================================

const AF_INET_VAL: c_int = 2;
const AF_INET6_VAL: c_int = 10;

unsafe fn inet_pton_v4(s: *const u8, a: *mut u8) -> c_int {
    let mut p = s;
    let mut i = 0;
    while i < 4 {
        let oct_start = p;
        let mut v: u32 = 0;
        let mut j: u32 = 0;
        while j < 3 && *p >= b'0' && *p <= b'9' {
            v = v * 10 + (*p - b'0') as u32;
            p = p.add(1);
            j += 1;
        }
        if j == 0 || (j > 1 && *oct_start == b'0') || v > 255 { return 0; }
        *a.add(i) = v as u8;
        if *p == 0 && i == 3 { return 1; }
        if *p != b'.' { return 0; }
        p = p.add(1);
        i += 1;
    }
    0
}

unsafe fn inet_pton_v6(s: *const u8, a: *mut u8) -> c_int {
    let mut ip = [0u16; 8];
    let mut p = s;
    let mut i: usize = 0;
    let mut brk: isize = -1;
    let mut need_v4 = false;
    let mut v4_start: *const u8 = core::ptr::null();

    if *p == b':' {
        p = p.add(1);
        if *p != b':' { return 0; }
    }

    loop {
        if *p == b':' && brk < 0 {
            brk = i as isize;
            ip[i & 7] = 0;
            p = p.add(1);
            if *p == 0 { break; }
            if i == 7 { return 0; }
            i += 1;
            continue;
        }
        v4_start = p;
        let mut v: u16 = 0;
        let mut j = 0;
        while j < 4 {
            let d = hex_digit(*p);
            if d < 0 { break; }
            v = v * 16 + d as u16;
            p = p.add(1);
            j += 1;
        }
        if j == 0 { return 0; }
        ip[i & 7] = v;
        if *p == 0 && (brk >= 0 || i == 7) { break; }
        if i == 7 { return 0; }
        if *p != b':' {
            if *p != b'.' || (i < 6 && brk < 0) { return 0; }
            need_v4 = true;
            i += 1;
            ip[i & 7] = 0;
            break;
        }
        p = p.add(1);
        i += 1;
    }

    if brk >= 0 {
        // expand :: : move trailing fields to the end and zero the gap
        let trail_count = i + 1 - brk as usize;
        let dest_start = brk as usize + 7 - i;
        // move backwards to avoid overwrite in the overlapping case
        for j in (0..trail_count).rev() {
            ip[dest_start + j] = ip[brk as usize + j];
        }
        for j in 0..(7 - i) {
            ip[brk as usize + j] = 0;
        }
    }

    for k in 0..8 {
        *a.add(k * 2) = (ip[k] >> 8) as u8;
        *a.add(k * 2 + 1) = (ip[k] & 0xff) as u8;
    }

    if need_v4 {
        if inet_pton_v4(v4_start, a.add(12)) <= 0 { return 0; }
    }
    1
}

unsafe fn hex_digit(c: u8) -> i32 {
    if c >= b'0' && c <= b'9' { (c - b'0') as i32 }
    else if c >= b'a' && c <= b'f' { (c - b'a' + 10) as i32 }
    else if c >= b'A' && c <= b'F' { (c - b'A' + 10) as i32 }
    else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn inet_pton(af: c_int, s: *const c_char, a: *mut c_void) -> c_int {
    if af == AF_INET_VAL {
        inet_pton_v4(s as *const u8, a as *mut u8)
    } else if af == AF_INET6_VAL {
        inet_pton_v6(s as *const u8, a as *mut u8)
    } else {
        ERRNO!() = EAFNOSUPPORT;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn inet_ntop(af: c_int, a: *const c_void, s: *mut c_char, size: u32) -> *mut c_char {
    if af == AF_INET_VAL {
        let b = a as *const u8;
        let needed = 16; // "255.255.255.255\0"
        if size < needed as u32 { ERRNO!() = ENOSPC; return core::ptr::null_mut(); }
        let mut pos = 0;
        for i in 0..4 {
            if i > 0 { *s.add(pos) = b'.' as c_char; pos += 1; }
            let (buf, len) = format_u64(*b.add(i) as u64);
            for j in 0..len { *s.add(pos) = buf[j] as c_char; pos += 1; }
        }
        *s.add(pos) = 0;
        s
    } else if af == AF_INET6_VAL {
        let b = a as *const u8;
        let needed = 46; // max IPv6
        if size < needed as u32 { ERRNO!() = ENOSPC; return core::ptr::null_mut(); }
        let mut words = [0u16; 8];
        for i in 0..8 {
            words[i] = ((*b.add(i * 2) as u16) << 8) | (*b.add(i * 2 + 1) as u16);
        }
        // IPv4-mapped: ::ffff:a.b.c.d
        let mut pos = 0usize;
        if words[0] == 0 && words[1] == 0 && words[2] == 0 && words[3] == 0
            && words[4] == 0 && words[5] == 0xffff
        {
            let prefix = b"::ffff:";
            for k in 0..prefix.len() { *s.add(pos) = prefix[k] as c_char; pos += 1; }
            for idx in 0..4 {
                if idx > 0 { *s.add(pos) = b'.' as c_char; pos += 1; }
                let octet = *b.add(12 + idx);
                if octet >= 100 {
                    *s.add(pos) = (b'0' + octet / 100) as c_char; pos += 1;
                    *s.add(pos) = (b'0' + (octet / 10) % 10) as c_char; pos += 1;
                } else if octet >= 10 {
                    *s.add(pos) = (b'0' + octet / 10) as c_char; pos += 1;
                }
                *s.add(pos) = (b'0' + octet % 10) as c_char; pos += 1;
            }
            *s.add(pos) = 0;
            return s;
        }
        // find longest run of zeros
        let mut best_start: isize = -1;
        let mut best_len: usize = 0;
        let mut j: usize = 0;
        while j < 8 {
            if words[j] == 0 {
                let mut k = j;
                while k < 8 && words[k] == 0 { k += 1; }
                let run = k - j;
                if run > best_len { best_start = j as isize; best_len = run; }
                j = k;
            } else {
                j += 1;
            }
        }
        let mut i: usize = 0;
        let mut prev_in_run = false;
        while i < 8 {
            let in_run = best_len >= 2 && i >= best_start as usize && i < best_start as usize + best_len;
            if in_run {
                if i == best_start as usize {
                    *s.add(pos) = b':' as c_char; pos += 1;
                    *s.add(pos) = b':' as c_char; pos += 1;
                }
                i += 1;
                prev_in_run = true;
                continue;
            }
            if i > 0 && !prev_in_run {
                *s.add(pos) = b':' as c_char; pos += 1;
            }
            let (buf, len) = format_hex(words[i] as u64, false);
            for k in 0..len { *s.add(pos) = buf[k] as c_char; pos += 1; }
            i += 1;
            prev_in_run = false;
        }
        *s.add(pos) = 0;
        s
    } else {
        ERRNO!() = EAFNOSUPPORT;
        core::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn inet_aton(s: *const c_char, dest: *mut c_void) -> c_int {
    let a = dest as *mut u8;
    let mut p = s as *const u8;
    let mut vals = [0u64; 4];
    let mut i = 0usize;
    while i < 4 {
        if *p < b'0' || *p > b'9' { return 0; }
        let mut base = 10u32;
        if *p == b'0' {
            let c = *p.add(1);
            if c == b'x' || c == b'X' {
                base = 16;
                p = p.add(2);
                if !((*p >= b'0' && *p <= b'9')
                    || (*p >= b'a' && *p <= b'f')
                    || (*p >= b'A' && *p <= b'F')) {
                    return 0;
                }
            } else {
                base = 8;
            }
        }
        let mut v: u64 = 0;
        let start = p;
        loop {
            let c = *p;
            let d = if c >= b'0' && c <= b'9' {
                (c - b'0') as u64
            } else if base == 16 && c >= b'a' && c <= b'f' {
                (c - b'a' + 10) as u64
            } else if base == 16 && c >= b'A' && c <= b'F' {
                (c - b'A' + 10) as u64
            } else {
                break;
            };
            v = v * (base as u64) + d;
            p = p.add(1);
        }
        if p == start { return 0; }
        vals[i] = v;
        if *p == 0 {
            break;
        }
        if *p != b'.' || i == 3 { return 0; }
        p = p.add(1);
        i += 1;
    }
    if i == 4 { return 0; }
    if i < 1 { vals[1] = vals[0] & 0xffffff; vals[0] >>= 24; }
    if i < 2 { vals[2] = vals[1] & 0xffff; vals[1] >>= 16; }
    if i < 3 { vals[3] = vals[2] & 0xff; vals[2] >>= 8; }
    for j in 0..4 {
        if vals[j] > 255 { return 0; }
        *a.add(j) = vals[j] as u8;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn inet_addr(s: *const c_char) -> u32 {
    let mut a = [0u8; 4];
    if inet_aton(s, a.as_mut_ptr() as *mut c_void) == 1 {
        u32::from_ne_bytes(a)
    } else {
        !0u32 // INADDR_NONE
    }
}

#[no_mangle]
pub unsafe extern "C" fn inet_ntoa(addr: u32) -> *mut c_char {
    static mut NTOA_BUF: [c_char; 16] = [0; 16];
    let b = addr.to_ne_bytes();
    let mut pos = 0;
    for i in 0..4 {
        if i > 0 { NTOA_BUF[pos] = b'.' as c_char; pos += 1; }
        let (buf, len) = format_u64(b[i] as u64);
        for j in 0..len { NTOA_BUF[pos] = buf[j] as c_char; pos += 1; }
    }
    NTOA_BUF[pos] = 0;
    core::ptr::addr_of_mut!(NTOA_BUF).cast::<c_char>()
}

// inet_network/inet_makeaddr/inet_lnaof/inet_netof - simple implementations
#[no_mangle]
pub unsafe extern "C" fn inet_network(s: *const c_char) -> u32 {
    inet_addr(s).to_be()
}

#[no_mangle]
pub extern "C" fn inet_makeaddr(net: c_int, host: c_int) -> u32 {
    // musl's inet_legacy: class A when the network number fits in 8 bits,
    // class B when it fits in 16, otherwise the top 24 bits.
    let n = net as u32;
    let mut h = host as u32;
    if n < 256 {
        h |= n << 24;
    } else if n < 65536 {
        h |= n << 16;
    } else {
        h |= n << 8;
    }
    h
}

#[no_mangle]
pub extern "C" fn inet_lnaof(addr: u32) -> c_int {
    // s_addr already carries the network-order numeric value, exactly like
    // musl's inet_legacy.c; do not byte-swap it again.
    let h = addr;
    if h < 0x80000000 { (h & 0x00ffffff) as c_int }
    else if h < 0xc0000000 { (h & 0x0000ffff) as c_int }
    else { (h & 0x000000ff) as c_int }
}

#[no_mangle]
pub extern "C" fn inet_netof(addr: u32) -> c_int {
    let h = addr;
    if h < 0x80000000 { (h >> 24) as c_int }
    else if h < 0xc0000000 { (h >> 16) as c_int }
    else { (h >> 8) as c_int }
}

// ============================================================
// stdlib: strtod / strtof / strtold
// ============================================================

// Correctly-rounded float parser.
// Decimal: collects digits into buffer, uses core::str::FromStr.
// Hex: u128 integer arithmetic with round-to-nearest-even.
// is_f32: true for strtof (f32::from_str), false for strtod/strtold (f64::from_str).
unsafe fn parse_float(s: *const u8, endptr: *mut *mut u8, is_f32: bool) -> f64 {
    let mut p = s;
    // skip whitespace
    while *p == b' ' || *p == b'\t' || *p == b'\n' || *p == b'\r' { p = p.add(1); }
    let mut neg = false;
    if *p == b'-' { neg = true; p = p.add(1); }
    else if *p == b'+' { p = p.add(1); }

    // inf
    if (*p | 0x20) == b'i'
        && (*p.add(1) | 0x20) == b'n'
        && (*p.add(2) | 0x20) == b'f'
    {
        p = p.add(3);
        // optionally consume "inity"
        if (*p | 0x20) == b'i'
            && (*p.add(1) | 0x20) == b't'
            && (*p.add(2) | 0x20) == b'y'
        {
            p = p.add(3);
        }
        if !endptr.is_null() { *endptr = p as *mut u8; }
        return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
    }
    // nan
    if (*p | 0x20) == b'n'
        && (*p.add(1) | 0x20) == b'a'
        && (*p.add(2) | 0x20) == b'n'
    {
        p = p.add(3);
        // optionally consume (n-char-sequence)
        if *p == b'(' {
            p = p.add(1);
            while *p != 0 && *p != b')' { p = p.add(1); }
            if *p == b')' { p = p.add(1); }
        }
        if !endptr.is_null() { *endptr = p as *mut u8; }
        return if neg { -f64::NAN } else { f64::NAN };
    }

    // hex float
    if *p == b'0' && (*p.add(1) == b'x' || *p.add(1) == b'X') {
        p = p.add(2);
        let mut mant: u128 = 0;
        let mut frac_hex: i64 = 0;
        let mut dropped: i64 = 0;
        let mut tail_nonzero = false;
        let mut found = false;
        let mut in_frac = false;

        loop {
            if let Some(d) = hex_val(*p) {
                if mant <= u128::MAX / 16 {
                    mant = mant * 16 + d as u128;
                } else {
                    // Too many significant digits for u128: keep the leading
                    // ones and remember that the dropped tail is nonzero.
                    dropped += 1;
                    if d != 0 { tail_nonzero = true; }
                }
                if in_frac { frac_hex += 1; }
                p = p.add(1);
                found = true;
            } else if *p == b'.' && !in_frac {
                in_frac = true;
                p = p.add(1);
            } else {
                break;
            }
        }

        if !found {
            if !endptr.is_null() { *endptr = s as *mut u8; }
            return 0.0;
        }

        // binary exponent p[+-]N
        let mut bin_exp: i32 = 0;
        if *p == b'p' || *p == b'P' {
            p = p.add(1);
            let mut exp_neg = false;
            if *p == b'-' { exp_neg = true; p = p.add(1); }
            else if *p == b'+' { p = p.add(1); }
            while *p >= b'0' && *p <= b'9' {
                if bin_exp < 100000 {
                    bin_exp = bin_exp * 10 + (*p - b'0') as i32;
                }
                p = p.add(1);
            }
            if exp_neg { bin_exp = -bin_exp; }
        }

        if !endptr.is_null() { *endptr = p as *mut u8; }
        if mant == 0 { return if neg { -0.0 } else { 0.0 }; }

        // The exponent must account for every fractional digit, including
        // the ones dropped from `mant`.
        let total_exp = bin_exp as i64 - 4 * (frac_hex - dropped);
        let r = hex_mant_to_f64(mant, total_exp, neg, tail_nonzero);
        note_float_range(r, true, is_f32);
        return r;
    }

    // decimal: collect the significant digits into a bounded buffer and let
    // Rust's parser do the correctly-rounded conversion. Keeping only the
    // leading digits (and folding the rest into the exponent) keeps the stack
    // footprint small without changing the result.
    const MAX_SIG_DIGITS: usize = 768;
    let mut buf = [0u8; MAX_SIG_DIGITS + 32];
    let mut n = 0usize;
    let mut found_digit = false;
    let mut nonzero = false;
    let mut frac_kept: i64 = 0;
    let mut frac_lead_zeros: i64 = 0;
    let mut int_dropped: i64 = 0;

    while *p >= b'0' && *p <= b'9' {
        if *p != b'0' { nonzero = true; }
        if nonzero {
            if n < MAX_SIG_DIGITS {
                buf[n] = *p;
                n += 1;
            } else {
                int_dropped += 1;
            }
        }
        p = p.add(1);
        found_digit = true;
    }
    if *p == b'.' {
        p = p.add(1);
        while *p >= b'0' && *p <= b'9' {
            if *p != b'0' { nonzero = true; }
            if !nonzero {
                // Leading fractional zeros still shift the exponent.
                frac_lead_zeros += 1;
            } else if n < MAX_SIG_DIGITS {
                buf[n] = *p;
                n += 1;
                frac_kept += 1;
            }
            p = p.add(1);
            found_digit = true;
        }
    }
    if !found_digit {
        ERRNO!() = EINVAL;
        if !endptr.is_null() { *endptr = s as *mut u8; }
        return 0.0;
    }
    let mut exp10: i64 = int_dropped - frac_kept - frac_lead_zeros;
    if *p == b'e' || *p == b'E' {
        let mut q = p.add(1);
        let mut exp_neg = false;
        if *q == b'-' { exp_neg = true; q = q.add(1); }
        else if *q == b'+' { q = q.add(1); }
        // Without at least one exponent digit the 'e' is not part of the
        // number; leave the end pointer on it.
        if *q >= b'0' && *q <= b'9' {
            let mut e: i64 = 0;
            while *q >= b'0' && *q <= b'9' {
                e = e.saturating_mul(10).saturating_add((*q - b'0') as i64);
                if e > 1_000_000 { e = 1_000_000; }
                q = q.add(1);
            }
            exp10 += if exp_neg { -e } else { e };
            p = q;
        }
    }
    if !endptr.is_null() { *endptr = p as *mut u8; }
    if n == 0 {
        // Every digit was zero.
        return if neg { -0.0 } else { 0.0 };
    }

    // Represent the kept digits as an integer times a power of ten.
    buf[n] = b'e';
    n += 1;
    let mut tmp = [0u8; 24];
    let mut tn = 0usize;
    let e_abs = exp10.unsigned_abs();
    if e_abs == 0 {
        tmp[0] = b'0';
        tn = 1;
    } else {
        let mut v = e_abs;
        while v > 0 && tn < tmp.len() {
            tmp[tn] = b'0' + (v % 10) as u8;
            v /= 10;
            tn += 1;
        }
    }
    if exp10 < 0 { buf[n] = b'-'; n += 1; }
    for k in (0..tn).rev() {
        buf[n] = tmp[k];
        n += 1;
    }

    let s_str = core::str::from_utf8_unchecked(
        core::slice::from_raw_parts(buf.as_ptr(), n),
    );
    if is_f32 {
        match <f32 as core::str::FromStr>::from_str(s_str) {
            Ok(v) => {
                let r = if neg { -(v as f64) } else { v as f64 };
                note_float_range(r, nonzero, true);
                r
            }
            Err(_) => { if !endptr.is_null() { *endptr = s as *mut u8; } 0.0 }
        }
    } else {
        match <f64 as core::str::FromStr>::from_str(s_str) {
            Ok(v) => {
                let r = if neg { -v } else { v };
                note_float_range(r, nonzero, false);
                r
            }
            Err(_) => { if !endptr.is_null() { *endptr = s as *mut u8; } 0.0 }
        }
    }
}

/// Set ERANGE for overflow and underflow, like musl's floatscan.
unsafe fn note_float_range(r: f64, nonzero: bool, is_f32: bool) {
    let (infinite, zero, subnormal) = if is_f32 {
        let v = r as f32;
        (v.is_infinite(), v == 0.0, v != 0.0 && v.abs() < f32::MIN_POSITIVE)
    } else {
        (r.is_infinite(), r == 0.0, r != 0.0 && r.abs() < f64::MIN_POSITIVE)
    };
    if infinite || subnormal || (zero && nonzero) {
        ERRNO!() = ERANGE;
    }
}

// Convert hex mantissa integer to f64 with correct rounding.
// mant: integer from parsed hex digits. total_exp: bin_exp - 4 * frac_hex_digits.
// tail_nonzero records dropped digits below `mant` for the sticky bit.
// Value = mant * 2^total_exp.
fn hex_mant_to_f64(mant: u128, total_exp: i64, neg: bool, tail_nonzero: bool) -> f64 {
    if mant == 0 { return if neg { -0.0 } else { 0.0 }; }

    let msb = (127 - mant.leading_zeros()) as i64;
    let unbiased_exp = msb + total_exp;
    let biased_exp = unbiased_exp + 1023;

    // Overflow → infinity
    if biased_exp >= 2047 {
        return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
    }

    // Subnormal or underflow
    if biased_exp <= 0 {
        // subnormal: biased_exp=0, effective exponent=-1022
        // mantissa = round(mant * 2^(total_exp + 1074))
        let shift = total_exp + 1074;
        if shift < 0 {
            return if neg { -0.0 } else { 0.0 };
        }
        if shift > 128 {
            return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
        }
        let shifted = mant << (shift as u32);
        let mantissa52 = (shifted as u64) & 0x000FFFFFFFFFFFFF;
        let guard = ((shifted >> 52) & 1) != 0;
        let sticky = (shifted >> 53) != 0 || tail_nonzero;
        let mut result = mantissa52;
        if guard && (sticky || (result & 1) != 0) {
            result += 1;
            if result > 0x000FFFFFFFFFFFFF {
                // carry → smallest normal
                return if neg { -f64::from_bits(1u64 << 52) } else { f64::from_bits(1u64 << 52) };
            }
        }
        return if neg { -f64::from_bits(result) } else { f64::from_bits(result) };
    }

    // Normal: biased_exp in [1, 2046]
    let shift = msb - 52;

    if shift <= 0 {
        // fewer than 53 bits, no rounding needed
        let mantissa = ((mant as u64) & ((1u64 << msb as u32) - 1)) << ((-shift) as u32);
        let bits = ((biased_exp as u64) << 52) | mantissa;
        return if neg { -f64::from_bits(bits) } else { f64::from_bits(bits) };
    }

    // shift > 0: round to nearest even
    let mantissa = ((mant >> shift as u32) as u64) & 0x000FFFFFFFFFFFFF;
    let guard = ((mant >> (shift as u32 - 1)) & 1) != 0;
    let sticky = (if shift > 1 {
        (mant & ((1u128 << (shift as u32 - 1)) - 1)) != 0
    } else { false }) || tail_nonzero;

    let mut result_mant = mantissa;
    let mut result_exp = biased_exp;
    if guard && (sticky || (result_mant & 1) != 0) {
        result_mant += 1;
        if result_mant > 0x000FFFFFFFFFFFFF {
            result_mant = 0;
            result_exp += 1;
            if result_exp >= 2047 {
                return if neg { f64::NEG_INFINITY } else { f64::INFINITY };
            }
        }
    }

    let bits = ((result_exp as u64) << 52) | result_mant;
    if neg { -f64::from_bits(bits) } else { f64::from_bits(bits) }
}

unsafe fn hex_val(c: u8) -> Option<u8> {
    if c >= b'0' && c <= b'9' { Some(c - b'0') }
    else if c >= b'a' && c <= b'f' { Some(c - b'a' + 10) }
    else if c >= b'A' && c <= b'F' { Some(c - b'A' + 10) }
    else { None }
}

#[no_mangle]
pub unsafe extern "C" fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    let mut end: *mut u8 = s as *mut u8;
    let r = parse_float(s as *const u8, &mut end, false);
    if !endptr.is_null() { *endptr = end as *mut c_char; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn strtof(s: *const c_char, endptr: *mut *mut c_char) -> f32 {
    let mut end: *mut u8 = s as *mut u8;
    let r = parse_float(s as *const u8, &mut end, true);
    if !endptr.is_null() { *endptr = end as *mut c_char; }
    r as f32
}

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f64 {
    strtod(s, endptr)
}

#[no_mangle]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f128 {
    f64_to_f128(strtod(s, endptr))
}

#[no_mangle]
#[cfg(any(target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
pub unsafe extern "C" fn strtold(s: *const c_char, endptr: *mut *mut c_char) -> f128 {
    f64_to_f128(strtod(s, endptr))
}

#[no_mangle]
pub unsafe extern "C" fn atof(s: *const c_char) -> f64 {
    strtod(s, core::ptr::null_mut())
}

// ============================================================
// stdio: open_memstream / fmemopen
// ============================================================

#[repr(C)]
struct MsCookie {
    bufp: *mut *mut c_char,
    sizep: *mut usize,
    pos: usize,
    len: usize,
    buf: *mut u8,
    space: usize,
}

#[repr(C)]
struct MfCookie {
    pos: usize,
    len: usize,
    size: usize,
    buf: *mut u8,
    mode: c_char,
}

unsafe extern "C" fn ms_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let c = (*f).cookie as *mut MsCookie;
    if whence < 0 || whence > 2 { ERRNO!() = EINVAL; return -1; }
    let base: i64 = match whence {
        0 => 0,
        1 => (*c).pos as i64,
        2 => (*c).len as i64,
        _ => { ERRNO!() = EINVAL; return -1; }
    };
    if off < -base || off > isize::MAX as i64 - base { ERRNO!() = EINVAL; return -1; }
    (*c).pos = (base + off) as usize;
    (*c).pos as i64
}

unsafe extern "C" fn ms_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MsCookie;
    let buffered = (*f).wpos as usize - (*f).wbase as usize;
    if buffered > 0 {
        (*f).wpos = (*f).wbase;
        if ms_write(f, (*f).wbase, buffered) < buffered { return 0; }
    }
    if len == 0 { return 0; }
    if (*c).pos + len >= (*c).space {
        let new_space = 2 * (*c).space + 1 | (*c).pos + len + 1;
        let newbuf = realloc((*c).buf as *mut c_void, new_space) as *mut u8;
        if newbuf.is_null() { return 0; }
        *(*c).bufp = newbuf as *mut c_char;
        core::ptr::write_bytes(newbuf.add((*c).space), 0, new_space - (*c).space);
        (*c).buf = newbuf;
        (*c).space = new_space;
    }
    core::ptr::copy_nonoverlapping(buf, (*c).buf.add((*c).pos), len);
    (*c).pos += len;
    if (*c).pos >= (*c).len { (*c).len = (*c).pos; }
    *(*c).sizep = (*c).pos;
    len
}

unsafe extern "C" fn ms_close(f: *mut FILE) -> c_int {
    // The byte buffer belongs to the caller; free only the cookie.
    if !f.is_null() && !(*f).cookie.is_null() {
        free((*f).cookie);
        (*f).cookie = core::ptr::null_mut();
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut FILE {
    if bufp.is_null() || sizep.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let f = calloc(1, core::mem::size_of::<FILE>() + UNGET + BUFSIZ) as *mut FILE;
    if f.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    let buf_area = buf_ptr(f);
    init_file(f, -1, b"w\0".as_ptr() as *const c_char, Some(ms_close), buf_area, BUFSIZ);
    (*f).flags = F_NORD | F_SVB;
    (*f).lbf = -1;
    let c = calloc(1, core::mem::size_of::<MsCookie>()) as *mut MsCookie;
    if c.is_null() { free(f as *mut c_void); ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    (*c).bufp = bufp;
    (*c).sizep = sizep;
    (*c).pos = 0;
    (*c).len = 0;
    let initial = crate::malloc(1) as *mut u8;
    if initial.is_null() {
        crate::free(c as *mut c_void);
        crate::free(f as *mut c_void);
        ERRNO!() = ENOMEM;
        return core::ptr::null_mut();
    }
    *initial = 0;
    (*c).buf = initial;
    (*c).space = 1;
    (*f).cookie = c as *mut c_void;
    (*f).write_fn = Some(ms_write);
    (*f).seek_fn = Some(ms_seek);
    *bufp = initial as *mut c_char;
    *sizep = 0;
    f
}

unsafe extern "C" fn mseek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let c = (*f).cookie as *mut MfCookie;
    if whence < 0 || whence > 2 { ERRNO!() = EINVAL; return -1; }
    let base: i64 = match whence {
        0 => 0,
        1 => (*c).pos as i64,
        2 => (*c).len as i64,
        _ => { ERRNO!() = EINVAL; return -1; }
    };
    if off < -base || off > (*c).size as i64 - base { ERRNO!() = EINVAL; return -1; }
    (*c).pos = (base + off) as usize;
    (*c).pos as i64
}

unsafe extern "C" fn mread(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MfCookie;
    let mut rem = if (*c).pos <= (*c).len { (*c).len - (*c).pos } else { 0 };
    let mut l = len;
    if l > rem {
        l = rem;
        (*f).flags |= F_EOF;
    }
    core::ptr::copy_nonoverlapping((*c).buf.add((*c).pos), buf, l);
    (*c).pos += l;
    rem -= l;
    if rem > (*f).buf_size { rem = (*f).buf_size; }
    (*f).rpos = (*f).buf;
    (*f).rend = (*f).buf.add(rem);
    core::ptr::copy_nonoverlapping((*c).buf.add((*c).pos), (*f).rpos, rem);
    (*c).pos += rem;
    l
}

unsafe extern "C" fn mwrite(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MfCookie;
    let buffered = (*f).wpos as usize - (*f).wbase as usize;
    if buffered > 0 {
        (*f).wpos = (*f).wbase;
        if mwrite(f, (*f).wbase, buffered) < buffered { return 0; }
    }
    if len == 0 { return 0; }
    if (*c).mode == b'a' as c_char { (*c).pos = (*c).len; }
    let rem = (*c).size - (*c).pos;
    let mut l = len;
    if l > rem { l = rem; }
    core::ptr::copy_nonoverlapping(buf, (*c).buf.add((*c).pos), l);
    (*c).pos += l;
    if (*c).pos > (*c).len {
        (*c).len = (*c).pos;
        if (*c).len < (*c).size {
            *(*c).buf.add((*c).len) = 0;
        } else if ((*f).flags & F_NORD) != 0 && (*c).size > 0 {
            *(*c).buf.add((*c).size - 1) = 0;
        }
    }
    l
}

unsafe extern "C" fn mclose(_f: *mut FILE) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE {
    if mode.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let m = *mode;
    if m != b'r' as c_char && m != b'w' as c_char && m != b'a' as c_char {
        ERRNO!() = EINVAL; return core::ptr::null_mut();
    }
    let has_plus = !strchr(mode as *const u8, b'+' as c_int).is_null();
    let need_alloc = buf.is_null();
    let alloc_size = core::mem::size_of::<FILE>() + UNGET + BUFSIZ + if need_alloc { size } else { 0 };
    let f = calloc(1, alloc_size) as *mut FILE;
    if f.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    let buf_area = buf_ptr(f);
    init_file(f, -1, mode, Some(mclose), buf_area, BUFSIZ);
    (*f).flags |= F_SVB;
    let actual_buf: *mut u8 = if need_alloc {
        (f as *mut u8).add(alloc_size - size)
    } else {
        buf as *mut u8
    };
    if need_alloc { core::ptr::write_bytes(actual_buf, 0, size); }
    let c = calloc(1, core::mem::size_of::<MfCookie>()) as *mut MfCookie;
    if c.is_null() { free(f as *mut c_void); ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    (*c).buf = actual_buf;
    (*c).size = size;
    (*c).mode = m;
    (*f).cookie = c as *mut c_void;
    (*f).read_fn = Some(mread);
    (*f).write_fn = Some(mwrite);
    (*f).seek_fn = Some(mseek);
    if !has_plus { (*f).flags = if m == b'r' as c_char { F_NOWR } else { F_NORD }; }
    if m == b'r' as c_char {
        (*c).len = size;
    } else if m == b'a' as c_char {
        (*c).len = strnlen(actual_buf, size);
        (*c).pos = (*c).len;
    } else if has_plus {
        *actual_buf = 0;
    }
    f
}

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
// ============================================================
// mntent: setmntent/endmntent/getmntent/getmntent_r/addmntent/hasmntopt
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn setmntent(name: *const c_char, mode: *const c_char) -> *mut FILE {
    fopen(name, mode)
}

#[no_mangle]
pub unsafe extern "C" fn endmntent(f: *mut FILE) -> c_int {
    if !f.is_null() {
        fclose(f);
    }
    1
}

// static buffers for getmntent (non-_r variant)
static mut MNTENT_MNT: MntEnt = MntEnt {
    mnt_fsname: core::ptr::null_mut(),
    mnt_dir: core::ptr::null_mut(),
    mnt_type: core::ptr::null_mut(),
    mnt_opts: core::ptr::null_mut(),
    mnt_freq: 0,
    mnt_passno: 0,
};

#[repr(C)]
pub struct MntEnt {
    mnt_fsname: *mut c_char,
    mnt_dir: *mut c_char,
    mnt_type: *mut c_char,
    mnt_opts: *mut c_char,
    mnt_freq: c_int,
    mnt_passno: c_int,
}

unsafe fn unescape_ent(beg: *mut c_char) -> *mut c_char {
    let mut dest = beg;
    let mut src = beg;
    while *src != 0 {
        if *src != b'\\' as c_char {
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
            continue;
        }
        // backslash
        if *src.add(1) == b'\\' as c_char {
            src = src.add(1);
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
            continue;
        }
        // octal escape
        let mut val: u8 = 0;
        let mut cnt = 0;
        let mut sp = src.add(1);
        while cnt < 3 && *sp >= b'0' as c_char && *sp <= b'7' as c_char {
            val = val * 8 + (*sp as u8 - b'0');
            sp = sp.add(1);
            cnt += 1;
        }
        if cnt > 0 {
            *dest = val as c_char;
            dest = dest.add(1);
            src = sp;
        } else {
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
        }
    }
    *dest = 0;
    beg
}

unsafe fn parse_mntent_line(
    linebuf: *mut c_char,
    mnt: *mut MntEnt,
) -> *mut MntEnt {
    let _len = strlen(linebuf as *const c_char);
    // skip comments and empty lines
    // parse: fsname dir type opts freq passno
    let mut p = linebuf;
    // skip leading whitespace
    while *p == b' ' as c_char || *p == b'\t' as c_char || *p == b'\n' as c_char || *p == b'\r' as c_char {
        p = p.add(1);
    }
    if *p == 0 || *p == b'#' as c_char {
        return core::ptr::null_mut();
    }

    // fsname
    (*mnt).mnt_fsname = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // dir
    (*mnt).mnt_dir = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // type
    (*mnt).mnt_type = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // opts
    (*mnt).mnt_opts = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p != 0 {
        *p = 0;
        p = p.add(1);
    }
    if *(*mnt).mnt_opts == 0 {
        (*mnt).mnt_opts = b"defaults\0".as_ptr() as *mut c_char;
    }

    // freq and passno
    (*mnt).mnt_freq = 0;
    (*mnt).mnt_passno = 0;
    // skip whitespace
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }
    if *p >= b'0' as c_char && *p <= b'9' as c_char {
        (*mnt).mnt_freq = (*p - b'0' as c_char) as c_int;
        p = p.add(1);
        while *p >= b'0' as c_char && *p <= b'9' as c_char {
            (*mnt).mnt_freq = (*mnt).mnt_freq * 10 + (*p - b'0' as c_char) as c_int;
            p = p.add(1);
        }
    }
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }
    if *p >= b'0' as c_char && *p <= b'9' as c_char {
        (*mnt).mnt_passno = (*p - b'0' as c_char) as c_int;
        p = p.add(1);
        while *p >= b'0' as c_char && *p <= b'9' as c_char {
            (*mnt).mnt_passno = (*mnt).mnt_passno * 10 + (*p - b'0' as c_char) as c_int;
            p = p.add(1);
        }
    }

    // unescape
    unescape_ent((*mnt).mnt_fsname);
    unescape_ent((*mnt).mnt_dir);
    unescape_ent((*mnt).mnt_type);
    unescape_ent((*mnt).mnt_opts);

    mnt
}

// Use a sentinel pointer to detect internal buffer usage
// cast integer to pointer for sentinel
static SENTINEL_PTR: usize = 1;

// Read one char from FILE buffer directly (avoids musl's static fgets ABI mismatch)
unsafe fn mntent_fgetc(f: *mut FILE) -> c_int {
    fgetc(f)
}

unsafe fn mntent_fgets(buf: *mut c_char, n: usize, f: *mut FILE) -> *mut c_char {
    if n == 0 { return core::ptr::null_mut(); }
    let max = n - 1;
    let mut i = 0usize;
    while i < max {
        let c = mntent_fgetc(f);
        if c == -1 { if i == 0 { return core::ptr::null_mut(); } break; }
        *buf.add(i) = c as c_char;
        i += 1;
        if c == b'\n' as c_int { break; }
    }
    *buf.add(i) = 0;
    buf
}

#[no_mangle]
pub unsafe extern "C" fn getmntent_r(
    f: *mut FILE,
    mnt: *mut MntEnt,
    mut linebuf: *mut c_char,
    buflen: c_int,
) -> *mut MntEnt {
    let use_internal = linebuf as usize == SENTINEL_PTR;

    (*mnt).mnt_freq = 0;
    (*mnt).mnt_passno = 0;

    static mut MNT_LINE: *mut c_char = core::ptr::null_mut();
    static mut MNT_LINE_CAP: usize = 0;

    loop {
        let res = if use_internal {
            let n = getline(&raw mut MNT_LINE, &raw mut MNT_LINE_CAP, f);
            if n < 0 {
                return core::ptr::null_mut();
            }
            linebuf = MNT_LINE;
            MNT_LINE
        } else {
            if buflen <= 0 {
                ERRNO!() = EINVAL;
                return core::ptr::null_mut();
            }
            mntent_fgets(linebuf, buflen as usize, f)
        };
        if res.is_null() {
            return core::ptr::null_mut();
        }
        let mut has_newline = false;
        let mut p = linebuf;
        while *p != 0 { if *p == b'\n' as c_char { has_newline = true; break; } p = p.add(1); }
        if !has_newline {
            ERRNO!() = ERANGE;
            return core::ptr::null_mut();
        }
        let result = parse_mntent_line(linebuf, mnt);
        if !result.is_null() {
            return result;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn getmntent(f: *mut FILE) -> *mut MntEnt {
    getmntent_r(f, core::ptr::addr_of_mut!(MNTENT_MNT), SENTINEL_PTR as *mut c_char, 0)
}

#[no_mangle]
pub unsafe extern "C" fn addmntent(f: *mut FILE, mnt: *const MntEnt) -> c_int {
    fseek(f, 0, 2); // SEEK_END
    let written = fprintf(
        f,
        b"%s\t%s\t%s\t%s\t%d\t%d\n\0".as_ptr() as *const c_char,
        (*mnt).mnt_fsname,
        (*mnt).mnt_dir,
        (*mnt).mnt_type,
        (*mnt).mnt_opts,
        (*mnt).mnt_freq,
        (*mnt).mnt_passno,
    );
    if written < 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn hasmntopt(mnt: *const MntEnt, opt: *const c_char) -> *mut c_char {
    let l = strlen(opt as *const c_char);
    let mut p = (*mnt).mnt_opts;
    loop {
        if strncmp(p as *const u8, opt as *const u8, l) == 0 {
            let after = *p.add(l);
            if after == 0 || after == b',' as c_char || after == b'=' as c_char {
                return p;
            }
        }
        let comma = strchr(p as *const u8, b',' as c_int);
        if comma.is_null() {
            return core::ptr::null_mut();
        }
        p = comma.add(1) as *mut c_char;
    }
}

// ============================================================
// SysV IPC: ftok, msg*, sem*, shm*
// x86_64 direct syscalls: msgget=68, msgsnd=69, msgrcv=70, msgctl=71
//   semget=64, semop=65, semctl=66, semtimedop=220
//   shmget=29, shmat=30, shmctl=31, shmdt=67
// ============================================================


#[inline]
unsafe fn sys_msgget(key: c_int, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_MSGGET, key as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgsnd(msqid: c_int, msgp: *const c_void, msgsz: usize, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_MSGSND, msqid as i64, msgp as i64, msgsz as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgrcv(msqid: c_int, msgp: *mut c_void, msgsz: usize, msgtyp: c_long, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall5(SYS_MSGRCV, msqid as i64, msgp as i64, msgsz as i64, msgtyp as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgctl(msqid: c_int, cmd: c_int, buf: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_MSGCTL, msqid as i64, cmd as i64, buf as i64)
}

#[inline]
unsafe fn sys_semget(key: c_int, nsems: c_int, semflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SEMGET, key as i64, nsems as i64, semflg as i64)
}

#[cfg(target_arch = "powerpc64")]
#[inline]
unsafe fn sys_semop(semid: c_int, sops: *const c_void, nsops: usize) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_SEMTIMEDOP, semid as i64, sops as i64, nsops as i64, 0)
}

#[cfg(not(target_arch = "powerpc64"))]
#[inline]
unsafe fn sys_semop(semid: c_int, sops: *const c_void, nsops: usize) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SEMOP, semid as i64, sops as i64, nsops as i64)
}

#[inline]
unsafe fn sys_semctl(semid: c_int, semnum: c_int, cmd: c_int, arg: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_SEMCTL, semid as i64, semnum as i64, cmd as i64, arg as i64)
}

#[inline]
unsafe fn sys_shmget(key: c_int, size: usize, shmflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMGET, key as i64, size as i64, shmflg as i64)
}

#[inline]
unsafe fn sys_shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMAT, shmid as i64, shmaddr as i64, shmflg as i64)
}

#[inline]
unsafe fn sys_shmdt(shmaddr: *const c_void) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_SHMDT, shmaddr as i64)
}

#[inline]
unsafe fn sys_shmctl(shmid: c_int, cmd: c_int, buf: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMCTL, shmid as i64, cmd as i64, buf as i64)
}

// ftok
#[no_mangle]
pub unsafe extern "C" fn ftok(path: *const c_char, id: c_int) -> c_int {
    let mut st: kernel_stat64 = core::mem::zeroed();
    let r = sys_stat(path as *const u8, &mut st);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    ((st.st_ino & 0xffff) | ((st.st_dev & 0xff) << 16) | (((id as u32 & 0xff) << 24)) as u64) as c_int
}

// msgget
#[no_mangle]
pub unsafe extern "C" fn msgget(key: c_int, msgflg: c_int) -> c_int {
    let r = sys_msgget(key, msgflg);
    syscall_ret(r) as c_int
}

// msgsnd
#[no_mangle]
pub unsafe extern "C" fn msgsnd(msqid: c_int, msgp: *const c_void, msgsz: usize, msgflg: c_int) -> c_int {
    let r = sys_msgsnd(msqid, msgp, msgsz, msgflg);
    syscall_ret(r) as c_int
}

// msgrcv
#[no_mangle]
pub unsafe extern "C" fn msgrcv(msqid: c_int, msgp: *mut c_void, msgsz: usize, msgtyp: c_long, msgflg: c_int) -> isize {
    let r = sys_msgrcv(msqid, msgp, msgsz, msgtyp, msgflg);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

// msgctl
#[no_mangle]
pub unsafe extern "C" fn msgctl(msqid: c_int, cmd: c_int, buf: *mut c_void) -> c_int {
    let r = sys_msgctl(msqid, cmd, buf);
    syscall_ret(r) as c_int
}

// semget
const SEM_MAX_NSEMS: c_int = 65535;

#[no_mangle]
pub unsafe extern "C" fn semget(key: c_int, nsems: c_int, semflg: c_int) -> c_int {
    if nsems > SEM_MAX_NSEMS { ERRNO!() = EINVAL; return -1; }
    let r = sys_semget(key, nsems, semflg);
    syscall_ret(r) as c_int
}

// semop
#[no_mangle]
pub unsafe extern "C" fn semop(semid: c_int, sops: *const c_void, nsops: usize) -> c_int {
    let r = sys_semop(semid, sops, nsops);
    syscall_ret(r) as c_int
}

// semctl - C variadic
#[no_mangle]
pub unsafe extern "C" fn semctl(semid: c_int, semnum: c_int, cmd: c_int, mut args: ...) -> c_int {
    let mut arg: *mut c_void = core::ptr::null_mut();
    // Commands that take the union semun argument
    match cmd {
        // SETVAL/SETALL/GETALL, IPC_SET/IPC_STAT/IPC_INFO, SEM_STAT,
        // SEM_INFO, SEM_STAT_ANY and the 32-bit compat variants all carry
        // the `union semun` argument.
        16 | 13 | 17 | 1 | 2 | 3 | 18 | 19 | 20 | 0x102 | 0x104 | 0x106 => {
            arg = args.next_arg::<*mut c_void>();
        }
        _ => {}
    }
    let r = sys_semctl(semid, semnum, cmd, arg);
    syscall_ret(r) as c_int
}

// shmget
#[no_mangle]
pub unsafe extern "C" fn shmget(key: c_int, size: usize, shmflg: c_int) -> c_int {
    let r = sys_shmget(key, size, shmflg);
    syscall_ret(r) as c_int
}

// shmat
#[no_mangle]
pub unsafe extern "C" fn shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> *mut c_void {
    let r = sys_shmat(shmid, shmaddr, shmflg);
    if (r as usize) > (-(crate::SYSCALL_ERR_MAX as isize)) as usize {
        ERRNO!() = (-r) as c_int;
        return -1isize as *mut c_void;
    }
    r as *mut c_void
}

// shmdt
#[no_mangle]
pub unsafe extern "C" fn shmdt(shmaddr: *const c_void) -> c_int {
    let r = sys_shmdt(shmaddr);
    syscall_ret(r) as c_int
}

// shmctl
#[no_mangle]
pub unsafe extern "C" fn shmctl(shmid: c_int, cmd: c_int, buf: *mut c_void) -> c_int {
    let r = sys_shmctl(shmid, cmd, buf);
    syscall_ret(r) as c_int
}

include!("startup.rs");
include!("crypt_impl.rs");
include!("statvfs.rs");
include!("daemon.rs");
include!("safety.rs");
include!("dn_expand.rs");
include!("lrand48.rs");
include!("strverscmp.rs");
include!("syscall.rs");
include!("pthread_atfork.rs");
include!("fenv.rs");
include!("locale_ctype.rs");
include!("regression_stubs.rs");
include!("ctype_tables.rs");
include!("wctype_tables.rs");
include!("netdb_extra.rs");
include!("compat_extra.rs");
include!("stdio_extra.rs");
include!("ns_parse.rs");
include!("fts.rs");
include!("ifaddrs.rs");
include!("dl_extra.rs");
include!("des.rs");
include!("path_extra.rs");
include!("wordexp.rs");
include!("getopt_impl.rs");

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

// ============================================================
// fmax/fmin and related helpers
// ============================================================

#[inline]
pub(crate) fn fmax_impl(x: f64, y: f64) -> f64 {
    if x.is_nan() { return y; }
    if y.is_nan() { return x; }
    if x == 0.0 && y == 0.0 {
        return if x.is_sign_negative() && y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    if x > y { x } else { y }
}

#[inline]
pub(crate) fn fmin_impl(x: f64, y: f64) -> f64 {
    if x.is_nan() { return y; }
    if y.is_nan() { return x; }
    if x == 0.0 && y == 0.0 {
        return if x.is_sign_negative() || y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    if x < y { x } else { y }
}

#[inline]
pub(crate) fn fmaxf_impl(x: f32, y: f32) -> f32 {
    if x.is_nan() { return y; }
    if y.is_nan() { return x; }
    if x == 0.0 && y == 0.0 {
        return if x.is_sign_negative() && y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    if x > y { x } else { y }
}

#[inline]
pub(crate) fn fminf_impl(x: f32, y: f32) -> f32 {
    if x.is_nan() { return y; }
    if y.is_nan() { return x; }
    if x == 0.0 && y == 0.0 {
        return if x.is_sign_negative() || y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    if x < y { x } else { y }
}

#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn fmax(x: f64, y: f64) -> f64 {
    fmax_impl(x, y)
}

#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn fmin(x: f64, y: f64) -> f64 {
    fmin_impl(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn fminf(x: f32, y: f32) -> f32 {
    fminf_impl(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn fdim(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x > y { x - y } else { 0.0 }
}

#[no_mangle]
pub unsafe extern "C" fn fma(x: f64, y: f64, z: f64) -> f64 {
    if (x.is_infinite() && y == 0.0) || (y.is_infinite() && x == 0.0) {
        feraiseexcept(FE_INVALID);
        return f64::NAN;
    }
    if x.is_nan() || y.is_nan() || z.is_nan() {
        return f64::NAN;
    }
    let r = libm::fma(x, y, z);
    // inf-inf is invalid; libm may return NaN without raising.
    if r.is_nan() {
        feraiseexcept(FE_INVALID);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn ilogb(x: f64) -> i32 {
    if x.is_nan() {
        feraiseexcept(FE_INVALID);
        return -0x7fffffff - 1; // FP_ILOGBNAN
    }
    if x == 0.0 || x.is_infinite() {
        feraiseexcept(FE_INVALID);
        return if x == 0.0 { -0x7fffffff - 1 } else { 0x7fffffff };
    }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp == 0 {
        // subnormal
        let mut v = bits & 0xfffffffffffff;
        let mut e = -1022;
        while v < (1u64 << 52) { v <<= 1; e -= 1; }
        e
    } else {
        exp - 1023
    }
}

#[no_mangle]
pub unsafe extern "C" fn logb(x: f64) -> f64 {
    if x.is_nan() { return x; }
    if x == 0.0 { return __math_divzero(1); }
    let bits = x.to_bits();
    let exp = ((bits >> 52) & 0x7ff) as i32;
    if exp == 0 {
        // subnormal
        let mut v = bits & 0xfffffffffffff;
        let mut e = -1022i32;
        while v < (1u64 << 52) { v <<= 1; e -= 1; }
        e as f64
    } else if exp == 0x7ff {
        x * x // infinity * infinity = infinity (preserves sign)
    } else {
        (exp - 1023) as f64
    }
}

#[no_mangle]
pub unsafe extern "C" fn nearbyint(x: f64) -> f64 {
    let had_inexact = fetestexcept(FE_INEXACT) & FE_INEXACT != 0;
    let r = libm::rint(x);
    if !had_inexact {
        feclearexcept(FE_INEXACT);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn nextafter(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() { return x + y; }
    if x == 0.0 && y == 0.0 { return y; }
    if x == y { return x; }
    let r = libm::nextafter(x, y);
    if r.is_infinite() && x.is_finite() {
        feraiseexcept(FE_OVERFLOW | FE_INEXACT);
    } else if r != x {
        let bits = r.to_bits();
        let is_subnormal = (bits & 0x7ff0000000000000) == 0 && (bits & 0xfffffffffffff) != 0;
        if r == 0.0 || is_subnormal {
            feraiseexcept(FE_UNDERFLOW | FE_INEXACT);
        }
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn nexttoward(x: f64, y: f64) -> f64 {
    // On x86_64, long double = f64, so nexttoward = nextafter
    if x == 0.0 && y == 0.0 {
        return if y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    nextafter(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn remainder(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() { return x + y; }
    if y == 0.0 || x.is_infinite() { return __math_invalid(x); }
    libm::remainder(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn remquo(x: f64, y: f64, quo: *mut i32) -> f64 {
    if x.is_nan() || y.is_nan() {
        if !quo.is_null() { *quo = 0; }
        return x + y;
    }
    if y == 0.0 || x.is_infinite() {
        if !quo.is_null() { *quo = 0; }
        return __math_invalid(x);
    }
    let (r, q) = libm::remquo(x, y);
    if !quo.is_null() { *quo = q; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn exp2(x: f64) -> f64 {
    exp2_impl(x)
}

#[no_mangle]
pub unsafe extern "C" fn expm1(x: f64) -> f64 {
    let r = libm::expm1(x);
    // Raise UNDERFLOW for very small results that underflow
    if r != 0.0 && x != 0.0 && libm::fabs(x) < 1e-307 {
        feraiseexcept(FE_UNDERFLOW | FE_INEXACT);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn log1p(x: f64) -> f64 {
    if x == -1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f64::NEG_INFINITY;
    }
    libm::log1p(x)
}

#[no_mangle]
pub unsafe extern "C" fn acosh(x: f64) -> f64 {
    if x < 1.0 {
        feraiseexcept(FE_INVALID);
        return f64::NAN;
    }
    libm::acosh(x)
}

#[no_mangle]
pub unsafe extern "C" fn asinh(x: f64) -> f64 {
    if x == 0.0 { return x; }
    libm::asinh(x)
}

#[no_mangle]
pub unsafe extern "C" fn atanh(x: f64) -> f64 {
    if x == 0.0 { return x; }
    let e = ((x.to_bits() >> 52) & 0x7ff) as i32;
    if e < 0x3ff - 32 {
        feraiseexcept(FE_INEXACT);
        if e == 0 { feraiseexcept(FE_UNDERFLOW); }
        return x;
    }
    if x == 1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f64::INFINITY;
    }
    if x == -1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f64::NEG_INFINITY;
    }
    if x.is_nan() { return x; }
    let ax = libm::fabs(x);
    if ax > 1.0 {
        feraiseexcept(FE_INVALID);
        return f64::NAN;
    }
    if ax < 0.5 {
        // For small x: use log1p formula with |x| and negate
        let r = libm::log1p(2.0 * (ax + ax * ax / (1.0 - ax))) / 2.0;
        if x < 0.0 { -r } else { r }
    } else {
        // For larger x: use log formula with |x| and negate
        let r = 0.5 * libm::log((1.0 + ax) / (1.0 - ax));
        if x < 0.0 { -r } else { r }
    }
}

#[no_mangle]
pub unsafe extern "C" fn cbrt(x: f64) -> f64 {
    libm::cbrt(x)
}

#[no_mangle]
pub unsafe extern "C" fn erf(x: f64) -> f64 {
    libm::erf(x)
}

#[no_mangle]
pub unsafe extern "C" fn erfc(x: f64) -> f64 {
    libm::erfc(x)
}

#[no_mangle]
pub unsafe extern "C" fn lgamma(x: f64) -> f64 {
    lgamma_impl(x)
}

#[no_mangle]
pub unsafe extern "C" fn tgamma(x: f64) -> f64 {
    tgamma_impl(x)
}

#[no_mangle]
pub unsafe extern "C" fn lround(x: f64) -> core::ffi::c_long {
    libm::round(x) as core::ffi::c_long
}

#[no_mangle]
pub unsafe extern "C" fn llround(x: f64) -> i64 {
    libm::round(x) as i64
}

// ============================================================
// Complex math functions - using (f64, f64) pairs
// ============================================================

// Helper: complex number as (real, imag) pair
// cabs: magnitude = sqrt(re^2 + im^2)
#[no_mangle]
pub unsafe extern "C" fn cabs(z: (f64, f64)) -> f64 {
    libm::sqrt(z.0 * z.0 + z.1 * z.1)
}

#[no_mangle]
pub unsafe extern "C" fn cabsf(z: (f32, f32)) -> f32 {
    libm::sqrtf(z.0 * z.0 + z.1 * z.1)
}

#[no_mangle]
pub unsafe extern "C" fn cabsl(z: (f64, f64)) -> f64 {
    cabs(z)
}

#[no_mangle]
pub unsafe extern "C" fn carg(z: (f64, f64)) -> f64 {
    libm::atan2(z.1, z.0)
}

#[no_mangle]
pub unsafe extern "C" fn cargf(z: (f32, f32)) -> f32 {
    libm::atan2f(z.1, z.0)
}

#[no_mangle]
pub unsafe extern "C" fn cargl(z: (f64, f64)) -> f64 {
    carg(z)
}

#[no_mangle]
pub unsafe extern "C" fn cimag(z: (f64, f64)) -> f64 {
    z.1
}

#[no_mangle]
pub unsafe extern "C" fn cimagf(z: (f32, f32)) -> f32 {
    z.1
}

#[no_mangle]
pub unsafe extern "C" fn cimagl(z: (f64, f64)) -> f64 {
    z.1
}

#[no_mangle]
pub unsafe extern "C" fn creal(z: (f64, f64)) -> f64 {
    z.0
}

#[no_mangle]
pub unsafe extern "C" fn crealf(z: (f32, f32)) -> f32 {
    z.0
}

#[no_mangle]
pub unsafe extern "C" fn creall(z: (f64, f64)) -> f64 {
    z.0
}

#[no_mangle]
pub unsafe extern "C" fn conj(z: (f64, f64)) -> (f64, f64) {
    (z.0, -z.1)
}

#[no_mangle]
pub unsafe extern "C" fn conjf(z: (f32, f32)) -> (f32, f32) {
    (z.0, -z.1)
}

#[no_mangle]
pub unsafe extern "C" fn conjl(z: (f64, f64)) -> (f64, f64) {
    conj(z)
}

#[no_mangle]
pub unsafe extern "C" fn cproj(z: (f64, f64)) -> (f64, f64) {
    if z.0.is_infinite() || z.1.is_infinite() {
        (f64::INFINITY, if z.1.is_sign_positive() { 0.0 } else { -0.0 })
    } else {
        z
    }
}

#[no_mangle]
pub unsafe extern "C" fn cprojf(z: (f32, f32)) -> (f32, f32) {
    if z.0.is_infinite() || z.1.is_infinite() {
        (f32::INFINITY, if z.1.is_sign_positive() { 0.0 } else { -0.0 })
    } else {
        z
    }
}

#[no_mangle]
pub unsafe extern "C" fn cprojl(z: (f64, f64)) -> (f64, f64) {
    cproj(z)
}

// cexp: e^z = e^(re + i*im) = e^re * (cos(im) + i*sin(im))
#[no_mangle]
pub unsafe extern "C" fn cexp(z: (f64, f64)) -> (f64, f64) {
    let r = libm::exp(z.0);
    (r * libm::cos(z.1), r * libm::sin(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn cexpf(z: (f32, f32)) -> (f32, f32) {
    let r = libm::expf(z.0);
    (r * libm::cosf(z.1), r * libm::sinf(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn cexpl(z: (f64, f64)) -> (f64, f64) {
    cexp(z)
}

// clog: ln(z) = ln|z| + i*arg(z)
#[no_mangle]
pub unsafe extern "C" fn clog(z: (f64, f64)) -> (f64, f64) {
    (libm::log(cabs(z)), carg(z))
}

#[no_mangle]
pub unsafe extern "C" fn clogf(z: (f32, f32)) -> (f32, f32) {
    (libm::logf(cabsf(z)), cargf(z))
}

#[no_mangle]
pub unsafe extern "C" fn clogl(z: (f64, f64)) -> (f64, f64) {
    clog(z)
}

// csqrt: sqrt(z) = sqrt((|z|+re)/2) + i*sign(im)*sqrt((|z|-re)/2)
#[no_mangle]
pub unsafe extern "C" fn csqrt(z: (f64, f64)) -> (f64, f64) {
    let r = cabs(z);
    let sign = if z.1 < 0.0 { -1.0 } else { 1.0 };
    (libm::sqrt((r + z.0) / 2.0), sign * libm::sqrt((r - z.0) / 2.0))
}

#[no_mangle]
pub unsafe extern "C" fn csqrtf(z: (f32, f32)) -> (f32, f32) {
    let r = cabsf(z);
    let sign = if z.1 < 0.0 { -1.0f32 } else { 1.0f32 };
    (libm::sqrtf((r + z.0) / 2.0), sign * libm::sqrtf((r - z.0) / 2.0))
}

#[no_mangle]
pub unsafe extern "C" fn csqrtl(z: (f64, f64)) -> (f64, f64) {
    csqrt(z)
}

// cpow: z^w = e^(w * log(z))
#[no_mangle]
pub unsafe extern "C" fn cpow(z: (f64, f64), w: (f64, f64)) -> (f64, f64) {
    let log_z = clog(z);
    let exp_arg = (w.0 * log_z.0 - w.1 * log_z.1, w.0 * log_z.1 + w.1 * log_z.0);
    cexp(exp_arg)
}

#[no_mangle]
pub unsafe extern "C" fn cpowf(z: (f32, f32), w: (f32, f32)) -> (f32, f32) {
    let log_z = clogf(z);
    let exp_arg = (w.0 * log_z.0 - w.1 * log_z.1, w.0 * log_z.1 + w.1 * log_z.0);
    cexpf(exp_arg)
}

#[no_mangle]
pub unsafe extern "C" fn cpowl(z: (f64, f64), w: (f64, f64)) -> (f64, f64) {
    cpow(z, w)
}

// ccos: cos(z) = cos(re)*cosh(im) - i*sin(re)*sinh(im)
#[no_mangle]
pub unsafe extern "C" fn ccos(z: (f64, f64)) -> (f64, f64) {
    (libm::cos(z.0) * libm::cosh(z.1), -libm::sin(z.0) * libm::sinh(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn ccosf(z: (f32, f32)) -> (f32, f32) {
    (libm::cosf(z.0) * libm::coshf(z.1), -libm::sinf(z.0) * libm::sinhf(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn ccosl(z: (f64, f64)) -> (f64, f64) {
    ccos(z)
}

// csin: sin(z) = sin(re)*cosh(im) + i*cos(re)*sinh(im)
#[no_mangle]
pub unsafe extern "C" fn csin(z: (f64, f64)) -> (f64, f64) {
    (libm::sin(z.0) * libm::cosh(z.1), libm::cos(z.0) * libm::sinh(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn csinf(z: (f32, f32)) -> (f32, f32) {
    (libm::sinf(z.0) * libm::coshf(z.1), libm::cosf(z.0) * libm::sinhf(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn csinl(z: (f64, f64)) -> (f64, f64) {
    csin(z)
}

// ctan: tan(z) = sin(2re)/(cos(2re)+cosh(2im)) + i*sinh(2im)/(cos(2re)+cosh(2im))
#[no_mangle]
pub unsafe extern "C" fn ctan(z: (f64, f64)) -> (f64, f64) {
    let d = libm::cos(2.0 * z.0) + libm::cosh(2.0 * z.1);
    (libm::sin(2.0 * z.0) / d, libm::sinh(2.0 * z.1) / d)
}

#[no_mangle]
pub unsafe extern "C" fn ctanf(z: (f32, f32)) -> (f32, f32) {
    let d = libm::cosf(2.0 * z.0) + libm::coshf(2.0 * z.1);
    (libm::sinf(2.0 * z.0) / d, libm::sinhf(2.0 * z.1) / d)
}

#[no_mangle]
pub unsafe extern "C" fn ctanl(z: (f64, f64)) -> (f64, f64) {
    ctan(z)
}

// ccosh: cosh(z) = cosh(re)*cos(im) + i*sinh(re)*sin(im)
#[no_mangle]
pub unsafe extern "C" fn ccosh(z: (f64, f64)) -> (f64, f64) {
    (libm::cosh(z.0) * libm::cos(z.1), libm::sinh(z.0) * libm::sin(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn ccoshf(z: (f32, f32)) -> (f32, f32) {
    (libm::coshf(z.0) * libm::cosf(z.1), libm::sinhf(z.0) * libm::sinf(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn ccoshl(z: (f64, f64)) -> (f64, f64) {
    ccosh(z)
}

// csinh: sinh(z) = sinh(re)*cos(im) + i*cosh(re)*sin(im)
#[no_mangle]
pub unsafe extern "C" fn csinh(z: (f64, f64)) -> (f64, f64) {
    (libm::sinh(z.0) * libm::cos(z.1), libm::cosh(z.0) * libm::sin(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn csinhf(z: (f32, f32)) -> (f32, f32) {
    (libm::sinhf(z.0) * libm::cosf(z.1), libm::coshf(z.0) * libm::sinf(z.1))
}

#[no_mangle]
pub unsafe extern "C" fn csinhl(z: (f64, f64)) -> (f64, f64) {
    csinh(z)
}

// ctanh: tanh(z) = sinh(2re)/(cosh(2re)+cos(2im)) + i*sin(2im)/(cosh(2re)+cos(2im))
#[no_mangle]
pub unsafe extern "C" fn ctanh(z: (f64, f64)) -> (f64, f64) {
    let d = libm::cosh(2.0 * z.0) + libm::cos(2.0 * z.1);
    (libm::sinh(2.0 * z.0) / d, libm::sin(2.0 * z.1) / d)
}

#[no_mangle]
pub unsafe extern "C" fn ctanhf(z: (f32, f32)) -> (f32, f32) {
    let d = libm::coshf(2.0 * z.0) + libm::cosf(2.0 * z.1);
    (libm::sinhf(2.0 * z.0) / d, libm::sinf(2.0 * z.1) / d)
}

#[no_mangle]
pub unsafe extern "C" fn ctanhl(z: (f64, f64)) -> (f64, f64) {
    ctanh(z)
}

// cacos: acos(z) = pi/2 - asin(z)
// asin(z) = -i * log(i*z + sqrt(1 - z^2))
#[no_mangle]
pub unsafe extern "C" fn cacos(z: (f64, f64)) -> (f64, f64) {
    let asin_z = casin(z);
    (3.14159265358979323846264338327950288419716939937510582097494459230781640 / 2.0 - asin_z.0, -asin_z.1)
}

#[no_mangle]
pub unsafe extern "C" fn cacosf(z: (f32, f32)) -> (f32, f32) {
    let asin_z = casinf(z);
    (3.14159265358979323846264338327950288419716939937510582097494459230781640f32 / 2.0 - asin_z.0, -asin_z.1)
}

#[no_mangle]
pub unsafe extern "C" fn cacosl(z: (f64, f64)) -> (f64, f64) {
    cacos(z)
}

// casin: asin(z) = -i * log(i*z + sqrt(1 - z^2))
#[no_mangle]
pub unsafe extern "C" fn casin(z: (f64, f64)) -> (f64, f64) {
    // i*z = (-z.1, z.0)
    let iz = (-z.1, z.0);
    // 1 - z^2 = 1 - (z.0^2 - z.1^2 + 2i*z.0*z.1) = (1 - z.0^2 + z.1^2, -2*z.0*z.1)
    let one_minus_z2 = (1.0 - z.0 * z.0 + z.1 * z.1, -2.0 * z.0 * z.1);
    let sqrt_val = csqrt(one_minus_z2);
    let arg = clog((iz.0 + sqrt_val.0, iz.1 + sqrt_val.1));
    // -i * log(...) = (arg.1, -arg.0)
    (arg.1, -arg.0)
}

#[no_mangle]
pub unsafe extern "C" fn casinf(z: (f32, f32)) -> (f32, f32) {
    let iz = (-z.1, z.0);
    let one_minus_z2 = (1.0 - z.0 * z.0 + z.1 * z.1, -2.0 * z.0 * z.1);
    let sqrt_val = csqrtf(one_minus_z2);
    let arg = clogf((iz.0 + sqrt_val.0, iz.1 + sqrt_val.1));
    (arg.1, -arg.0)
}

#[no_mangle]
pub unsafe extern "C" fn casinl(z: (f64, f64)) -> (f64, f64) {
    casin(z)
}

// catan: atan(z) = (i/2) * log((1 - i*z)/(1 + i*z))
#[no_mangle]
pub unsafe extern "C" fn catan(z: (f64, f64)) -> (f64, f64) {
    // i*z = (-z.1, z.0)
    // 1 - i*z = (1 + z.1, -z.0)
    // 1 + i*z = (1 - z.1, z.0)
    let num = (1.0 + z.1, -z.0);
    let den = (1.0 - z.1, z.0);
    let ratio = cdiv(num, den);
    let log_ratio = clog(ratio);
    // (i/2) * log = (-log.1/2, log.0/2)
    (-log_ratio.1 / 2.0, log_ratio.0 / 2.0)
}

#[no_mangle]
pub unsafe extern "C" fn catanf(z: (f32, f32)) -> (f32, f32) {
    let num = (1.0 + z.1, -z.0);
    let den = (1.0 - z.1, z.0);
    let ratio = cdivf(num, den);
    let log_ratio = clogf(ratio);
    (-log_ratio.1 / 2.0, log_ratio.0 / 2.0)
}

#[no_mangle]
pub unsafe extern "C" fn catanl(z: (f64, f64)) -> (f64, f64) {
    catan(z)
}

// Helper: complex division
fn cdiv(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

fn cdivf(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    let d = b.0 * b.0 + b.1 * b.1;
    ((a.0 * b.0 + a.1 * b.1) / d, (a.1 * b.0 - a.0 * b.1) / d)
}

// cacosh: acosh(z) = log(z + sqrt(z^2 - 1))
#[no_mangle]
pub unsafe extern "C" fn cacosh(z: (f64, f64)) -> (f64, f64) {
    let z2 = (z.0 * z.0 - z.1 * z.1, 2.0 * z.0 * z.1);
    let z2_minus_1 = (z2.0 - 1.0, z2.1);
    let sqrt_val = csqrt(z2_minus_1);
    clog((z.0 + sqrt_val.0, z.1 + sqrt_val.1))
}

#[no_mangle]
pub unsafe extern "C" fn cacoshf(z: (f32, f32)) -> (f32, f32) {
    let z2 = (z.0 * z.0 - z.1 * z.1, 2.0 * z.0 * z.1);
    let z2_minus_1 = (z2.0 - 1.0, z2.1);
    let sqrt_val = csqrtf(z2_minus_1);
    clogf((z.0 + sqrt_val.0, z.1 + sqrt_val.1))
}

#[no_mangle]
pub unsafe extern "C" fn cacoshl(z: (f64, f64)) -> (f64, f64) {
    cacosh(z)
}

// casinh: asinh(z) = log(z + sqrt(z^2 + 1))
#[no_mangle]
pub unsafe extern "C" fn casinh(z: (f64, f64)) -> (f64, f64) {
    let z2 = (z.0 * z.0 - z.1 * z.1, 2.0 * z.0 * z.1);
    let z2_plus_1 = (z2.0 + 1.0, z2.1);
    let sqrt_val = csqrt(z2_plus_1);
    clog((z.0 + sqrt_val.0, z.1 + sqrt_val.1))
}

#[no_mangle]
pub unsafe extern "C" fn casinhf(z: (f32, f32)) -> (f32, f32) {
    let z2 = (z.0 * z.0 - z.1 * z.1, 2.0 * z.0 * z.1);
    let z2_plus_1 = (z2.0 + 1.0, z2.1);
    let sqrt_val = csqrtf(z2_plus_1);
    clogf((z.0 + sqrt_val.0, z.1 + sqrt_val.1))
}

#[no_mangle]
pub unsafe extern "C" fn casinhl(z: (f64, f64)) -> (f64, f64) {
    casinh(z)
}

// catanh: atanh(z) = (1/2) * log((1+z)/(1-z))
#[no_mangle]
pub unsafe extern "C" fn catanh(z: (f64, f64)) -> (f64, f64) {
    let num = (1.0 + z.0, z.1);
    let den = (1.0 - z.0, -z.1);
    let ratio = cdiv(num, den);
    let log_ratio = clog(ratio);
    (log_ratio.0 / 2.0, log_ratio.1 / 2.0)
}

#[no_mangle]
pub unsafe extern "C" fn catanhf(z: (f32, f32)) -> (f32, f32) {
    let num = (1.0 + z.0, z.1);
    let den = (1.0 - z.0, -z.1);
    let ratio = cdivf(num, den);
    let log_ratio = clogf(ratio);
    (log_ratio.0 / 2.0, log_ratio.1 / 2.0)
}

#[no_mangle]
pub unsafe extern "C" fn catanhl(z: (f64, f64)) -> (f64, f64) {
    catanh(z)
}

// ============================================================
// Float math functions (f32 versions)
// ============================================================

#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn fmaxf(x: f32, y: f32) -> f32 {
    fmaxf_impl(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn fdimf(x: f32, y: f32) -> f32 {
    if x.is_nan() || y.is_nan() {
        return f32::NAN;
    }
    if x > y { x - y } else { 0.0 }
}

#[no_mangle]
pub unsafe extern "C" fn fmaf(x: f32, y: f32, z: f32) -> f32 {
    if (x.is_infinite() && y == 0.0) || (y.is_infinite() && x == 0.0) {
        feraiseexcept(FE_INVALID);
        return f32::NAN;
    }
    if x.is_nan() || y.is_nan() || z.is_nan() {
        return f32::NAN;
    }
    let r = libm::fmaf(x, y, z);
    if r.is_nan() {
        feraiseexcept(FE_INVALID);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn ilogbf(x: f32) -> i32 {
    if x.is_nan() {
        feraiseexcept(FE_INVALID);
        return -0x7fffffff - 1;
    }
    if x == 0.0 || x.is_infinite() {
        feraiseexcept(FE_INVALID);
        return if x == 0.0 { -0x7fffffff - 1 } else { 0x7fffffff };
    }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32;
    if exp == 0 {
        let mut v = bits & 0x7fffff;
        let mut e = -126;
        while v < (1u32 << 23) { v <<= 1; e -= 1; }
        e
    } else {
        exp - 127
    }
}

#[no_mangle]
pub unsafe extern "C" fn logbf(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x == 0.0 { return __math_divzerof(1); }
    let bits = x.to_bits();
    let exp = ((bits >> 23) & 0xff) as i32;
    if exp == 0 {
        let mut v = bits & 0x7fffff;
        let mut e = -126i32;
        while v < (1u32 << 23) { v <<= 1; e -= 1; }
        e as f32
    } else if exp == 0xff {
        x * x
    } else {
        (exp - 127) as f32
    }
}

#[no_mangle]
pub unsafe extern "C" fn nearbyintf(x: f32) -> f32 {
    let had_inexact = fetestexcept(FE_INEXACT) & FE_INEXACT != 0;
    let r = libm::rintf(x);
    if !had_inexact {
        feclearexcept(FE_INEXACT);
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn nextafterf(x: f32, y: f32) -> f32 {
    if x.is_nan() || y.is_nan() { return x + y; }
    if x == 0.0 && y == 0.0 { return y; }
    if x == y { return x; }
    let r = libm::nextafterf(x, y);
    if r.is_infinite() && x.is_finite() {
        feraiseexcept(FE_OVERFLOW | FE_INEXACT);
    } else if r != x {
        let bits = r.to_bits();
        let is_subnormal = (bits & 0x7f800000) == 0 && (bits & 0x7fffff) != 0;
        if r == 0.0 || is_subnormal {
            feraiseexcept(FE_UNDERFLOW | FE_INEXACT);
        }
    }
    r
}

#[no_mangle]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn nexttowardf(x: f32, y: f64) -> f32 {
    if y.is_nan() { return (x as f64 + y) as f32; }
    if x == 0.0 && y == 0.0 {
        return if y.is_sign_negative() { -0.0 } else { 0.0 };
    }
    if (x as f64) == y { return x; }
    nextafterf(x, if y > x as f64 { f32::INFINITY } else { f32::NEG_INFINITY })
}

#[no_mangle]
#[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
pub unsafe extern "C" fn nexttowardf(x: f32, y: f128) -> f32 {
    if f128_is_nan(y) { return f32::NAN; }
    let yf = f128_to_f64(y);
    if x as f64 == yf { return x; }
    nextafterf(x, if yf > x as f64 { f32::INFINITY } else { f32::NEG_INFINITY })
}

#[no_mangle]
pub unsafe extern "C" fn remainderf(x: f32, y: f32) -> f32 {
    if x.is_nan() || y.is_nan() { return x + y; }
    if y == 0.0 || x.is_infinite() { return __math_invalidf(x); }
    libm::remainderf(x, y)
}

#[no_mangle]
pub unsafe extern "C" fn remquof(x: f32, y: f32, quo: *mut i32) -> f32 {
    if x.is_nan() || y.is_nan() {
        if !quo.is_null() { *quo = 0; }
        return x + y;
    }
    if y == 0.0 || x.is_infinite() {
        if !quo.is_null() { *quo = 0; }
        return __math_invalidf(x);
    }
    let (r, q) = libm::remquof(x, y);
    if !quo.is_null() { *quo = q; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn exp2f(x: f32) -> f32 {
    if x == f32::INFINITY { return f32::INFINITY; }
    if x == f32::NEG_INFINITY { return 0.0; }
    if x.is_nan() { return f32::NAN; }
    let r = libm::exp2f(x);
    if r.is_infinite() && x.is_finite() {
        feraiseexcept(FE_OVERFLOW | FE_INEXACT);
    } else if r == 0.0 && x < -149.0 {
        feraiseexcept(FE_UNDERFLOW | FE_INEXACT);
    } else if r != 0.0 && x != libm::truncf(x) {
        let bits = r.to_bits();
        if (bits & 0x7f800000) == 0 {
            feraiseexcept(FE_UNDERFLOW | FE_INEXACT);
        }
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn expm1f(x: f32) -> f32 {
    libm::expm1f(x)
}

#[no_mangle]
pub unsafe extern "C" fn log1pf(x: f32) -> f32 {
    if x == -1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f32::NEG_INFINITY;
    }
    libm::log1pf(x)
}

#[no_mangle]
pub unsafe extern "C" fn acoshf(x: f32) -> f32 {
    if x < 1.0 {
        feraiseexcept(FE_INVALID);
        return f32::NAN;
    }
    libm::acoshf(x)
}

#[no_mangle]
pub unsafe extern "C" fn asinhf(x: f32) -> f32 {
    if x == 0.0 { return x; }
    libm::asinhf(x)
}

#[no_mangle]
pub unsafe extern "C" fn atanhf(x: f32) -> f32 {
    if x == 0.0 { return x; }
    let e = ((x.to_bits() >> 23) & 0xff) as i32;
    if e < 0x7f - 32 {
        feraiseexcept(FE_INEXACT);
        if e == 0 { feraiseexcept(FE_UNDERFLOW); }
        return x;
    }
    if x == 1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f32::INFINITY;
    }
    if x == -1.0 {
        feraiseexcept(FE_DIVBYZERO);
        return f32::NEG_INFINITY;
    }
    if x.is_nan() { return x; }
    let ax = libm::fabsf(x);
    if ax > 1.0 {
        feraiseexcept(FE_INVALID);
        return f32::NAN;
    }
    if ax < 0.5 {
        let r = libm::log1pf(2.0 * (ax + ax * ax / (1.0 - ax))) / 2.0;
        if x < 0.0 { -r } else { r }
    } else {
        let r = 0.5 * libm::logf((1.0 + ax) / (1.0 - ax));
        if x < 0.0 { -r } else { r }
    }
}

#[no_mangle]
pub unsafe extern "C" fn cbrtf(x: f32) -> f32 {
    libm::cbrtf(x)
}

#[no_mangle]
pub unsafe extern "C" fn erff(x: f32) -> f32 {
    libm::erff(x)
}

#[no_mangle]
pub unsafe extern "C" fn erfcf(x: f32) -> f32 {
    libm::erfcf(x)
}

#[no_mangle]
pub unsafe extern "C" fn lgammaf(x: f32) -> f32 {
    if x.is_nan() {
        crate::set_signgam(1);
        return x;
    }
    if x == 0.0 {
        crate::set_signgam(if x.is_sign_negative() { -1 } else { 1 });
        feraiseexcept(FE_DIVBYZERO);
        return f32::INFINITY;
    }
    let (r, sign) = libm::lgammaf_r(x);
    crate::set_signgam(sign);
    r
}

#[no_mangle]
pub unsafe extern "C" fn tgammaf(x: f32) -> f32 {
    if x.is_nan() { return x; }
    if x == 0.0 {
        feraiseexcept(FE_DIVBYZERO);
        return if x.is_sign_negative() { f32::NEG_INFINITY } else { f32::INFINITY };
    }
    if x.is_infinite() {
        return if x.is_sign_negative() { __math_invalidf(x) } else { f32::INFINITY };
    }
    if x < 0.0 && x == libm::floorf(x) {
        feraiseexcept(FE_INVALID);
        return f32::NAN;
    }
    libm::tgammaf(x)
}

#[no_mangle]
pub unsafe extern "C" fn lroundf(x: f32) -> core::ffi::c_long {
    libm::roundf(x) as core::ffi::c_long
}

#[no_mangle]
pub unsafe extern "C" fn llroundf(x: f32) -> i64 {
    libm::roundf(x) as i64
}

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

// ============================================================
// Resolver (UDP DNS)
// ============================================================

const MAX_NS: usize = 3;
const MAX_SEARCH: usize = 6;
const HOST_NOT_FOUND: c_int = 1;
const TRY_AGAIN: c_int = 2;
const NO_RECOVERY: c_int = 3;
const NO_DATA: c_int = 4;

static mut NS_COUNT: usize = 0;
static mut NS_ADDRS: [u32; MAX_NS] = [0; MAX_NS];
static mut SEARCH_COUNT: usize = 0;
static mut SEARCH_DOMAINS: [[u8; 256]; MAX_SEARCH] = [[0; 256]; MAX_SEARCH];
static RESOLV_PARSED: AtomicBool = AtomicBool::new(false);
static mut NAME_LOCK: c_int = 0;
static mut NAME_WAITERS: c_int = 0;
static mut RESOLV_INIT_LOCK: c_int = 0;
static mut RESOLV_INIT_WAITERS: c_int = 0;

unsafe fn name_lock() {
    spinlock_lock(&raw mut NAME_LOCK, &raw mut NAME_WAITERS);
}

unsafe fn name_unlock() {
    spinlock_unlock(&raw mut NAME_LOCK, &raw mut NAME_WAITERS);
}

unsafe fn resolv_ensure() {
    if RESOLV_PARSED.load(Ordering::Acquire) {
        return;
    }
    spinlock_lock(&raw mut RESOLV_INIT_LOCK, &raw mut RESOLV_INIT_WAITERS);
    if !RESOLV_PARSED.load(Ordering::Acquire) {
        let _ = resolv_parse_config();
        RESOLV_PARSED.store(true, Ordering::Release);
    }
    spinlock_unlock(&raw mut RESOLV_INIT_LOCK, &raw mut RESOLV_INIT_WAITERS);
}

#[repr(C)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe fn resolv_skip_ws(line: &[u8], mut i: usize) -> usize {
    while i < line.len() && (line[i] == b' ' || line[i] == b'\t') { i += 1; }
    i
}

unsafe fn resolv_token<'a>(line: &'a [u8], mut i: usize) -> (&'a [u8], usize) {
    let start = i;
    while i < line.len() && line[i] != b' ' && line[i] != b'\t' && line[i] != b'#' { i += 1; }
    (&line[start..i], i)
}

unsafe fn resolv_parse_config() -> c_int {
    NS_COUNT = 0;
    SEARCH_COUNT = 0;
    let path = b"/etc/resolv.conf\0";
    let fd = crate::open(path.as_ptr() as *const c_char, 0, 0);
    if fd < 0 { return -1; }
    let mut buf = [0u8; 4096];
    let n = crate::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
    crate::close(fd);
    if n <= 0 { return -1; }
    let len = n as usize;
    let mut i = 0;
    while i < len {
        let start = i;
        while i < len && buf[i] != b'\n' { i += 1; }
        let line = &buf[start..i];
        i += 1;
        let mut j = resolv_skip_ws(line, 0);
        if j >= line.len() || line[j] == b'#' { continue; }
        let (key, next) = resolv_token(line, j);
        j = next;
        if key == b"nameserver" {
            j = resolv_skip_ws(line, j);
            let (tok, _) = resolv_token(line, j);
            if !tok.is_empty() && NS_COUNT < MAX_NS {
                let mut name = [0u8; 64];
                let l = core::cmp::min(tok.len(), 63);
                name[..l].copy_from_slice(&tok[..l]);
                let addr = crate::inet_addr(name.as_ptr() as *const c_char);
                if addr != u32::MAX {
                    NS_ADDRS[NS_COUNT] = addr;
                    NS_COUNT += 1;
                }
            }
        } else if key == b"search" || key == b"domain" {
            loop {
                j = resolv_skip_ws(line, j);
                if j >= line.len() || line[j] == b'#' { break; }
                let (tok, next) = resolv_token(line, j);
                j = next;
                if tok.is_empty() { break; }
                if SEARCH_COUNT < MAX_SEARCH {
                    let l = core::cmp::min(tok.len(), 255);
                    SEARCH_DOMAINS[SEARCH_COUNT][..l].copy_from_slice(&tok[..l]);
                    SEARCH_DOMAINS[SEARCH_COUNT][l] = 0;
                    SEARCH_COUNT += 1;
                }
                if key == b"domain" { break; }
            }
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn res_init() -> c_int {
    resolv_ensure();
    0
}

#[no_mangle]
pub unsafe extern "C" fn res_mkquery(
    op: c_int,
    dname: *const c_char,
    class: c_int,
    type_: c_int,
    _data: *const u8,
    _datalen: c_int,
    _newrr: *const u8,
    buf: *mut u8,
    buflen: c_int,
) -> c_int {
    if dname.is_null() || buf.is_null() { return -1; }
    let mut l = crate::strnlen(dname as *const u8, 255);
    if l > 0 && *dname.add(l - 1) == b'.' as c_char { l -= 1; }
    if l > 0 && *dname.add(l - 1) == b'.' as c_char { return -1; }
    let n = 17 + l + if l != 0 { 1 } else { 0 };
    if l > 253 || buflen < n as c_int || op < 0 || op > 15 || class < 0 || class > 255 || type_ < 0 || type_ > 255 {
        return -1;
    }
    let mut q = [0u8; 280];
    crate::memset(q.as_mut_ptr() as *mut c_void, 0, n);
    q[2] = (op * 8 + 1) as u8;
    q[3] = 32;
    q[5] = 1;
    core::ptr::copy_nonoverlapping(dname as *const u8, q.as_mut_ptr().add(13), l);
    let mut i = 13;
    while q[i] != 0 {
        let mut j = i;
        while q[j] != 0 && q[j] != b'.' { j += 1; }
        if j - i > 63 { return -1; }
        q[i - 1] = (j - i) as u8;
        i = j + 1;
    }
    q[i + 1] = type_ as u8;
    q[i + 3] = class as u8;
    let mut ts = timespec { tv_sec: 0, tv_nsec: 0 };
    let _ = clock_gettime(CLOCK_REALTIME, &mut ts);
    let id = (((ts.tv_nsec as u64) + (ts.tv_nsec as u64) / 65536) & 0xffff) as u16;
    q[0] = (id >> 8) as u8;
    q[1] = id as u8;
    core::ptr::copy_nonoverlapping(q.as_ptr(), buf, n);
    n as c_int
}

unsafe fn res_send_inner(msg: *const u8, msglen: c_int, answer: *mut u8, anslen: c_int) -> c_int {
    if msg.is_null() || answer.is_null() || msglen < 12 || anslen < 12 { return -1; }
    resolv_ensure();
    if NS_COUNT == 0 {
        H_ERRNO!() = TRY_AGAIN;
        return -1;
    }
    for n in 0..NS_COUNT {
        let fd = crate::socket(AF_INET, SOCK_DGRAM, 0);
        if fd < 0 { continue; }
        let sa = SockAddrIn {
            sin_family: AF_INET as u16,
            sin_port: 53u16.to_be(),
            sin_addr: NS_ADDRS[n],
            sin_zero: [0; 8],
        };
        let sent = crate::sendto(fd, msg as *const c_void, msglen as usize, 0, &sa as *const SockAddrIn as *const crate::sockaddr, 16);
        if sent != msglen as isize {
            crate::close(fd);
            continue;
        }
        let mut pfd = PollFd { fd, events: 1, revents: 0 };
        let pr = crate::poll(&mut pfd as *mut PollFd as *mut c_void, 1, 5000);
        if pr <= 0 {
            crate::close(fd);
            continue;
        }
        let r = crate::recvfrom(fd, answer as *mut c_void, anslen as usize, 0, core::ptr::null_mut(), core::ptr::null_mut());
        crate::close(fd);
        if r >= 12 {
            let id_ok = *answer == *msg && *answer.add(1) == *msg.add(1);
            let qr = (*answer.add(2) & 0x80) != 0;
            if id_ok && qr {
                return r as c_int;
            }
        }
    }
    H_ERRNO!() = TRY_AGAIN;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn res_send(msg: *const u8, msglen: c_int, answer: *mut u8, anslen: c_int) -> c_int {
    if anslen < 512 {
        let mut buf = [0u8; 512];
        let r = res_send_inner(msg, msglen, buf.as_mut_ptr(), 512);
        if r >= 0 {
            let copy = core::cmp::min(r, anslen) as usize;
            if copy > 0 { core::ptr::copy_nonoverlapping(buf.as_ptr(), answer, copy); }
        }
        return r;
    }
    res_send_inner(msg, msglen, answer, anslen)
}

#[no_mangle]
pub unsafe extern "C" fn res_query(
    dname: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    let mut q = [0u8; 280];
    let ql = res_mkquery(0, dname, class, type_, core::ptr::null(), 0, core::ptr::null(), q.as_mut_ptr(), q.len() as c_int);
    if ql < 0 { return ql; }
    let r = res_send(q.as_ptr(), ql, answer, anslen);
    if r < 12 {
        H_ERRNO!() = TRY_AGAIN;
        return -1;
    }
    let rcode = answer.add(3).read() & 0x0f;
    if rcode == 3 {
        H_ERRNO!() = HOST_NOT_FOUND;
        return -1;
    }
    if rcode == 0 && *answer.add(6) == 0 && *answer.add(7) == 0 {
        H_ERRNO!() = NO_DATA;
        return -1;
    }
    if rcode != 0 {
        H_ERRNO!() = NO_RECOVERY;
        return -1;
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn res_querydomain(
    name: *const c_char,
    domain: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    if domain.is_null() || *domain == 0 { return res_query(name, class, type_, answer, anslen); }
    let mut buf = [0u8; 512];
    let nl = crate::strlen(name);
    let dl = crate::strlen(domain);
    if nl + 1 + dl >= buf.len() { return -1; }
    core::ptr::copy_nonoverlapping(name, buf.as_mut_ptr() as *mut c_char, nl);
    buf[nl] = b'.';
    core::ptr::copy_nonoverlapping(domain, buf.as_mut_ptr().add(nl + 1) as *mut c_char, dl);
    buf[nl + 1 + dl] = 0;
    res_query(buf.as_ptr() as *const c_char, class, type_, answer, anslen)
}

#[no_mangle]
pub unsafe extern "C" fn res_search(
    name: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    if name.is_null() { return -1; }
    resolv_ensure();
    let len = crate::strlen(name);
    let mut dots = 0;
    for i in 0..len {
        if *name.add(i) == b'.' as c_char { dots += 1; }
    }
    let trailing_dot = len > 0 && *name.add(len - 1) == b'.' as c_char;
    if trailing_dot || dots >= 1 {
        let r = res_query(name, class, type_, answer, anslen);
        if r >= 0 { return r; }
        if trailing_dot { return -1; }
    }
    for i in 0..SEARCH_COUNT {
        let dom = SEARCH_DOMAINS[i].as_ptr() as *const c_char;
        let r = res_querydomain(name, dom, class, type_, answer, anslen);
        if r >= 0 { return r; }
        if H_ERRNO!() != HOST_NOT_FOUND && H_ERRNO!() != NO_DATA { return -1; }
    }
    if dots == 0 {
        return res_query(name, class, type_, answer, anslen);
    }
    -1
}

// ============================================================
// Host and address resolution
// ============================================================

#[repr(C)]
pub struct HostEnt {
    pub h_name: *mut c_char,
    pub h_aliases: *mut *mut c_char,
    pub h_addrtype: c_int,
    pub h_length: c_int,
    pub h_addr_list: *mut *mut c_char,
}

#[repr(C)]
pub struct AddrInfo {
    pub ai_flags: c_int,
    pub ai_family: c_int,
    pub ai_socktype: c_int,
    pub ai_protocol: c_int,
    pub ai_addrlen: u32,
    pub ai_addr: *mut c_void,
    pub ai_canonname: *mut c_char,
    pub ai_next: *mut AddrInfo,
}

static mut HOST_ENT: HostEnt = HostEnt {
    h_name: core::ptr::null_mut(),
    h_aliases: core::ptr::null_mut(),
    h_addrtype: 0,
    h_length: 0,
    h_addr_list: core::ptr::null_mut(),
};
static mut HOST_NAME: [c_char; 256] = [0; 256];
static mut HOST_ALIASES: [*mut c_char; 8] = [core::ptr::null_mut(); 8];
static mut HOST_ADDR: [u8; 16] = [0; 16];
static mut HOST_ADDR_PTRS: [*mut c_char; 2] = [core::ptr::null_mut(); 2];

// DNS wire-format offsets used by host_from_answer and friends.
const DNS_HDR_LEN: usize = 12;
const DNS_QDCOUNT_OFF: usize = 4;
const DNS_ANCOUNT_OFF: usize = 6;
const DNS_QUESTION_LEN: usize = 4;
const DNS_RR_TYPE_OFF: usize = 0;
const DNS_RR_CLASS_OFF: usize = 2;
const DNS_RR_RDLEN_OFF: usize = 8;
const DNS_RR_RDATA_OFF: usize = 10;
const DNS_CLASS_IN: u16 = 1;
const DNS_TYPE_A: u16 = 1;
const DNS_TYPE_AAAA: u16 = 28;

unsafe fn host_from_answer(answer: *const u8, len: c_int, af: c_int) -> *mut HostEnt {
    if (len as usize) < DNS_HDR_LEN { return core::ptr::null_mut(); }
    let msg = match InBuf::new(answer, len as usize) {
        Some(msg) => msg,
        None => return core::ptr::null_mut(),
    };
    let qd = match msg.peek_u16_be(DNS_QDCOUNT_OFF) {
        Some(v) => v as usize,
        None => return core::ptr::null_mut(),
    };
    let an = match msg.peek_u16_be(DNS_ANCOUNT_OFF) {
        Some(v) => v as usize,
        None => return core::ptr::null_mut(),
    };
    let mut p = DNS_HDR_LEN;
    for _ in 0..qd {
        p = match dns_skip_name(&msg, p) {
            Some(v) => v,
            None => return core::ptr::null_mut(),
        };
        p = match p.checked_add(DNS_QUESTION_LEN) {
            Some(v) if v <= msg.len() => v,
            _ => return core::ptr::null_mut(),
        };
    }
    let want_type: u16 = if af == AF_INET6 { DNS_TYPE_AAAA } else { DNS_TYPE_A };
    let want_len = if af == AF_INET6 { 16 } else { 4 };
    for _ in 0..an {
        p = match dns_skip_name(&msg, p) {
            Some(v) => v,
            None => break,
        };
        let typ = match msg.peek_u16_be(p + DNS_RR_TYPE_OFF) {
            Some(v) => v,
            None => break,
        };
        let class = match msg.peek_u16_be(p + DNS_RR_CLASS_OFF) {
            Some(v) => v,
            None => break,
        };
        let rdlen = match msg.peek_u16_be(p + DNS_RR_RDLEN_OFF) {
            Some(v) => v as usize,
            None => break,
        };
        let rdata_off = match p.checked_add(DNS_RR_RDATA_OFF) {
            Some(v) => v,
            None => break,
        };
        let rdata = match msg.slice_at(rdata_off, rdlen) {
            Some(v) => v,
            None => break,
        };
        if typ == want_type && class == DNS_CLASS_IN && rdata.len() == want_len {
            core::ptr::copy_nonoverlapping(rdata.as_ptr(), HOST_ADDR.as_mut_ptr(), want_len);
            HOST_ENT.h_addrtype = af;
            HOST_ENT.h_length = want_len as c_int;
            HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
            HOST_ADDR_PTRS[1] = core::ptr::null_mut();
            HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
            HOST_ALIASES[0] = core::ptr::null_mut();
            HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
            HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
            H_ERRNO!() = 0;
            return &raw mut HOST_ENT;
        }
        p = match rdata_off.checked_add(rdlen) {
            Some(v) => v,
            None => break,
        };
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyname(name: *const c_char) -> *mut HostEnt {
    gethostbyname2(name, AF_INET)
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyname2(name: *const c_char, af: c_int) -> *mut HostEnt {
    if name.is_null() { H_ERRNO!() = HOST_NOT_FOUND; return core::ptr::null_mut(); }
    let mut addr = [0u8; 16];
    let addrlen = if af == AF_INET6 { 16 } else { 4 };
    if inet_pton(af, name, addr.as_mut_ptr() as *mut c_void) == 1 {
        core::ptr::copy_nonoverlapping(addr.as_ptr(), HOST_ADDR.as_mut_ptr(), addrlen);
        let n = crate::strlen(name);
        let l = core::cmp::min(n, 255);
        core::ptr::copy_nonoverlapping(name, HOST_NAME.as_mut_ptr(), l);
        HOST_NAME[l] = 0;
        HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
        HOST_ALIASES[0] = core::ptr::null_mut();
        HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
        HOST_ENT.h_addrtype = af;
        HOST_ENT.h_length = addrlen as c_int;
        HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
        HOST_ADDR_PTRS[1] = core::ptr::null_mut();
        HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
        H_ERRNO!() = 0;
        return &raw mut HOST_ENT;
    }
    let mut answer = [0u8; 512];
    let qtype = if af == AF_INET6 { 28 } else { 1 };
    let r = res_query(name, 1, qtype, answer.as_mut_ptr(), answer.len() as c_int);
    if r < 0 {
        if H_ERRNO!() == 0 { H_ERRNO!() = TRY_AGAIN; }
        return core::ptr::null_mut();
    }
    let ent = host_from_answer(answer.as_ptr(), r, af);
    if !ent.is_null() {
        let n = crate::strlen(name);
        let l = core::cmp::min(n, 255);
        core::ptr::copy_nonoverlapping(name, HOST_NAME.as_mut_ptr(), l);
        HOST_NAME[l] = 0;
    } else {
        H_ERRNO!() = HOST_NOT_FOUND;
    }
    ent
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyaddr(addr: *const c_void, len: u32, type_: c_int) -> *mut HostEnt {
    // musl implements gethostbyaddr() on top of getnameinfo(), so /etc/hosts
    // and the DNS reverse path behave identically for both APIs.
    let (addr_len, salen) = match type_ {
        AF_INET => (4usize, 16u32),
        AF_INET6 => (16usize, 28u32),
        _ => {
            H_ERRNO!() = HOST_NOT_FOUND;
            return core::ptr::null_mut();
        }
    };
    if addr.is_null() || len as usize != addr_len {
        H_ERRNO!() = HOST_NOT_FOUND;
        return core::ptr::null_mut();
    }
    let mut sa = [0u8; 28];
    *(sa.as_mut_ptr() as *mut u16) = type_ as u16;
    if type_ == AF_INET {
        core::ptr::copy_nonoverlapping(addr as *const u8, sa.as_mut_ptr().add(4), 4);
    } else {
        core::ptr::copy_nonoverlapping(addr as *const u8, sa.as_mut_ptr().add(8), 16);
    }
    let mut name = [0 as c_char; 256];
    let rc = getnameinfo(
        sa.as_ptr() as *const c_void,
        salen,
        name.as_mut_ptr(),
        name.len() as u32,
        core::ptr::null_mut(),
        0,
        0,
    );
    if rc != 0 {
        H_ERRNO!() = match rc {
            EAI_AGAIN_V => TRY_AGAIN,
            EAI_NONAME_V => HOST_NOT_FOUND,
            _ => NO_RECOVERY,
        };
        return core::ptr::null_mut();
    }
    let l = crate::strlen(name.as_ptr());
    if l >= HOST_NAME.len() {
        H_ERRNO!() = NO_RECOVERY;
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(name.as_ptr(), HOST_NAME.as_mut_ptr(), l + 1);
    core::ptr::copy_nonoverlapping(addr as *const u8, HOST_ADDR.as_mut_ptr(), addr_len);
    HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
    HOST_ALIASES[0] = HOST_NAME.as_mut_ptr();
    HOST_ALIASES[1] = core::ptr::null_mut();
    HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
    HOST_ENT.h_addrtype = type_;
    HOST_ENT.h_length = addr_len as c_int;
    HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
    HOST_ADDR_PTRS[1] = core::ptr::null_mut();
    HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
    H_ERRNO!() = 0;
    &raw mut HOST_ENT
}

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

// ============================================================
// Shadow database
// ============================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Spwd {
    pub sp_namp: *mut c_char,
    pub sp_pwdp: *mut c_char,
    pub sp_lstchg: c_long,
    pub sp_min: c_long,
    pub sp_max: c_long,
    pub sp_warn: c_long,
    pub sp_inact: c_long,
    pub sp_expire: c_long,
    pub sp_flag: c_ulong,
}

static mut SPWD_ENT: Spwd = Spwd {
    sp_namp: core::ptr::null_mut(), sp_pwdp: core::ptr::null_mut(),
    sp_lstchg: 0, sp_min: 0, sp_max: 0, sp_warn: 0, sp_inact: 0,
    sp_expire: 0, sp_flag: 0,
};
static mut SPWD_BUF: *mut c_char = core::ptr::null_mut();
static mut SPWD_BUF_CAP: usize = 0;
static mut SPWD_LINE: *mut c_char = core::ptr::null_mut();
static mut SPWD_LINE_CAP: usize = 0;
static mut SPWD_FILE: *mut FILE = core::ptr::null_mut();

unsafe fn ensure_spwd_buf(need: usize) -> bool {
    if SPWD_BUF_CAP >= need {
        return true;
    }
    let mut cap = if SPWD_BUF_CAP == 0 { 1024 } else { SPWD_BUF_CAP };
    while cap < need {
        cap = match cap.checked_mul(2) {
            Some(v) => v,
            None => return false,
        };
    }
    let grown = crate::realloc(SPWD_BUF as *mut c_void, cap) as *mut c_char;
    if grown.is_null() {
        ERRNO!() = ENOMEM;
        return false;
    }
    SPWD_BUF = grown;
    SPWD_BUF_CAP = cap;
    true
}

/// Read one line from `f` with getline and parse it into the static entry.
/// Lines that do not have all nine shadow fields are skipped.
unsafe fn spwd_next(f: *mut FILE) -> *mut Spwd {
    loop {
        let n = crate::getline(&raw mut SPWD_LINE, &raw mut SPWD_LINE_CAP, f);
        if n < 0 {
            return core::ptr::null_mut();
        }
        let len = n as usize;
        if len > 0 && *SPWD_LINE.add(len - 1) == b'\n' as c_char {
            *SPWD_LINE.add(len - 1) = 0;
        }
        if !ensure_spwd_buf(len + 1) {
            return core::ptr::null_mut();
        }
        if parse_spwd(SPWD_LINE, &raw mut SPWD_ENT, SPWD_BUF, SPWD_BUF_CAP) {
            return &raw mut SPWD_ENT;
        }
    }
}

unsafe fn parse_spwd(line: *const c_char, ent: *mut Spwd, buf: *mut c_char, buflen: usize) -> bool {
    let mut fields: [*mut c_char; 9] = [core::ptr::null_mut(); 9];
    let n = match split_colon_fields(line, buf, buflen, &mut fields) {
        Some(n) => n,
        None => return false,
    };
    if n < 9 { return false; }
    (*ent).sp_namp = fields[0];
    (*ent).sp_pwdp = fields[1];
    (*ent).sp_lstchg = crate::strtol(fields[2], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_min = crate::strtol(fields[3], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_max = crate::strtol(fields[4], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_warn = crate::strtol(fields[5], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_inact = crate::strtol(fields[6], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_expire = crate::strtol(fields[7], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_flag = crate::strtoul(fields[8], core::ptr::null_mut(), 10) as c_ulong;
    true
}

#[no_mangle]
pub unsafe extern "C" fn setspent() {
    endspent();
    SPWD_FILE = crate::fopen(b"/etc/shadow\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub unsafe extern "C" fn endspent() {
    if !SPWD_FILE.is_null() { crate::fclose(SPWD_FILE); SPWD_FILE = core::ptr::null_mut(); }
}

#[no_mangle]
pub unsafe extern "C" fn getspent() -> *mut Spwd {
    if SPWD_FILE.is_null() { setspent(); if SPWD_FILE.is_null() { return core::ptr::null_mut(); } }
    spwd_next(SPWD_FILE)
}

#[no_mangle]
pub unsafe extern "C" fn fgetspent(f: *mut FILE) -> *mut Spwd {
    if f.is_null() { return core::ptr::null_mut(); }
    spwd_next(f)
}

#[no_mangle]
pub unsafe extern "C" fn getspnam(name: *const c_char) -> *mut Spwd {
    if name.is_null() { return core::ptr::null_mut(); }
    setspent();
    loop {
        let sp = getspent();
        if sp.is_null() { break; }
        if crate::strcmp((*sp).sp_namp as *const u8, name as *const u8) == 0 { endspent(); return sp; }
    }
    endspent();
    core::ptr::null_mut()
}

unsafe fn getsp_r_inner(
    name: *const c_char,
    sp: *mut Spwd,
    buf: *mut c_char,
    buflen: usize,
    result: *mut *mut Spwd,
) -> c_int {
    if name.is_null() || sp.is_null() || buf.is_null() || result.is_null() {
        return EINVAL;
    }
    *result = core::ptr::null_mut();
    let f = crate::fopen(b"/etc/shadow\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
    if f.is_null() { return ERRNO!(); }
    let mut line: *mut c_char = core::ptr::null_mut();
    let mut cap: usize = 0;
    let mut err = 0;
    loop {
        let n = crate::getline(&raw mut line, &raw mut cap, f);
        if n < 0 { break; }
        let mut len = n as usize;
        if len > 0 && *line.add(len - 1) == b'\n' as c_char {
            *line.add(len - 1) = 0;
            len -= 1;
        }
        if len + 1 > buflen {
            err = ERANGE;
            break;
        }
        if parse_spwd(line, sp, buf, buflen) {
            if crate::strcmp((*sp).sp_namp as *const u8, name as *const u8) == 0 {
                *result = sp;
                break;
            }
        }
    }
    crate::free(line as *mut c_void);
    crate::fclose(f);
    err
}

#[no_mangle]
pub unsafe extern "C" fn getspnam_r(name: *const c_char, sp: *mut Spwd, buf: *mut c_char, buflen: usize, result: *mut *mut Spwd) -> c_int {
    getsp_r_inner(name, sp, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn putspent(sp: *const Spwd, f: *mut FILE) -> c_int {
    if sp.is_null() || f.is_null() { return -1; }
    crate::fprintf(f, b"%s:%s:%ld:%ld:%ld:%ld:%ld:%ld:%lu\\n\0".as_ptr() as *const c_char,
        (*sp).sp_namp, (*sp).sp_pwdp, (*sp).sp_lstchg, (*sp).sp_min, (*sp).sp_max,
        (*sp).sp_warn, (*sp).sp_inact, (*sp).sp_expire, (*sp).sp_flag)
}

#[no_mangle]
pub unsafe extern "C" fn getgrouplist(user: *const c_char, group: u32, groups: *mut u32, ngroups: *mut i32) -> c_int {
    if user.is_null() || groups.is_null() || ngroups.is_null() { return -1; }
    if *ngroups < 0 { return -1; }
    let max = *ngroups as usize;
    let mut n = 0usize;
    if n < max { *groups.add(n) = group; }
    n += 1;
    setgrent();
    loop {
        let g = getgrent();
        if g.is_null() { break; }
        if (*g).gr_gid == group { continue; }
        let mut found = false;
        let mut i = 0;
        while !(*(*g).gr_mem.add(i)).is_null() {
            if crate::strcmp(*(*g).gr_mem.add(i) as *const u8, user as *const u8) == 0 {
                found = true;
                break;
            }
            i += 1;
        }
        if found {
            if n < max { *groups.add(n) = (*g).gr_gid; }
            n += 1;
        }
    }
    endgrent();
    if n > max { *ngroups = n as i32; return -1; }
    *ngroups = n as i32;
    n as c_int
}
