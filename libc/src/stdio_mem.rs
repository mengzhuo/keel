// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// stdio: open_memstream / fmemopen
// ============================================================

#[repr(C)]
struct MsCookie {
    bufp: *mut *mut c_char,
    sizep: *mut usize,
    pos: usize,
    len: usize,
    buf: *mut u8,
    space: usize,
}

#[repr(C)]
struct MfCookie {
    pos: usize,
    len: usize,
    size: usize,
    buf: *mut u8,
    mode: c_char,
}

unsafe extern "C" fn ms_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let c = (*f).cookie as *mut MsCookie;
    if whence < 0 || whence > 2 { ERRNO!() = EINVAL; return -1; }
    let base: i64 = match whence {
        0 => 0,
        1 => (*c).pos as i64,
        2 => (*c).len as i64,
        _ => { ERRNO!() = EINVAL; return -1; }
    };
    if off < -base || off > isize::MAX as i64 - base { ERRNO!() = EINVAL; return -1; }
    (*c).pos = (base + off) as usize;
    (*c).pos as i64
}

unsafe extern "C" fn ms_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MsCookie;
    let buffered = (*f).wpos as usize - (*f).wbase as usize;
    if buffered > 0 {
        (*f).wpos = (*f).wbase;
        if ms_write(f, (*f).wbase, buffered) < buffered { return 0; }
    }
    if len == 0 { return 0; }
    if (*c).pos + len >= (*c).space {
        let new_space = 2 * (*c).space + 1 | (*c).pos + len + 1;
        let newbuf = realloc((*c).buf as *mut c_void, new_space) as *mut u8;
        if newbuf.is_null() { return 0; }
        *(*c).bufp = newbuf as *mut c_char;
        core::ptr::write_bytes(newbuf.add((*c).space), 0, new_space - (*c).space);
        (*c).buf = newbuf;
        (*c).space = new_space;
    }
    core::ptr::copy_nonoverlapping(buf, (*c).buf.add((*c).pos), len);
    (*c).pos += len;
    if (*c).pos >= (*c).len { (*c).len = (*c).pos; }
    *(*c).sizep = (*c).pos;
    len
}

unsafe extern "C" fn ms_close(f: *mut FILE) -> c_int {
    // The byte buffer belongs to the caller; free only the cookie.
    if !f.is_null() && !(*f).cookie.is_null() {
        free((*f).cookie);
        (*f).cookie = core::ptr::null_mut();
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn open_memstream(bufp: *mut *mut c_char, sizep: *mut usize) -> *mut FILE {
    if bufp.is_null() || sizep.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let f = calloc(1, core::mem::size_of::<FILE>() + UNGET + BUFSIZ) as *mut FILE;
    if f.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    let buf_area = buf_ptr(f);
    init_file(f, -1, b"w\0".as_ptr() as *const c_char, Some(ms_close), buf_area, BUFSIZ);
    (*f).flags = F_NORD;
    (*f).lbf = -1;
    let c = calloc(1, core::mem::size_of::<MsCookie>()) as *mut MsCookie;
    if c.is_null() { free(f as *mut c_void); ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    (*c).bufp = bufp;
    (*c).sizep = sizep;
    (*c).pos = 0;
    (*c).len = 0;
    let initial = crate::malloc(1) as *mut u8;
    if initial.is_null() {
        crate::free(c as *mut c_void);
        crate::free(f as *mut c_void);
        ERRNO!() = ENOMEM;
        return core::ptr::null_mut();
    }
    *initial = 0;
    (*c).buf = initial;
    (*c).space = 1;
    (*f).cookie = c as *mut c_void;
    (*f).write_fn = Some(ms_write);
    (*f).seek_fn = Some(ms_seek);
    *bufp = initial as *mut c_char;
    *sizep = 0;
    f
}

unsafe extern "C" fn mseek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let c = (*f).cookie as *mut MfCookie;
    if whence < 0 || whence > 2 { ERRNO!() = EINVAL; return -1; }
    let base: i64 = match whence {
        0 => 0,
        1 => (*c).pos as i64,
        2 => (*c).len as i64,
        _ => { ERRNO!() = EINVAL; return -1; }
    };
    if off < -base || off > (*c).size as i64 - base { ERRNO!() = EINVAL; return -1; }
    (*c).pos = (base + off) as usize;
    (*c).pos as i64
}

unsafe extern "C" fn mread(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MfCookie;
    let rem = if (*c).pos <= (*c).len { (*c).len - (*c).pos } else { 0 };
    let l = if len > rem {
        (*f).flags |= F_EOF;
        rem
    } else {
        len
    };
    core::ptr::copy_nonoverlapping((*c).buf.add((*c).pos), buf, l);
    (*c).pos += l;
    // fgetc() already owns the destination buffer and installs rpos/rend
    // from the returned count.  Do not pre-read here: copying a second time
    // into f.buf overwrites the bytes just returned and advances the stream
    // position twice (fmemopen streams larger than BUFSIZ returned garbage).
    l
}

unsafe extern "C" fn mwrite(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let c = (*f).cookie as *mut MfCookie;
    let buffered = (*f).wpos as usize - (*f).wbase as usize;
    if buffered > 0 {
        (*f).wpos = (*f).wbase;
        if mwrite(f, (*f).wbase, buffered) < buffered { return 0; }
    }
    if len == 0 { return 0; }
    if (*c).mode == b'a' as c_char { (*c).pos = (*c).len; }
    let rem = (*c).size - (*c).pos;
    let mut l = len;
    if l > rem { l = rem; }
    core::ptr::copy_nonoverlapping(buf, (*c).buf.add((*c).pos), l);
    (*c).pos += l;
    if (*c).pos > (*c).len {
        (*c).len = (*c).pos;
        if (*c).len < (*c).size {
            *(*c).buf.add((*c).len) = 0;
        } else if ((*f).flags & F_NORD) != 0 && (*c).size > 0 {
            *(*c).buf.add((*c).size - 1) = 0;
        }
    }
    l
}

unsafe extern "C" fn mclose(_f: *mut FILE) -> c_int {
    0
}

#[no_mangle]
pub unsafe extern "C" fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE {
    if mode.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let m = *mode;
    if m != b'r' as c_char && m != b'w' as c_char && m != b'a' as c_char {
        ERRNO!() = EINVAL; return core::ptr::null_mut();
    }
    let has_plus = !strchr(mode as *const u8, b'+' as c_int).is_null();
    let need_alloc = buf.is_null();
    let base = core::mem::size_of::<FILE>() + UNGET + BUFSIZ;
    let alloc_size = match base.checked_add(if need_alloc { size } else { 0 }) {
        Some(v) => v,
        None => {
            ERRNO!() = ENOMEM;
            return core::ptr::null_mut();
        }
    };
    let f = calloc(1, alloc_size) as *mut FILE;
    if f.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    let buf_area = buf_ptr(f);
    init_file(f, -1, mode, Some(mclose), buf_area, BUFSIZ);
    let actual_buf: *mut u8 = if need_alloc {
        // Do not derive this from alloc_size - size: that subtraction wraps
        // for sizes close to SIZE_MAX.  The private area always starts right
        // after the FILE struct.
        (f as *mut u8).add(base)
    } else {
        buf as *mut u8
    };
    if need_alloc { core::ptr::write_bytes(actual_buf, 0, size); }
    let c = calloc(1, core::mem::size_of::<MfCookie>()) as *mut MfCookie;
    if c.is_null() { free(f as *mut c_void); ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    (*c).buf = actual_buf;
    (*c).size = size;
    (*c).mode = m;
    (*f).cookie = c as *mut c_void;
    (*f).read_fn = Some(mread);
    (*f).write_fn = Some(mwrite);
    (*f).seek_fn = Some(mseek);
    if !has_plus { (*f).flags = if m == b'r' as c_char { F_NOWR } else { F_NORD }; }
    if m == b'r' as c_char {
        (*c).len = size;
    } else if m == b'a' as c_char {
        (*c).len = strnlen(actual_buf, size);
        (*c).pos = (*c).len;
    } else if has_plus && size > 0 {
        *actual_buf = 0;
    }
    f
}

