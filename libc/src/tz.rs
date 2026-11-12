// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Time zone support, ported from musl's src/time/__tz.c.  The parser handles
// TZif v1/v2/v3 files and POSIX TZ strings (including the M/J/n DST rules).

use core::sync::atomic::AtomicBool;

const TZNAME_LEN: usize = 255;
const PATH_BUF: usize = 512;

static TZ_LOCK: AtomicBool = AtomicBool::new(false);

static mut TZ_MAP: *const u8 = core::ptr::null();
static mut TZ_MAP_SIZE: usize = 0;
static mut TZ_TRANS: *const u8 = core::ptr::null();
static mut TZ_INDEX: *const u8 = core::ptr::null();
static mut TZ_TYPES: *const u8 = core::ptr::null();
static mut TZ_ABBREVS: *const u8 = core::ptr::null();
static mut TZ_ABBREVS_END: *const u8 = core::ptr::null();

static mut TZ_STD_NAME: [c_char; TZNAME_LEN + 1] = [0; TZNAME_LEN + 1];
static mut TZ_DST_NAME: [c_char; TZNAME_LEN + 1] = [0; TZNAME_LEN + 1];
static mut TZ_DST_OFF: i32 = 0;
static mut TZ_R0: [i32; 5] = [0; 5];
static mut TZ_R1: [i32; 5] = [0; 5];

static mut TZ_OLD: [c_char; TZNAME_LEN + 1] = [0; TZNAME_LEN + 1];
static mut TZ_OLD_LEN: usize = usize::MAX;

static TZ_UTC: [c_char; 4] = [b'U' as c_char, b'T' as c_char, b'C' as c_char, 0];

unsafe fn tz_lock() {
    while TZ_LOCK.swap(true, Ordering::Acquire) {}
}

unsafe fn tz_unlock() {
    TZ_LOCK.store(false, Ordering::Release);
}

unsafe fn utc_name() -> *const c_char {
    TZ_UTC.as_ptr()
}

unsafe fn sync_public_tz() {
    crate::timezone = crate::__timezone;
    crate::daylight = crate::__daylight;
    crate::tzname[0] = crate::__tzname[0];
    crate::tzname[1] = crate::__tzname[1];
}

unsafe fn zi_read32(z: *const u8) -> u32 {
    ((*z as u32) << 24)
        | ((*z.add(1) as u32) << 16)
        | ((*z.add(2) as u32) << 8)
        | *z.add(3) as u32
}

unsafe fn zi_dotprod(z: *const u8, v: &[u8; 6]) -> usize {
    let mut y = 0usize;
    for i in 0..6 {
        y = y.wrapping_add(zi_read32(z.add(i * 4)) as usize * v[i] as usize);
    }
    y
}

unsafe fn map_tzfile(path: *const c_char) -> (*const u8, usize) {
    let fd = crate::open(path, 0, 0);
    if fd < 0 {
        return (core::ptr::null(), 0);
    }
    let mut st: crate::Stat = core::mem::zeroed();
    if crate::fstat(fd, &mut st) < 0 || st.st_size <= 0 {
        crate::close(fd);
        return (core::ptr::null(), 0);
    }
    let size = st.st_size as usize;
    let p = crate::sys_mmap(
        core::ptr::null_mut(),
        size,
        crate::PROT_READ,
        crate::MAP_PRIVATE,
        fd,
        0,
    );
    crate::close(fd);
    if p == crate::MMAP_FAILED {
        return (core::ptr::null(), 0);
    }
    (p as *const u8, size)
}

unsafe fn is_digit(c: c_char) -> bool {
    c >= b'0' as c_char && c <= b'9' as c_char
}

unsafe fn is_alpha(c: c_char) -> bool {
    (c | 32) >= b'a' as c_char && (c | 32) <= b'z' as c_char
}

unsafe fn getint(p: &mut *const c_char) -> i32 {
    let mut x = 0i32;
    while is_digit(**p) {
        x = x * 10 + (**p - b'0' as c_char) as i32;
        *p = (*p).add(1);
    }
    x
}

unsafe fn getoff(p: &mut *const c_char) -> i32 {
    let mut neg = false;
    if **p == b'-' as c_char {
        neg = true;
        *p = (*p).add(1);
    } else if **p == b'+' as c_char {
        *p = (*p).add(1);
    }
    let mut off = 3600 * getint(p);
    if **p == b':' as c_char {
        *p = (*p).add(1);
        off += 60 * getint(p);
        if **p == b':' as c_char {
            *p = (*p).add(1);
            off += getint(p);
        }
    }
    if neg { -off } else { off }
}

unsafe fn getname(d: *mut c_char, p: &mut *const c_char) {
    let mut i = 0usize;
    if **p == b'<' as c_char {
        *p = (*p).add(1);
        while **p != 0 && **p != b'>' as c_char {
            if i < TZNAME_LEN {
                *d.add(i) = **p;
                i += 1;
            }
            *p = (*p).add(1);
        }
        if **p == b'>' as c_char {
            *p = (*p).add(1);
        }
    } else {
        while is_alpha(**p) {
            if i < TZNAME_LEN {
                *d.add(i) = **p;
                i += 1;
            }
            *p = (*p).add(1);
        }
    }
    *d.add(core::cmp::min(i, TZNAME_LEN)) = 0;
}

unsafe fn getrule(p: &mut *const c_char, rule: &mut [i32; 5]) {
    let r = **p as i32;
    rule[0] = r;
    if r != b'M' as i32 {
        if r == b'J' as i32 {
            *p = (*p).add(1);
        } else {
            rule[0] = 0;
        }
        rule[1] = getint(p);
    } else {
        *p = (*p).add(1);
        rule[1] = getint(p);
        *p = (*p).add(1);
        rule[2] = getint(p);
        *p = (*p).add(1);
        rule[3] = getint(p);
    }
    if **p == b'/' as c_char {
        *p = (*p).add(1);
        rule[4] = getoff(p);
    } else {
        rule[4] = 7200;
    }
}

unsafe fn days_in_month(m: i32, is_leap: bool) -> i32 {
    if m == 2 {
        28 + is_leap as i32
    } else {
        30 + ((0xad5 >> (m - 1)) & 1)
    }
}

unsafe fn rule_to_secs(rule: &[i32; 5], year: i64) -> i64 {
    let mut is_leap = false;
    let mut t = year_to_secs(year, &mut is_leap);
    if rule[0] != b'M' as i32 {
        let mut x = rule[1];
        if rule[0] == b'J' as i32 && (x < 60 || !is_leap) {
            x -= 1;
        }
        t += 86400 * x as i64;
    } else {
        let m = rule[1];
        let mut n = rule[2];
        let d = rule[3];
        t += month_to_secs(m - 1, is_leap) as i64;
        let wday = ((t + 4 * 86400) % (7 * 86400)) / 86400;
        let mut days = d - wday as i32;
        if days < 0 {
            days += 7;
        }
        if n == 5 && days + 28 >= days_in_month(m, is_leap) {
            n = 4;
        }
        t += 86400 * (days + 7 * (n - 1)) as i64;
    }
    t + rule[4] as i64
}

unsafe fn scan_trans(t: i64, local: c_int, alt: *mut usize) -> usize {
    let scale = if TZ_TRANS == TZ_MAP.add(44) { 2 } else { 3 };
    let mut a = 0usize;
    let mut n = (TZ_INDEX as usize - TZ_TRANS as usize) >> scale;

    if n == 0 {
        if !alt.is_null() {
            *alt = 0;
        }
        return 0;
    }

    let mut off = 0i32;
    while n > 1 {
        let m = a + n / 2;
        let mut x = zi_read32(TZ_TRANS.add(m << scale)) as u64;
        if scale == 3 {
            x = (x << 32) | zi_read32(TZ_TRANS.add((m << scale) + 4)) as u64;
        } else {
            x = (x as u32 as i32 as i64) as u64;
        }
        if local != 0 {
            let idx = *TZ_INDEX.add(m - 1) as usize;
            off = zi_read32(TZ_TYPES.add(6 * idx)) as i32;
        }
        if t - (off as i64) < (x as i64) {
            n /= 2;
        } else {
            a = m;
            n -= n / 2;
        }
    }

    n = (TZ_INDEX as usize - TZ_TRANS as usize) >> scale;
    if a == n - 1 {
        return usize::MAX;
    }
    if a == 0 {
        let mut x = zi_read32(TZ_TRANS) as u64;
        if scale == 3 {
            x = (x << 32) | zi_read32(TZ_TRANS.add(4)) as u64;
        } else {
            x = (x as u32 as i32 as i64) as u64;
        }
        let mut j = 0usize;
        let mut i = TZ_ABBREVS as usize - TZ_TYPES as usize;
        while i > 0 {
            if *TZ_TYPES.add(i - 6 + 4) == 0 {
                j = i - 6;
            }
            i -= 6;
        }
        if local != 0 {
            off = zi_read32(TZ_TYPES.add(j)) as i32;
        }
        if t - (off as i64) < (x as i64) {
            if !alt.is_null() {
                *alt = *TZ_INDEX as usize;
            }
            return j / 6;
        }
    }

    if !alt.is_null() {
        let cur = *TZ_INDEX.add(a) as usize;
        if a > 0
            && *TZ_TYPES.add(6 * (*TZ_INDEX.add(a - 1) as usize) + 4)
                != *TZ_TYPES.add(6 * cur + 4)
        {
            *alt = *TZ_INDEX.add(a - 1) as usize;
        } else if a + 1 < n
            && *TZ_TYPES.add(6 * (*TZ_INDEX.add(a + 1) as usize) + 4)
                != *TZ_TYPES.add(6 * cur + 4)
        {
            *alt = *TZ_INDEX.add(a + 1) as usize;
        } else {
            *alt = cur;
        }
    }
    *TZ_INDEX.add(a) as usize
}

unsafe fn do_tzset() {
    let mut s = crate::getenv(b"TZ\0".as_ptr() as *const c_char) as *const c_char;
    if s.is_null() {
        s = b"/etc/localtime\0".as_ptr() as *const c_char;
    }
    if *s == 0 {
        s = utc_name();
    }

    let mut len = crate::strlen(s);
    if len >= TZ_OLD.len() {
        s = utc_name();
        len = 3;
    }
    if TZ_OLD_LEN == len && crate::memcmp(TZ_OLD.as_ptr() as *const c_void, s as *const c_void, len) == 0 {
        return;
    }
    core::ptr::copy_nonoverlapping(s, TZ_OLD.as_mut_ptr(), len);
    TZ_OLD[len] = 0;
    TZ_OLD_LEN = len;

    for i in 0..5 {
        TZ_R0[i] = 0;
        TZ_R1[i] = 0;
    }

    let mut posix_form = false;
    if *s != b':' as c_char {
        let mut p = s;
        let mut dummy = [0 as c_char; TZNAME_LEN + 1];
        getname(dummy.as_mut_ptr(), &mut p);
        if p != s
            && (*p == b'+' as c_char
                || *p == b'-' as c_char
                || is_digit(*p)
                || crate::strcmp(dummy.as_ptr() as *const u8, b"UTC\0".as_ptr()) == 0
                || crate::strcmp(dummy.as_ptr() as *const u8, b"GMT\0".as_ptr()) == 0)
        {
            posix_form = true;
        }
    }

    let mut map: *const u8 = core::ptr::null();
    let mut map_size = 0usize;
    if !posix_form {
        if *s == b':' as c_char {
            s = s.add(1);
        }
        if *s == b'/' as c_char || *s == b'.' as c_char {
            let (m, n) = map_tzfile(s);
            map = m;
            map_size = n;
        } else {
            let mut l = 0usize;
            while *s.add(l) != 0 {
                l += 1;
            }
            let mut has_dot = false;
            let mut i = 0usize;
            while i < l {
                if *s.add(i) == b'.' as c_char {
                    has_dot = true;
                    break;
                }
                i += 1;
            }
            if l <= TZNAME_LEN && !has_dot {
                let prefixes: [&[u8]; 3] = [
                    b"/usr/share/zoneinfo/",
                    b"/share/zoneinfo/",
                    b"/etc/zoneinfo/",
                ];
                for prefix in prefixes {
                    if prefix.len() + l + 1 >= PATH_BUF {
                        continue;
                    }
                    let mut path = [0 as c_char; PATH_BUF];
                    core::ptr::copy_nonoverlapping(
                        prefix.as_ptr(),
                        path.as_mut_ptr() as *mut u8,
                        prefix.len(),
                    );
                    core::ptr::copy_nonoverlapping(
                        s as *const u8,
                        path.as_mut_ptr().add(prefix.len()) as *mut u8,
                        l,
                    );
                    path[prefix.len() + l] = 0;
                    let (m, n) = map_tzfile(path.as_ptr());
                    if !m.is_null() {
                        map = m;
                        map_size = n;
                        break;
                    }
                }
            }
        }
        if map.is_null()
            || map_size < 44
            || crate::memcmp(map as *const c_void, b"TZif\0".as_ptr() as *const c_void, 4) != 0
        {
            map = core::ptr::null();
            s = utc_name();
        }
    }

    TZ_MAP = map;
    TZ_MAP_SIZE = map_size;
    if !map.is_null() {
        let mut scale = 2usize;
        if *map.add(4) != b'1' {
            let skip = zi_dotprod(map.add(20), &[1, 1, 8, 5, 6, 1]);
            TZ_TRANS = map.add(skip + 44 + 44);
            scale = 3;
        } else {
            TZ_TRANS = map.add(44);
        }
        TZ_INDEX = TZ_TRANS.add((zi_read32(TZ_TRANS.sub(12)) as usize) << scale);
        TZ_TYPES = TZ_INDEX.add(zi_read32(TZ_TRANS.sub(12)) as usize);
        TZ_ABBREVS = TZ_TYPES.add(6 * zi_read32(TZ_TRANS.sub(8)) as usize);
        TZ_ABBREVS_END = TZ_ABBREVS.add(zi_read32(TZ_TRANS.sub(4)) as usize);

        if *map.add(map_size - 1) == b'\n' as u8 {
            let mut p = map.add(map_size - 2);
            while *p != b'\n' as u8 {
                p = p.sub(1);
            }
            s = p.add(1) as *const c_char;
        } else {
            crate::__tzname[0] = core::ptr::null_mut();
            crate::__tzname[1] = core::ptr::null_mut();
            crate::__daylight = 0;
            crate::__timezone = 0;
            TZ_DST_OFF = 0;
            let mut p = TZ_TYPES;
            while p < TZ_ABBREVS {
                if *p.add(4) == 0 && crate::__tzname[0].is_null() {
                    crate::__tzname[0] = TZ_ABBREVS.add(*p.add(5) as usize) as *mut c_char;
                    crate::__timezone = -(zi_read32(p) as i32) as c_long;
                }
                if *p.add(4) != 0 && crate::__tzname[1].is_null() {
                    crate::__tzname[1] = TZ_ABBREVS.add(*p.add(5) as usize) as *mut c_char;
                    TZ_DST_OFF = -(zi_read32(p) as i32);
                    crate::__daylight = 1;
                }
                p = p.add(6);
            }
            if crate::__tzname[0].is_null() {
                crate::__tzname[0] = crate::__tzname[1];
            }
            if crate::__tzname[0].is_null() {
                crate::__tzname[0] = utc_name() as *mut c_char;
            }
            if crate::__daylight == 0 {
                crate::__tzname[1] = crate::__tzname[0];
                TZ_DST_OFF = crate::__timezone as i32;
            }
            sync_public_tz();
            return;
        }
    }

    if map.is_null() && !posix_form {
        s = utc_name();
    }
    getname(TZ_STD_NAME.as_mut_ptr(), &mut s);
    crate::__tzname[0] = TZ_STD_NAME.as_mut_ptr();
    crate::__timezone = getoff(&mut s) as c_long;
    getname(TZ_DST_NAME.as_mut_ptr(), &mut s);
    crate::__tzname[1] = TZ_DST_NAME.as_mut_ptr();
    if TZ_DST_NAME[0] != 0 {
        crate::__daylight = 1;
        if *s == b'+' as c_char || *s == b'-' as c_char || is_digit(*s) {
            TZ_DST_OFF = getoff(&mut s);
        } else {
            TZ_DST_OFF = crate::__timezone as i32 - 3600;
        }
    } else {
        crate::__daylight = 0;
        TZ_DST_OFF = crate::__timezone as i32;
    }

    if *s == b',' as c_char {
        s = s.add(1);
        getrule(&mut s, &mut TZ_R0);
    }
    if *s == b',' as c_char {
        s = s.add(1);
        getrule(&mut s, &mut TZ_R1);
    }
    sync_public_tz();
}

pub unsafe fn tz_set() {
    tz_lock();
    do_tzset();
    sync_public_tz();
    tz_unlock();
}

unsafe fn secs_to_zone(
    t: i64,
    local: c_int,
    isdst: *mut c_int,
    offset: *mut c_long,
    oppoff: *mut c_long,
    zonename: *mut *const c_char,
) {
    tz_lock();
    do_tzset();

    if !TZ_MAP.is_null() {
        let mut alt = 0usize;
        let i = scan_trans(t, local, if oppoff.is_null() { core::ptr::null_mut() } else { &mut alt });
        if i != usize::MAX {
            let typ = TZ_TYPES.add(6 * i);
            *isdst = *typ.add(4) as c_int;
            *offset = zi_read32(typ) as i32 as c_long;
            *zonename = TZ_ABBREVS.add(*typ.add(5) as usize) as *const c_char;
            if !oppoff.is_null() {
                *oppoff = zi_read32(TZ_TYPES.add(6 * alt)) as i32 as c_long;
            }
            tz_unlock();
            return;
        }
    }

    if crate::__daylight == 0 {
        *isdst = 0;
        *offset = -crate::__timezone;
        if !oppoff.is_null() {
            *oppoff = -TZ_DST_OFF as c_long;
        }
        *zonename = crate::__tzname[0];
        tz_unlock();
        return;
    }

    let mut y = t / 31556952 + 70;
    let mut is_leap = false;
    while year_to_secs(y, &mut is_leap) > t {
        y -= 1;
    }
    while year_to_secs(y + 1, &mut is_leap) < t {
        y += 1;
    }

    let mut t0 = rule_to_secs(&TZ_R0, y);
    let mut t1 = rule_to_secs(&TZ_R1, y);
    if local == 0 {
        t0 += crate::__timezone;
        t1 += TZ_DST_OFF as i64;
    }

    let dst = if t0 < t1 {
        t >= t0 && t < t1
    } else {
        !(t >= t1 && t < t0)
    };

    if dst {
        *isdst = 1;
        *offset = -TZ_DST_OFF as c_long;
        if !oppoff.is_null() {
            *oppoff = -crate::__timezone;
        }
        *zonename = crate::__tzname[1];
    } else {
        *isdst = 0;
        *offset = -crate::__timezone;
        if !oppoff.is_null() {
            *oppoff = -TZ_DST_OFF as c_long;
        }
        *zonename = crate::__tzname[0];
    }
    tz_unlock();
}

pub unsafe fn tz_localtime_r(t: *const TimeT, tm: *mut tm) -> *mut tm {
    if t.is_null() || tm.is_null() {
        return core::ptr::null_mut();
    }
    let v = *t as i64;
    if v < i32::MIN as i64 * 31622400 || v > i32::MAX as i64 * 31622400 {
        ERRNO!() = EOVERFLOW;
        return core::ptr::null_mut();
    }
    let mut isdst = 0;
    let mut gmtoff = 0;
    let mut zone: *const c_char = core::ptr::null();
    secs_to_zone(
        v,
        0,
        &mut isdst,
        &mut gmtoff,
        core::ptr::null_mut(),
        &mut zone,
    );
    if !secs_to_tm(v + gmtoff as i64, &mut *tm) {
        ERRNO!() = EOVERFLOW;
        return core::ptr::null_mut();
    }
    (*tm).tm_isdst = isdst;
    (*tm).tm_gmtoff = gmtoff;
    (*tm).tm_zone = zone;
    tm
}

pub unsafe fn tz_mktime(tm: *mut tm) -> TimeT {
    if tm.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let mut t = tm_to_secs(tm);
    let mut isdst = 0;
    let mut gmtoff = 0;
    let mut opp = 0;
    let mut zone: *const c_char = core::ptr::null();

    secs_to_zone(t, 1, &mut isdst, &mut gmtoff, &mut opp, &mut zone);
    if (*tm).tm_isdst >= 0 && isdst != (*tm).tm_isdst {
        t -= opp - gmtoff;
    }
    t -= gmtoff;
    if (t as TimeT) as i64 != t {
        ERRNO!() = EOVERFLOW;
        return -1;
    }
    secs_to_zone(t, 0, &mut isdst, &mut gmtoff, &mut opp, &mut zone);
    let mut new: tm = core::mem::zeroed();
    if !secs_to_tm(t + gmtoff, &mut new) {
        ERRNO!() = EOVERFLOW;
        return -1;
    }
    new.tm_isdst = isdst;
    new.tm_gmtoff = gmtoff;
    new.tm_zone = zone;
    *tm = new;
    t as TimeT
}
