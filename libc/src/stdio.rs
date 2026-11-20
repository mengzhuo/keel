// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// FILE / stdio (buffered, musl-compatible layout)
// ============================================================

const UNGET: usize = 8;
const F_PERM: u32 = 1;
const F_NORD: u32 = 4;
const F_NOWR: u32 = 8;
const F_EOF: u32 = 16;
const F_ERR: u32 = 32;
const F_SVB: u32 = 64;
const F_APP: u32 = 128;

const BUFSIZ: usize = 1024;

const _IOFBF: c_int = 0;
const _IOLBF: c_int = 1;
const _IONBF: c_int = 2;

const SEEK_SET: c_int = 0;
const SEEK_CUR: c_int = 1;
const SEEK_END: c_int = 2;

// Buffered FILE using musl's 64-bit layout.
#[repr(C)]
pub struct FILE {
    flags: c_uint,
    rpos: *mut u8,
    rend: *mut u8,
    close: Option<unsafe extern "C" fn(*mut FILE) -> c_int>,
    wend: *mut u8,
    wpos: *mut u8,
    mustbezero_1: *mut u8,
    wbase: *mut u8,
    read_fn: Option<unsafe extern "C" fn(*mut FILE, *mut u8, usize) -> usize>,
    write_fn: Option<unsafe extern "C" fn(*mut FILE, *const u8, usize) -> usize>,
    seek_fn: Option<unsafe extern "C" fn(*mut FILE, i64, c_int) -> i64>,
    buf: *mut u8,
    buf_size: usize,
    prev: *mut FILE,
    next: *mut FILE,
    fd: c_int,
    pipe_pid: c_int,
    lockcount: c_long,
    mode: c_int,
    lock: c_int,
    lbf: c_int,
    cookie: *mut c_void,
    off: i64,
    getln_buf: *mut c_char,
    mustbezero_2: *mut c_void,
    shend: *mut u8,
    shlim: i64,
    shcnt: i64,
    ungotten: [c_int; UNGET],
    ungotten_count: c_int,
    _eof: c_int,
    _err: c_int,
}

static mut STDIN_BUF: [u8; BUFSIZ] = [0; BUFSIZ];
static mut STDOUT_BUF: [u8; BUFSIZ] = [0; BUFSIZ];
static mut STDERR_BUF: [u8; BUFSIZ] = [0; BUFSIZ];

static mut STDIN_FILE: FILE = FILE {
    flags: 0, rpos: core::ptr::null_mut(), rend: core::ptr::null_mut(),
    close: None, wend: core::ptr::null_mut(), wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(), wbase: core::ptr::null_mut(),
    read_fn: None, write_fn: None, seek_fn: None,
    buf: core::ptr::null_mut(), buf_size: BUFSIZ,
    prev: core::ptr::null_mut(), next: core::ptr::null_mut(),
    fd: 0, pipe_pid: 0, lockcount: 0, mode: 0, lock: 0, lbf: -1,
    cookie: core::ptr::null_mut(), off: 0,
    getln_buf: core::ptr::null_mut(), mustbezero_2: core::ptr::null_mut(),
    shend: core::ptr::null_mut(), shlim: 0, shcnt: 0,
    ungotten: [0; UNGET], ungotten_count: 0, _eof: 0, _err: 0,
};
static mut STDOUT_FILE: FILE = FILE {
    flags: F_NOWR as u32 + F_SVB as u32, rpos: core::ptr::null_mut(), rend: core::ptr::null_mut(),
    close: None, wend: core::ptr::null_mut(), wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(), wbase: core::ptr::null_mut(),
    read_fn: None, write_fn: None, seek_fn: None,
    buf: core::ptr::null_mut(), buf_size: BUFSIZ,
    prev: core::ptr::null_mut(), next: core::ptr::null_mut(),
    fd: 1, pipe_pid: 0, lockcount: 0, mode: 0, lock: 0, lbf: -1,
    cookie: core::ptr::null_mut(), off: 0,
    getln_buf: core::ptr::null_mut(), mustbezero_2: core::ptr::null_mut(),
    shend: core::ptr::null_mut(), shlim: 0, shcnt: 0,
    ungotten: [0; UNGET], ungotten_count: 0, _eof: 0, _err: 0,
};
static mut STDERR_FILE: FILE = FILE {
    flags: F_NOWR as u32, rpos: core::ptr::null_mut(), rend: core::ptr::null_mut(),
    close: None, wend: core::ptr::null_mut(), wpos: core::ptr::null_mut(),
    mustbezero_1: core::ptr::null_mut(), wbase: core::ptr::null_mut(),
    read_fn: None, write_fn: None, seek_fn: None,
    buf: core::ptr::null_mut(), buf_size: BUFSIZ,
    prev: core::ptr::null_mut(), next: core::ptr::null_mut(),
    fd: 2, pipe_pid: 0, lockcount: 0, mode: 0, lock: 0, lbf: -1,
    cookie: core::ptr::null_mut(), off: 0,
    getln_buf: core::ptr::null_mut(), mustbezero_2: core::ptr::null_mut(),
    shend: core::ptr::null_mut(), shlim: 0, shcnt: 0,
    ungotten: [0; UNGET], ungotten_count: 0, _eof: 0, _err: 0,
};

#[no_mangle]
pub static mut stdin: *mut FILE = &raw mut STDIN_FILE as *mut FILE;
#[no_mangle]
pub static mut stdout: *mut FILE = &raw mut STDOUT_FILE as *mut FILE;
#[no_mangle]
pub static mut stderr: *mut FILE = &raw mut STDERR_FILE as *mut FILE;

// All open streams, so fflush(NULL) can flush everything and fclose can
// unlink/free dynamically allocated FILEs.
static mut OFL_HEAD: *mut FILE = core::ptr::null_mut();

unsafe fn ofl_add(f: *mut FILE) {
    (*f).prev = core::ptr::null_mut();
    (*f).next = OFL_HEAD;
    if !OFL_HEAD.is_null() {
        (*OFL_HEAD).prev = f;
    }
    OFL_HEAD = f;
}

unsafe fn ofl_remove(f: *mut FILE) {
    let prev = (*f).prev;
    let next = (*f).next;
    if prev.is_null() {
        OFL_HEAD = next;
    } else {
        (*prev).next = next;
    }
    if !next.is_null() {
        (*next).prev = prev;
    }
    (*f).prev = core::ptr::null_mut();
    (*f).next = core::ptr::null_mut();
}

// ============================================================
// FILE helpers
// ============================================================

unsafe fn __stdio_init() {
    STDIN_FILE.buf = core::ptr::addr_of_mut!(STDIN_BUF) as *mut u8;
    STDIN_FILE.buf_size = BUFSIZ;
    STDIN_FILE.read_fn = Some(__stdio_read);
    STDIN_FILE.write_fn = Some(__stdio_write);
    STDIN_FILE.seek_fn = Some(__stdio_seek);
    STDOUT_FILE.buf = core::ptr::addr_of_mut!(STDOUT_BUF) as *mut u8;
    STDOUT_FILE.buf_size = BUFSIZ;
    STDOUT_FILE.lbf = b'\n' as c_int;
    STDOUT_FILE.read_fn = Some(__stdio_read);
    STDOUT_FILE.write_fn = Some(__stdio_write);
    STDOUT_FILE.seek_fn = Some(__stdio_seek);
    STDERR_FILE.buf = core::ptr::addr_of_mut!(STDERR_BUF) as *mut u8;
    STDERR_FILE.buf_size = BUFSIZ;
    STDERR_FILE.read_fn = Some(__stdio_read);
    STDERR_FILE.write_fn = Some(__stdio_write);
    STDERR_FILE.seek_fn = Some(__stdio_seek);
    STDIN_FILE.flags |= F_PERM;
    STDOUT_FILE.flags |= F_PERM;
    STDERR_FILE.flags |= F_PERM;
    STDIN_FILE.close = Some(__stdio_close);
    STDOUT_FILE.close = Some(__stdio_close);
    STDERR_FILE.close = Some(__stdio_close);
    ofl_add(&raw mut STDIN_FILE as *mut FILE);
    ofl_add(&raw mut STDOUT_FILE as *mut FILE);
    ofl_add(&raw mut STDERR_FILE as *mut FILE);
}

unsafe fn buf_ptr(f: *mut FILE) -> *mut u8 {
    (f as *mut u8).add(core::mem::size_of::<FILE>())
}

unsafe extern "C" fn __stdio_read(f: *mut FILE, buf: *mut u8, len: usize) -> usize {
    let n = sys_read((*f).fd as i64, buf, len);
    if n < 0 {
        // Keep the syscall error visible to callers that use nonblocking
        // streams (BusyBox wget checks errno == EAGAIN after a short fread).
        ERRNO!() = (-n) as c_int;
        (*f).flags |= F_ERR;
        (*f)._err = 1;
        return 0;
    }
    if n == 0 {
        (*f).flags |= F_EOF;
        return 0;
    }
    n as usize
}

unsafe extern "C" fn __stdio_write(f: *mut FILE, buf: *const u8, len: usize) -> usize {
    let l = (*f).wpos as usize - (*f).wbase as usize;
    let mut iov = [
        ((*f).wbase as *const u8, l),
        (buf, len),
    ];
    if iov[0].1 == 0 { iov = [iov[1], (core::ptr::null(), 0)]; }
    if (*f).flags & F_APP != 0 {
        let _ = sys_lseek((*f).fd as i64, 0, SEEK_END as i64);
    }
    let mut rem = iov[0].1 + iov[1].1;
    let mut idx = 0usize;
    while rem > 0 {
        let ptr = iov[idx].0;
        let cnt = iov[idx].1;
        if cnt == 0 { idx += 1; continue; }
        let n = sys_write((*f).fd as i64, ptr, cnt);
        if n < 0 { ERRNO!() = (-n) as c_int; }
        if n <= 0 {
            (*f).wpos = core::ptr::null_mut();
            (*f).wbase = core::ptr::null_mut();
            (*f).wend = core::ptr::null_mut();
            (*f).flags |= F_ERR;
            return if iov[idx].0 == buf { 0 } else { len };
        }
        rem -= n as usize;
        iov[idx].0 = iov[idx].0.add(n as usize);
        iov[idx].1 -= n as usize;
        if iov[idx].1 == 0 { idx += 1; }
    }
    (*f).wend = (*f).buf.add((*f).buf_size);
    (*f).wbase = (*f).buf;
    (*f).wpos = (*f).buf;
    len
}

unsafe extern "C" fn __stdio_seek(f: *mut FILE, off: i64, whence: c_int) -> i64 {
    let r = sys_lseek((*f).fd as i64, off, whence as i64);
    if r < 0 { ERRNO!() = (-r) as c_int; }
    r
}

unsafe fn init_file(
    f: *mut FILE,
    fd: c_int,
    mode: *const c_char,
    close_fn: Option<unsafe extern "C" fn(*mut FILE) -> c_int>,
    buf_area: *mut u8,
    buf_sz: usize,
) {
    core::ptr::write_bytes(f as *mut u8, 0, core::mem::size_of::<FILE>());
    (*f).fd = fd;
    (*f).close = close_fn;
    (*f).lock = 0;
    (*f).lbf = -1;
    (*f).buf = buf_area;
    (*f).buf_size = buf_sz;
    (*f).read_fn = Some(__stdio_read);
    (*f).write_fn = Some(__stdio_write);
    (*f).seek_fn = Some(__stdio_seek);
    let m = *mode;
    let has_plus = !strchr(mode as *const u8, b'+' as c_int).is_null();
    if m == b'r' as c_char && !has_plus {
        (*f).flags = F_NORD;
    } else if !has_plus {
        (*f).flags = F_NOWR;
    }
    ofl_add(f);
}

unsafe extern "C" fn __stdio_close(f: *mut FILE) -> c_int {
    sys_close((*f).fd as i64);
    0
}

unsafe fn fmodeflags(mode: *const c_char) -> c_int {
    let mut flags: c_int;
    let m = *mode;
    if !strchr(mode as *const u8, b'+' as c_int).is_null() {
        flags = O_RDWR;
    } else if m == b'r' as c_char {
        flags = O_RDONLY;
    } else {
        flags = O_WRONLY;
    }
    if !strchr(mode as *const u8, b'x' as c_int).is_null() { flags |= O_EXCL; }
    if !strchr(mode as *const u8, b'e' as c_int).is_null() { flags |= O_CLOEXEC; }
    if m != b'r' as c_char { flags |= O_CREAT; }
    if m == b'w' as c_char { flags |= O_TRUNC; }
    if m == b'a' as c_char { flags |= O_APPEND; }
    flags
}

const O_ACCMODE: c_int = O_RDONLY | O_WRONLY | O_RDWR;

unsafe fn flush_buf(f: *mut FILE) -> c_int {
    if (*f).wpos == (*f).wbase {
        if !(*f).wpos.is_null() {
            (*f).wend = (*f).buf.add((*f).buf_size);
            (*f).wpos = (*f).wbase;
        }
        return 0;
    }
    if let Some(wfunc) = (*f).write_fn {
        let ret = wfunc(f, core::ptr::null(), 0);
        if ret == 0 && (*f).wpos.is_null() { return -1; }
        return 0;
    }
    (*f).flags |= F_ERR;
    -1
}

// ============================================================
// fopen / fdopen / freopen / fclose
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fopen(filename: *const c_char, mode: *const c_char) -> *mut FILE {
    if filename.is_null() || mode.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let m = *mode;
    if m != b'r' as c_char && m != b'w' as c_char && m != b'a' as c_char {
        ERRNO!() = EINVAL; return core::ptr::null_mut();
    }
    let flags = fmodeflags(mode);
    let fd = sys_open(filename as *const u8, flags as i64, 0o666);
    if fd < 0 { ERRNO!() = (-fd) as c_int; return core::ptr::null_mut(); }
    let f = fdopen(fd as c_int, mode);
    if f.is_null() {
        sys_close(fd);
    }
    f
}

#[no_mangle]
pub unsafe extern "C" fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE {
    if fd < 0 || mode.is_null() { ERRNO!() = EINVAL; return core::ptr::null_mut(); }
    let m = *mode;
    if m != b'r' as c_char && m != b'w' as c_char && m != b'a' as c_char {
        ERRNO!() = EINVAL; return core::ptr::null_mut();
    }
    let f = calloc(1, core::mem::size_of::<FILE>() + UNGET + BUFSIZ) as *mut FILE;
    if f.is_null() { ERRNO!() = ENOMEM; return core::ptr::null_mut(); }
    let buf = buf_ptr(f);
    init_file(f, fd, mode, Some(__stdio_close), buf, BUFSIZ);
    if !strchr(mode as *const u8, b'a' as c_int).is_null() { (*f).flags |= F_APP; }
    if !strchr(mode as *const u8, b'e' as c_int).is_null() { sys_fcntl(fd, F_SETFD, FD_CLOEXEC as i64); }
    if (*f).flags & F_NOWR == 0 {
        let mut ws: winsize = core::mem::zeroed();
        if sys_ioctl(fd, TIOCGWINSZ, &mut ws as *mut winsize as *mut u8) == 0 {
            (*f).lbf = b'\n' as c_int;
        }
    }
    f
}

#[no_mangle]
pub unsafe extern "C" fn freopen(
    filename: *const c_char, mode: *const c_char, f: *mut FILE,
) -> *mut FILE {
    fflush(f);
    if (*f).pipe_pid > 0 { let _ = __stdio_close(f); }
    if filename.is_null() {
        let m = *mode;
        if m != b'r' as c_char && m != b'w' as c_char && m != b'a' as c_char {
            ERRNO!() = EINVAL;
            return core::ptr::null_mut();
        }
        let flags = fmodeflags(mode);
        let mut fl = flags & !(O_CREAT | O_EXCL | O_CLOEXEC);
        if (*f).flags & F_APP != 0 { fl = (fl & !O_ACCMODE) | O_WRONLY | O_APPEND; }
        if sys_fcntl((*f).fd, F_SETFL, fl as i64) < 0 { return core::ptr::null_mut(); }
        let has_plus = !strchr(mode as *const u8, b'+' as c_int).is_null();
        if has_plus {
            (*f).flags &= !(F_NORD | F_NOWR);
        } else if *mode == b'r' as c_char {
            (*f).flags = ((*f).flags & !F_NOWR) | F_NORD;
        } else {
            (*f).flags = ((*f).flags & !F_NORD) | F_NOWR;
        }
        return f;
    }
    let new_f = fopen(filename, mode);
    if new_f.is_null() { let _ = __stdio_close(f); return core::ptr::null_mut(); }
    if (*new_f).fd == (*f).fd {
        (*new_f).fd = -1;
    } else {
        sys_dup3((*new_f).fd, (*f).fd, 0);
        let _ = __stdio_close(new_f);
    }
    (*f).flags = (*new_f).flags;
    (*f).rpos = core::ptr::null_mut();
    (*f).rend = core::ptr::null_mut();
    (*f).wpos = core::ptr::null_mut();
    (*f).wbase = core::ptr::null_mut();
    (*f).wend = core::ptr::null_mut();
    (*f).lbf = (*new_f).lbf;
    (*f).pipe_pid = 0;
    ofl_remove(new_f);
    free(new_f as *mut c_void);
    f
}

#[no_mangle]
pub unsafe extern "C" fn fclose(f: *mut FILE) -> c_int {
    if f.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let r = fflush(f);
    let close_r = if let Some(close_fn) = (*f).close { close_fn(f) } else { 0 };
    let permanent = (*f).flags & F_PERM != 0;
    let allocated_buf = if (*f).flags & F_SVB != 0 { (*f).buf } else { core::ptr::null_mut() };
    let getln = (*f).getln_buf;
    ofl_remove(f);
    if !permanent {
        if !allocated_buf.is_null() { free(allocated_buf as *mut c_void); }
        if !getln.is_null() { free(getln as *mut c_void); }
        free(f as *mut c_void);
    }
    if r != 0 || close_r != 0 { -1 } else { 0 }
}

// ============================================================
// setvbuf / setbuf / setbuffer / fflush
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn setvbuf(f: *mut FILE, buf: *mut c_char, mode: c_int, size: usize) -> c_int {
    if f.is_null() || (mode != _IOFBF && mode != _IOLBF && mode != _IONBF) {
        ERRNO!() = EINVAL;
        return -1;
    }
    fflush(f);
    (*f).rpos = core::ptr::null_mut();
    (*f).rend = core::ptr::null_mut();
    (*f).wpos = core::ptr::null_mut();
    (*f).wbase = core::ptr::null_mut();
    (*f).wend = core::ptr::null_mut();
    if mode == _IONBF {
        (*f).buf = core::ptr::null_mut();
        (*f).buf_size = 0;
    } else {
        let wanted = if size == 0 { BUFSIZ } else { size };
        let actual = if buf.is_null() {
            let p = malloc(wanted) as *mut c_char;
            if p.is_null() {
                ERRNO!() = ENOMEM;
                return -1;
            }
            (*f).flags |= F_SVB;
            p
        } else {
            (*f).flags &= !F_SVB;
            buf
        };
        (*f).buf = actual as *mut u8;
        (*f).buf_size = wanted;
    }
    (*f).lbf = if mode == _IOLBF { b'\n' as c_int } else { -1 };
    0
}

#[no_mangle]
pub unsafe extern "C" fn setbuf(f: *mut FILE, buf: *mut c_char) {
    if buf.is_null() { setvbuf(f, core::ptr::null_mut(), _IONBF, 0); }
    else { setvbuf(f, buf, _IOFBF, BUFSIZ); }
}

#[no_mangle]
pub unsafe extern "C" fn setbuffer(f: *mut FILE, buf: *mut c_char, size: usize) {
    if buf.is_null() { setvbuf(f, core::ptr::null_mut(), _IONBF, 0); }
    else { setvbuf(f, buf, _IOFBF, size); }
}

#[no_mangle]
pub unsafe extern "C" fn fflush(f: *mut FILE) -> c_int {
    if f.is_null() {
        let mut r = 0;
        let mut cur = OFL_HEAD;
        while !cur.is_null() {
            let next = (*cur).next;
            if !(*cur).wpos.is_null() && (*cur).wpos != (*cur).wbase {
                if flush_buf(cur) != 0 { r = -1; }
            }
            cur = next;
        }
        return r;
    }
    if !(*f).wpos.is_null() && (*f).wpos != (*f).wbase {
        return flush_buf(f);
    }
    0
}

// ============================================================
// fwide / feof / ferror / clearerr / fileno
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fwide(_f: *mut FILE, mode: c_int) -> c_int { mode }

#[no_mangle]
pub unsafe extern "C" fn feof(f: *mut FILE) -> c_int { if (*f)._eof != 0 { 1 } else { 0 } }

#[no_mangle]
pub unsafe extern "C" fn ferror(f: *mut FILE) -> c_int { if (*f)._err != 0 { 1 } else { 0 } }

#[no_mangle]
pub unsafe extern "C" fn clearerr(f: *mut FILE) { (*f)._eof = 0; (*f)._err = 0; }

#[no_mangle]
pub unsafe extern "C" fn fileno(f: *mut FILE) -> c_int { (*f).fd }

// ============================================================
// fseek / ftell / rewind / fseeko / ftello / fgetpos / fsetpos
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fseek(stream: *mut FILE, offset: c_long, whence: c_int) -> c_int {
    fseeko(stream, offset as i64, whence)
}

#[no_mangle]
pub unsafe extern "C" fn ftell(stream: *mut FILE) -> c_long {
    ftello(stream) as c_long
}

#[no_mangle]
pub unsafe extern "C" fn rewind(stream: *mut FILE) {
    fseek(stream, 0, SEEK_SET);
    (*stream)._eof = 0;
    (*stream)._err = 0;
}

#[no_mangle]
pub unsafe extern "C" fn fseeko(stream: *mut FILE, offset: i64, whence: c_int) -> c_int {
    if whence != SEEK_SET && whence != SEEK_CUR && whence != SEEK_END {
        ERRNO!() = EINVAL;
        return -1;
    }
    let f = &mut *stream;
    let mut adj_offset = offset;
    if whence == SEEK_CUR {
        if !f.rpos.is_null() {
            let buffered = f.rend as i64 - f.rpos as i64;
            if adj_offset < i64::MIN + buffered {
                ERRNO!() = EOVERFLOW;
                return -1;
            }
            adj_offset -= buffered;
        }
        if adj_offset < i64::MIN + f.ungotten_count as i64 {
            ERRNO!() = EOVERFLOW;
            return -1;
        }
        adj_offset -= f.ungotten_count as i64;
    }
    if !f.wpos.is_null() && f.wpos != f.wbase {
        if flush_buf(stream) != 0 { return -1; }
    }
    f.wpos = core::ptr::null_mut();
    f.wbase = core::ptr::null_mut();
    f.wend = core::ptr::null_mut();
    if let Some(sfunc) = f.seek_fn {
        let r = sfunc(stream, adj_offset, whence);
        if r < 0 { return -1; }
    } else {
        ERRNO!() = EINVAL;
        return -1;
    }
    f.rpos = core::ptr::null_mut();
    f.rend = core::ptr::null_mut();
    f._eof = 0;
    f.ungotten_count = 0;
    0
}

#[no_mangle]
pub unsafe extern "C" fn ftello(stream: *mut FILE) -> i64 {
    let f = &mut *stream;
    if f.flags & F_APP != 0 && !f.wpos.is_null() && f.wpos != f.wbase {
        if fflush(stream) != 0 { return -1; }
    }
    let sfunc = match f.seek_fn { Some(s) => s, None => return -1 };
    let pos = sfunc(stream, 0,
        if (f.flags & F_APP) != 0 && !f.wpos.is_null() && f.wpos != f.wbase { SEEK_END } else { SEEK_CUR });
    if pos < 0 { return -1; }
    let mut pos = pos;
    if !f.rpos.is_null() {
        pos += f.rpos as i64 - f.rend as i64;
    } else if !f.wbase.is_null() {
        let buffered = f.wpos as i64 - f.wbase as i64;
        if buffered > i64::MAX - pos {
            ERRNO!() = EOVERFLOW;
            return -1;
        }
        pos += buffered;
    }
    pos - f.ungotten_count as i64
}

#[no_mangle]
pub unsafe extern "C" fn fgetpos(stream: *mut FILE, pos: *mut i64) -> c_int {
    let off = ftello(stream);
    if off < 0 { return -1; }
    *pos = off;
    0
}

#[no_mangle]
pub unsafe extern "C" fn fsetpos(stream: *mut FILE, pos: *const i64) -> c_int {
    fseeko(stream, *pos, SEEK_SET)
}

// ============================================================
// fgetc / getc / getchar / ungetc
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fgetc(stream: *mut FILE) -> c_int {
    let f = &mut *stream;
    if f.ungotten_count > 0 {
        f.ungotten_count -= 1;
        return f.ungotten[f.ungotten_count as usize];
    }
    if !f.rpos.is_null() && f.rpos < f.rend {
        let c = *f.rpos;
        f.rpos = f.rpos.add(1);
        return c as c_int;
    }
    if f._eof != 0 { return -1; }
    if let Some(rfunc) = f.read_fn {
        // Fill the stdio buffer in one read. Reading one byte per syscall is
        // not only slow, it is wrong for procfs files that only return their
        // contents on the first read: a second one-byte read sees EOF.
        if !f.buf.is_null() && f.buf_size > 1 {
            let n = rfunc(stream, f.buf, f.buf_size);
            if n == 0 {
                if f.flags & F_ERR != 0 { f._err = 1; } else { f._eof = 1; }
                return -1;
            }
            f.rpos = f.buf;
            f.rend = f.buf.add(n);
            let c = *f.rpos;
            f.rpos = f.rpos.add(1);
            return c as c_int;
        }
        let mut buf = [0u8; 1];
        let n = rfunc(stream, buf.as_mut_ptr(), 1);
        if n == 0 {
            if f.flags & F_ERR != 0 { f._err = 1; } else { f._eof = 1; }
            return -1;
        }
        buf[0] as c_int
    } else {
        f._eof = 1;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn getc(stream: *mut FILE) -> c_int { fgetc(stream) }

#[no_mangle]
pub unsafe extern "C" fn getchar() -> c_int { fgetc(stdin) }

#[no_mangle]
pub unsafe extern "C" fn ungetc(c: c_int, stream: *mut FILE) -> c_int {
    if c == -1 { return -1; }
    let f = &mut *stream;
    if f.ungotten_count >= UNGET as c_int { return -1; }
    f.ungotten[f.ungotten_count as usize] = c;
    f.ungotten_count += 1;
    f._eof = 0;
    c
}

// ============================================================
// fgets / fread
// ============================================================

unsafe fn fgets_common(s: *mut c_char, max: usize, stream: *mut FILE, keep_newline: bool) -> *mut c_char {
    let mut i = 0usize;
    while i < max {
        let c = fgetc(stream);
        if c == -1 {
            if i == 0 { return null_mut(); }
            break;
        }
        if c == b'\n' as c_int && !keep_newline { break; }
        *s.add(i) = c as c_char;
        i += 1;
        if c == b'\n' as c_int { break; }
    }
    *s.add(i) = 0;
    s
}

#[no_mangle]
pub unsafe extern "C" fn fgets(s: *mut c_char, n: c_int, stream: *mut FILE) -> *mut c_char {
    if n <= 0 { return null_mut(); }
    fgets_common(s, (n - 1) as usize, stream, true)
}

// gets() was removed from C11 and POSIX.1-2008; it remains an unbounded
// convenience wrapper for legacy callers.
#[no_mangle]
pub unsafe extern "C" fn gets(s: *mut c_char) -> *mut c_char {
    fgets_common(s, usize::MAX, stdin, false)
}

#[no_mangle]
pub unsafe extern "C" fn fopen64(filename: *const c_char, mode: *const c_char) -> *mut FILE {
    fopen(filename, mode)
}

// perror(): minimal implementation printing "msg: errno\n".
const PERROR_BUF_SIZE: usize = 64;
static mut PERROR_BUF: [u8; PERROR_BUF_SIZE] = [0; PERROR_BUF_SIZE];

#[no_mangle]
pub unsafe extern "C" fn perror(msg: *const c_char) {
    let fd = (*stderr).fd;
    if !msg.is_null() && *msg != 0 {
        let len = strlen(msg as *const c_char);
        write_str(fd, msg as *const u8, len);
        write_str(fd, b": ".as_ptr(), 2);
    }
    let e = ERRNO!();
    let text = strerror(e) as *const u8;
    write_str(fd, text, strlen(text as *const c_char));
    write_str(fd, b"\n".as_ptr(), 1);
}

#[no_mangle]
pub unsafe extern "C" fn fread(
    ptr: *mut c_void, size: SizeT, nmemb: SizeT, stream: *mut FILE,
) -> SizeT {
    if size == 0 || nmemb == 0 { return 0; }
    let total = match size.checked_mul(nmemb) {
        Some(v) => v,
        None => { ERRNO!() = EINVAL; return 0; }
    };
    let mut rd = 0usize;
    while rd < total {
        let c = fgetc(stream);
        if c == -1 { break; }
        *(ptr as *mut u8).add(rd) = c as u8;
        rd += 1;
    }
    rd / size
}

// ============================================================
// puts / fputs / fputc / putc / putchar / fwrite
// ============================================================

unsafe fn write_str(fd: c_int, s: *const u8, len: usize) {
    let mut written = 0usize;
    while written < len {
        let n = sys_write(fd as i64, s.add(written), len - written);
        if n <= 0 { break; }
        written += n as usize;
    }
}

#[no_mangle]
pub unsafe extern "C" fn __overflow(f: *mut FILE, c: c_int) -> c_int {
    if (*f).wpos.is_null() {
        (*f).wbase = (*f).buf;
        (*f).wpos = (*f).buf;
        (*f).wend = (*f).buf.add((*f).buf_size);
    }
    if flush_buf(f) == -1 { return -1; }
    if (*f).wpos >= (*f).wend {
        if let Some(wfunc) = (*f).write_fn {
            let byte = c as u8;
            let ret = wfunc(f, &byte as *const u8, 1);
            if ret == 0 { return -1; }
        }
        return c;
    }
    *(*f).wpos = c as u8;
    (*f).wpos = (*f).wpos.add(1);
    if c == (*f).lbf || (*f).wpos >= (*f).wend { let _ = flush_buf(f); }
    c
}

#[no_mangle]
pub unsafe extern "C" fn puts(s: *const c_char) -> c_int {
    let len = strlen(s as *const c_char);
    let _ = fwrite(s as *const c_void, 1, len, stdout);
    let _ = fwrite(b"\n".as_ptr() as *const c_void, 1, 1, stdout);
    let f = &mut *stdout;
    if f.lbf >= 0 { let _ = flush_buf(stdout); }
    (len + 1) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fputs(s: *const c_char, stream: *mut FILE) -> c_int {
    let len = strlen(s as *const c_char);
    fwrite(s as *const c_void, 1, len, stream) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn fputc(c: c_int, stream: *mut FILE) -> c_int {
    let f = &mut *stream;
    if f.wpos.is_null() || f.wpos >= f.wend {
        if __overflow(f, c) == -1 { return -1; }
        return c;
    }
    *f.wpos = c as u8;
    f.wpos = f.wpos.add(1);
    if c == f.lbf || f.wpos >= f.wend { let _ = flush_buf(f); }
    c
}

#[no_mangle]
pub unsafe extern "C" fn putc(c: c_int, stream: *mut FILE) -> c_int { fputc(c, stream) }

#[no_mangle]
pub unsafe extern "C" fn putchar(c: c_int) -> c_int { fputc(c, stdout) }

#[no_mangle]
pub unsafe extern "C" fn fwrite(
    ptr: *const c_void, size: SizeT, nmemb: SizeT, stream: *mut FILE,
) -> SizeT {
    if size == 0 || nmemb == 0 { return 0; }
    let total = match size.checked_mul(nmemb) {
        Some(v) => v,
        None => { ERRNO!() = EINVAL; return 0; }
    };
    let f = &mut *stream;
    if f.buf_size == 0 {
        if let Some(wfunc) = f.write_fn {
            let ret = wfunc(stream, ptr as *const u8, total);
            if ret < total { f._err = 1; f.flags |= F_ERR; }
            return ret / size;
        }
        f._err = 1;
        f.flags |= F_ERR;
        return 0;
    }
    if f.wpos.is_null() {
        f.wbase = f.buf;
        f.wpos = f.buf;
        f.wend = f.buf.add(f.buf_size);
    }
    let mut remaining = total;
    let mut src = ptr as *const u8;
    while remaining > 0 {
        let space = f.wend as usize - f.wpos as usize;
        if space == 0 {
            if flush_buf(f as *mut FILE) == -1 { break; }
            continue;
        }
        let n = if remaining < space { remaining } else { space };
        core::ptr::copy_nonoverlapping(src, f.wpos, n);
        f.wpos = f.wpos.add(n);
        src = src.add(n);
        remaining -= n;
        if f.wpos >= f.wend { let _ = flush_buf(f as *mut FILE); }
    }
    if f.lbf >= 0 { let _ = flush_buf(f as *mut FILE); }
    (total - remaining) / size
}

