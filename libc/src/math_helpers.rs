// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Shared bit, classification, fenv and gamma-sign helpers used by the local
// math ports and the libm-backed wrappers.

// IEEE-754 layout constants shared by the math wrappers.
const F64_EXP_MASK: u64 = 0x7ff;
const F32_EXP_MASK: u64 = 0xff;
const F64_EXP_BIAS: i32 = 0x3ff;
const F32_EXP_BIAS: i32 = 0x7f;

#[inline]
const fn asuint(f: f32) -> u32 {
    f.to_bits()
}

#[inline]
const fn asfloat(i: u32) -> f32 {
    f32::from_bits(i)
}

#[inline]
const fn asuint64(f: f64) -> u64 {
    f.to_bits()
}

#[inline]
const fn asdouble(i: u64) -> f64 {
    f64::from_bits(i)
}

#[inline]
unsafe fn extract_words(hi: *mut u32, lo: *mut u32, d: f64) {
    let u = asuint64(d);
    *hi = (u >> 32) as u32;
    *lo = u as u32;
}

#[inline]
fn get_high_word(d: f64) -> u32 {
    (asuint64(d) >> 32) as u32
}

#[inline]
fn get_low_word(d: f64) -> u32 {
    asuint64(d) as u32
}

#[inline]
unsafe fn set_low_word(d: *mut f64, lo: u32) {
    let hi = asuint64(*d) >> 32;
    *d = asdouble((hi << 32) | (lo as u64));
}

#[inline]
unsafe fn get_float_word(f: f32) -> u32 {
    asuint(f)
}

#[inline]
fn eval_as_float(x: f32) -> f32 {
    x
}

#[inline]
fn eval_as_double(x: f64) -> f64 {
    x
}

#[inline]
fn fp_barrierf(x: f32) -> f32 {
    unsafe { core::ptr::read_volatile(&x) }
}

#[inline]
fn fp_barrier(x: f64) -> f64 {
    unsafe { core::ptr::read_volatile(&x) }
}

#[inline]
fn fp_force_evalf(x: f32) {
    unsafe {
        core::ptr::read_volatile(&x);
    }
}

#[inline]
fn fp_force_eval(x: f64) {
    unsafe {
        core::ptr::read_volatile(&x);
    }
}

#[inline]
fn force_eval<T: Copy>(x: T) {
    unsafe {
        core::ptr::read_volatile(&x);
    }
}

#[inline]
fn predict_false(x: bool) -> bool {
    x
}


const SIN_PI_PI: f64 = asdouble(0x400921FB54442D18);

/// sin(pi*x) for x > 2^-100.  If the result is zero its sign is unspecified,
/// matching musl's lgamma/tgamma helper.
#[inline]
fn sin_pi(x: f64) -> f64 {
    // Reduce x modulo 2; a spurious inexact is raised for odd integers.
    let x = 2.0 * (x * 0.5 - floor(x * 0.5));
    let mut n = (x * 4.0) as i32;
    n = (n + 1) / 2;
    let x = x - n as f64 * 0.5;
    let x = x * SIN_PI_PI;
    match n {
        1 => __cos(x, 0.0),
        2 => __sin(-x, 0.0, 0),
        3 => -__cos(x, 0.0),
        _ => __sin(x, 0.0, 0),
    }
}

#[inline]
pub fn __math_invalid(x: f64) -> f64 {
    if x.is_nan() { return x; }
    unsafe { feraiseexcept(FE_INVALID); }
    (x - x) / (x - x)
}

#[inline]
pub fn __math_invalidf(x: f32) -> f32 {
    if x.is_nan() { return x; }
    unsafe { feraiseexcept(FE_INVALID); }
    (x - x) / (x - x)
}

#[inline]
pub fn __math_divzero(sign: u32) -> f64 {
    unsafe { feraiseexcept(FE_DIVBYZERO); }
    let x = if sign != 0 { -1.0f64 } else { 1.0f64 };
    x / 0.0
}

#[inline]
pub fn __math_divzerof(sign: u32) -> f32 {
    unsafe { feraiseexcept(FE_DIVBYZERO); }
    let x = if sign != 0 { -1.0f32 } else { 1.0f32 };
    x / 0.0
}

#[inline]
pub fn __math_xflow(sign: u32, y: f64) -> f64 {
    // musl: (sign ? -y : y) * y  (preserves sign of infinity/zero)
    if sign != 0 {
        -y * y
    } else {
        y * y
    }
}

#[inline]
pub fn __math_xflowf(sign: u32, y: f32) -> f32 {
    if sign != 0 {
        -y * y
    } else {
        y * y
    }
}

#[inline]
pub fn __math_oflow(sign: u32) -> f64 {
    unsafe { feraiseexcept(FE_OVERFLOW | FE_INEXACT); }
    __math_xflow(sign, 1e231f64)
}

#[inline]
pub fn __math_oflowf(sign: u32) -> f32 {
    unsafe { feraiseexcept(FE_OVERFLOW | FE_INEXACT); }
    __math_xflowf(sign, 1.7e38f32)
}

#[inline]
pub fn __math_uflow(sign: u32) -> f64 {
    unsafe { feraiseexcept(FE_UNDERFLOW | FE_INEXACT); }
    __math_xflow(sign, 1e-231f64)
}

#[inline]
pub fn __math_uflowf(sign: u32) -> f32 {
    unsafe { feraiseexcept(FE_UNDERFLOW | FE_INEXACT); }
    __math_xflowf(sign, 1.2e-38f32)
}

// musl exports signgam as an alias of __signgam. rustc's cdylib export list
// localizes linker-level aliases, so keep two real statics and update both
// together wherever the gamma sign is produced.
#[no_mangle]
pub static mut signgam: c_int = 0;

#[no_mangle]
pub static mut __signgam: c_int = 0;

#[inline]
pub(crate) unsafe fn set_signgam(v: c_int) {
    signgam = v;
    __signgam = v;
}

pub const FP_NAN: c_int = 0;
pub const FP_INFINITE: c_int = 1;
pub const FP_ZERO: c_int = 2;
pub const FP_SUBNORMAL: c_int = 3;
pub const FP_NORMAL: c_int = 4;

pub const FP_ILOGB0: c_int = c_int::MIN;
pub const FP_ILOGBNAN: c_int = c_int::MAX;

#[no_mangle]
pub unsafe extern "C" fn __fpclassify(x: f64) -> c_int {
    let u = asuint64(x);
    let e = (u >> 52) & F64_EXP_MASK;
    if e == 0 {
        if (u << 1) != 0 { FP_SUBNORMAL } else { FP_ZERO }
    } else if e == F64_EXP_MASK {
        if (u << 12) != 0 { FP_NAN } else { FP_INFINITE }
    } else {
        FP_NORMAL
    }
}

#[no_mangle]
pub unsafe extern "C" fn __fpclassifyf(x: f32) -> c_int {
    let u = asuint(x);
    let e = (u >> 23) & F32_EXP_MASK as u32;
    if e == 0 {
        if (u << 1) != 0 { FP_SUBNORMAL } else { FP_ZERO }
    } else if e == F32_EXP_MASK as u32 {
        if (u << 9) != 0 { FP_NAN } else { FP_INFINITE }
    } else {
        FP_NORMAL
    }
}

#[no_mangle]
pub extern "C" fn __signbit(x: f64) -> c_int {
    ((asuint64(x) >> 63) & 1) as c_int
}

#[no_mangle]
pub extern "C" fn __signbitf(x: f32) -> c_int {
    ((asuint(x) >> 31) & 1) as c_int
}

#[inline]
pub fn is_nan(x: f64) -> bool {
    let u = asuint64(x);
    ((u >> 52) & F64_EXP_MASK) == F64_EXP_MASK && (u << 12) != 0
}

#[inline]
pub fn is_nanf(x: f32) -> bool {
    let u = asuint(x);
    ((u >> 23) & F32_EXP_MASK as u32) == F32_EXP_MASK as u32 && (u << 9) != 0
}

#[inline]
pub fn is_finite(x: f64) -> bool {
    let u = asuint64(x);
    ((u >> 52) & F64_EXP_MASK) != F64_EXP_MASK
}
