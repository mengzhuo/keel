// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

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

