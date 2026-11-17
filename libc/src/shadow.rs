// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Shadow database
// ============================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Spwd {
    pub sp_namp: *mut c_char,
    pub sp_pwdp: *mut c_char,
    pub sp_lstchg: c_long,
    pub sp_min: c_long,
    pub sp_max: c_long,
    pub sp_warn: c_long,
    pub sp_inact: c_long,
    pub sp_expire: c_long,
    pub sp_flag: c_ulong,
}

static mut SPWD_ENT: Spwd = Spwd {
    sp_namp: core::ptr::null_mut(), sp_pwdp: core::ptr::null_mut(),
    sp_lstchg: 0, sp_min: 0, sp_max: 0, sp_warn: 0, sp_inact: 0,
    sp_expire: 0, sp_flag: 0,
};
static mut SPWD_BUF: *mut c_char = core::ptr::null_mut();
static mut SPWD_BUF_CAP: usize = 0;
static mut SPWD_LINE: *mut c_char = core::ptr::null_mut();
static mut SPWD_LINE_CAP: usize = 0;
static mut SPWD_FILE: *mut FILE = core::ptr::null_mut();

unsafe fn ensure_spwd_buf(need: usize) -> bool {
    if SPWD_BUF_CAP >= need {
        return true;
    }
    let mut cap = if SPWD_BUF_CAP == 0 { 1024 } else { SPWD_BUF_CAP };
    while cap < need {
        cap = match cap.checked_mul(2) {
            Some(v) => v,
            None => return false,
        };
    }
    let grown = crate::realloc(SPWD_BUF as *mut c_void, cap) as *mut c_char;
    if grown.is_null() {
        ERRNO!() = ENOMEM;
        return false;
    }
    SPWD_BUF = grown;
    SPWD_BUF_CAP = cap;
    true
}

/// Read one line from `f` with getline and parse it into the static entry.
/// Lines that do not have all nine shadow fields are skipped.
unsafe fn spwd_next(f: *mut FILE) -> *mut Spwd {
    loop {
        let n = crate::getline(&raw mut SPWD_LINE, &raw mut SPWD_LINE_CAP, f);
        if n < 0 {
            return core::ptr::null_mut();
        }
        let len = n as usize;
        if len > 0 && *SPWD_LINE.add(len - 1) == b'\n' as c_char {
            *SPWD_LINE.add(len - 1) = 0;
        }
        if !ensure_spwd_buf(len + 1) {
            return core::ptr::null_mut();
        }
        if parse_spwd(SPWD_LINE, &raw mut SPWD_ENT, SPWD_BUF, SPWD_BUF_CAP) {
            return &raw mut SPWD_ENT;
        }
    }
}

unsafe fn parse_spwd(line: *const c_char, ent: *mut Spwd, buf: *mut c_char, buflen: usize) -> bool {
    let mut fields: [*mut c_char; 9] = [core::ptr::null_mut(); 9];
    let n = match split_colon_fields(line, buf, buflen, &mut fields) {
        Some(n) => n,
        None => return false,
    };
    if n < 9 { return false; }
    (*ent).sp_namp = fields[0];
    (*ent).sp_pwdp = fields[1];
    (*ent).sp_lstchg = crate::strtol(fields[2], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_min = crate::strtol(fields[3], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_max = crate::strtol(fields[4], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_warn = crate::strtol(fields[5], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_inact = crate::strtol(fields[6], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_expire = crate::strtol(fields[7], core::ptr::null_mut(), 10) as c_long;
    (*ent).sp_flag = crate::strtoul(fields[8], core::ptr::null_mut(), 10) as c_ulong;
    true
}

#[no_mangle]
pub unsafe extern "C" fn setspent() {
    endspent();
    SPWD_FILE = crate::fopen(b"/etc/shadow\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub unsafe extern "C" fn endspent() {
    if !SPWD_FILE.is_null() { crate::fclose(SPWD_FILE); SPWD_FILE = core::ptr::null_mut(); }
}

#[no_mangle]
pub unsafe extern "C" fn getspent() -> *mut Spwd {
    if SPWD_FILE.is_null() { setspent(); if SPWD_FILE.is_null() { return core::ptr::null_mut(); } }
    spwd_next(SPWD_FILE)
}

#[no_mangle]
pub unsafe extern "C" fn fgetspent(f: *mut FILE) -> *mut Spwd {
    if f.is_null() { return core::ptr::null_mut(); }
    spwd_next(f)
}

#[no_mangle]
pub unsafe extern "C" fn getspnam(name: *const c_char) -> *mut Spwd {
    if name.is_null() { return core::ptr::null_mut(); }
    setspent();
    loop {
        let sp = getspent();
        if sp.is_null() { break; }
        if crate::strcmp((*sp).sp_namp as *const u8, name as *const u8) == 0 { endspent(); return sp; }
    }
    endspent();
    core::ptr::null_mut()
}

unsafe fn getsp_r_inner(
    name: *const c_char,
    sp: *mut Spwd,
    buf: *mut c_char,
    buflen: usize,
    result: *mut *mut Spwd,
) -> c_int {
    if name.is_null() || sp.is_null() || buf.is_null() || result.is_null() {
        return EINVAL;
    }
    *result = core::ptr::null_mut();
    let f = crate::fopen(b"/etc/shadow\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
    if f.is_null() { return ERRNO!(); }
    let mut line: *mut c_char = core::ptr::null_mut();
    let mut cap: usize = 0;
    let mut err = 0;
    loop {
        let n = crate::getline(&raw mut line, &raw mut cap, f);
        if n < 0 { break; }
        let mut len = n as usize;
        if len > 0 && *line.add(len - 1) == b'\n' as c_char {
            *line.add(len - 1) = 0;
            len -= 1;
        }
        if len + 1 > buflen {
            err = ERANGE;
            break;
        }
        if parse_spwd(line, sp, buf, buflen) {
            if crate::strcmp((*sp).sp_namp as *const u8, name as *const u8) == 0 {
                *result = sp;
                break;
            }
        }
    }
    crate::free(line as *mut c_void);
    crate::fclose(f);
    err
}

#[no_mangle]
pub unsafe extern "C" fn getspnam_r(name: *const c_char, sp: *mut Spwd, buf: *mut c_char, buflen: usize, result: *mut *mut Spwd) -> c_int {
    getsp_r_inner(name, sp, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn putspent(sp: *const Spwd, f: *mut FILE) -> c_int {
    if sp.is_null() || f.is_null() { return -1; }
    crate::fprintf(f, b"%s:%s:%ld:%ld:%ld:%ld:%ld:%ld:%lu\\n\0".as_ptr() as *const c_char,
        (*sp).sp_namp, (*sp).sp_pwdp, (*sp).sp_lstchg, (*sp).sp_min, (*sp).sp_max,
        (*sp).sp_warn, (*sp).sp_inact, (*sp).sp_expire, (*sp).sp_flag)
}

#[no_mangle]
pub unsafe extern "C" fn getgrouplist(user: *const c_char, group: u32, groups: *mut u32, ngroups: *mut i32) -> c_int {
    if user.is_null() || groups.is_null() || ngroups.is_null() { return -1; }
    if *ngroups < 0 { return -1; }
    let max = *ngroups as usize;
    let mut n = 0usize;
    if n < max { *groups.add(n) = group; }
    n += 1;
    setgrent();
    loop {
        let g = getgrent();
        if g.is_null() { break; }
        if (*g).gr_gid == group { continue; }
        let mut found = false;
        let mut i = 0;
        while !(*(*g).gr_mem.add(i)).is_null() {
            if crate::strcmp(*(*g).gr_mem.add(i) as *const u8, user as *const u8) == 0 {
                found = true;
                break;
            }
            i += 1;
        }
        if found {
            if n < max { *groups.add(n) = (*g).gr_gid; }
            n += 1;
        }
    }
    endgrent();
    if n > max { *ngroups = n as i32; return -1; }
    *ngroups = n as i32;
    n as c_int
}
