// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Bit-manipulation and scaling entry points.  Most are delegated to libm;
// round/roundf keep the musl algorithm because libm signals INEXACT for exact
// integer inputs, which breaks the libc-test fenv expectations.

#[inline]
fn p2d(e: i32) -> f64 {
    f64::from_bits(((0x3ff + e) as u64) << 52)
}

#[inline]
fn p2f(e: i32) -> f32 {
    f32::from_bits(((0x7f + e) as u32) << 23)
}

#[no_mangle]
pub extern "C" fn fabs(x: f64) -> f64 {
    libm::fabs(x)
}

#[no_mangle]
pub extern "C" fn fabsf(x: f32) -> f32 {
    libm::fabsf(x)
}

#[no_mangle]
pub extern "C" fn copysign(x: f64, y: f64) -> f64 {
    libm::copysign(x, y)
}

#[no_mangle]
pub extern "C" fn copysignf(x: f32, y: f32) -> f32 {
    libm::copysignf(x, y)
}

#[no_mangle]
pub extern "C" fn trunc(x: f64) -> f64 {
    libm::trunc(x)
}

#[no_mangle]
pub extern "C" fn truncf(x: f32) -> f32 {
    libm::truncf(x)
}

#[no_mangle]
pub extern "C" fn floor(x: f64) -> f64 {
    libm::floor(x)
}

#[no_mangle]
pub extern "C" fn floorf(x: f32) -> f32 {
    libm::floorf(x)
}

#[no_mangle]
pub extern "C" fn ceil(x: f64) -> f64 {
    libm::ceil(x)
}

#[no_mangle]
pub extern "C" fn ceilf(x: f32) -> f32 {
    libm::ceilf(x)
}

#[no_mangle]
pub extern "C" fn round(x: f64) -> f64 {
    let ix = x.to_bits();
    let e = ((ix >> 52) & F64_EXP_MASK) as i32;
    let mut ax = x;

    if e >= F64_EXP_BIAS + 52 {
        return x;
    }
    let neg = ix >> 63 != 0;
    if neg {
        ax = -ax;
    }
    if e < F64_EXP_BIAS - 1 {
        force_eval(ax + p2d(52));
        return 0.0 * x;
    }
    let y = ax + p2d(52) - p2d(52) - ax;
    let y = if y > 0.5 {
        y + ax - 1.0
    } else if y <= -0.5 {
        y + ax + 1.0
    } else {
        y + ax
    };
    if neg { -y } else { y }
}

#[no_mangle]
pub extern "C" fn roundf(x: f32) -> f32 {
    let ix = x.to_bits();
    let e = ((ix >> 23) & F32_EXP_MASK as u32) as i32;
    let mut ax = x;

    if e >= F32_EXP_BIAS + 23 {
        return x;
    }
    let neg = ix >> 31 != 0;
    if neg {
        ax = -ax;
    }
    if e < F32_EXP_BIAS - 1 {
        force_eval(ax + p2f(23));
        return 0.0f32 * x;
    }
    let y = ax + p2f(23) - p2f(23) - ax;
    let y = if y > 0.5f32 {
        y + ax - 1.0f32
    } else if y <= -0.5f32 {
        y + ax + 1.0f32
    } else {
        y + ax
    };
    if neg { -y } else { y }
}

#[no_mangle]
pub extern "C" fn scalbn(x: f64, n: core::ffi::c_int) -> f64 {
    libm::scalbn(x, n)
}

#[no_mangle]
pub extern "C" fn scalbnf(x: f32, n: core::ffi::c_int) -> f32 {
    libm::scalbnf(x, n)
}

#[no_mangle]
pub extern "C" fn scalbln(x: f64, n: core::ffi::c_long) -> f64 {
    let n = if n > core::ffi::c_int::MAX as core::ffi::c_long {
        core::ffi::c_int::MAX
    } else if n < core::ffi::c_int::MIN as core::ffi::c_long {
        core::ffi::c_int::MIN
    } else {
        n as core::ffi::c_int
    };
    libm::scalbn(x, n)
}

#[no_mangle]
pub extern "C" fn scalblnf(x: f32, n: core::ffi::c_long) -> f32 {
    let n = if n > core::ffi::c_int::MAX as core::ffi::c_long {
        core::ffi::c_int::MAX
    } else if n < core::ffi::c_int::MIN as core::ffi::c_long {
        core::ffi::c_int::MIN
    } else {
        n as core::ffi::c_int
    };
    libm::scalbnf(x, n)
}

#[no_mangle]
pub extern "C" fn ldexp(x: f64, n: core::ffi::c_int) -> f64 {
    libm::scalbn(x, n)
}

#[no_mangle]
pub extern "C" fn ldexpf(x: f32, n: core::ffi::c_int) -> f32 {
    libm::scalbnf(x, n)
}

#[no_mangle]
pub unsafe extern "C" fn frexp(x: f64, e: *mut core::ffi::c_int) -> f64 {
    let (r, exp) = libm::frexp(x);
    if !e.is_null() { *e = exp; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn frexpf(x: f32, e: *mut core::ffi::c_int) -> f32 {
    let (r, exp) = libm::frexpf(x);
    if !e.is_null() { *e = exp; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn modf(x: f64, iptr: *mut f64) -> f64 {
    let (frac, int) = libm::modf(x);
    if !iptr.is_null() { *iptr = int; }
    frac
}

#[no_mangle]
pub unsafe extern "C" fn modff(x: f32, iptr: *mut f32) -> f32 {
    let (frac, int) = libm::modff(x);
    if !iptr.is_null() { *iptr = int; }
    frac
}
