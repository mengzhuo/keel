// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// sinh/sinhf and the expm1 helpers are ported from musl (sinh.c, sinhf.c,
// expm1.c, expm1f.c, __expo2.c, __expo2f.c).  cosh/coshf/tanh/tanhf delegate
// to libm.
//
// All constants/tables are prefixed with HYP_ to avoid collisions with
// math_exp.rs; private helpers use the `hyper_` prefix.

// ============================================================
// Constants for hyper_expm1 (double precision)
// ============================================================

const HYP_EPM1_O_THRESHOLD: f64 = asdouble(0x40862E42FEFA39EF);
const HYP_EPM1_LN2_HI: f64 = asdouble(0x3FE62E42FEE00000);
const HYP_EPM1_LN2_LO: f64 = asdouble(0x3DEA39EF35793C76);
const HYP_EPM1_INVLN2: f64 = asdouble(0x3FF71547652B82FE);
const HYP_EPM1_Q1: f64 = asdouble(0xBFA11111111110F4);
const HYP_EPM1_Q2: f64 = asdouble(0x3F5A01A019FE5585);
const HYP_EPM1_Q3: f64 = asdouble(0xBF14CE199EAADBB7);
const HYP_EPM1_Q4: f64 = asdouble(0x3ED0CFCA86E65239);
const HYP_EPM1_Q5: f64 = asdouble(0xBE8AFDB76E09C32D);

// ============================================================
// Constants for hyper_expm1f (float precision)
// ============================================================

const HYP_EPM1F_LN2_HI: f32 = asfloat(0x3F317180);
const HYP_EPM1F_LN2_LO: f32 = asfloat(0x3717F7D1);
const HYP_EPM1F_INVLN2: f32 = asfloat(0x3FB8AA3B);
const HYP_EPM1F_Q1: f32 = asfloat(0xBD088868);
const HYP_EPM1F_Q2: f32 = asfloat(0x3ACF3010);

// ============================================================
// Constants for hyper_expo2 / hyper_expo2f
// ============================================================

const HYP_EXPO2_KLN2: f64 = asdouble(0x40962066151ADD8B);
const HYP_EXPO2_SCALE: f64 = asdouble(0x7FC0000000000000);

const HYP_EXPO2F_KLN2: f32 = asfloat(0x4322E3BC);
const HYP_EXPO2F_SCALE: f32 = asfloat(0x7A000000);

// 2^120 as double (for raising inexact)
const HYP_TWO120: f64 = asdouble(0x4770000000000000);
// 2^1023 as double (for overflow scaling in expm1)
const HYP_TWO1023: f64 = asdouble(0x7FE0000000000000);
// 2^127 as float (for overflow scaling in expm1f)
const HYP_TWO127F: f32 = asfloat(0x7F000000);

// ============================================================
// hyper_expo2: exp(x)/2 for x >= log(DBL_MAX)
// ============================================================

#[inline]
fn hyper_expo2(x: f64, sign: f64) -> f64 {
    let scale = HYP_EXPO2_SCALE;
    exp(x - HYP_EXPO2_KLN2) * (sign * scale) * scale
}

// ============================================================
// hyper_expo2f: expf(x)/2 for x >= log(FLT_MAX)
// ============================================================

#[inline]
fn hyper_expo2f(x: f32, sign: f32) -> f32 {
    let scale = HYP_EXPO2F_SCALE;
    expf(x - HYP_EXPO2F_KLN2) * (sign * scale) * scale
}

// ============================================================
// hyper_expm1: exp(x) - 1 (double precision)
// ============================================================

#[inline]
fn hyper_expm1(x: f64) -> f64 {
    let u = asuint64(x);
    let hx = ((u >> 32) & 0x7fffffff) as u32;
    let sign = (u >> 63) as i32;

    // filter out huge and non-finite argument
    let mut x = x;
    if hx >= 0x4043687A {
        if x.is_nan() {
            return x;
        }
        if sign != 0 {
            return -1.0;
        }
        if x > HYP_EPM1_O_THRESHOLD {
            x *= HYP_TWO1023;
            return x;
        }
    }

    let k: i32;
    let hi: f64;
    let lo: f64;
    let c: f64;

    // argument reduction
    if hx > 0x3fd62e42 {
        if hx < 0x3FF0A2B2 {
            if sign == 0 {
                hi = x - HYP_EPM1_LN2_HI;
                lo = HYP_EPM1_LN2_LO;
                k = 1;
            } else {
                hi = x + HYP_EPM1_LN2_HI;
                lo = -HYP_EPM1_LN2_LO;
                k = -1;
            }
        } else {
            let kt = if sign == 0 {
                HYP_EPM1_INVLN2 * x + 0.5
            } else {
                HYP_EPM1_INVLN2 * x - 0.5
            };
            k = kt as i32;
            let t = k as f64;
            hi = x - t * HYP_EPM1_LN2_HI;
            lo = t * HYP_EPM1_LN2_LO;
        }
        x = hi - lo;
        c = (hi - x) - lo;
    } else if hx < 0x3c900000 {
        // |x| < 2^-54, return x
        if hx < 0x00100000 {
            force_eval(x as f32);
        }
        return x;
    } else {
        k = 0;
        c = 0.0; // unused when k == 0
    }

    // x is now in primary range
    let hfx = 0.5 * x;
    let hxs = x * hfx;
    let r1 = 1.0 + hxs * (HYP_EPM1_Q1 + hxs * (HYP_EPM1_Q2 + hxs * (HYP_EPM1_Q3 + hxs * (HYP_EPM1_Q4 + hxs * HYP_EPM1_Q5))));
    let t = 3.0 - r1 * hfx;
    let e = hxs * ((r1 - t) / (6.0 - x * t));
    if k == 0 {
        // c is 0
        return x - (x * e - hxs);
    }
    let e = x * (e - c) - c;
    let e = e - hxs;
    // exp(x) ~ 2^k (x_reduced - e + 1)
    if k == -1 {
        return 0.5 * (x - e) - 0.5;
    }
    if k == 1 {
        if x < -0.25 {
            return -2.0 * (e - (x + 0.5));
        }
        return 1.0 + 2.0 * (x - e);
    }
    let twopk = asdouble(((0x3ff_i64 + k as i64) as u64) << 52);
    if k < 0 || k > 56 {
        // suffice to return exp(x)-1
        let mut y = x - e + 1.0;
        if k == 1024 {
            y = y * 2.0 * HYP_TWO1023;
        } else {
            y = y * twopk;
        }
        return y - 1.0;
    }
    let neg_twopk = asdouble(((0x3ff_i64 - k as i64) as u64) << 52);
    if k < 20 {
        (x - e + (1.0 - neg_twopk)) * twopk
    } else {
        (x - (e + neg_twopk) + 1.0) * twopk
    }
}

// ============================================================
// hyper_expm1f: expf(x) - 1 (float precision)
// ============================================================

#[inline]
fn hyper_expm1f(x: f32) -> f32 {
    let u = asuint(x);
    let hx = u & 0x7fffffff;
    let sign = (u >> 31) as i32;

    // filter out huge and non-finite argument
    let mut x = x;
    if hx >= 0x4195b844 {
        if hx > 0x7f800000 {
            // NaN
            return x;
        }
        if sign != 0 {
            return -1.0;
        }
        if hx > 0x42b17217 {
            // x > log(FLT_MAX)
            x *= HYP_TWO127F;
            return x;
        }
    }

    let k: i32;
    let hi: f32;
    let lo: f32;
    let c: f32;

    // argument reduction
    if hx > 0x3eb17218 {
        if hx < 0x3F851592 {
            if sign == 0 {
                hi = x - HYP_EPM1F_LN2_HI;
                lo = HYP_EPM1F_LN2_LO;
                k = 1;
            } else {
                hi = x + HYP_EPM1F_LN2_HI;
                lo = -HYP_EPM1F_LN2_LO;
                k = -1;
            }
        } else {
            let kt = if sign == 0 {
                HYP_EPM1F_INVLN2 * x + 0.5
            } else {
                HYP_EPM1F_INVLN2 * x - 0.5
            };
            k = kt as i32;
            let t = k as f32;
            hi = x - t * HYP_EPM1F_LN2_HI;
            lo = t * HYP_EPM1F_LN2_LO;
        }
        x = hi - lo;
        c = (hi - x) - lo;
    } else if hx < 0x33000000 {
        // |x| < 2^-25, return x
        if hx < 0x00800000 {
            force_eval(x * x);
        }
        return x;
    } else {
        k = 0;
        c = 0.0;
    }

    // x is now in primary range
    let hfx = 0.5 * x;
    let hxs = x * hfx;
    let r1 = 1.0 + hxs * (HYP_EPM1F_Q1 + hxs * HYP_EPM1F_Q2);
    let t = 3.0 - r1 * hfx;
    let e = hxs * ((r1 - t) / (6.0 - x * t));
    if k == 0 {
        // c is 0
        return x - (x * e - hxs);
    }
    let e = x * (e - c) - c;
    let e = e - hxs;
    // exp(x) ~ 2^k (x_reduced - e + 1)
    if k == -1 {
        return 0.5 * (x - e) - 0.5;
    }
    if k == 1 {
        if x < -0.25 {
            return -2.0 * (e - (x + 0.5));
        }
        return 1.0 + 2.0 * (x - e);
    }
    let twopk = asfloat(((0x7f + k) as u32) << 23);
    if k < 0 || k > 56 {
        // suffice to return exp(x)-1
        let mut y = x - e + 1.0;
        if k == 128 {
            y = y * 2.0 * HYP_TWO127F;
        } else {
            y = y * twopk;
        }
        return y - 1.0;
    }
    let neg_twopk = asfloat(((0x7f - k) as u32) << 23);
    if k < 23 {
        (x - e + (1.0 - neg_twopk)) * twopk
    } else {
        (x - (e + neg_twopk) + 1.0) * twopk
    }
}

// ============================================================
// Public API: sinh, cosh, tanh (double precision)
// ============================================================

#[no_mangle]
pub extern "C" fn sinh(x: f64) -> f64 {
    let u = asuint64(x);
    let mut h = 0.5f64;
    if u >> 63 != 0 {
        h = -h;
    }
    // |x|
    let u = u & 0x7FFFFFFFFFFFFFFF;
    let absx = asdouble(u);
    let w = (u >> 32) as u32;

    // |x| < log(DBL_MAX)
    if w < 0x40862e42 {
        let t = hyper_expm1(absx);
        if w < 0x3ff00000 {
            if w < 0x3ff00000 - (26 << 20) {
                // note: inexact and underflow are raised by expm1
                // note: this branch avoids spurious underflow
                return x;
            }
            return h * (2.0 * t - t * t / (t + 1.0));
        }
        // note: |x|>log(0x1p26)+eps could be just h*exp(x)
        return h * (t + t / (t + 1.0));
    }

    // |x| > log(DBL_MAX) or nan
    // note: the result is stored to handle overflow
    hyper_expo2(absx, 2.0 * h)
}

#[no_mangle]
pub extern "C" fn cosh(x: f64) -> f64 {
    libm::cosh(x)
}

#[no_mangle]
pub extern "C" fn tanh(x: f64) -> f64 {
    libm::tanh(x)
}

// ============================================================
// Public API: sinhf, coshf, tanhf (float precision)
// ============================================================

#[no_mangle]
pub extern "C" fn sinhf(x: f32) -> f32 {
    let u = asuint(x);
    let mut h = 0.5f32;
    if u >> 31 != 0 {
        h = -h;
    }
    // |x|
    let u = u & 0x7fffffff;
    let absx = asfloat(u);
    let w = u;

    // |x| < log(FLT_MAX)
    if w < 0x42b17217 {
        let t = hyper_expm1f(absx);
        if w < 0x3f800000 {
            if w < 0x3f800000 - (12 << 23) {
                return x;
            }
            return h * (2.0 * t - t * t / (t + 1.0));
        }
        return h * (t + t / (t + 1.0));
    }

    // |x| > logf(FLT_MAX) or nan
    hyper_expo2f(absx, 2.0 * h)
}

#[no_mangle]
pub extern "C" fn coshf(x: f32) -> f32 {
    libm::coshf(x)
}

#[no_mangle]
pub extern "C" fn tanhf(x: f32) -> f32 {
    libm::tanhf(x)
}
