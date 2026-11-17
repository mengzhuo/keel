// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

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
