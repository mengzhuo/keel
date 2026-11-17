// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Group and passwd functions
// ============================================================

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Passwd {
    pub pw_name: *mut c_char,
    pub pw_passwd: *mut c_char,
    pub pw_uid: u32,
    pub pw_gid: u32,
    pub pw_gecos: *mut c_char,
    pub pw_dir: *mut c_char,
    pub pw_shell: *mut c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Group {
    pub gr_name: *mut c_char,
    pub gr_passwd: *mut c_char,
    pub gr_gid: u32,
    pub gr_mem: *mut *mut c_char,
}

static mut PWD_ENT: Passwd = Passwd {
    pw_name: core::ptr::null_mut(), pw_passwd: core::ptr::null_mut(),
    pw_uid: 0, pw_gid: 0, pw_gecos: core::ptr::null_mut(),
    pw_dir: core::ptr::null_mut(), pw_shell: core::ptr::null_mut(),
};
static mut PWD_LINE: *mut c_char = core::ptr::null_mut();
static mut PWD_LINE_CAP: usize = 0;
static mut PWD_BUF: *mut c_char = core::ptr::null_mut();
static mut PWD_BUF_CAP: usize = 0;
static mut PWD_FILE: *mut FILE = core::ptr::null_mut();

unsafe fn ensure_pwd_buf(n: usize) -> bool {
    if n <= PWD_BUF_CAP {
        return true;
    }
    let mut cap = if PWD_BUF_CAP == 0 { 1024 } else { PWD_BUF_CAP };
    while cap < n {
        cap *= 2;
    }
    let p = realloc(PWD_BUF as *mut c_void, cap) as *mut c_char;
    if p.is_null() {
        return false;
    }
    PWD_BUF = p;
    PWD_BUF_CAP = cap;
    true
}

static mut GRP_ENT: Group = Group {
    gr_name: core::ptr::null_mut(), gr_passwd: core::ptr::null_mut(),
    gr_gid: 0, gr_mem: core::ptr::null_mut(),
};
static mut GRP_LINE: *mut c_char = core::ptr::null_mut();
static mut GRP_LINE_CAP: usize = 0;
static mut GRP_BUF: *mut c_char = core::ptr::null_mut();
static mut GRP_BUF_CAP: usize = 0;
static mut GRP_MEM: *mut *mut c_char = core::ptr::null_mut();
static mut GRP_MEM_CAP: usize = 0;
static mut GRP_FILE: *mut FILE = core::ptr::null_mut();

unsafe fn ensure_grp_buf(n: usize) -> bool {
    if n <= GRP_BUF_CAP {
        return true;
    }
    let mut cap = if GRP_BUF_CAP == 0 { 2048 } else { GRP_BUF_CAP };
    while cap < n {
        cap *= 2;
    }
    let p = realloc(GRP_BUF as *mut c_void, cap) as *mut c_char;
    if p.is_null() {
        return false;
    }
    GRP_BUF = p;
    GRP_BUF_CAP = cap;
    true
}

unsafe fn ensure_grp_mem(n: usize) -> bool {
    if n <= GRP_MEM_CAP {
        return true;
    }
    let mut cap = if GRP_MEM_CAP == 0 { 32 } else { GRP_MEM_CAP };
    while cap < n {
        cap *= 2;
    }
    let p = realloc(
        GRP_MEM as *mut c_void,
        cap * core::mem::size_of::<*mut c_char>(),
    ) as *mut *mut c_char;
    if p.is_null() {
        return false;
    }
    GRP_MEM = p;
    GRP_MEM_CAP = cap;
    true
}

/// Copy colon-separated fields from `line` into `buf`, NUL-terminating each.
/// Returns the number of fields found, or None when `buf` is too small.
unsafe fn split_colon_fields(
    line: *const c_char,
    buf: *mut c_char,
    buflen: usize,
    fields: &mut [*mut c_char],
) -> Option<usize> {
    let mut src = line;
    let mut o = 0usize;
    let mut n = 0usize;
    while n < fields.len() {
        let start = o;
        while *src != 0 && *src != b':' as c_char {
            if o + 1 >= buflen { return None; }
            *buf.add(o) = *src;
            o += 1;
            src = src.add(1);
        }
        *buf.add(o) = 0;
        o += 1;
        fields[n] = buf.add(start);
        n += 1;
        if *src == 0 { break; }
        src = src.add(1);
    }
    Some(n)
}

unsafe fn parse_passwd(line: *const c_char, ent: *mut Passwd, buf: *mut c_char, buflen: usize) -> bool {
    let mut fields: [*mut c_char; 7] = [core::ptr::null_mut(); 7];
    let n = match split_colon_fields(line, buf, buflen, &mut fields) {
        Some(n) => n,
        None => return false,
    };
    if n < 7 { return false; }
    (*ent).pw_name = fields[0];
    (*ent).pw_passwd = fields[1];
    (*ent).pw_uid = crate::strtol(fields[2], core::ptr::null_mut(), 10) as u32;
    (*ent).pw_gid = crate::strtol(fields[3], core::ptr::null_mut(), 10) as u32;
    (*ent).pw_gecos = fields[4];
    (*ent).pw_dir = fields[5];
    (*ent).pw_shell = fields[6];
    true
}

unsafe fn parse_group(line: *const c_char, ent: *mut Group, buf: *mut c_char, buflen: usize) -> bool {
    let mut fields: [*mut c_char; 4] = [core::ptr::null_mut(); 4];
    let n = match split_colon_fields(line, buf, buflen, &mut fields) {
        Some(n) => n,
        None => return false,
    };
    if n < 4 { return false; }
    (*ent).gr_name = fields[0];
    (*ent).gr_passwd = fields[1];
    (*ent).gr_gid = crate::strtol(fields[2], core::ptr::null_mut(), 10) as u32;
    let mut count = 1usize;
    let mut m = fields[3];
    while *m != 0 {
        if *m == b',' as c_char { count += 1; }
        m = m.add(1);
    }
    if !ensure_grp_mem(count + 1) { return false; }
    let mut mi = 0usize;
    m = fields[3];
    if *m != 0 {
        loop {
            *GRP_MEM.add(mi) = m;
            mi += 1;
            while *m != 0 && *m != b',' as c_char { m = m.add(1); }
            if *m == 0 { break; }
            *m = 0;
            m = m.add(1);
        }
    }
    *GRP_MEM.add(mi) = core::ptr::null_mut();
    (*ent).gr_mem = GRP_MEM;
    true
}

unsafe fn scan_passwd(mut matches: impl FnMut(*const Passwd) -> bool) -> *mut Passwd {
    setpwent();
    loop {
        let p = getpwent();
        if p.is_null() { break; }
        if matches(p) {
            endpwent();
            return p;
        }
    }
    endpwent();
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn getpwnam(name: *const c_char) -> *mut Passwd {
    if name.is_null() { return core::ptr::null_mut(); }
    scan_passwd(|p| crate::strcmp((*p).pw_name as *const u8, name as *const u8) == 0)
}

#[no_mangle]
pub unsafe extern "C" fn getpwuid(uid: u32) -> *mut Passwd {
    scan_passwd(|p| (*p).pw_uid == uid)
}

#[no_mangle]
pub unsafe extern "C" fn setpwent() {
    endpwent();
    PWD_FILE = crate::fopen(b"/etc/passwd\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub unsafe extern "C" fn getpwent() -> *mut Passwd {
    if PWD_FILE.is_null() { setpwent(); if PWD_FILE.is_null() { return core::ptr::null_mut(); } }
    loop {
        let n = crate::getline(&raw mut PWD_LINE, &raw mut PWD_LINE_CAP, PWD_FILE);
        if n < 0 { return core::ptr::null_mut(); }
        let mut len = n as usize;
        if len > 0 && *PWD_LINE.add(len - 1) == b'\n' as c_char {
            *PWD_LINE.add(len - 1) = 0;
            len -= 1;
        }
        let need = len + 1;
        if !ensure_pwd_buf(need) { return core::ptr::null_mut(); }
        if parse_passwd(PWD_LINE, &raw mut PWD_ENT, PWD_BUF, need) {
            return &raw mut PWD_ENT;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn endpwent() {
    if !PWD_FILE.is_null() { crate::fclose(PWD_FILE); PWD_FILE = core::ptr::null_mut(); }
}

unsafe fn getpw_r_inner(
    name: *const c_char,
    uid: Option<u32>,
    pwd: *mut Passwd,
    buf: *mut c_char,
    buflen: usize,
    result: *mut *mut Passwd,
) -> c_int {
    if pwd.is_null() || buf.is_null() || result.is_null() || (uid.is_none() && name.is_null()) {
        return EINVAL;
    }
    *result = core::ptr::null_mut();
    let f = crate::fopen(b"/etc/passwd\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
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
        if parse_passwd(line, pwd, buf, buflen) {
            let hit = if let Some(uid) = uid {
                (*pwd).pw_uid == uid
            } else {
                crate::strcmp((*pwd).pw_name as *const u8, name as *const u8) == 0
            };
            if hit {
                *result = pwd;
                break;
            }
        }
    }
    crate::free(line as *mut c_void);
    crate::fclose(f);
    err
}

#[no_mangle]
pub unsafe extern "C" fn getpwnam_r(name: *const c_char, pwd: *mut Passwd, buf: *mut c_char, buflen: usize, result: *mut *mut Passwd) -> c_int {
    getpw_r_inner(name, None, pwd, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn getpwuid_r(uid: u32, pwd: *mut Passwd, buf: *mut c_char, buflen: usize, result: *mut *mut Passwd) -> c_int {
    getpw_r_inner(core::ptr::null(), Some(uid), pwd, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn fgetpwent(f: *mut FILE) -> *mut Passwd {
    if f.is_null() { return core::ptr::null_mut(); }
    loop {
        let n = crate::getline(&raw mut PWD_LINE, &raw mut PWD_LINE_CAP, f);
        if n < 0 { return core::ptr::null_mut(); }
        let mut len = n as usize;
        if len > 0 && *PWD_LINE.add(len - 1) == b'\n' as c_char {
            *PWD_LINE.add(len - 1) = 0;
            len -= 1;
        }
        let need = len + 1;
        if !ensure_pwd_buf(need) { return core::ptr::null_mut(); }
        if parse_passwd(PWD_LINE, &raw mut PWD_ENT, PWD_BUF, need) {
            return &raw mut PWD_ENT;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn putpwent(f: *mut FILE, p: *const Passwd) -> c_int {
    if f.is_null() || p.is_null() { return -1; }
    crate::fprintf(f, b"%s:%s:%u:%u:%s:%s:%s\n\0".as_ptr() as *const c_char,
        (*p).pw_name, (*p).pw_passwd, (*p).pw_uid, (*p).pw_gid,
        (*p).pw_gecos, (*p).pw_dir, (*p).pw_shell)
}

unsafe fn scan_group(mut matches: impl FnMut(*const Group) -> bool) -> *mut Group {
    setgrent();
    loop {
        let g = getgrent();
        if g.is_null() { break; }
        if matches(g) {
            endgrent();
            return g;
        }
    }
    endgrent();
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn getgrnam(name: *const c_char) -> *mut Group {
    if name.is_null() { return core::ptr::null_mut(); }
    scan_group(|g| crate::strcmp((*g).gr_name as *const u8, name as *const u8) == 0)
}

#[no_mangle]
pub unsafe extern "C" fn getgrgid(gid: u32) -> *mut Group {
    scan_group(|g| (*g).gr_gid == gid)
}

#[no_mangle]
pub unsafe extern "C" fn setgrent() {
    endgrent();
    GRP_FILE = crate::fopen(b"/etc/group\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
}

#[no_mangle]
pub unsafe extern "C" fn getgrent() -> *mut Group {
    if GRP_FILE.is_null() { setgrent(); if GRP_FILE.is_null() { return core::ptr::null_mut(); } }
    loop {
        let n = crate::getline(&raw mut GRP_LINE, &raw mut GRP_LINE_CAP, GRP_FILE);
        if n < 0 { return core::ptr::null_mut(); }
        let mut len = n as usize;
        if len > 0 && *GRP_LINE.add(len - 1) == b'\n' as c_char {
            *GRP_LINE.add(len - 1) = 0;
            len -= 1;
        }
        let need = len + 1;
        if !ensure_grp_buf(need) { return core::ptr::null_mut(); }
        if parse_group(GRP_LINE, &raw mut GRP_ENT, GRP_BUF, need) {
            return &raw mut GRP_ENT;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn endgrent() {
    if !GRP_FILE.is_null() { crate::fclose(GRP_FILE); GRP_FILE = core::ptr::null_mut(); }
}

// Parses one group line into the caller's buffer.  The member pointer array
// is placed in the same buffer, after the strings, aligned to pointer size.
unsafe fn parse_group_r(line: *const c_char, ent: *mut Group, buf: *mut c_char, buflen: usize) -> c_int {
    const OK: c_int = 0;
    const BAD: c_int = -1;
    const NOSPACE: c_int = -2;

    let mut fields: [*mut c_char; 4] = [core::ptr::null_mut(); 4];
    let n = match split_colon_fields(line, buf, buflen, &mut fields) {
        Some(n) => n,
        None => return NOSPACE,
    };
    if n < 4 { return BAD; }

    let mut count = 1usize;
    let mut m = fields[3];
    while *m != 0 {
        if *m == b',' as c_char { count += 1; }
        m = m.add(1);
    }
    // The member pointer array lives after the copied strings in `buf`.
    let ptr_size = core::mem::size_of::<*mut c_char>();
    let base = buf as usize;
    let o = crate::strlen(buf) + 1;
    let aligned = match base
        .checked_add(o)
        .and_then(|v| v.checked_add(ptr_size - 1))
    {
        Some(v) => v & !(ptr_size - 1),
        None => return NOSPACE,
    };
    let ptr_bytes = match (count + 1).checked_mul(ptr_size) {
        Some(v) => v,
        None => return NOSPACE,
    };
    let need = match aligned.checked_sub(base).and_then(|v| v.checked_add(ptr_bytes)) {
        Some(v) => v,
        None => return NOSPACE,
    };
    if need > buflen { return NOSPACE; }

    let mem = aligned as *mut *mut c_char;
    let mut mi = 0usize;
    m = fields[3];
    if *m != 0 {
        loop {
            *mem.add(mi) = m;
            mi += 1;
            while *m != 0 && *m != b',' as c_char { m = m.add(1); }
            if *m == 0 { break; }
            *m = 0;
            m = m.add(1);
        }
    }
    *mem.add(mi) = core::ptr::null_mut();

    (*ent).gr_name = fields[0];
    (*ent).gr_passwd = fields[1];
    (*ent).gr_gid = crate::strtol(fields[2], core::ptr::null_mut(), 10) as u32;
    (*ent).gr_mem = mem;
    OK
}

unsafe fn getgr_r_inner(
    name: *const c_char,
    gid: Option<u32>,
    grp: *mut Group,
    buf: *mut c_char,
    buflen: usize,
    result: *mut *mut Group,
) -> c_int {
    if grp.is_null() || buf.is_null() || result.is_null() || (gid.is_none() && name.is_null()) {
        return EINVAL;
    }
    *result = core::ptr::null_mut();
    let f = crate::fopen(b"/etc/group\0".as_ptr() as *const c_char, b"r\0".as_ptr() as *const c_char);
    if f.is_null() { return ERRNO!(); }
    let mut line: *mut c_char = core::ptr::null_mut();
    let mut cap: usize = 0;
    let mut err = 0;
    loop {
        let n = crate::getline(&raw mut line, &raw mut cap, f);
        if n < 0 { break; }
        if n > 0 && *line.add(n as usize - 1) == b'\n' as c_char {
            *line.add(n as usize - 1) = 0;
        }
        let r = parse_group_r(line, grp, buf, buflen);
        if r == -2 {
            err = ERANGE;
            break;
        }
        if r == 0 {
            let hit = if let Some(gid) = gid {
                (*grp).gr_gid == gid
            } else {
                crate::strcmp((*grp).gr_name as *const u8, name as *const u8) == 0
            };
            if hit {
                *result = grp;
                break;
            }
        }
    }
    crate::free(line as *mut c_void);
    crate::fclose(f);
    err
}

#[no_mangle]
pub unsafe extern "C" fn getgrnam_r(name: *const c_char, grp: *mut Group, buf: *mut c_char, buflen: usize, result: *mut *mut Group) -> c_int {
    getgr_r_inner(name, None, grp, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn getgrgid_r(gid: u32, grp: *mut Group, buf: *mut c_char, buflen: usize, result: *mut *mut Group) -> c_int {
    getgr_r_inner(core::ptr::null(), Some(gid), grp, buf, buflen, result)
}

#[no_mangle]
pub unsafe extern "C" fn fgetgrent(f: *mut FILE) -> *mut Group {
    if f.is_null() { return core::ptr::null_mut(); }
    loop {
        let n = crate::getline(&raw mut GRP_LINE, &raw mut GRP_LINE_CAP, f);
        if n < 0 { return core::ptr::null_mut(); }
        let mut len = n as usize;
        if len > 0 && *GRP_LINE.add(len - 1) == b'\n' as c_char {
            *GRP_LINE.add(len - 1) = 0;
            len -= 1;
        }
        let need = len + 1;
        if !ensure_grp_buf(need) { return core::ptr::null_mut(); }
        if parse_group(GRP_LINE, &raw mut GRP_ENT, GRP_BUF, need) {
            return &raw mut GRP_ENT;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn putgrent(f: *mut FILE, g: *const Group) -> c_int {
    if f.is_null() || g.is_null() { return -1; }
    crate::fprintf(f, b"%s:%s:%u:\0".as_ptr() as *const c_char, (*g).gr_name, (*g).gr_passwd, (*g).gr_gid);
    if !(*g).gr_mem.is_null() {
        let mut i = 0;
        while !(*(*g).gr_mem.add(i)).is_null() {
            if i > 0 { crate::fputc(b',' as c_int, f); }
            crate::fputs(*(*g).gr_mem.add(i), f);
            i += 1;
        }
    }
    crate::fputc(b'\n' as c_int, f);
    0
}

