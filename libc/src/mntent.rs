// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// mntent: setmntent/endmntent/getmntent/getmntent_r/addmntent/hasmntopt
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn setmntent(name: *const c_char, mode: *const c_char) -> *mut FILE {
    fopen(name, mode)
}

#[no_mangle]
pub unsafe extern "C" fn endmntent(f: *mut FILE) -> c_int {
    if !f.is_null() {
        fclose(f);
    }
    1
}

// static buffers for getmntent (non-_r variant)
static mut MNTENT_MNT: MntEnt = MntEnt {
    mnt_fsname: core::ptr::null_mut(),
    mnt_dir: core::ptr::null_mut(),
    mnt_type: core::ptr::null_mut(),
    mnt_opts: core::ptr::null_mut(),
    mnt_freq: 0,
    mnt_passno: 0,
};

#[repr(C)]
pub struct MntEnt {
    mnt_fsname: *mut c_char,
    mnt_dir: *mut c_char,
    mnt_type: *mut c_char,
    mnt_opts: *mut c_char,
    mnt_freq: c_int,
    mnt_passno: c_int,
}

unsafe fn unescape_ent(beg: *mut c_char) -> *mut c_char {
    let mut dest = beg;
    let mut src = beg;
    while *src != 0 {
        if *src != b'\\' as c_char {
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
            continue;
        }
        // backslash
        if *src.add(1) == b'\\' as c_char {
            src = src.add(1);
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
            continue;
        }
        // octal escape
        let mut val: u8 = 0;
        let mut cnt = 0;
        let mut sp = src.add(1);
        while cnt < 3 && *sp >= b'0' as c_char && *sp <= b'7' as c_char {
            val = val * 8 + (*sp as u8 - b'0');
            sp = sp.add(1);
            cnt += 1;
        }
        if cnt > 0 {
            *dest = val as c_char;
            dest = dest.add(1);
            src = sp;
        } else {
            *dest = *src;
            dest = dest.add(1);
            src = src.add(1);
        }
    }
    *dest = 0;
    beg
}

unsafe fn parse_mntent_line(
    linebuf: *mut c_char,
    mnt: *mut MntEnt,
) -> *mut MntEnt {
    let _len = strlen(linebuf as *const c_char);
    // skip comments and empty lines
    // parse: fsname dir type opts freq passno
    let mut p = linebuf;
    // skip leading whitespace
    while *p == b' ' as c_char || *p == b'\t' as c_char || *p == b'\n' as c_char || *p == b'\r' as c_char {
        p = p.add(1);
    }
    if *p == 0 || *p == b'#' as c_char {
        return core::ptr::null_mut();
    }

    // fsname
    (*mnt).mnt_fsname = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // dir
    (*mnt).mnt_dir = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // type
    (*mnt).mnt_type = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p == 0 { return core::ptr::null_mut(); }
    *p = 0;
    p = p.add(1);
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }

    // opts
    (*mnt).mnt_opts = p;
    while *p != 0 && *p != b' ' as c_char && *p != b'\t' as c_char && *p != b'\n' as c_char {
        p = p.add(1);
    }
    if *p != 0 {
        *p = 0;
        p = p.add(1);
    }
    if *(*mnt).mnt_opts == 0 {
        (*mnt).mnt_opts = b"defaults\0".as_ptr() as *mut c_char;
    }

    // freq and passno
    (*mnt).mnt_freq = 0;
    (*mnt).mnt_passno = 0;
    // skip whitespace
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }
    if *p >= b'0' as c_char && *p <= b'9' as c_char {
        (*mnt).mnt_freq = (*p - b'0' as c_char) as c_int;
        p = p.add(1);
        while *p >= b'0' as c_char && *p <= b'9' as c_char {
            (*mnt).mnt_freq = (*mnt).mnt_freq * 10 + (*p - b'0' as c_char) as c_int;
            p = p.add(1);
        }
    }
    while *p == b' ' as c_char || *p == b'\t' as c_char { p = p.add(1); }
    if *p >= b'0' as c_char && *p <= b'9' as c_char {
        (*mnt).mnt_passno = (*p - b'0' as c_char) as c_int;
        p = p.add(1);
        while *p >= b'0' as c_char && *p <= b'9' as c_char {
            (*mnt).mnt_passno = (*mnt).mnt_passno * 10 + (*p - b'0' as c_char) as c_int;
            p = p.add(1);
        }
    }

    // unescape
    unescape_ent((*mnt).mnt_fsname);
    unescape_ent((*mnt).mnt_dir);
    unescape_ent((*mnt).mnt_type);
    unescape_ent((*mnt).mnt_opts);

    mnt
}

// Use a sentinel pointer to detect internal buffer usage
// cast integer to pointer for sentinel
static SENTINEL_PTR: usize = 1;

// Read one char from FILE buffer directly (avoids musl's static fgets ABI mismatch)
unsafe fn mntent_fgetc(f: *mut FILE) -> c_int {
    fgetc(f)
}

unsafe fn mntent_fgets(buf: *mut c_char, n: usize, f: *mut FILE) -> *mut c_char {
    if n == 0 { return core::ptr::null_mut(); }
    let max = n - 1;
    let mut i = 0usize;
    while i < max {
        let c = mntent_fgetc(f);
        if c == -1 { if i == 0 { return core::ptr::null_mut(); } break; }
        *buf.add(i) = c as c_char;
        i += 1;
        if c == b'\n' as c_int { break; }
    }
    *buf.add(i) = 0;
    buf
}

#[no_mangle]
pub unsafe extern "C" fn getmntent_r(
    f: *mut FILE,
    mnt: *mut MntEnt,
    mut linebuf: *mut c_char,
    buflen: c_int,
) -> *mut MntEnt {
    let use_internal = linebuf as usize == SENTINEL_PTR;

    (*mnt).mnt_freq = 0;
    (*mnt).mnt_passno = 0;

    static mut MNT_LINE: *mut c_char = core::ptr::null_mut();
    static mut MNT_LINE_CAP: usize = 0;

    loop {
        let res = if use_internal {
            let n = getline(&raw mut MNT_LINE, &raw mut MNT_LINE_CAP, f);
            if n < 0 {
                return core::ptr::null_mut();
            }
            linebuf = MNT_LINE;
            MNT_LINE
        } else {
            if buflen <= 0 {
                ERRNO!() = EINVAL;
                return core::ptr::null_mut();
            }
            mntent_fgets(linebuf, buflen as usize, f)
        };
        if res.is_null() {
            return core::ptr::null_mut();
        }
        let mut has_newline = false;
        let mut p = linebuf;
        while *p != 0 { if *p == b'\n' as c_char { has_newline = true; break; } p = p.add(1); }
        if !has_newline {
            ERRNO!() = ERANGE;
            return core::ptr::null_mut();
        }
        let result = parse_mntent_line(linebuf, mnt);
        if !result.is_null() {
            return result;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn getmntent(f: *mut FILE) -> *mut MntEnt {
    getmntent_r(f, core::ptr::addr_of_mut!(MNTENT_MNT), SENTINEL_PTR as *mut c_char, 0)
}

#[no_mangle]
pub unsafe extern "C" fn addmntent(f: *mut FILE, mnt: *const MntEnt) -> c_int {
    fseek(f, 0, 2); // SEEK_END
    let written = fprintf(
        f,
        b"%s\t%s\t%s\t%s\t%d\t%d\n\0".as_ptr() as *const c_char,
        (*mnt).mnt_fsname,
        (*mnt).mnt_dir,
        (*mnt).mnt_type,
        (*mnt).mnt_opts,
        (*mnt).mnt_freq,
        (*mnt).mnt_passno,
    );
    if written < 0 { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn hasmntopt(mnt: *const MntEnt, opt: *const c_char) -> *mut c_char {
    let l = strlen(opt as *const c_char);
    let mut p = (*mnt).mnt_opts;
    loop {
        if strncmp(p as *const u8, opt as *const u8, l) == 0 {
            let after = *p.add(l);
            if after == 0 || after == b',' as c_char || after == b'=' as c_char {
                return p;
            }
        }
        let comma = strchr(p as *const u8, b',' as c_int);
        if comma.is_null() {
            return core::ptr::null_mut();
        }
        p = comma.add(1) as *mut c_char;
    }
}

