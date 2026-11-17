// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// wchar: wide stdio
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn fgetwc(f: *mut FILE) -> wint_t {
    let mut state: c_uint = 0;
    let mut wc: wchar_t = 0;
    let mut buf = [0u8; 4];
    let c = fgetc(f);
    if c == -1 { return WEOF; }
    buf[0] = c as u8;
    if buf[0] < 0x80 { return buf[0] as wint_t; }
    let len = if buf[0] < 0xE0 { 2 } else if buf[0] < 0xF0 { 3 } else { 4 };
    let mut n = 1usize;
    while n < len {
        let c2 = fgetc(f);
        if c2 == -1 {
            // Put the partial sequence back so a later read sees it.
            while n > 1 {
                n -= 1;
                ungetc(buf[n] as c_int, f);
            }
            return WEOF;
        }
        buf[n] = c2 as u8;
        n += 1;
    }
    let r = mbrtowc(&mut wc, buf.as_ptr() as *const c_char, n, &mut state);
    if r == !0usize {
        ERRNO!() = EILSEQ;
        WEOF
    } else if r == !1usize {
        while n > 0 {
            n -= 1;
            ungetc(buf[n] as c_int, f);
        }
        WEOF
    } else {
        wc as wint_t
    }
}

#[no_mangle]
pub unsafe extern "C" fn fputwc(c: wchar_t, f: *mut FILE) -> wint_t {
    let mut buf = [0u8; 4];
    let n = wcrtomb(buf.as_mut_ptr() as *mut c_char, c, core::ptr::null_mut());
    if n == !0usize || n == 0 { return WEOF; }
    if fwrite(buf.as_ptr() as *const c_void, 1, n, f) != n { return WEOF; }
    c as wint_t
}

#[no_mangle]
pub unsafe extern "C" fn fputws(s: *const wchar_t, f: *mut FILE) -> c_int {
    let mut i = 0;
    while *s.add(i) != 0 {
        if fputwc(*s.add(i), f) == WEOF { return -1; }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn ungetwc(c: wint_t, f: *mut FILE) -> wint_t {
    if c == WEOF { return WEOF; }
    let mut buf = [0u8; 4];
    let n = wcrtomb(buf.as_mut_ptr() as *mut c_char, c as wchar_t, core::ptr::null_mut());
    if n == !0usize || n == 0 { return WEOF; }
    let mut i = n;
    while i > 0 {
        i -= 1;
        if ungetc(buf[i] as c_int, f) == -1 { return WEOF; }
    }
    c
}

#[no_mangle]
pub unsafe extern "C" fn getwchar() -> wint_t { fgetwc(stdin) }

#[no_mangle]
pub unsafe extern "C" fn putwchar(c: wchar_t) -> wint_t { fputwc(c, stdout) }

// ============================================================
// wchar: swprintf / vswprintf / fwprintf / vfwprintf
// ============================================================

unsafe fn wfmt_write_str(dst: *mut wchar_t, pos: usize, cap: usize, s: *const u8, len: usize) -> usize {
    let mut i = 0;
    while i < len {
        if pos + i < cap { *dst.add(pos + i) = *s.add(i) as wchar_t; }
        i += 1;
    }
    pos + len
}

struct HeapGuard(*mut u8);

impl Drop for HeapGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { crate::free(self.0 as *mut c_void); }
        }
    }
}

#[no_mangle]
unsafe fn vswprintf_impl(
    s: *mut *mut wchar_t,
    n: usize,
    fmt: *const wchar_t,
    mut args: VaList,
    grow: bool,
) -> c_int {
    if !grow && n == 0 { return -1; }
    let mut buf = *s;
    let mut cap = if grow { n } else { n - 1 };
    let mut pos = 0usize;
    // Format specifiers are ASCII. Keep one byte per wide character so a
    // non-ASCII literal can still be emitted from the original string.
    let fmt_len = wcslen(fmt);
    let fmt_cap = fmt_len.saturating_add(1);
    let fmt_guard = HeapGuard(crate::malloc(fmt_cap) as *mut u8);
    let fmt_narrow = fmt_guard.0;
    if fmt_narrow.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(fmt_narrow, fmt_cap, fmt);
    let mut fi = 0usize;
    macro_rules! wc {
        ($c:expr) => {
            if grow && pos + 1 >= cap {
                let newcap = cap.saturating_mul(2).max(pos + 2);
                let grown = crate::realloc(buf as *mut c_void, newcap * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
                if grown.is_null() { return -1; }
                buf = grown;
                cap = newcap;
            }
            if pos < cap { *buf.add(pos) = $c as wchar_t; }
            pos += 1;
        }
    }
    macro_rules! wemit {
        ($buf:expr, $len:expr) => {
            for k in 0..$len { wc!($buf[k]); }
        };
    }
    loop {
        let c = *fmt_narrow.add(fi);
        if c == 0 { break; }
        if c != b'%' {
            wc!(*fmt.add(fi) as u32);
            fi += 1;
            continue;
        }
        fi += 1;
        let fs = parse_format_spec(fmt_narrow, fi, &mut args);
        let flags = fs.flags;
        let width = fs.width;
        let precision = fs.precision;
        let len_mod = fs.len1;
        fi = fs.next;
        if fs.len2 != 0 {
            let spec = fs.len2;
            let spec2 = fs.spec;
            match (len_mod, spec, spec2) {
                    (b'h', b'h', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut u8) = pos as u8; } }
                    (b'l', b'l', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_longlong) = pos as c_longlong; } }
                    (b'h', b'h', b'd') | (b'h', b'h', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i8 as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'h', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'd') | (b'l', b'l', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_longlong>() as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'l', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    _ => { wc!(b'%'); wc!(len_mod); wc!(spec); wc!(spec2); }
                }
            } else if len_mod != 0 {
                let spec = fs.spec;
                let lm = match len_mod {
                        b'z' | b't' if spec != b'n' => b'l',
                        // intmax_t is long on every target keel supports.
                        b'j' if spec != b'n' => b'l',
                        _ => len_mod,
                    };
                match (lm, spec) {
                    (b'h', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut u16) = pos as u16; } }
                    (b'l', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_long) = pos as c_long; } }
                    (b'z', b'n') | (b't', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut usize) = pos; } }
                    (b'j', b'n') => { let p = args.next_arg::<*mut c_void>(); if !p.is_null() { *(p as *mut c_ulonglong) = pos as c_ulonglong; } }
                    (b'h', b'd') | (b'h', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i16 as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'h', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'd') | (b'l', b'i') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_long>() as i64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'u') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'x') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, false, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'X') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, true, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'o') => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                        wemit!(fbuf, len);
                    },
                    (b'l', b'c') => {
                        let ch = args.next_arg::<wint_t>();
                        let pad = width.saturating_sub(1);
                        if flags & FLAG_MINUS != 0 {
                            wc!(ch);
                            for _ in 0..pad { wc!(b' '); }
                        } else {
                            for _ in 0..pad { wc!(b' '); }
                            wc!(ch);
                        }
                    }
                    (b'l', b's') => {
                        let ws_arg = args.next_arg::<*const wchar_t>();
                        if ws_arg.is_null() {
                            let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                            if flags & FLAG_MINUS != 0 {
                                for k in 0..slen { wc!(b"(null)"[k]); }
                                for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                            } else {
                                for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                                for k in 0..slen { wc!(b"(null)"[k]); }
                            }
                        } else {
                            let mut wlen = 0usize;
                            loop {
                                let ch = *ws_arg.add(wlen);
                                if ch == 0 { break; }
                                if precision >= 0 && wlen >= precision as usize { break; }
                                wlen += 1;
                            }
                            if flags & FLAG_MINUS != 0 {
                                for k in 0..wlen { wc!(*ws_arg.add(k)); }
                                for _ in 0..width.saturating_sub(wlen) { wc!(b' '); }
                            } else {
                                for _ in 0..width.saturating_sub(wlen) { wc!(b' '); }
                                for k in 0..wlen { wc!(*ws_arg.add(k)); }
                            }
                        }
                    }
                    (b'l', b'f') | (b'l', b'F') | (b'l', b'e') | (b'l', b'E')
                    | (b'l', b'g') | (b'l', b'G') | (b'l', b'a') | (b'l', b'A') => {
                        let mut wbuf = [0u8; FMT_FLOAT_BUF];
                        let wlen = format_float_into(wbuf.as_mut_ptr(), args.next_arg::<f64>(), spec, precision, width, flags);
                        wemit!(wbuf, wlen);
                    },
                    (b'L', b'f') | (b'L', b'F') | (b'L', b'e') | (b'L', b'E')
                    | (b'L', b'g') | (b'L', b'G') | (b'L', b'a') | (b'L', b'A') => {
                        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "powerpc64"))]
                        let val = f128_to_f64(valist_read_f128(&mut args));
                        #[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
                        let val = args.next_arg::<f64>();
                        let mut wbuf = [0u8; FMT_FLOAT_BUF];
                        let wlen = format_float_into(wbuf.as_mut_ptr(), val, spec, precision, width, flags);
                        wemit!(wbuf, wlen);
                    },
                    _ => { wc!(b'%'); wc!(len_mod); wc!(spec); }
                }
            } else {
            let spec = fs.spec;
            match spec {
                b'd' | b'i' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'u' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'x' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, false, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'X' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, true, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'o' => {
                    let mut fbuf = [0u8; FMT_INT_BUF];
                    let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                    wemit!(fbuf, len);
                },
                b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                    let val = args.next_arg::<f64>();
                    let ucase = spec == b'F' || spec == b'E' || spec == b'G' || spec == b'A';
                    let ftype = match spec | 0x20 { b'f' => FMT_F, b'e' => FMT_E, b'g' => FMT_G, b'a' => FMT_A, _ => FMT_F };
                    let mut fbuf = [0u8; FMT_FLOAT_BUF];
                    let flen = format_f64_full(fbuf.as_mut_ptr(), val, ftype, precision, flags, ucase);
                    let mut wbuf = [0u8; FMT_FLOAT_BUF];
                    let wlen = apply_width_flags(wbuf.as_mut_ptr(), fbuf.as_ptr(), flen, width, flags);
                    for k in 0..wlen { wc!(wbuf[k]); }
                }
                b'c' => {
                    let ch = args.next_arg::<c_int>();
                    wc!(ch as u8);
                }
                b's' => {
                    let p = args.next_arg::<*const c_char>();
                    if p.is_null() {
                        let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                        if flags & FLAG_MINUS != 0 {
                            for k in 0..slen { wc!(b"(null)"[k]); }
                            for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                        } else {
                            for _ in 0..width.saturating_sub(slen) { wc!(b' '); }
                            for k in 0..slen { wc!(b"(null)"[k]); }
                        }
                    } else {
                        let mut wbuf_local = [0i32; 256];
                        let mut wcount = 0usize;
                        let mut byte_pos = 0usize;
                        let pbytes = p as *const u8;
                        loop {
                            if precision >= 0 && wcount >= precision as usize { break; }
                            if wcount >= 256 { break; }
                            let mut wc_val: c_int = 0;
                            let consumed = mbrtowc(&mut wc_val as *mut c_int, p.add(byte_pos) as *const c_char, 256, core::ptr::null_mut());
                            if consumed == !0usize || consumed == !1usize {
                                wbuf_local[wcount] = *pbytes.add(byte_pos) as c_int;
                                byte_pos += 1;
                                wcount += 1;
                            } else if consumed == 0 {
                                break;
                            } else {
                                wbuf_local[wcount] = wc_val;
                                byte_pos += consumed;
                                wcount += 1;
                            }
                        }
                        if flags & FLAG_MINUS != 0 {
                            for k in 0..wcount { wc!(wbuf_local[k]); }
                            for _ in 0..width.saturating_sub(wcount) { wc!(b' '); }
                        } else {
                            for _ in 0..width.saturating_sub(wcount) { wc!(b' '); }
                            for k in 0..wcount { wc!(wbuf_local[k]); }
                        }
                    }
                }
                b'p' => {
                    wc!(b'0'); wc!(b'x');
                    let p = args.next_arg::<*const c_void>();
                    let b = format_hex(p as u64, false);
                    for k in 0..b.1 { wc!(b.0[k]); }
                }
                b'n' => {
                    let p = args.next_arg::<*mut c_void>();
                    if !p.is_null() { *(p as *mut c_int) = pos as c_int; }
                }
                b'%' => { wc!(b'%'); }
                _ => { wc!(b'%'); wc!(spec); }
            }
        }
        fi += 1;
    }
    let null_pos = if pos < cap { pos } else { cap };
    *buf.add(null_pos) = 0;
    if grow { *s = buf; }
    if pos > cap { -1 } else { pos as c_int }
}

#[no_mangle]
pub unsafe extern "C" fn vswprintf(s: *mut wchar_t, n: usize, fmt: *const wchar_t, args: VaList) -> c_int {
    let mut buf = s;
    vswprintf_impl(&mut buf, n, fmt, args, false)
}

#[no_mangle]
pub unsafe extern "C" fn swprintf(s: *mut wchar_t, n: usize, fmt: *const wchar_t, args: ...) -> c_int {
    vswprintf(s, n, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vfwprintf(f: *mut FILE, fmt: *const wchar_t, args: VaList) -> c_int {
    // Grow the wide staging buffer instead of failing when the formatted
    // output exceeds a fixed size.
    let cap = 512usize;
    let mut buf = crate::malloc(cap * core::mem::size_of::<wchar_t>()) as *mut wchar_t;
    if buf.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    let r = vswprintf_impl(&mut buf, cap, fmt, args, true);
    if r < 0 {
        crate::free(buf as *mut c_void);
        return r;
    }
    let mut i = 0;
    while i < r as usize {
        let mut mb = [0u8; 4];
        let n = wcrtomb(mb.as_mut_ptr() as *mut c_char, *buf.add(i), core::ptr::null_mut());
        if n == !0usize {
            crate::free(buf as *mut c_void);
            return -1;
        }
        if fwrite(mb.as_ptr() as *const c_void, 1, n, f) != n {
            crate::free(buf as *mut c_void);
            return -1;
        }
        i += 1;
    }
    crate::free(buf as *mut c_void);
    r
}

#[no_mangle]
pub unsafe extern "C" fn fwprintf(f: *mut FILE, fmt: *const wchar_t, args: ...) -> c_int {
    vfwprintf(f, fmt, args)
}

// ============================================================
// wchar: vswscanf / swscanf / vfwscanf / fwscanf / vwscanf / wscanf
// ============================================================

// Convert wchar_t format to byte format (ASCII cast, format specifiers are ASCII)
unsafe fn wcsfmt_to_mbs(dst: *mut u8, dst_len: usize, src: *const wchar_t) {
    let mut i = 0usize;
    loop {
        let ch = *src.add(i) as u32;
        if ch == 0 || i >= dst_len - 1 { break; }
        *dst.add(i) = if ch < 128 { ch as u8 } else { b'?' };
        i += 1;
    }
    *dst.add(i) = 0;
}

#[no_mangle]
pub unsafe extern "C" fn vswscanf(s: *const wchar_t, fmt: *const wchar_t, mut args: VaList) -> c_int {
    if s.is_null() || fmt.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let wlen = wcslen(s);
    let mut cap = wlen.saturating_mul(4).saturating_add(1);
    let mut mbs_buf = crate::malloc(cap) as *mut u8;
    if mbs_buf.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    let mut i = 0usize;
    let mut mb_pos = 0usize;
    loop {
        let wc = *s.add(i) as c_int;
        if wc == 0 { break; }
        let mut mb = [0u8; 4];
        let n = wcrtomb(mb.as_mut_ptr() as *mut c_char, wc, core::ptr::null_mut());
        if n == !0usize || n == 0 { break; }
        if mb_pos + n >= cap {
            let next = match cap.checked_mul(2) {
                Some(v) => v,
                None => { crate::free(mbs_buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            };
            let grown = crate::realloc(mbs_buf as *mut c_void, next) as *mut u8;
            if grown.is_null() { crate::free(mbs_buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            mbs_buf = grown;
            cap = next;
        }
        core::ptr::copy_nonoverlapping(mb.as_ptr(), mbs_buf.add(mb_pos), n);
        mb_pos += n;
        i += 1;
    }
    *mbs_buf.add(mb_pos) = 0;
    let flen = wcslen(fmt);
    let fcap = flen.saturating_add(1);
    let mbs_fmt = crate::malloc(fcap) as *mut u8;
    if mbs_fmt.is_null() {
        crate::free(mbs_buf as *mut c_void);
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(mbs_fmt, fcap, fmt);
    let r = do_vsscanf(mbs_buf, mb_pos, mbs_fmt as *const c_char, &mut args, core::ptr::null_mut());
    crate::free(mbs_fmt as *mut c_void);
    crate::free(mbs_buf as *mut c_void);
    r
}

#[no_mangle]
pub unsafe extern "C" fn swscanf(s: *const wchar_t, fmt: *const wchar_t, args: ...) -> c_int {
    vswscanf(s, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vfwscanf(f: *mut FILE, fmt: *const wchar_t, mut args: VaList) -> c_int {
    if f.is_null() || fmt.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let start_pos = ftello(f);
    if start_pos < 0 { return -1; }
    let mut cap = SCAN_BUF_INIT;
    let mut buf = crate::malloc(cap) as *mut u8;
    if buf.is_null() { ERRNO!() = ENOMEM; return -1; }
    let mut n = 0usize;
    loop {
        if n + 1 >= cap {
            let next = match cap.checked_mul(2) {
                Some(v) => v,
                None => { crate::free(buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            };
            let grown = crate::realloc(buf as *mut c_void, next) as *mut u8;
            if grown.is_null() { crate::free(buf as *mut c_void); ERRNO!() = ENOMEM; return -1; }
            buf = grown;
            cap = next;
        }
        let c = fgetc(f);
        if c == -1 { break; }
        *buf.add(n) = c as u8;
        n += 1;
    }
    *buf.add(n) = 0;
    let flen = wcslen(fmt);
    let fcap = flen.saturating_add(1);
    let mbs_fmt = crate::malloc(fcap) as *mut u8;
    if mbs_fmt.is_null() {
        crate::free(buf as *mut c_void);
        ERRNO!() = ENOMEM;
        return -1;
    }
    wcsfmt_to_mbs(mbs_fmt, fcap, fmt);
    let mut consumed = 0usize;
    let hit_eof = (*f)._eof != 0;
    let result = do_vsscanf(buf, n, mbs_fmt as *const c_char, &mut args, &mut consumed);
    crate::free(mbs_fmt as *mut c_void);
    crate::free(buf as *mut c_void);
    let _ = fseeko(f, start_pos + consumed as i64, SEEK_SET);
    if hit_eof && consumed == n {
        (*f)._eof = 1;
    }
    result
}

#[no_mangle]
pub unsafe extern "C" fn fwscanf(f: *mut FILE, fmt: *const wchar_t, args: ...) -> c_int {
    vfwscanf(f, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn vwscanf(fmt: *const wchar_t, args: VaList) -> c_int {
    vfwscanf(stdin, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn wscanf(fmt: *const wchar_t, args: ...) -> c_int {
    vfwscanf(stdin, fmt, args)
}

