// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// scanf / vscanf / fscanf / vfscanf / sscanf / vsscanf
// ============================================================


#[inline]
fn is_ws_byte(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c
}

// Scan integer with position tracking. Returns (value, is_negative, success).
// base: 0=auto, 8=octal, 10=decimal, 16=hex
unsafe fn scan_int_val(
    buf: *const u8, pos: &mut usize, buf_len: usize, base: u32, width: usize,
) -> (u64, bool, bool) {
    let start = *pos;
    let max = if width > 0 { (start + width).min(buf_len) } else { buf_len };
    let neg = if *pos < max && *buf.add(*pos) == b'-' { *pos += 1; true }
    else if *pos < max && *buf.add(*pos) == b'+' { *pos += 1; false }
    else { false };
    if *pos >= max { *pos = start; return (0, false, false); }

    let mut base = base;
    if base == 0 || base == 16 {
        if *buf.add(*pos) == b'0' && *pos + 1 < max
            && (*buf.add(*pos + 1) == b'x' || *buf.add(*pos + 1) == b'X')
        {
            // musl treats "0x" without a following hex digit as a failed
            // conversion, not as the number zero.
            let has_digit = *pos + 2 < max && hex_val(*buf.add(*pos + 2)).is_some();
            *pos += 2;
            if !has_digit {
                return (0, neg, false);
            }
            base = 16;
        } else if base == 0 {
            base = 8;
        }
    }
    if base == 0 { base = 10; }

    let mut val: u64 = 0;
    let mut found = false;
    while *pos < max {
        let c = *buf.add(*pos);
        let d: Option<u64> = match base {
            8 => if c >= b'0' && c <= b'7' { Some((c - b'0') as u64) } else { None },
            10 => if c >= b'0' && c <= b'9' { Some((c - b'0') as u64) } else { None },
            16 => hex_val(c).map(|v| v as u64),
            _ => None,
        };
        match d {
            Some(digit) => {
                val = val.wrapping_mul(base as u64).wrapping_add(digit);
                *pos += 1;
                found = true;
            }
            None => break,
        }
    }
    if !found { *pos = start; return (0, false, false); }
    (val, neg, true)
}

// Scan float with position tracking. Returns (value, success).
unsafe fn scan_float_val(
    buf: *const u8, pos: &mut usize, buf_len: usize, width: usize,
) -> (f64, bool) {
    let start = *pos;
    let max = if width > 0 { (start + width).min(buf_len) } else { buf_len };
    let neg = if *pos < max && *buf.add(*pos) == b'-' { *pos += 1; true }
    else if *pos < max && *buf.add(*pos) == b'+' { *pos += 1; false }
    else { false };
    // inf/nan
    if *pos + 2 < max {
        let (c0,c1,c2) = (*buf.add(*pos), *buf.add(*pos+1), *buf.add(*pos+2));
        if (c0==b'i'||c0==b'I')&&(c1==b'n'||c1==b'N')&&(c2==b'f'||c2==b'F') { *pos+=3; return (if neg{f64::NEG_INFINITY}else{f64::INFINITY}, true); }
        if (c0==b'n'||c0==b'N')&&(c1==b'a'||c1==b'A')&&(c2==b'n'||c2==b'N') { *pos+=3; return (if neg{-f64::NAN}else{f64::NAN}, true); }
    }
    // hex float
    if *pos+1 < max && *buf.add(*pos)==b'0' && (*buf.add(*pos+1)==b'x'||*buf.add(*pos+1)==b'X') {
        *pos += 2;
        let mut val: f64 = 0.0; let mut found = false; let mut frac_scale = 1.0f64; let mut in_frac = false;
        while *pos < max {
            if let Some(d) = hex_val(*buf.add(*pos)) {
                if in_frac { frac_scale /= 16.0; val += d as f64 * frac_scale; } else { val = val * 16.0 + d as f64; }
                *pos += 1; found = true;
            } else if *buf.add(*pos)==b'.' && !in_frac { in_frac = true; *pos += 1; } else { break; }
        }
        if !found { *pos = start; return (0.0, false); }
        // p exponent
        if *pos < max && (*buf.add(*pos)==b'p'||*buf.add(*pos)==b'P') {
            *pos += 1;
            let mut eneg = false;
            if *pos<max && *buf.add(*pos)==b'-' { eneg=true; *pos+=1; }
            else if *pos<max && *buf.add(*pos)==b'+' { *pos+=1; }
            let mut ev: i32 = 0; let mut ef = false;
            while *pos<max && *buf.add(*pos)>=b'0' && *buf.add(*pos)<=b'9' {
                ev = ev.saturating_mul(10).saturating_add((*buf.add(*pos)-b'0') as i32).min(1_000_000);
                *pos+=1; ef=true;
            }
            if !ef { return (0.0, false); }
            if eneg { ev = -ev; }
            val *= libm::pow(2.0, ev as f64);
        }
        return (if neg{-val}else{val}, true);
    }
    let token_start = start;
    let mut i = start;
    let mut saw_digit = false;
    let mut saw_dot = false;
    let mut saw_exp = false;
    let mut first = true;
    while i < max {
        let c = *buf.add(i);
        if (c == b'+' || c == b'-') && first {
            i += 1; first = false; continue;
        }
        if c >= b'0' && c <= b'9' {
            saw_digit = true; i += 1; first = false; continue;
        }
        if c == b'.' && !saw_dot && !saw_exp {
            saw_dot = true; i += 1; first = false; continue;
        }
        if (c == b'e' || c == b'E') && !saw_exp && saw_digit {
            saw_exp = true; i += 1; first = true; continue;
        }
        break;
    }
    if !saw_digit { return (0.0, false); }
    let s = core::str::from_utf8_unchecked(core::slice::from_raw_parts(buf.add(token_start), i - token_start));
    match <f64 as core::str::FromStr>::from_str(s) {
        Ok(v) => { *pos = i; return (v, true); }
        Err(_) => { return (0.0, false); }
    }
}

/// Store a scanned integer with the width selected by the length modifier.
/// Without this, `%ld`/`%lu` silently truncate to 32 bits.
// Length-modifier encoding shared by the scanf parser and integer store.
const LEN_NONE: u8 = 0;
const LEN_SHORT: u8 = 1;
const LEN_CHAR: u8 = 2;
const LEN_LONG: u8 = 3;
const LEN_LONG_LONG: u8 = 4;
const LEN_LONG_DOUBLE: u8 = 5;

unsafe fn store_scan_int(dest: *mut c_void, len_mod: u8, val: u64, neg: bool) {
    let sv: i64 = if neg { (val as i64).wrapping_neg() } else { val as i64 };
    match len_mod {
        LEN_CHAR => *(dest as *mut i8) = sv as i8,
        LEN_SHORT => *(dest as *mut i16) = sv as i16,
        LEN_LONG | LEN_LONG_LONG => *(dest as *mut i64) = sv,
        _ => *(dest as *mut i32) = sv as i32,
    }
}

/// Advance over up to `max` input bytes accepted by `accept` and return how
/// many were consumed.  The scanf text conversions differ only in this
/// predicate; the caller copies the resulting span into its destination.
#[inline]
unsafe fn scan_span<F: Fn(u8) -> bool>(
    buf: *const u8,
    buf_len: usize,
    p: &mut usize,
    max: usize,
    accept: F,
) -> usize {
    let start = *p;
    let mut n = 0usize;
    while n < max && *p < buf_len {
        let c = *buf.add(*p);
        if !accept(c) { break; }
        *p += 1;
        n += 1;
    }
    *p - start
}

const SCAN_BUF_INIT: usize = 4096;

// Comprehensive scanf parser with position tracking.
// consumed: if non-null, stores number of bytes consumed from buf.
unsafe fn do_vsscanf(
    buf: *const u8, buf_len: usize, fmt: *const c_char, args: &mut VaList<'_>, consumed: *mut usize,
) -> c_int {
    let mut p = 0usize;
    let mut fi = 0usize;
    let mut assigned = 0i32;
    let mut input_eof = false;
    loop {
        let fc = *fmt.add(fi) as u8;
        if fc == 0 { break; }
        if is_ws_byte(fc) {
            while is_ws_byte(*fmt.add(fi) as u8) { fi += 1; }
            while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
            continue;
        }
        if fc != b'%' {
            if p >= buf_len { input_eof = true; break; }
            if *buf.add(p) != fc { break; }
            p += 1; fi += 1; continue;
        }
        fi += 1;
        // %%
        if *fmt.add(fi) as u8 == b'%' {
            if p >= buf_len { input_eof = true; break; }
            if *buf.add(p) != b'%' { break; }
            p += 1; fi += 1; continue;
        }
        let suppress = if *fmt.add(fi) as u8 == b'*' { fi += 1; true } else { false };
        let mut width: usize = 0;
        let mut width_overflow = false;
        while (*fmt.add(fi) as u8) >= b'0' && (*fmt.add(fi) as u8) <= b'9' {
            let digit = ((*fmt.add(fi) as u8) - b'0') as usize;
            if width > (c_int::MAX as usize - digit) / 10 {
                width_overflow = true;
                break;
            }
            width = width * 10 + digit;
            fi += 1;
        }
        if width_overflow { input_eof = true; break; }
        let mut alloc = false;
        if *fmt.add(fi) as u8 == b'm' {
            alloc = !suppress;
            fi += 1;
        }
        let mut len_mod = LEN_NONE;
        match *fmt.add(fi) as u8 {
            b'h' => { fi += 1; if *fmt.add(fi) as u8 == b'h' { fi += 1; len_mod = LEN_CHAR; } else { len_mod = LEN_SHORT; } }
            b'l' => { fi += 1; if *fmt.add(fi) as u8 == b'l' { fi += 1; len_mod = LEN_LONG_LONG; } else { len_mod = LEN_LONG; } }
            b'j' => { fi += 1; len_mod = LEN_LONG_LONG; }
            b'z' | b't' => { fi += 1; len_mod = LEN_LONG; }
            b'L' => { fi += 1; len_mod = LEN_LONG_DOUBLE; }
            _ => {}
        }
        let spec = *fmt.add(fi) as u8;
        let (real_spec, _real_len) = if spec==b'C' {(b'c',3u8)} else if spec==b'S' {(b's',3u8)} else {(spec,len_mod)};
        match real_spec {
            b'n' => {
                if !suppress {
                    let out = args.next_arg::<*mut c_void>();
                    if !out.is_null() {
                        match len_mod {
                            LEN_SHORT => *(out as *mut i16) = p as i16,
                            LEN_CHAR => *(out as *mut i8) = p as i8,
                            LEN_LONG | LEN_LONG_LONG => *(out as *mut i64) = p as i64,
                            _ => *(out as *mut c_int) = p as c_int,
                        }
                    }
                }
            }
            b'd' | b'u' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let dest = if !suppress { Some(args.next_arg::<*mut c_void>()) } else { None };
                let (val, neg, ok) = scan_int_val(buf, &mut p, buf_len, 10, width);
                if !ok { break; }
                if let Some(out) = dest { if !out.is_null() {
                    store_scan_int(out, len_mod, val, neg);
                }}
                if !suppress { assigned += 1; }
            }
            b'i' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let dest = if !suppress { Some(args.next_arg::<*mut c_void>()) } else { None };
                let (val, neg, ok) = scan_int_val(buf, &mut p, buf_len, 0, width);
                if !ok { break; }
                if let Some(out) = dest { if !out.is_null() { store_scan_int(out, len_mod, val, neg); } }
                if !suppress { assigned += 1; }
            }
            b'o' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let dest = if !suppress { Some(args.next_arg::<*mut c_void>()) } else { None };
                let (val, neg, ok) = scan_int_val(buf, &mut p, buf_len, 8, width);
                if !ok { break; }
                if let Some(out) = dest { if !out.is_null() { store_scan_int(out, len_mod, val, neg); } }
                if !suppress { assigned += 1; }
            }
            b'x' | b'X' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let dest = if !suppress { Some(args.next_arg::<*mut c_void>()) } else { None };
                let (val, neg, ok) = scan_int_val(buf, &mut p, buf_len, 16, width);
                if !ok { break; }
                if let Some(out) = dest { if !out.is_null() { store_scan_int(out, len_mod, val, neg); } }
                if !suppress { assigned += 1; }
            }
            b'p' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let dest = if !suppress { Some(args.next_arg::<*mut *mut c_void>()) } else { None };
                let (val, _, ok) = scan_int_val(buf, &mut p, buf_len, 16, width);
                if !ok { break; }
                if let Some(out) = dest { if !out.is_null() { *out = val as usize as *mut c_void; } }
                if !suppress { assigned += 1; }
            }
            b'a' | b'e' | b'f' | b'g' | b'A' | b'E' | b'F' | b'G' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let (val, ok) = scan_float_val(buf, &mut p, buf_len, width);
                if !ok { break; }
                if !suppress {
                    if _real_len == LEN_LONG {
                        let out = args.next_arg::<*mut f64>(); if !out.is_null() { *out = val; }
                    } else if _real_len == LEN_LONG_DOUBLE {
                        #[cfg(any(target_arch = "aarch64", target_arch = "riscv64", target_arch = "powerpc64", target_arch = "loongarch64"))]
                        { let out = args.next_arg::<*mut f128>(); if !out.is_null() { *out = f64_to_f128(val); } }
                        #[cfg(target_arch = "x86_64")]
                        { let out = args.next_arg::<*mut f64>(); if !out.is_null() { *out = val; } }
                    } else {
                        let out = args.next_arg::<*mut f32>(); if !out.is_null() { *out = val as f32; }
                    }
                }
                if !suppress { assigned += 1; }
            }
            b'c' => {
                let w = if width > 0 { width } else { 1 };
                if p >= buf_len { input_eof = true; break; }
                let start_p = p;
                let n = scan_span(buf, buf_len, &mut p, w, |_| true);
                if n != w { p = start_p; input_eof = true; break; }
                if alloc {
                    let out = args.next_arg::<*mut *mut c_char>();
                    let s = crate::malloc(w + 1) as *mut c_char;
                    if s.is_null() {
                        if !consumed.is_null() { *consumed = p; }
                        return if assigned == 0 { -1 } else { assigned };
                    }
                    core::ptr::copy_nonoverlapping(buf.add(start_p), s as *mut u8, w);
                    *s.add(w) = 0;
                    if !out.is_null() { *out = s; } else { crate::free(s as *mut c_void); }
                } else {
                    let dest_ptr: *mut c_char = if !suppress { args.next_arg::<*mut c_char>() } else { core::ptr::null_mut() };
                    if !dest_ptr.is_null() {
                        core::ptr::copy_nonoverlapping(buf.add(start_p), dest_ptr as *mut u8, w);
                    }
                }
                if !suppress { assigned += 1; }
            }
            b's' => {
                while p < buf_len && is_ws_byte(*buf.add(p)) { p += 1; }
                if p >= buf_len { input_eof = true; break; }
                let w = if width > 0 { width } else { usize::MAX };
                let start_p = p;
                let n = scan_span(buf, buf_len, &mut p, w, |c| c != 0 && !is_ws_byte(c));
                if n == 0 { break; }
                if alloc {
                    let out = args.next_arg::<*mut *mut c_char>();
                    let s = crate::malloc(n + 1) as *mut c_char;
                    if s.is_null() {
                        if !consumed.is_null() { *consumed = p; }
                        return if assigned == 0 { -1 } else { assigned };
                    }
                    core::ptr::copy_nonoverlapping(buf.add(start_p), s as *mut u8, n);
                    *s.add(n) = 0;
                    if !out.is_null() { *out = s; } else { crate::free(s as *mut c_void); }
                } else {
                    let dest_ptr: *mut c_char = if !suppress { args.next_arg::<*mut c_char>() } else { core::ptr::null_mut() };
                    if !dest_ptr.is_null() {
                        core::ptr::copy_nonoverlapping(buf.add(start_p), dest_ptr as *mut u8, n);
                        *dest_ptr.add(n) = 0;
                    }
                }
                if !suppress { assigned += 1; }
            }
            b'[' => {
                fi += 1;
                let negate = if *fmt.add(fi) as u8 == b'^' { fi += 1; true } else { false };
                let mut charset = [0u8; 256];
                let mut prev: Option<u8> = None;
                if *fmt.add(fi) as u8 == b']' {
                    charset[b']' as usize] = 1;
                    prev = Some(b']');
                    fi += 1;
                }
                loop {
                    let c = *fmt.add(fi) as u8;
                    if c == b']' || c == 0 { break; }
                    // "a-z" is a range; a '-' next to ']' or at the start/end
                    // of the set is a literal.
                    if c == b'-' && prev.is_some() {
                        let hi = *fmt.add(fi + 1) as u8;
                        if hi != b']' && hi != 0 {
                            let lo = prev.unwrap();
                            let mut k = lo;
                            while k <= hi {
                                charset[k as usize] = 1;
                                if k == 255 { break; }
                                k += 1;
                            }
                            prev = None;
                            fi += 2;
                            continue;
                        }
                    }
                    charset[c as usize] = 1;
                    prev = Some(c);
                    fi += 1;
                }
                if *fmt.add(fi) as u8 == b']' { fi += 1; }
                fi -= 1; // common fi += 1 will advance past the closing ]
                if p >= buf_len { input_eof = true; break; }
                let w = if width > 0 { width } else { usize::MAX };
                let start_p = p;
                let n = scan_span(buf, buf_len, &mut p, w, |c| {
                    if c == 0 { return false; }
                    let in_set = charset[c as usize] != 0;
                    if negate { !in_set } else { in_set }
                });
                if n == 0 { break; }
                if alloc {
                    let out = args.next_arg::<*mut *mut c_char>();
                    let s = crate::malloc(n + 1) as *mut c_char;
                    if s.is_null() {
                        if !consumed.is_null() { *consumed = p; }
                        return if assigned == 0 { -1 } else { assigned };
                    }
                    core::ptr::copy_nonoverlapping(buf.add(start_p), s as *mut u8, n);
                    *s.add(n) = 0;
                    if !out.is_null() { *out = s; } else { crate::free(s as *mut c_void); }
                } else {
                    let dest_ptr: *mut c_char = if !suppress { args.next_arg::<*mut c_char>() } else { core::ptr::null_mut() };
                    if !dest_ptr.is_null() {
                        core::ptr::copy_nonoverlapping(buf.add(start_p), dest_ptr as *mut u8, n);
                        *dest_ptr.add(n) = 0;
                    }
                }
                if !suppress { assigned += 1; }
            }
            _ => break,
        }
        fi += 1;
    }
    if !consumed.is_null() { *consumed = p; }
    if assigned == 0 && input_eof { -1 } else { assigned }
}

unsafe fn vsscanf_inner(buf: *const u8, fmt: *const c_char, args: &mut VaList<'_>) -> c_int {
    let len = strlen(buf as *const c_char);
    do_vsscanf(buf, len, fmt, args, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn vsscanf(buf: *const c_char, fmt: *const c_char, mut args: VaList) -> c_int {
    vsscanf_inner(buf as *const u8, fmt, &mut args)
}

unsafe fn scan_buf_grow(buf: &mut *mut u8, cap: &mut usize, need: usize) -> bool {
    if need <= *cap {
        return true;
    }
    let mut next = core::cmp::max(*cap, 256);
    while next < need {
        next = match next.checked_mul(2) {
            Some(v) => v,
            None => return false,
        };
    }
    let grown = crate::realloc(*buf as *mut c_void, next) as *mut u8;
    if grown.is_null() {
        return false;
    }
    *buf = grown;
    *cap = next;
    true
}

#[no_mangle]
pub unsafe extern "C" fn vfscanf(stream: *mut FILE, fmt: *const c_char, mut args: VaList) -> c_int {
    let f = &mut *stream;
    let start_ungotten_count = f.ungotten_count as usize;
    let ungotten_saved = f.ungotten;
    let sfunc = f.seek_fn;
    // Logical position, not the raw fd offset: fgetc may have read ahead.
    let start_pos = if sfunc.is_some() { ftello(stream) } else { -1 };

    let mut cap = SCAN_BUF_INIT;
    let mut line = crate::malloc(cap) as *mut u8;
    if line.is_null() {
        ERRNO!() = ENOMEM;
        return -1;
    }
    let mut pos = 0usize;

    if start_pos >= 0 {
        // Seekable: read via fgetc until newline or EOF.
        loop {
            // Reserve room for the data byte and the terminating NUL: the
            // newline path below leaves pos == cap if only pos+1 is reserved.
            if !scan_buf_grow(&mut line, &mut cap, pos + 2) {
                crate::free(line as *mut c_void);
                ERRNO!() = ENOMEM;
                return -1;
            }
            let c = fgetc(stream);
            if c == -1 { break; }
            *line.add(pos) = c as u8;
            pos += 1;
            if c == b'\n' as c_int { break; }
        }
    } else {
        // Non-seekable (pipe/socket): drain pushback, then read all available
        // data non-blocking so we don't stop at newline boundaries.

        // 1. Drain ungotten bytes (LIFO order)
        while f.ungotten_count > 0 {
            if !scan_buf_grow(&mut line, &mut cap, pos + 1) {
                crate::free(line as *mut c_void);
                ERRNO!() = ENOMEM;
                return -1;
            }
            f.ungotten_count -= 1;
            *line.add(pos) = f.ungotten[f.ungotten_count as usize] as u8;
            pos += 1;
        }
        // 2. Drain rpos/rend pushback buffer
        while !f.rpos.is_null() && f.rpos < f.rend {
            if !scan_buf_grow(&mut line, &mut cap, pos + 1) {
                crate::free(line as *mut c_void);
                ERRNO!() = ENOMEM;
                return -1;
            }
            *line.add(pos) = *f.rpos;
            f.rpos = f.rpos.add(1);
            pos += 1;
        }
        // 3. Wait for the first byte (blocking), then drain whatever else is
        // available without waiting. A pure non-blocking read would race with
        // a producer such as popen() that has not written yet.
        let fd = f.fd;
        if pos == 0 {
            let n = sys_read(fd as i64, line, 1);
            if n == 1 { pos = 1; }
        }
        let old_flags = sys_fcntl(fd, F_GETFL, 0);
        if old_flags >= 0 {
            let _ = sys_fcntl(fd, F_SETFL, old_flags | O_NONBLOCK as i64);
        }
        loop {
            if pos + 1 >= cap && !scan_buf_grow(&mut line, &mut cap, pos + 2) {
                if old_flags >= 0 { let _ = sys_fcntl(fd, F_SETFL, old_flags); }
                crate::free(line as *mut c_void);
                ERRNO!() = ENOMEM;
                return -1;
            }
            let n = sys_read(fd as i64, line.add(pos), cap - 1 - pos);
            if n <= 0 { break; }
            pos += n as usize;
        }
        if old_flags >= 0 {
            let _ = sys_fcntl(fd, F_SETFL, old_flags);
        }
        // Clear flags that the failed non-blocking read may have set
        f._eof = 0;
        f._err = 0;
        f.flags &= !(F_EOF | F_ERR);
    }

    *line.add(pos) = 0;
    if pos == 0 {
        crate::free(line as *mut c_void);
        return 0;
    }

    let mut consumed = 0usize;
    let assigned = do_vsscanf(line, pos, fmt, &mut args, &mut consumed);

    if start_pos >= 0 {
        // Seek back to the logical position just past the consumed bytes and
        // restore any ungetc pushback the scan did not consume.
        let remaining = start_ungotten_count.saturating_sub(consumed);
        // The restored ungetc bytes will be subtracted by ftello(), so the
        // raw fd offset must include them.
        let target = start_pos + consumed as i64 + remaining as i64;
        if let Some(sfunc) = sfunc {
            sfunc(stream, target, SEEK_SET);
        }
        f.rpos = core::ptr::null_mut();
        f.rend = core::ptr::null_mut();
        if consumed < pos { f._eof = 0; }
        f.ungotten_count = 0;
        for i in 0..remaining {
            f.ungotten[f.ungotten_count as usize] = ungotten_saved[i];
            f.ungotten_count += 1;
        }
    } else {
        // Non-seekable: push back unconsumed bytes into FILE read buffer
        let unconsumed = pos.saturating_sub(consumed);
        if unconsumed > 0 && !f.buf.is_null() {
            let copy_count = core::cmp::min(unconsumed, f.buf_size);
            core::ptr::copy(
                line.add(consumed),
                f.buf,
                copy_count,
            );
            f.rpos = f.buf;
            f.rend = f.buf.add(copy_count);
        }
        f.ungotten_count = 0;
        f._eof = 0;
        f.flags &= !F_EOF;
    }
    crate::free(line as *mut c_void);
    assigned
}

#[no_mangle]
pub unsafe extern "C" fn vscanf(fmt: *const c_char, args: VaList) -> c_int {
    vfscanf(stdin, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn sscanf(buf: *const c_char, fmt: *const c_char, mut args: ...) -> c_int {
    vsscanf_inner(buf as *const u8, fmt, &mut args)
}

#[no_mangle]
pub unsafe extern "C" fn fscanf(stream: *mut FILE, fmt: *const c_char, args: ...) -> c_int {
    vfscanf(stream, fmt, args)
}

#[no_mangle]
pub unsafe extern "C" fn scanf(fmt: *const c_char, args: ...) -> c_int {
    vfscanf(stdin, fmt, args)
}

