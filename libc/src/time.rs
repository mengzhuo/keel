// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

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
    // ISO C fixed this format at 26 bytes ("Www Mmm dd hh:mm:ss yyyy\n\0").
    // Build it in a bounded local first: tm fields are caller-controlled and
    // a year outside 1000..9999 makes the canonical form longer.  musl uses
    // snprintf(buf, 26, ...) and deliberately aborts when it does not fit;
    // reproduce that without ever writing past the caller's 26-byte buffer.
    let mut out = [0 as c_char; 64];
    let dst = out.as_mut_ptr();
    let wday = if (*tm).tm_wday >= 0 && (*tm).tm_wday < 7 { (*tm).tm_wday as usize } else { 0 };
    let mon = if (*tm).tm_mon >= 0 && (*tm).tm_mon < 12 { (*tm).tm_mon as usize } else { 0 };
    let dn = DAY_NAMES[wday];
    let mn = MON_NAMES[mon];
    let mut p = 0;
    for k in 0..3 { *dst.add(p) = *dn.as_ptr().add(k) as c_char; p += 1; }
    *dst.add(p) = b' ' as c_char; p += 1;
    for k in 0..3 { *dst.add(p) = *mn.as_ptr().add(k) as c_char; p += 1; }
    // %.3s %.3s%3d: the day is right-aligned in a three-column field (the
    // leading padding belongs to %3d, there is no separate space here).
    let v = (*tm).tm_mday;
    if v < 10 {
        *dst.add(p) = b' ' as c_char; p += 1;
        *dst.add(p) = b' ' as c_char; p += 1;
        *dst.add(p) = (b'0' + v as u8) as c_char; p += 1;
    } else if v < 100 {
        *dst.add(p) = b' ' as c_char; p += 1;
        *dst.add(p) = (b'0' + (v / 10) as u8) as c_char; p += 1;
        *dst.add(p) = (b'0' + (v % 10) as u8) as c_char; p += 1;
    } else {
        *dst.add(p) = (b'0' + (v / 100) as u8) as c_char; p += 1;
        *dst.add(p) = (b'0' + ((v / 10) % 10) as u8) as c_char; p += 1;
        *dst.add(p) = (b'0' + (v % 10) as u8) as c_char; p += 1;
    }
    *dst.add(p) = b' ' as c_char; p += 1;
    let hh = (*tm).tm_hour;
    *dst.add(p) = (b'0' + (hh / 10) as u8) as c_char; p += 1;
    *dst.add(p) = (b'0' + (hh % 10) as u8) as c_char; p += 1;
    *dst.add(p) = b':' as c_char; p += 1;
    let mm = (*tm).tm_min;
    *dst.add(p) = (b'0' + (mm / 10) as u8) as c_char; p += 1;
    *dst.add(p) = (b'0' + (mm % 10) as u8) as c_char; p += 1;
    *dst.add(p) = b':' as c_char; p += 1;
    let ss = (*tm).tm_sec;
    *dst.add(p) = (b'0' + (ss / 10) as u8) as c_char; p += 1;
    *dst.add(p) = (b'0' + (ss % 10) as u8) as c_char; p += 1;
    *dst.add(p) = b' ' as c_char; p += 1;
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
        *dst.add(p) = b'-' as c_char;
        p += 1;
    }
    for k in (0..n).rev() {
        *dst.add(p) = tmp[k] as c_char;
        p += 1;
    }
    *dst.add(p) = b'\n' as c_char; p += 1;
    if p >= 26 {
        crate::abort();
    }
    *dst.add(p) = 0;
    core::ptr::copy_nonoverlapping(dst as *const u8, buf as *mut u8, p + 1);
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
                let mut digits = 0usize;
                while *p >= b'0' && *p <= b'9' && digits < 19 {
                    val = match val.checked_mul(10).and_then(|v| v.checked_add((*p - b'0') as i64)) {
                        Some(v) => v,
                        None => return core::ptr::null_mut(),
                    };
                    p = p.add(1);
                    digits += 1;
                }
                if *p >= b'0' && *p <= b'9' { return core::ptr::null_mut(); }
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

