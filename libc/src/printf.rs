// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// printf / fprintf / vprintf / vfprintf / dprintf / vdprintf
// ============================================================

unsafe fn wide_to_mb(dst: *mut u8, cap: usize, src: *const wchar_t) -> usize {
    let mut o = 0usize;
    let mut i = 0usize;
    let mut st: c_uint = 0;
    while i < cap && *src.add(i) != 0 {
        let mut tmp = [0u8; 8];
        let n = wcrtomb(tmp.as_mut_ptr() as *mut c_char, *src.add(i), &mut st);
        if n == !0usize || n == 0 { break; }
        if o + n > cap { break; }
        core::ptr::copy_nonoverlapping(tmp.as_ptr(), dst.add(o), n);
        o += n;
        i += 1;
    }
    o
}

const FMT_MAX_WIDTH: usize = 4096;
const FMT_MAX_PRECISION: i32 = 4096;
const FMT_MAX_INT_PRECISION: i32 = 1022;
const FMT_MAX_FLOAT_PRECISION: i32 = 1099;
const FMT_MAX_EG_PRECISION: i32 = 18;
const FMT_INT_BUF: usize = 4224;
const FMT_FLOAT_BUF: usize = 4224;
const FMT_WRITE_BUF: usize = 4096;

#[inline]
fn fmt_width(width: usize) -> usize {
    if width > FMT_MAX_WIDTH { FMT_MAX_WIDTH } else { width }
}

#[inline]
fn fmt_precision(precision: i32) -> i32 {
    if precision > FMT_MAX_PRECISION { FMT_MAX_PRECISION } else { precision }
}

#[inline]
fn fmt_star_width(width: c_int, flags: &mut u8) -> usize {
    if width < 0 {
        *flags |= FLAG_MINUS;
        fmt_width(width.unsigned_abs() as usize)
    } else {
        fmt_width(width as usize)
    }
}

/// Parsed conversion specifier shared by the narrow and wide formatters.
struct FormatSpec {
    flags: u8,
    width: usize,
    precision: i32,
    len1: u8,
    len2: u8,
    spec: u8,
    /// Index of the conversion character; the caller advances past it.
    next: usize,
}

/// Parse flags, width, precision and the length modifier at `i` (the first
/// byte after `%`).  `*` width/precision arguments are consumed from `args`.
unsafe fn parse_format_spec(
    fmt: *const u8,
    mut i: usize,
    args: &mut VaList<'_>,
) -> FormatSpec {
    let mut flags: u8 = 0;
    loop {
        match *fmt.add(i) {
            b'-' => { flags |= FLAG_MINUS; i += 1; }
            b'+' => { flags |= FLAG_PLUS; i += 1; }
            b' ' => { flags |= FLAG_SPACE; i += 1; }
            b'0' => { flags |= FLAG_ZERO; i += 1; }
            b'#' => { flags |= FLAG_HASH; i += 1; }
            _ => break,
        }
    }
    let mut width: usize = 0;
    if *fmt.add(i) == b'*' {
        width = fmt_star_width(args.next_arg::<c_int>(), &mut flags);
        i += 1;
    } else {
        while *fmt.add(i) >= b'0' && *fmt.add(i) <= b'9' {
            width = width.saturating_mul(10).saturating_add((*fmt.add(i) - b'0') as usize);
            i += 1;
        }
    }
    width = fmt_width(width);
    let mut precision: i32 = -1;
    if *fmt.add(i) == b'.' {
        i += 1;
        if *fmt.add(i) == b'*' {
            let p = args.next_arg::<c_int>();
            precision = if p < 0 { -1 } else { fmt_precision(p) };
            i += 1;
        } else {
            precision = 0;
            while *fmt.add(i) >= b'0' && *fmt.add(i) <= b'9' {
                precision = precision.saturating_mul(10).saturating_add((*fmt.add(i) - b'0') as i32);
                i += 1;
            }
            precision = fmt_precision(precision);
        }
    }
    let mut len1 = 0u8;
    let mut len2 = 0u8;
    let c = *fmt.add(i);
    if c == b'h' || c == b'l' || c == b'j' || c == b'z' || c == b't' || c == b'L' {
        len1 = c;
        i += 1;
        let c2 = *fmt.add(i);
        if (c == b'h' && c2 == b'h') || (c == b'l' && c2 == b'l') {
            len2 = c2;
            i += 1;
        }
    }
    FormatSpec { flags, width, precision, len1, len2, spec: *fmt.add(i), next: i }
}

macro_rules! impl_format {
    ($fmt:expr, $args:expr, $write_char:expr, $write_str:expr) => {{
        let fmt = $fmt;
        let args = &mut $args;
        let mut count: usize = 0;
        let mut i = 0usize;
        loop {
            let c = *fmt.add(i) as u8;
            if c == 0 { break; }
            if c != b'%' {
                ($write_char)(c);
                count += 1; i += 1; continue;
            }
            i += 1;
            let fs = parse_format_spec(fmt as *const u8, i, args);
            let flags = fs.flags;
            let width = fs.width;
            let precision = fs.precision;
            let len_mod = fs.len1;
            let spec = fs.spec;
            i = fs.next;
            if fs.len2 != 0 {
                let len_mod2 = fs.len2;
                // Handle double-length modifiers
                match (len_mod, len_mod2, spec) {
                        (b'h', b'h', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut u8) = count as u8; }
                        }
                        (b'l', b'l', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut c_longlong) = count as c_longlong; }
                        }
                        (b'h', b'h', b'd') | (b'h', b'h', b'i') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i8 as i64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'h', b'u') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'h', b'x') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, false, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'h', b'X') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, true, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'h', b'o') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u8 as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'l', b'd') | (b'l', b'l', b'i') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_longlong>() as i64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'l', b'u') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'l', b'o') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'l', b'x') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, false, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'l', b'X') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulonglong>() as u64, true, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        _ => { ($write_char)(b'%'); ($write_char)(len_mod); ($write_char)(len_mod2); ($write_char)(spec); count += 4; }
                    }
                } else if len_mod != 0 {
                    // size_t/ptrdiff_t are 64-bit on all supported targets;
                    // treat %z/%t integer conversions like %l.
                    let lm = match len_mod {
                        b'z' | b't' if spec != b'n' => b'l',
                        // intmax_t is long on every target keel supports.
                        b'j' if spec != b'n' => b'l',
                        _ => len_mod,
                    };
                    match (lm, spec) {
                        (b'h', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut u16) = count as u16; }
                        }
                        (b'l', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut c_long) = count as c_long; }
                        }
                        (b'z', b'n') | (b't', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut usize) = count; }
                        }
                        (b'j', b'n') => {
                            let p = args.next_arg::<*mut c_void>();
                            if !p.is_null() { *(p as *mut c_ulonglong) = count as c_ulonglong; }
                        }
                        (b'h', b'd') | (b'h', b'i') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i16 as i64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'u') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'x') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, false, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'X') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, true, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'h', b'o') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u16 as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'd') | (b'l', b'i') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_long>() as i64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'u') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b's') => {
                            let p = args.next_arg::<*const wchar_t>();
                            if p.is_null() {
                                let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                                let pad = width.saturating_sub(slen);
                                if flags & FLAG_MINUS != 0 { ($write_str)(b"(null)".as_ptr(), slen); for _ in 0..pad { ($write_char)(b' '); } }
                                else { for _ in 0..pad { ($write_char)(b' '); } ($write_str)(b"(null)".as_ptr(), slen); }
                                count += slen + pad;
                            } else {
                                let mut mb = [0u8; 4096];
                                let mut n = wide_to_mb(mb.as_mut_ptr(), mb.len(), p);
                                if precision >= 0 && (precision as usize) < n { n = precision as usize; }
                                let pad = width.saturating_sub(n);
                                if flags & FLAG_MINUS != 0 { ($write_str)(mb.as_ptr(), n); for _ in 0..pad { ($write_char)(b' '); } }
                                else { for _ in 0..pad { ($write_char)(b' '); } ($write_str)(mb.as_ptr(), n); }
                                count += n + pad;
                            }
                        }
                        (b'l', b'c') => {
                            let ch = args.next_arg::<c_int>();
                            let mut mb = [0u8; 8];
                            let mut st: c_uint = 0;
                            let n = wcrtomb(mb.as_mut_ptr() as *mut c_char, ch, &mut st);
                            if n != !0usize {
                                let pad = width.saturating_sub(n);
                                if flags & FLAG_MINUS != 0 {
                                    ($write_str)(mb.as_ptr(), n);
                                    for _ in 0..pad { ($write_char)(b' '); }
                                } else {
                                    for _ in 0..pad { ($write_char)(b' '); }
                                    ($write_str)(mb.as_ptr(), n);
                                }
                                count += n + pad;
                            }
                        }
                        (b'l', b'x') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, false, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'X') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, true, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'o') => {
                            let mut fbuf = [0u8; FMT_INT_BUF];
                            let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_ulong>() as u64, precision, width, flags);
                            ($write_str)(fbuf.as_ptr(), len); count += len;
                        }
                        (b'l', b'f') | (b'l', b'F') | (b'l', b'e') | (b'l', b'E')
                        | (b'l', b'g') | (b'l', b'G') | (b'l', b'a') | (b'l', b'A') => {
                            let mut wbuf = [0u8; FMT_FLOAT_BUF];
                            let total = format_float_into(wbuf.as_mut_ptr(), args.next_arg::<f64>(), spec, precision, width, flags);
                            ($write_str)(wbuf.as_ptr(), total); count += total;
                        }
                        (b'L', b'f') | (b'L', b'F') | (b'L', b'e') | (b'L', b'E')
                        | (b'L', b'g') | (b'L', b'G') | (b'L', b'a') | (b'L', b'A') => {
                            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "powerpc64"))]
                            let val = f128_to_f64(valist_read_f128(args));
                            #[cfg(any(target_arch = "x86_64", target_arch = "loongarch64"))]
                            let val = args.next_arg::<f64>();
                            let mut wbuf = [0u8; FMT_FLOAT_BUF];
                            let total = format_float_into(wbuf.as_mut_ptr(), val, spec, precision, width, flags);
                            ($write_str)(wbuf.as_ptr(), total); count += total;
                        }
                        _ => { ($write_char)(b'%'); ($write_char)(len_mod); ($write_char)(spec); count += 3; }
                    }
                } else {
                match spec {
                    b's' => {
                        let s = args.next_arg::<*const c_char>();
                        if s.is_null() {
                            let slen = if precision >= 0 { (precision as usize).min(6) } else { 6 };
                            let padded_len = if width > slen { width } else { slen };
                            let pad = padded_len - slen;
                            if flags & FLAG_MINUS != 0 {
                                ($write_str)(b"(null)".as_ptr(), slen);
                                for _ in 0..pad { ($write_char)(b' '); }
                            } else {
                                for _ in 0..pad { ($write_char)(b' '); }
                                ($write_str)(b"(null)".as_ptr(), slen);
                            }
                            count += padded_len;
                        } else {
                            let full_len = strlen(s as *const c_char);
                            let slen = if precision >= 0 { (precision as usize).min(full_len) } else { full_len };
                            let padded_len = if width > slen { width } else { slen };
                            let pad = padded_len - slen;
                            if flags & FLAG_MINUS != 0 {
                                ($write_str)(s as *const u8, slen);
                                for _ in 0..pad { ($write_char)(b' '); }
                            } else {
                                for _ in 0..pad { ($write_char)(b' '); }
                                ($write_str)(s as *const u8, slen);
                            }
                            count += padded_len;
                        }
                    }
                    b'd' | b'i' => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_signed_into(fbuf.as_mut_ptr(), args.next_arg::<c_int>() as i64, precision, width, flags);
                        ($write_str)(fbuf.as_ptr(), len); count += len;
                    }
                    b'u' => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_unsigned_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                        ($write_str)(fbuf.as_ptr(), len); count += len;
                    }
                    b'x' => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, false, precision, width, flags);
                        ($write_str)(fbuf.as_ptr(), len); count += len;
                    }
                    b'X' => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_hex_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, true, precision, width, flags);
                        ($write_str)(fbuf.as_ptr(), len); count += len;
                    }
                    b'o' => {
                        let mut fbuf = [0u8; FMT_INT_BUF];
                        let len = format_octal_into(fbuf.as_mut_ptr(), args.next_arg::<c_uint>() as u64, precision, width, flags);
                        ($write_str)(fbuf.as_ptr(), len); count += len;
                    }
                    b'c' => {
                        let ch = args.next_arg::<c_int>();
                        if width > 1 {
                            let pad = width - 1;
                            if flags & FLAG_MINUS != 0 {
                                ($write_char)(ch as u8); count += 1;
                                for _ in 0..pad { ($write_char)(b' '); count += 1; }
                            } else {
                                for _ in 0..pad { ($write_char)(b' '); count += 1; }
                                ($write_char)(ch as u8); count += 1;
                            }
                        } else {
                            ($write_char)(ch as u8); count += 1;
                        }
                    }
                    b'p' => {
                        let p = args.next_arg::<*const c_void>();
                        let mut fbuf = [0u8; 20];
                        fbuf[0] = b'0'; fbuf[1] = b'x';
                        let b = format_hex(p as u64, false);
                        core::ptr::copy_nonoverlapping(b.0.as_ptr(), fbuf.as_mut_ptr().add(2), b.1);
                        ($write_str)(fbuf.as_ptr(), b.1 + 2); count += b.1 + 2;
                    }
                    b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                        let mut wbuf = [0u8; FMT_FLOAT_BUF];
                        let total = format_float_into(wbuf.as_mut_ptr(), args.next_arg::<f64>(), spec, precision, width, flags);
                        ($write_str)(wbuf.as_ptr(), total); count += total;
                    }
                    b'%' => { ($write_char)(b'%'); count += 1; }
                    b'n' => {
                        let p = args.next_arg::<*mut c_void>();
                        if !p.is_null() { *(p as *mut c_int) = count as c_int; }
                    }
                    _ => { ($write_char)(b'%'); ($write_char)(spec); count += 2; }
                }
            }
            i += 1;
        }
        count as c_int
    }};
}

unsafe fn format_u64(mut val: u64) -> ([u8; 20], usize) {
    let mut buf = [0u8; 20];
    if val == 0 { buf[0] = b'0'; return (buf, 1); }
    let mut tmp = [0u8; 20];
    let mut pos = 20;
    while val > 0 { pos -= 1; tmp[pos] = b'0' + (val % 10) as u8; val /= 10; }
    let len = 20 - pos;
    core::ptr::copy_nonoverlapping(tmp.as_ptr().add(pos), buf.as_mut_ptr(), len);
    (buf, len)
}


unsafe fn format_hex(mut val: u64, uppercase: bool) -> ([u8; 16], usize) {
    let digits = if uppercase { b"0123456789ABCDEF" } else { b"0123456789abcdef" };
    let mut buf = [0u8; 16];
    if val == 0 { buf[0] = b'0'; return (buf, 1); }
    let mut tmp = [0u8; 16];
    let mut pos = 16;
    while val > 0 { pos -= 1; tmp[pos] = digits[(val & 0xf) as usize]; val >>= 4; }
    let len = 16 - pos;
    core::ptr::copy_nonoverlapping(tmp.as_ptr().add(pos), buf.as_mut_ptr(), len);
    (buf, len)
}

unsafe fn format_octal(mut val: u64, alt: bool) -> ([u8; 24], usize) {
    let mut buf = [0u8; 24];
    if val == 0 {
        buf[0] = b'0';
        return (buf, 1);
    }
    let mut tmp = [0u8; 24];
    let mut pos = 24;
    while val > 0 {
        pos -= 1;
        tmp[pos] = b'0' + (val & 7) as u8;
        val >>= 3;
    }
    let len = 24 - pos;
    if alt {
        tmp[pos - 1] = b'0';
        core::ptr::copy_nonoverlapping(tmp.as_ptr().add(pos - 1), buf.as_mut_ptr(), len + 1);
        (buf, len + 1)
    } else {
        core::ptr::copy_nonoverlapping(tmp.as_ptr().add(pos), buf.as_mut_ptr(), len);
        (buf, len)
    }
}

unsafe fn format_int(
    dst: *mut u8,
    digits: *const u8,
    digits_len: usize,
    sign: Option<u8>,
    prefix: Option<u8>,
    precision: i32,
    width: usize,
    flags: u8,
    octal_alt: bool,
) -> usize {
    let precision = if precision > FMT_MAX_INT_PRECISION { FMT_MAX_INT_PRECISION } else { precision };
    let num_digits = if precision == 0 && digits_len == 1 && *digits == b'0' && !octal_alt {
        0
    } else {
        digits_len
    };
    let prec = precision as usize;
    let zero_pad = if precision >= 0 && prec > num_digits { prec - num_digits } else { 0 };

    let sign_len = sign.map_or(0, |_| 1);
    let prefix_len = prefix.map_or(0, |_| 2);
    let content_len = sign_len + prefix_len + zero_pad + num_digits;

    let mut buf = [0u8; FMT_INT_BUF];
    let mut pos = 0usize;
    if let Some(c) = sign { buf[pos] = c; pos += 1; }
    if let Some(c) = prefix { buf[pos] = b'0'; pos += 1; buf[pos] = c; pos += 1; }
    for _ in 0..zero_pad { buf[pos] = b'0'; pos += 1; }
    if num_digits > 0 {
        core::ptr::copy_nonoverlapping(digits, buf.as_mut_ptr().add(pos), num_digits);
    }

    if content_len >= width {
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, content_len);
        return content_len;
    }
    let pad = width - content_len;
    if flags & FLAG_MINUS != 0 {
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, content_len);
        for i in 0..pad { *dst.add(content_len + i) = b' '; }
    } else if flags & FLAG_ZERO != 0 && precision < 0 {
        let fixed = sign_len + prefix_len;
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, fixed);
        for i in fixed..width - num_digits { *dst.add(i) = b'0'; }
        if num_digits > 0 {
            core::ptr::copy_nonoverlapping(digits, dst.add(width - num_digits), num_digits);
        }
    } else {
        for i in 0..pad { *dst.add(i) = b' '; }
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst.add(pad), content_len);
    }
    width
}

// Render one integer conversion into `dst` (an FMT_INT_BUF-sized scratch
// buffer) and return its length.  The wide and narrow formatters share the
// same conversion rules; only the final character sink differs.
unsafe fn format_signed_into(dst: *mut u8, val: i64, precision: i32, width: usize, flags: u8) -> usize {
    let neg = val < 0;
    let abs = if neg { val.wrapping_neg() as u64 } else { val as u64 };
    let b = format_u64(abs);
    let sign = if neg { Some(b'-') } else if flags & FLAG_PLUS != 0 { Some(b'+') } else if flags & FLAG_SPACE != 0 { Some(b' ') } else { None };
    format_int(dst, b.0.as_ptr(), b.1, sign, None, precision, width, flags, false)
}

unsafe fn format_unsigned_into(dst: *mut u8, val: u64, precision: i32, width: usize, flags: u8) -> usize {
    let b = format_u64(val);
    format_int(dst, b.0.as_ptr(), b.1, None, None, precision, width, flags, false)
}

unsafe fn format_hex_into(dst: *mut u8, val: u64, upper: bool, precision: i32, width: usize, flags: u8) -> usize {
    let b = format_hex(val, upper);
    let prefix = if flags & FLAG_HASH != 0 && !(b.1 == 1 && b.0[0] == b'0') {
        Some(if upper { b'X' } else { b'x' })
    } else {
        None
    };
    format_int(dst, b.0.as_ptr(), b.1, None, prefix, precision, width, flags, false)
}

unsafe fn format_octal_into(dst: *mut u8, val: u64, precision: i32, width: usize, flags: u8) -> usize {
    let alt = flags & FLAG_HASH != 0;
    let b = format_octal(val, alt);
    format_int(dst, b.0.as_ptr(), b.1, None, None, precision, width, flags, alt)
}

unsafe fn format_float_into(dst: *mut u8, val: f64, spec: u8, precision: i32, width: usize, flags: u8) -> usize {
    let ucase = spec == b'F' || spec == b'E' || spec == b'G' || spec == b'A';
    let ftype = match spec | 0x20 { b'f' => FMT_F, b'e' => FMT_E, b'g' => FMT_G, b'a' => FMT_A, _ => FMT_F };
    let mut fbuf = [0u8; FMT_FLOAT_BUF];
    let flen = format_f64_full(fbuf.as_mut_ptr(), val, ftype, precision, flags, ucase);
    apply_width_flags(dst, fbuf.as_ptr(), flen, width, flags)
}


// Float format specifier type
const FMT_F: u8 = 0;
const FMT_E: u8 = 1;
const FMT_G: u8 = 2;
const FMT_A: u8 = 3;

const FLAG_HASH: u8 = 1;
const FLAG_PLUS: u8 = 2;
const FLAG_SPACE: u8 = 4;
const FLAG_ZERO: u8 = 8;
const FLAG_MINUS: u8 = 16;

// === %f formatting helpers ===

/// Convert (mant << shift) to decimal string. Returns byte count written to buf.
/// mant is a 53-bit significand, shift is the left-shift amount (can be 0..~971).
unsafe fn bigint_to_decimal(buf: *mut u8, mant: u64, shift: u32) -> usize {
    // Store as little-endian u32 limbs. Max bits: 53 + 971 ≈ 1024 → 32 limbs.
    let nlimbs = (2 + (shift + 31) / 32) as usize;
    let mut limbs = [0u32; 36];
    limbs[0] = mant as u32;
    limbs[1] = (mant >> 32) as u32;

    let ws = (shift / 32) as usize;
    let bs = shift % 32;

    // Word-level shift left
    if ws > 0 {
        let mut i = 1usize;
        loop {
            limbs[i + ws] = limbs[i];
            if i == 0 { break; }
            i -= 1;
        }
        for j in 0..ws { limbs[j] = 0; }
    }
    // Bit-level shift left
    if bs > 0 {
        let mut carry = 0u32;
        let end = nlimbs.min(ws + 3);
        for i in ws..end {
            let new_val = (limbs[i] << bs) | carry;
            carry = limbs[i] >> (32 - bs);
            limbs[i] = new_val;
        }
    }

    // Find highest non-zero limb
    let mut used = nlimbs;
    while used > 0 && limbs[used - 1] == 0 { used -= 1; }
    if used == 0 { *buf = b'0'; return 1; }

    // Repeated division by 10 to extract decimal digits
    let mut digits = [0u8; 400];
    let mut nd = 0usize;
    loop {
        let mut all_zero = true;
        for i in 0..used { if limbs[i] != 0 { all_zero = false; break; } }
        if all_zero { break; }
        let mut rem = 0u64;
        for i in (0..used).rev() {
            let val = (rem << 32) | limbs[i] as u64;
            limbs[i] = (val / 10) as u32;
            rem = val % 10;
        }
        digits[nd] = rem as u8;
        nd += 1;
    }
    // Write digits in big-endian order
    for i in 0..nd { *buf.add(i) = b'0' + digits[nd - 1 - i]; }
    nd
}

/// Compute `count` fractional digits of frac_bits / 2^shift using u128 arithmetic.
/// shift must be <= 124. Returns true if any value remains after the last digit.
unsafe fn frac_digits_u128(buf: *mut u8, frac_bits: u64, shift: u32, count: usize) -> bool {
    let mask: u128 = (1u128 << shift) - 1;
    let mut rem = frac_bits as u128;
    for i in 0..count {
        rem *= 10;
        let digit = (rem >> shift) as u8;
        rem &= mask;
        *buf.add(i) = digit;
    }
    rem != 0
}

/// Compute `count` fractional digits of frac_bits / 2^shift using big-integer arithmetic.
/// For shift > 124 (up to ~1074). Returns true if any value remains after the last digit.
unsafe fn frac_digits_bigint(buf: *mut u8, frac_bits: u64, shift: u32, count: usize) -> bool {
    let nlimbs = ((shift + 31) / 32) as usize;
    let mut rem = [0u32; 36]; // enough for shift up to 1088
    rem[0] = frac_bits as u32;
    if nlimbs > 1 { rem[1] = (frac_bits >> 32) as u32; }
    // Mask to exactly `shift` bits
    let hi_word = (shift / 32) as usize;
    let hi_bit = shift % 32;
    if hi_bit > 0 && hi_word < nlimbs {
        rem[hi_word] &= (1u32 << hi_bit) - 1;
    }
    // Zero upper limbs beyond shift
    let clear_from = if hi_bit > 0 { hi_word + 1 } else { hi_word };
    for j in clear_from..nlimbs { rem[j] = 0; }

    for i in 0..count {
        // Multiply remainder by 10
        let mut carry = 0u64;
        for j in 0..nlimbs {
            let val = rem[j] as u64 * 10 + carry;
            rem[j] = val as u32;
            carry = val >> 32;
        }
        // Extract digit from bit position `shift`
        let digit: u32;
        if hi_bit == 0 {
            digit = if hi_word < nlimbs { rem[hi_word] } else { carry as u32 };
        } else {
            let mut d = if hi_word < nlimbs { rem[hi_word] >> hi_bit } else { 0 };
            if hi_word + 1 < nlimbs {
                d |= rem[hi_word + 1] << (32 - hi_bit);
            } else if carry > 0 {
                d |= (carry as u32) << (32 - hi_bit);
            }
            digit = d;
        }
        *buf.add(i) = digit.min(9) as u8;
        // Clear bits from position `shift` upward (keep remainder < 2^shift)
        if hi_bit == 0 {
            if hi_word < nlimbs { rem[hi_word] = 0; }
        } else {
            if hi_word < nlimbs {
                rem[hi_word] &= (1u32 << hi_bit) - 1;
            }
            for j in (hi_word + 1)..nlimbs { rem[j] = 0; }
        }
    }
    // Check if remainder is non-zero
    for i in 0..nlimbs { if rem[i] != 0 { return true; } }
    false
}

/// Increment a decimal string stored at buf[0..*len] in-place.
unsafe fn increment_decimal(buf: *mut u8, len: &mut usize) {
    let mut j = *len;
    loop {
        if j == 0 {
            // Shift right and prepend '1'
            let l = *len;
            for i in (0..l).rev() { *buf.add(i + 1) = *buf.add(i); }
            *buf = b'1';
            *len = l + 1;
            return;
        }
        j -= 1;
        let d = *buf.add(j);
        if d < b'9' { *buf.add(j) = d + 1; return; }
        *buf.add(j) = b'0';
    }
}

// Write float to dst. Returns bytes written. Does NOT handle width/flags padding.
unsafe fn format_f64_full(
    dst: *mut u8, val: f64, fmt_type: u8, precision: i32, flags: u8, uppercase: bool,
) -> usize {
    let precision = match fmt_type {
        FMT_E | FMT_G => precision.min(FMT_MAX_EG_PRECISION),
        _ => precision.min(FMT_MAX_FLOAT_PRECISION),
    };
    let mut pos = 0usize;

    // NaN
    if val.is_nan() {
        let nan_s: &[u8] = if uppercase { b"NAN" } else { b"nan" };
        core::ptr::copy_nonoverlapping(nan_s.as_ptr(), dst, 3);
        return 3;
    }
    let neg = val.is_sign_negative();
    let v = if neg { -val } else { val };

    // Inf
    if v.is_infinite() {
        if neg { *dst.add(pos) = b'-'; pos += 1; }
        else if flags & FLAG_PLUS != 0 { *dst.add(pos) = b'+'; pos += 1; }
        else if flags & FLAG_SPACE != 0 { *dst.add(pos) = b' '; pos += 1; }
        let inf_s: &[u8] = if uppercase { b"INF" } else { b"inf" };
        core::ptr::copy_nonoverlapping(inf_s.as_ptr(), dst.add(pos), 3);
        return pos + 3;
    }

    // Sign
    if neg { *dst.add(pos) = b'-'; pos += 1; }
    else if flags & FLAG_PLUS != 0 { *dst.add(pos) = b'+'; pos += 1; }
    else if flags & FLAG_SPACE != 0 { *dst.add(pos) = b' '; pos += 1; }

    // === Hex float ===
    if fmt_type == FMT_A {
        return pos + fmt_hex_float(dst.add(pos), val, precision, flags, uppercase);
    }

    // === Decimal formats ===
    let p: usize;
    let mut exp: i32;
    let use_f: bool;

    match fmt_type {
        FMT_G => {
            let gp = if precision < 0 { 6usize } else if precision == 0 { 1usize } else { precision as usize };
            exp = if v > 0.0 { compute_exp10(v) } else { 0 };
            use_f = exp >= -4 && exp < gp as i32;
            if use_f {
                p = if exp < 0 { gp + ((-exp) as usize - 1) }
                    else if (exp as usize) < gp { gp - exp as usize - 1 }
                    else { 0 };
            } else {
                p = gp - 1;
            }
        }
        FMT_E => {
            p = if precision < 0 { 6usize } else { precision as usize };
            exp = if v > 0.0 { compute_exp10(v) } else { 0 };
            use_f = false;
        }
        _ => {
            p = if precision < 0 { 6usize } else { precision as usize };
            exp = 0;
            use_f = true;
        }
    }

    if v == 0.0 {
        *dst.add(pos) = b'0'; pos += 1;
        if !(fmt_type == FMT_G && flags & FLAG_HASH == 0) {
            if p > 0 || flags & FLAG_HASH != 0 {
                *dst.add(pos) = b'.'; pos += 1;
                for _ in 0..p { *dst.add(pos) = b'0'; pos += 1; }
            }
        }
        if !use_f {
            let ec = if uppercase { b'E' } else { b'e' };
            *dst.add(pos) = ec; pos += 1;
            *dst.add(pos) = b'+'; pos += 1;
            *dst.add(pos) = b'0'; pos += 1;
            *dst.add(pos) = b'0'; pos += 1;
        }
        return pos;
    }

    // === %f format: exact bit-based arithmetic, returns early ===
    if use_f {
        let bits = v.to_bits();
        let raw_mant = bits & 0x000FFFFFFFFFFFFF;
        let raw_exp_bits = ((bits >> 52) & 0x7FF) as i32;
        let (mant_b, bin_exp) = if raw_exp_bits == 0 {
            (raw_mant, -1022i32)
        } else {
            (raw_mant | (1u64 << 52), raw_exp_bits - 1023)
        };

        // Integer part as decimal string
        let mut ibuf = [0u8; 400];
        let mut ilen: usize;
        if bin_exp < 0 {
            ibuf[0] = b'0'; ilen = 1;
        } else if bin_exp <= 63 {
            let int_val = if bin_exp <= 52 { mant_b >> (52 - bin_exp) }
                          else { mant_b << (bin_exp - 52) };
            let (buf2, len2) = format_u64(int_val);
            ibuf[..len2].copy_from_slice(&buf2[..len2]);
            ilen = len2;
        } else {
            ilen = bigint_to_decimal(ibuf.as_mut_ptr(), mant_b, (bin_exp - 52) as u32);
        }

        // Fractional digits
        let mut fbuf = [0u8; 1100];
        let mut has_rest = false;
        if p > 0 && bin_exp < 52 {
            let shift = (52 - bin_exp) as u32;
            let frac_bits = if bin_exp >= 0 { mant_b & ((1u64 << shift) - 1) } else { mant_b };
            if shift <= 124 {
                has_rest = frac_digits_u128(fbuf.as_mut_ptr(), frac_bits, shift, p + 1);
            } else {
                has_rest = frac_digits_bigint(fbuf.as_mut_ptr(), frac_bits, shift, p + 1);
            }
        }

        // Rounding
        if p > 0 && bin_exp < 52 {
            let next_d = fbuf[p];
            let last_d = fbuf[p - 1];
            if next_d > 5 || (next_d == 5 && (has_rest || last_d & 1 != 0)) {
                let mut carry = true;
                for j in (0..p).rev() {
                    if !carry { break; }
                    fbuf[j] += 1;
                    if fbuf[j] < 10 { carry = false; break; }
                    fbuf[j] = 0;
                }
                if carry { increment_decimal(ibuf.as_mut_ptr(), &mut ilen); }
            }
        } else if p == 0 {
            // %.0f: round integer based on fractional part
            if bin_exp >= 0 && bin_exp < 52 {
                let shift = (52 - bin_exp) as u32;
                let frac_bits = mant_b & ((1u64 << shift) - 1);
                let half = 1u64 << (shift - 1);
                let last_d = ibuf[ilen - 1] - b'0';
                if frac_bits > half || (frac_bits == half && last_d & 1 != 0) {
                    increment_decimal(ibuf.as_mut_ptr(), &mut ilen);
                }
            }
        }

        // Output
        core::ptr::copy_nonoverlapping(ibuf.as_ptr(), dst.add(pos), ilen);
        pos += ilen;
        if p > 0 || flags & FLAG_HASH != 0 {
            *dst.add(pos) = b'.'; pos += 1;
            for i in 0..p { *dst.add(pos) = b'0' + fbuf[i]; pos += 1; }
        }
        if fmt_type == FMT_G && flags & FLAG_HASH == 0 {
            while pos > 0 && *dst.add(pos - 1) == b'0' { pos -= 1; }
            if pos > 0 && *dst.add(pos - 1) == b'.' { pos -= 1; }
        }
        return pos;
    }

    // === %e/%g format: exact big-integer digit extraction ===
    // D = floor(v * 10^(p+1-exp)), using v = mant_b * 2^(bin_exp-52)
    let mut fbuf = [0u8; 20];
    let mut int_part: u64;
    {
        let bits_v = v.to_bits();
        let raw_mant_v = bits_v & 0x000FFFFFFFFFFFFF;
        let raw_exp_v = ((bits_v >> 52) & 0x7FF) as i32;
        let (mant_b_v, bin_exp_v) = if raw_exp_v == 0 {
            (raw_mant_v, -1022i32)
        } else {
            (raw_mant_v | (1u64 << 52), raw_exp_v - 1023)
        };

        let s: i32 = (p as i32) + 1 - exp;
        let bs: i32 = bin_exp_v - 52 + s;

        let mut limbs = [0u32; 40];
        limbs[0] = mant_b_v as u32;
        limbs[1] = (mant_b_v >> 32) as u32;
        let mut nlimbs: usize = if limbs[1] != 0 { 2 } else { 1 };

        if s > 0 {
            for _ in 0..(s as usize) {
                let mut carry = 0u64;
                for j in 0..nlimbs {
                    let val = limbs[j] as u64 * 5 + carry;
                    limbs[j] = val as u32;
                    carry = val >> 32;
                }
                if carry > 0 { limbs[nlimbs] = carry as u32; nlimbs += 1; }
            }
        }

        if bs > 0 {
            let ws = (bs as u32 / 32) as usize;
            let bit = bs as u32 % 32;
            if ws > 0 {
                let mut i = nlimbs;
                while i > 0 { i -= 1; if i + ws < 40 { limbs[i + ws] = limbs[i]; } limbs[i] = 0; }
                nlimbs += ws;
            }
            if bit > 0 {
                let mut carry = 0u32;
                for i in ws..nlimbs {
                    let new_val = (limbs[i] << bit) | carry;
                    carry = limbs[i] >> (32 - bit);
                    limbs[i] = new_val;
                }
                if carry > 0 && nlimbs < 40 { limbs[nlimbs] = carry; nlimbs += 1; }
            }
        }

        let mut has_rest = false;
        if s < 0 {
            for _ in 0..(-s as usize) {
                let mut rem = 0u64;
                for i in (0..nlimbs).rev() {
                    let val = (rem << 32) | limbs[i] as u64;
                    limbs[i] = (val / 5) as u32;
                    rem = val % 5;
                }
                if rem != 0 { has_rest = true; }
            }
        }

        if bs < 0 {
            let rshift = (-bs) as u32;
            let ws = (rshift / 32) as usize;
            let bit = rshift % 32;
            if ws > 0 {
                for j in 0..ws.min(nlimbs) { if limbs[j] != 0 { has_rest = true; } }
                for i in ws..nlimbs { limbs[i - ws] = limbs[i]; }
                nlimbs = nlimbs.saturating_sub(ws);
            }
            if bit > 0 && nlimbs > 0 {
                let mask = (1u32 << bit) - 1;
                if limbs[0] & mask != 0 { has_rest = true; }
                for i in 0..nlimbs - 1 {
                    limbs[i] = (limbs[i] >> bit) | (limbs[i + 1] << (32 - bit));
                }
                limbs[nlimbs - 1] >>= bit;
            }
        }

        while nlimbs > 0 && limbs[nlimbs - 1] == 0 { nlimbs -= 1; }

        let ndigits = p + 2;
        let mut all_digits = [0u8; 25];
        let mut nd = 0usize;
        if nlimbs > 0 {
            loop {
                let mut all_zero = true;
                for i in 0..nlimbs { if limbs[i] != 0 { all_zero = false; break; } }
                if all_zero { break; }
                let mut rem64 = 0u64;
                for i in (0..nlimbs).rev() {
                    let val = (rem64 << 32) | limbs[i] as u64;
                    limbs[i] = (val / 10) as u32;
                    rem64 = val % 10;
                }
                if nd < 25 { all_digits[nd] = rem64 as u8; }
                nd += 1;
            }
        }

        if nd >= ndigits {
            for i in 0..ndigits { fbuf[ndigits - 1 - i] = all_digits[i]; }
        } else {
            for i in 0..nd { fbuf[ndigits - 1 - i] = all_digits[i]; }
            for i in nd..ndigits { fbuf[ndigits - 1 - i] = 0; }
        }

        int_part = fbuf[0] as u64;

        // Round half to even: next_d is the rounding digit beyond precision
        if p > 0 {
            let next_d = fbuf[p + 1];
            let last_d = fbuf[p];
            if next_d > 5 || (next_d == 5 && (has_rest || last_d & 1 != 0)) {
                let mut carry = true;
                for j in (1..=p).rev() {
                    if !carry { break; }
                    let d = fbuf[j] + 1;
                    if d < 10 { fbuf[j] = d; carry = false; break; }
                    fbuf[j] = 0;
                }
                if carry { int_part += 1; }
            }
        } else {
            let next_d = fbuf[1];
            if next_d > 5 || (next_d == 5 && (has_rest || int_part & 1 != 0)) {
                int_part += 1;
            }
        }

        // fbuf[1..=p] are now the fractional digits for output
        // Shift them to fbuf[0..p] so output code can index fbuf[0..p]
        for i in 0..p { fbuf[i] = fbuf[i + 1]; }
    }

    if !use_f && int_part >= 10 {
        exp += 1;
        int_part = 1;
    }

    // Format output
    if use_f {
        // %f style
        let (ibuf, ilen) = format_u64(int_part);
        core::ptr::copy_nonoverlapping(ibuf.as_ptr(), dst.add(pos), ilen);
        pos += ilen;
        if p > 0 || flags & FLAG_HASH != 0 {
            *dst.add(pos) = b'.'; pos += 1;
            for idx in 0..p { *dst.add(pos) = b'0' + fbuf[idx]; pos += 1; }
        }
        if fmt_type == FMT_G && flags & FLAG_HASH == 0 {
            while pos > 0 && *dst.add(pos - 1) == b'0' { pos -= 1; }
            if pos > 0 && *dst.add(pos - 1) == b'.' { pos -= 1; }
        }
    } else {
        // %e style
        *dst.add(pos) = b'0' + int_part as u8; pos += 1;
        if p > 0 || flags & FLAG_HASH != 0 {
            *dst.add(pos) = b'.'; pos += 1;
            for idx in 0..p { *dst.add(pos) = b'0' + fbuf[idx]; pos += 1; }
        }
        if fmt_type == FMT_G && flags & FLAG_HASH == 0 {
            while pos > 0 && *dst.add(pos - 1) == b'0' { pos -= 1; }
            if pos > 0 && *dst.add(pos - 1) == b'.' { pos -= 1; }
        }
        let ec = if uppercase { b'E' } else { b'e' };
        *dst.add(pos) = ec; pos += 1;
        *dst.add(pos) = if exp < 0 { b'-' } else { b'+' }; pos += 1;
        let eabs = if exp < 0 { (-exp) as u32 } else { exp as u32 };
        if eabs < 10 { *dst.add(pos) = b'0'; pos += 1; }
        let (ebuf, elen) = format_u64(eabs as u64);
        core::ptr::copy_nonoverlapping(ebuf.as_ptr(), dst.add(pos), elen);
        pos += elen;
    }

    pos
}

// Compute decimal exponent: floor(log10(v)) with correction
unsafe fn compute_exp10(v: f64) -> i32 {
    let e = libm::floor(libm::log10(v)) as i32;
    let probe = v / libm::pow(10.0, e as f64);
    if probe >= 10.0 { e + 1 }
    else if probe < 1.0 { e - 1 }
    else { e }
}

// Write hex float to dst. Returns bytes written.
unsafe fn fmt_hex_float(dst: *mut u8, val: f64, precision: i32, _flags: u8, uppercase: bool) -> usize {
    // Handle zero (and negative zero) specially
    if val.to_bits() & 0x7FFFFFFFFFFFFFFF == 0 {
        let lc = !uppercase;
        let pfx: &[u8] = if lc { b"0x" } else { b"0X" };
        let p = if precision < 0 { 0usize } else { precision as usize };
        let mut pos = 0usize;
        core::ptr::copy_nonoverlapping(pfx.as_ptr(), dst.add(pos), 2); pos += 2;
        *dst.add(pos) = b'0'; pos += 1;
        if p > 0 {
            *dst.add(pos) = b'.'; pos += 1;
            for _ in 0..p { *dst.add(pos) = b'0'; pos += 1; }
        }
        let pc = if lc { b'p' } else { b'P' };
        *dst.add(pos) = pc; pos += 1;
        *dst.add(pos) = b'+'; pos += 1;
        *dst.add(pos) = b'0'; pos += 1;
        return pos;
    }
    let bits = val.to_bits();
    let exp_bits = ((bits >> 52) & 0x7FF) as i32;
    let mant_bits = bits & 0x000FFFFFFFFFFFFF;
    let (mut e, mut m) = if exp_bits == 0 { (1 - 1023, mant_bits) } else { (exp_bits - 1023, mant_bits | (1u64 << 52)) };
    // musl normalizes subnormals so the leading digit is always 1, which
    // lets the exponent go below -1022 (e.g. 0x1p-1074 for the smallest).
    if exp_bits == 0 && m != 0 {
        let shift = m.leading_zeros() as i32 - 11;
        m <<= shift;
        e -= shift;
    }
    let explicit = precision >= 0;
    let p = if explicit { precision as usize } else { 13usize };
    let mut pos = 0usize;
    let lc = !uppercase;
    let pfx: &[u8] = if lc { b"0x" } else { b"0X" };
    core::ptr::copy_nonoverlapping(pfx.as_ptr(), dst.add(pos), 2); pos += 2;

    // Leading digit from bit 52 (1 for every normalized value).
    let lead_pos = pos;
    let mut lead = ((m >> 52) & 0xF) as u8;
    *dst.add(pos) = hex_digit_char(lead, lc); pos += 1;
    let mut frac_start = pos;
    if p > 0 {
        *dst.add(pos) = b'.'; pos += 1;
        frac_start = pos;
    }

    // Extract p hex digits from the fraction, most significant nibble first.
    let mut rm = m;
    for _ in 0..p {
        let nib = ((rm >> 48) & 0xF) as u8;
        *dst.add(pos) = hex_digit_char(nib, lc); pos += 1;
        rm <<= 4;
    }

    // Round to nearest, ties to even, based on the next nibble.
    let round_nib = ((rm >> 48) & 0xF) as u8;
    let rest = rm & 0x0000FFFFFFFFFFFF;
    let last_nib = if p > 0 {
        hex_val(*dst.add(pos - 1)).unwrap_or(0)
    } else {
        lead
    };
    let should_round = round_nib > 8 || (round_nib == 8 && (rest != 0 || (last_nib & 1) != 0));
    if should_round {
        let mut j = pos;
        loop {
            if j == frac_start {
                // Carry out of the fraction into the leading digit.
                lead += 1;
                *dst.add(lead_pos) = hex_digit_char(lead, lc);
                break;
            }
            j -= 1;
            let v = hex_val(*dst.add(j)).unwrap_or(0);
            if v < 15 {
                *dst.add(j) = hex_digit_char(v + 1, lc);
                break;
            }
            *dst.add(j) = b'0';
        }
    }

    // With no explicit precision, print the shortest exact representation:
    // trim trailing fractional zeros and drop the radix point if empty.
    if !explicit {
        while pos > frac_start && *dst.add(pos - 1) == b'0' {
            pos -= 1;
        }
        if pos == frac_start {
            pos = frac_start - 1;
        }
    }

    // Exponent
    let pc = if lc { b'p' } else { b'P' };
    *dst.add(pos) = pc; pos += 1;
    *dst.add(pos) = if e < 0 { b'-' } else { b'+' }; pos += 1;
    let eabs = if e < 0 { (-e) as u32 } else { e as u32 };
    let (ebuf, elen) = format_u64(eabs as u64);
    core::ptr::copy_nonoverlapping(ebuf.as_ptr(), dst.add(pos), elen);
    pos += elen;
    pos
}

#[inline]
fn hex_digit_char(v: u8, lowercase: bool) -> u8 {
    if v < 10 { b'0' + v }
    else if lowercase { b'a' + v - 10 }
    else { b'A' + v - 10 }
}

#[inline]

// Apply width and flags padding to a formatted string in buf[0..len].
// Writes the final result to dst. Returns total bytes written.
unsafe fn apply_width_flags(
    dst: *mut u8, buf: *const u8, len: usize, width: usize, flags: u8,
) -> usize {
    if len >= width {
        core::ptr::copy_nonoverlapping(buf, dst, len);
        return len;
    }
    let pad = width - len;
    let pad_char = if flags & FLAG_ZERO != 0 && flags & FLAG_MINUS == 0 { b'0' } else { b' ' };
    if flags & FLAG_MINUS != 0 {
        // Left align
        core::ptr::copy_nonoverlapping(buf, dst, len);
        for i in 0..pad { *dst.add(len + i) = pad_char; }
    } else {
        // Right align
        // For zero-padded numbers, sign goes before zeros
        if flags & FLAG_ZERO != 0 && len > 0 {
            let first = *buf;
            if first == b'-' || first == b'+' || first == b' ' {
                *dst = first;
                for i in 0..pad { *dst.add(1 + i) = pad_char; }
                core::ptr::copy_nonoverlapping(buf.add(1), dst.add(1 + pad), len - 1);
                return width;
            }
        }
        for i in 0..pad { *dst.add(i) = pad_char; }
        core::ptr::copy_nonoverlapping(buf, dst.add(pad), len);
    }
    width
}

#[no_mangle]
pub unsafe extern "C" fn vprintf(fmt: *const c_char, mut args: VaList) -> c_int {
    impl_format!(fmt, args,
        |c: u8| { let _ = fwrite(&c as *const u8 as *const c_void, 1, 1, stdout); },
        |s: *const u8, len: usize| { let _ = fwrite(s as *const c_void, 1, len, stdout); }
    )
}

#[no_mangle]
pub unsafe extern "C" fn vfprintf(stream: *mut FILE, fmt: *const c_char, mut args: VaList) -> c_int {
    impl_format!(fmt, args,
        |c: u8| { let _ = fwrite(&c as *const u8 as *const c_void, 1, 1, stream); },
        |s: *const u8, len: usize| { let _ = fwrite(s as *const c_void, 1, len, stream); }
    )
}

#[no_mangle]
pub unsafe extern "C" fn printf(fmt: *const c_char, mut args: ...) -> c_int {
    impl_format!(fmt, args,
        |c: u8| { let _ = fwrite(&c as *const u8 as *const c_void, 1, 1, stdout); },
        |s: *const u8, len: usize| { let _ = fwrite(s as *const c_void, 1, len, stdout); }
    )
}

#[no_mangle]
pub unsafe extern "C" fn fprintf(stream: *mut FILE, fmt: *const c_char, mut args: ...) -> c_int {
    impl_format!(fmt, args,
        |c: u8| { let _ = fwrite(&c as *const u8 as *const c_void, 1, 1, stream); },
        |s: *const u8, len: usize| { let _ = fwrite(s as *const c_void, 1, len, stream); }
    )
}

unsafe fn format_write_fd(fd: c_int, fmt: *const c_char, args: &mut VaList<'_>) -> c_int {
    // cgroupfs (and some other pseudo-filesystems) parse one command per
    // write() call, so the whole formatted string must go out at once.
    let mut stack = [0u8; FMT_WRITE_BUF];
    let mut a1 = args.clone();
    let n = format_to_buf(stack.as_mut_ptr(), stack.len(), fmt, &mut a1);
    if n < 0 { return -1; }
    if (n as usize) < stack.len() {
        let w = crate::write(fd, stack.as_ptr() as *const c_void, n as usize);
        if w < 0 { return -1; }
        return n;
    }
    let heap = crate::malloc(n as usize + 1) as *mut u8;
    if heap.is_null() { return -1; }
    let mut a2 = args.clone();
    let n2 = format_to_buf(heap, n as usize + 1, fmt, &mut a2);
    if n2 < 0 { crate::free(heap as *mut c_void); return -1; }
    let w = crate::write(fd, heap as *const c_void, n2 as usize);
    crate::free(heap as *mut c_void);
    if w < 0 { return -1; }
    n2
}

#[no_mangle]
pub unsafe extern "C" fn dprintf(fd: c_int, fmt: *const c_char, mut args: ...) -> c_int {
    format_write_fd(fd, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn vdprintf(fd: c_int, fmt: *const c_char, mut args: VaList) -> c_int {
    format_write_fd(fd, fmt, &mut args)
}

// ============================================================
// sprintf / snprintf / vsprintf / vsnprintf
// ============================================================

unsafe fn format_to_buf(buf: *mut u8, cap: usize, fmt: *const c_char, args: &mut VaList<'_>) -> c_int {
    // Reuse the shared format engine with buffer writer closures instead of
    // maintaining a second copy of the conversion table.
    let pos = core::cell::Cell::new(0usize);
    let write_char = |c: u8| {
        let p = pos.get();
        if p < cap { *buf.add(p) = c; }
        pos.set(p + 1);
    };
    let write_str = |s: *const u8, len: usize| {
        let p = pos.get();
        for k in 0..len {
            if p + k < cap { *buf.add(p + k) = *s.add(k); }
        }
        pos.set(p + len);
    };
    let count = impl_format!(fmt, *args, write_char, write_str);
    if cap > 0 {
        let end = core::cmp::min(count as usize, cap - 1);
        *buf.add(end) = 0;
    }
    count
}

#[no_mangle]
pub unsafe extern "C" fn vsprintf(buf: *mut c_char, fmt: *const c_char, mut args: VaList) -> c_int {
    format_to_buf(buf as *mut u8, usize::MAX, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn vsnprintf(buf: *mut c_char, size: usize, fmt: *const c_char, mut args: VaList) -> c_int {
    format_to_buf(buf as *mut u8, size, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn sprintf(buf: *mut c_char, fmt: *const c_char, mut args: ...) -> c_int {
    format_to_buf(buf as *mut u8, usize::MAX, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, mut args: ...) -> c_int {
    format_to_buf(buf as *mut u8, size, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn vasprintf(strp: *mut *mut c_char, fmt: *const c_char, ap: VaList) -> c_int {
    if strp.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let len = vsnprintf(core::ptr::null_mut(), 0, fmt, ap.clone());
    if len < 0 {
        *strp = core::ptr::null_mut();
        return -1;
    }
    let p = malloc((len as usize) + 1) as *mut c_char;
    if p.is_null() {
        *strp = core::ptr::null_mut();
        return -1;
    }
    let ret = vsnprintf(p, (len as usize) + 1, fmt, ap.clone());
    if ret < 0 {
        free(p as *mut c_void);
        *strp = core::ptr::null_mut();
        return -1;
    }
    *strp = p;
    ret
}

#[no_mangle]
pub unsafe extern "C" fn asprintf(strp: *mut *mut c_char, fmt: *const c_char, args: ...) -> c_int {
    vasprintf(strp, fmt, args)
}

