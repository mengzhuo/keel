// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

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
        set_signgam(1);
        return x;
    }
    if x == 0.0 {
        set_signgam(if x.is_sign_negative() { -1 } else { 1 });
        feraiseexcept(FE_DIVBYZERO);
        return f32::INFINITY;
    }
    let (r, sign) = libm::lgammaf_r(x);
    set_signgam(sign);
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

