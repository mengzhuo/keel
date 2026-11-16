// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Inverse trigonometric entry points backed by the libm crate.

#[no_mangle]
pub unsafe extern "C" fn asin(x: f64) -> f64 {
    libm::asin(x)
}

#[no_mangle]
pub unsafe extern "C" fn acos(x: f64) -> f64 {
    libm::acos(x)
}

#[no_mangle]
pub unsafe extern "C" fn atan(x: f64) -> f64 {
    libm::atan(x)
}

#[no_mangle]
pub unsafe extern "C" fn atan2(y: f64, x: f64) -> f64 {
    libm::atan2(y, x)
}

#[no_mangle]
pub unsafe extern "C" fn asinf(x: f32) -> f32 {
    libm::asinf(x)
}

#[no_mangle]
pub unsafe extern "C" fn acosf(x: f32) -> f32 {
    libm::acosf(x)
}

#[no_mangle]
pub unsafe extern "C" fn atanf(x: f32) -> f32 {
    libm::atanf(x)
}

#[no_mangle]
pub unsafe extern "C" fn atan2f(y: f32, x: f32) -> f32 {
    libm::atan2f(y, x)
}
