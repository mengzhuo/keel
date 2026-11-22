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

#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "loongarch64"
))]
#[inline]
fn f128_from_bits(bits: u128) -> f128 {
    unsafe { core::mem::transmute::<u128, f128>(bits) }
}

#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "loongarch64"
))]
#[inline]
fn f128_to_bits(x: f128) -> u128 {
    unsafe { core::mem::transmute::<f128, u128>(x) }
}

#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "loongarch64"
))]
fn f128_is_nan(x: f128) -> bool {
    let bits = f128_to_bits(x);
    ((bits >> 112) & 0x7fff) == 0x7fff && (bits & ((1u128 << 112) - 1)) != 0
}

#[cfg(target_arch = "powerpc64")]
fn f128_is_nan(x: f128) -> bool {
    x.hi.is_nan() || x.lo.is_nan()
}

#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "loongarch64"
))]
fn f128_to_f64(x: f128) -> f64 {
    let bits = f128_to_bits(x);
    let sign = (bits >> 127) as u64;
    let exp = ((bits >> 112) & 0x7fff) as i32;
    let frac = bits & ((1u128 << 112) - 1);
    let signbit = sign << 63;
    if exp == 0x7fff {
        let m = if frac == 0 {
            0x7ff0_0000_0000_0000
        } else {
            0x7ff8_0000_0000_0000
        };
        return f64::from_bits(signbit | m);
    }
    if exp == 0 {
        return f64::from_bits(signbit);
    }
    let e = exp - 16383;
    if e > 1023 {
        return f64::from_bits(signbit | 0x7ff0_0000_0000_0000);
    }
    if e < -1074 {
        return f64::from_bits(signbit);
    }
    let sig = (1u128 << 112) | frac;
    if e >= -1022 {
        let mut m = (sig >> 60) as u64;
        let rem = sig & ((1u128 << 60) - 1);
        let half = 1u128 << 59;
        if rem > half || (rem == half && (m & 1) != 0) {
            m += 1;
        }
        let mut ef = (e + 1023) as u64;
        if m == (1u64 << 53) {
            m >>= 1;
            ef += 1;
        }
        if ef >= 0x7ff {
            return f64::from_bits(signbit | 0x7ff0_0000_0000_0000);
        }
        return f64::from_bits(signbit | (ef << 52) | (m & ((1u64 << 52) - 1)));
    }
    let rshift = (-(e + 962)) as u32;
    let mut m = (sig >> rshift) as u64;
    let rem = sig & ((1u128 << rshift) - 1);
    let half = 1u128 << (rshift - 1);
    if rem > half || (rem == half && (m & 1) != 0) {
        m += 1;
    }
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

#[cfg(any(
    target_arch = "aarch64",
    target_arch = "riscv64",
    target_arch = "loongarch64"
))]
fn f64_to_f128(v: f64) -> f128 {
    let bits = v.to_bits();
    let sign = (bits >> 63) as u128;
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let frac = bits & ((1u64 << 52) - 1);
    if exp == 0x7ff {
        let m = if frac == 0 {
            0
        } else {
            ((frac as u128) << 60) | (1u128 << 111)
        };
        return f128_from_bits((sign << 127) | (0x7fff_u128 << 112) | m);
    }
    if exp == 0 {
        if frac == 0 {
            return f128_from_bits(sign << 127);
        }
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
    () => {
        *errno_ptr()
    };
}

// h_errno shares the TCB with errno: 0x30 is errno, 0x34 is the free slot
// before the vDSO getrandom state at 0x38.  The exported `h_errno` symbol is
// the fallback for static programs and threads without a TCB, matching musl.
macro_rules! H_ERRNO {
    () => {
        *h_errno_ptr()
    };
}

include!("encoding_tables.rs");
mod math {
    #[allow(unused_imports)]
    use super::*;

    include!("math_helpers.rs");
    include!("math_bitmanip.rs");
    include!("math_sqrtfmod.rs");
    include!("math_trig.rs");
    include!("math_exp.rs");
    include!("math_lgamma.rs");
    include!("math_tgamma.rs");
    include!("math_bessel.rs");
    include!("math_log.rs");
    include!("math_pow.rs");
    include!("math_hypot.rs");
    include!("math_hyperbolic.rs");
    include!("math_invtrig.rs");
    include!("math_lrint.rs");
    include!("math_long_double.rs");
    include!("math_complex.rs");
    include!("math_extra.rs");
}
pub use math::*;
include!("alloc.rs");
include!("string_fast.rs");
include!("cstr.rs");
include!("vdso.rs");
include!("iconv_iso8859.rs");
include!("arch.rs");

include!("errno.rs");
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
    <Arch as Syscalls>::syscall4(
        SYS_OPENAT,
        AT_FDCWD as i64,
        path as i64,
        flags as i64,
        mode as i64,
    )
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
    let result = <Arch as Syscalls>::syscall6(
        SYS_MMAP,
        addr as i64,
        length as i64,
        prot as i64,
        flags as i64,
        fd as i64,
        offset,
    );
    if result < 0 && result > -crate::SYSCALL_ERR_MAX {
        ERRNO!() = (-result) as c_int;
        return MMAP_FAILED;
    }
    result as *mut u8
}

unsafe fn sys_munmap(addr: *mut u8, length: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_MUNMAP, addr as i64, length as i64)
}

include!("string_more.rs");
include!("types.rs");
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

include!("stdlib_num.rs");
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
unsafe fn sys_renameat2(
    olddirfd: i32,
    oldpath: *const u8,
    newdirfd: i32,
    newpath: *const u8,
    flags: u32,
) -> i64 {
    <Arch as Syscalls>::syscall5(
        SYS_RENAMEAT2,
        olddirfd as i64,
        oldpath as i64,
        newdirfd as i64,
        newpath as i64,
        flags as i64,
    )
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

include!("qsort.rs");
include!("atexit.rs");
include!("env.rs");
include!("random.rs");
include!("system.rs");
// ============================================================
// errno: strerror / strerror_r / environ alias
// ============================================================

// static error string table indexed by errno
static ERR_STRS: [&[u8]; 134] = [
    b"Success\0",                                           // 0
    b"Operation not permitted\0",                           // 1
    b"No such file or directory\0",                         // 2
    b"No such process\0",                                   // 3
    b"Interrupted system call\0",                           // 4
    b"Input/output error\0",                                // 5
    b"No such device or address\0",                         // 6
    b"Argument list too long\0",                            // 7
    b"Exec format error\0",                                 // 8
    b"Bad file descriptor\0",                               // 9
    b"No child processes\0",                                // 10
    b"Resource temporarily unavailable\0",                  // 11
    b"Cannot allocate memory\0",                            // 12
    b"Permission denied\0",                                 // 13
    b"Bad address\0",                                       // 14
    b"Block device required\0",                             // 15
    b"Device or resource busy\0",                           // 16
    b"File exists\0",                                       // 17
    b"Invalid cross-device link\0",                         // 18
    b"No such device\0",                                    // 19
    b"Not a directory\0",                                   // 20
    b"Is a directory\0",                                    // 21
    b"Invalid argument\0",                                  // 22
    b"Too many open files in system\0",                     // 23
    b"Too many open files\0",                               // 24
    b"Inappropriate ioctl for device\0",                    // 25
    b"Text file busy\0",                                    // 26
    b"File too large\0",                                    // 27
    b"No space left on device\0",                           // 28
    b"Illegal seek\0",                                      // 29
    b"Read-only file system\0",                             // 30
    b"Too many links\0",                                    // 31
    b"Broken pipe\0",                                       // 32
    b"Numerical argument out of domain\0",                  // 33
    b"Numerical result out of range\0",                     // 34
    b"Resource deadlock avoided\0",                         // 35
    b"File name too long\0",                                // 36
    b"No locks available\0",                                // 37
    b"Function not implemented\0",                          // 38
    b"Directory not empty\0",                               // 39
    b"Too many levels of symbolic links\0",                 // 40
    b"Unknown error 41\0",                                  // 41
    b"No message of desired type\0",                        // 42
    b"Identifier removed\0",                                // 43
    b"Channel number out of range\0",                       // 44
    b"Level 2 not synchronized\0",                          // 45
    b"Level 3 halted\0",                                    // 46
    b"Level 3 reset\0",                                     // 47
    b"Link number out of range\0",                          // 48
    b"Protocol driver not attached\0",                      // 49
    b"No CSI structure available\0",                        // 50
    b"Level 2 halted\0",                                    // 51
    b"Invalid exchange\0",                                  // 52
    b"Invalid request descriptor\0",                        // 53
    b"Exchange full\0",                                     // 54
    b"No anode\0",                                          // 55
    b"Invalid request code\0",                              // 56
    b"Invalid slot\0",                                      // 57
    b"Unknown error 58\0",                                  // 58
    b"Bad font file format\0",                              // 59
    b"Device not a stream\0",                               // 60
    b"No data available\0",                                 // 61
    b"Timer expired\0",                                     // 62
    b"Out of streams resources\0",                          // 63
    b"Machine is not on the network\0",                     // 64
    b"Package not installed\0",                             // 65
    b"Object is remote\0",                                  // 66
    b"Link has been severed\0",                             // 67
    b"Advertise error\0",                                   // 68
    b"Srmount error\0",                                     // 69
    b"Communication error on send\0",                       // 70
    b"Protocol error\0",                                    // 71
    b"Multihop attempted\0",                                // 72
    b"RFS specific error\0",                                // 73
    b"Bad message\0",                                       // 74
    b"Value too large for defined data type\0",             // 75
    b"Name not unique on network\0",                        // 76
    b"File descriptor in bad state\0",                      // 77
    b"Remote address changed\0",                            // 78
    b"Can not access a needed shared library\0",            // 79
    b"Accessing a corrupted shared library\0",              // 80
    b".lib section in a.out corrupted\0",                   // 81
    b"Attempting to link in too many shared libraries\0",   // 82
    b"Cannot exec a shared library directly\0",             // 83
    b"Invalid or incomplete multibyte or wide character\0", // 84
    b"Interrupted system call should be restarted\0",       // 85
    b"Streams pipe error\0",                                // 86
    b"Too many users\0",                                    // 87
    b"Socket operation on non-socket\0",                    // 88
    b"Destination address required\0",                      // 89
    b"Message too long\0",                                  // 90
    b"Protocol wrong type for socket\0",                    // 91
    b"Protocol not available\0",                            // 92
    b"Protocol not supported\0",                            // 93
    b"Socket type not supported\0",                         // 94
    b"Operation not supported\0",                           // 95
    b"Protocol family not supported\0",                     // 96
    b"Address family not supported by protocol\0",          // 97
    b"Address already in use\0",                            // 98
    b"Cannot assign requested address\0",                   // 99
    b"Network is down\0",                                   // 100
    b"Network is unreachable\0",                            // 101
    b"Network dropped connection on reset\0",               // 102
    b"Software caused connection abort\0",                  // 103
    b"Connection reset by peer\0",                          // 104
    b"No buffer space available\0",                         // 105
    b"Transport endpoint is already connected\0",           // 106
    b"Transport endpoint is not connected\0",               // 107
    b"Cannot send after transport endpoint shutdown\0",     // 108
    b"Too many references: cannot splice\0",                // 109
    b"Connection timed out\0",                              // 110
    b"Connection refused\0",                                // 111
    b"Host is down\0",                                      // 112
    b"No route to host\0",                                  // 113
    b"Operation already in progress\0",                     // 114
    b"Operation now in progress\0",                         // 115
    b"Stale file handle\0",                                 // 116
    b"Structure needs cleaning\0",                          // 117
    b"Not a XENIX named type file\0",                       // 118
    b"No XENIX semaphores available\0",                     // 119
    b"Is a named type file\0",                              // 120
    b"Remote I/O error\0",                                  // 121
    b"Quota exceeded\0",                                    // 122
    b"No medium found\0",                                   // 123
    b"Wrong medium type\0",                                 // 124
    b"Operation canceled\0",                                // 125
    b"Required key not available\0",                        // 126
    b"Key has expired\0",                                   // 127
    b"Key has been revoked\0",                              // 128
    b"Key was rejected by service\0",                       // 129
    b"Owner died\0",                                        // 130
    b"State not recoverable\0",                             // 131
    b"Operation not possible due to RF-kill\0",             // 132
    b"Memory page has hardware error\0",                    // 133
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
    <Arch as Syscalls>::syscall4(
        SYS_NEWFSTATAT,
        AT_FDCWD as i64,
        path as i64,
        statbuf as i64,
        0,
    )
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

include!("wchar_misc.rs");
include!("sys_misc.rs");
include!("compat_syscalls.rs");
