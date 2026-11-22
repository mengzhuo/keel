// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// arpa/inet: inet_pton / inet_ntop / inet_addr / inet_ntoa / inet_aton
// ============================================================

const AF_INET_VAL: c_int = 2;
const AF_INET6_VAL: c_int = 10;

unsafe fn inet_pton_v4(s: *const u8, a: *mut u8) -> c_int {
    let mut p = s;
    let mut i = 0;
    while i < 4 {
        let oct_start = p;
        let mut v: u32 = 0;
        let mut j: u32 = 0;
        while j < 3 && *p >= b'0' && *p <= b'9' {
            v = v * 10 + (*p - b'0') as u32;
            p = p.add(1);
            j += 1;
        }
        if j == 0 || (j > 1 && *oct_start == b'0') || v > 255 { return 0; }
        *a.add(i) = v as u8;
        if *p == 0 && i == 3 { return 1; }
        if *p != b'.' { return 0; }
        p = p.add(1);
        i += 1;
    }
    0
}

unsafe fn inet_pton_v6(s: *const u8, a: *mut u8) -> c_int {
    let mut ip = [0u16; 8];
    let mut p = s;
    let mut i: usize = 0;
    let mut brk: isize = -1;
    let mut need_v4 = false;
    let mut v4_start: *const u8 = core::ptr::null();

    if *p == b':' {
        p = p.add(1);
        if *p != b':' { return 0; }
    }

    loop {
        if *p == b':' && brk < 0 {
            brk = i as isize;
            ip[i & 7] = 0;
            p = p.add(1);
            if *p == 0 { break; }
            if i == 7 { return 0; }
            i += 1;
            continue;
        }
        v4_start = p;
        let mut v: u16 = 0;
        let mut j = 0;
        while j < 4 {
            let d = hex_digit(*p);
            if d < 0 { break; }
            v = v * 16 + d as u16;
            p = p.add(1);
            j += 1;
        }
        if j == 0 { return 0; }
        ip[i & 7] = v;
        if *p == 0 && (brk >= 0 || i == 7) { break; }
        if i == 7 { return 0; }
        if *p != b':' {
            if *p != b'.' || (i < 6 && brk < 0) { return 0; }
            need_v4 = true;
            i += 1;
            ip[i & 7] = 0;
            break;
        }
        p = p.add(1);
        i += 1;
    }

    if brk >= 0 {
        // expand :: : move trailing fields to the end and zero the gap
        let trail_count = i + 1 - brk as usize;
        let dest_start = brk as usize + 7 - i;
        // move backwards to avoid overwrite in the overlapping case
        for j in (0..trail_count).rev() {
            ip[dest_start + j] = ip[brk as usize + j];
        }
        for j in 0..(7 - i) {
            ip[brk as usize + j] = 0;
        }
    }

    for k in 0..8 {
        *a.add(k * 2) = (ip[k] >> 8) as u8;
        *a.add(k * 2 + 1) = (ip[k] & 0xff) as u8;
    }

    if need_v4 {
        if inet_pton_v4(v4_start, a.add(12)) <= 0 { return 0; }
    }
    1
}

unsafe fn hex_digit(c: u8) -> i32 {
    if c >= b'0' && c <= b'9' { (c - b'0') as i32 }
    else if c >= b'a' && c <= b'f' { (c - b'a' + 10) as i32 }
    else if c >= b'A' && c <= b'F' { (c - b'A' + 10) as i32 }
    else { -1 }
}

#[no_mangle]
pub unsafe extern "C" fn inet_pton(af: c_int, s: *const c_char, a: *mut c_void) -> c_int {
    if af == AF_INET_VAL {
        inet_pton_v4(s as *const u8, a as *mut u8)
    } else if af == AF_INET6_VAL {
        inet_pton_v6(s as *const u8, a as *mut u8)
    } else {
        ERRNO!() = EAFNOSUPPORT;
        -1
    }
}

#[no_mangle]
pub unsafe extern "C" fn inet_ntop(af: c_int, a: *const c_void, s: *mut c_char, size: u32) -> *mut c_char {
    if af == AF_INET_VAL {
        let b = a as *const u8;
        let needed = 16; // "255.255.255.255\0"
        if size < needed as u32 { ERRNO!() = ENOSPC; return core::ptr::null_mut(); }
        let mut pos = 0;
        for i in 0..4 {
            if i > 0 { *s.add(pos) = b'.' as c_char; pos += 1; }
            let (buf, len) = format_u64(*b.add(i) as u64);
            for j in 0..len { *s.add(pos) = buf[j] as c_char; pos += 1; }
        }
        *s.add(pos) = 0;
        s
    } else if af == AF_INET6_VAL {
        let b = a as *const u8;
        let needed = 46; // max IPv6
        if size < needed as u32 { ERRNO!() = ENOSPC; return core::ptr::null_mut(); }
        let mut words = [0u16; 8];
        for i in 0..8 {
            words[i] = ((*b.add(i * 2) as u16) << 8) | (*b.add(i * 2 + 1) as u16);
        }
        // IPv4-mapped: ::ffff:a.b.c.d
        let mut pos = 0usize;
        if words[0] == 0 && words[1] == 0 && words[2] == 0 && words[3] == 0
            && words[4] == 0 && words[5] == 0xffff
        {
            let prefix = b"::ffff:";
            for k in 0..prefix.len() { *s.add(pos) = prefix[k] as c_char; pos += 1; }
            for idx in 0..4 {
                if idx > 0 { *s.add(pos) = b'.' as c_char; pos += 1; }
                let octet = *b.add(12 + idx);
                if octet >= 100 {
                    *s.add(pos) = (b'0' + octet / 100) as c_char; pos += 1;
                    *s.add(pos) = (b'0' + (octet / 10) % 10) as c_char; pos += 1;
                } else if octet >= 10 {
                    *s.add(pos) = (b'0' + octet / 10) as c_char; pos += 1;
                }
                *s.add(pos) = (b'0' + octet % 10) as c_char; pos += 1;
            }
            *s.add(pos) = 0;
            return s;
        }
        // find longest run of zeros
        let mut best_start: isize = -1;
        let mut best_len: usize = 0;
        let mut j: usize = 0;
        while j < 8 {
            if words[j] == 0 {
                let mut k = j;
                while k < 8 && words[k] == 0 { k += 1; }
                let run = k - j;
                if run > best_len { best_start = j as isize; best_len = run; }
                j = k;
            } else {
                j += 1;
            }
        }
        let mut i: usize = 0;
        let mut prev_in_run = false;
        while i < 8 {
            let in_run = best_len >= 2 && i >= best_start as usize && i < best_start as usize + best_len;
            if in_run {
                if i == best_start as usize {
                    *s.add(pos) = b':' as c_char; pos += 1;
                    *s.add(pos) = b':' as c_char; pos += 1;
                }
                i += 1;
                prev_in_run = true;
                continue;
            }
            if i > 0 && !prev_in_run {
                *s.add(pos) = b':' as c_char; pos += 1;
            }
            let (buf, len) = format_hex(words[i] as u64, false);
            for k in 0..len { *s.add(pos) = buf[k] as c_char; pos += 1; }
            i += 1;
            prev_in_run = false;
        }
        *s.add(pos) = 0;
        s
    } else {
        ERRNO!() = EAFNOSUPPORT;
        core::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn inet_aton(s: *const c_char, dest: *mut c_void) -> c_int {
    let a = dest as *mut u8;
    let mut p = s as *const u8;
    let mut vals = [0u64; 4];
    let mut i = 0usize;
    while i < 4 {
        if *p < b'0' || *p > b'9' { return 0; }
        let mut base = 10u32;
        if *p == b'0' {
            let c = *p.add(1);
            if c == b'x' || c == b'X' {
                base = 16;
                p = p.add(2);
                if !((*p >= b'0' && *p <= b'9')
                    || (*p >= b'a' && *p <= b'f')
                    || (*p >= b'A' && *p <= b'F')) {
                    return 0;
                }
            } else {
                base = 8;
            }
        }
        let mut v: u64 = 0;
        let start = p;
        loop {
            let c = *p;
            let d = if c >= b'0' && c <= b'9' {
                (c - b'0') as u64
            } else if base == 16 && c >= b'a' && c <= b'f' {
                (c - b'a' + 10) as u64
            } else if base == 16 && c >= b'A' && c <= b'F' {
                (c - b'A' + 10) as u64
            } else {
                break;
            };
            v = match v
                .checked_mul(base as u64)
                .and_then(|n| n.checked_add(d))
            {
                // Valid dotted-quad components never exceed 32 bits; reject
                // overflow instead of silently wrapping (musl's strtoul
                // saturates, and the later >255 check rejects the result).
                Some(n) if n <= 0xFFFF_FFFF => n,
                _ => return 0,
            };
            p = p.add(1);
        }
        if p == start { return 0; }
        vals[i] = v;
        if *p == 0 {
            break;
        }
        if *p != b'.' || i == 3 { return 0; }
        p = p.add(1);
        i += 1;
    }
    if i == 4 { return 0; }
    if i < 1 { vals[1] = vals[0] & 0xffffff; vals[0] >>= 24; }
    if i < 2 { vals[2] = vals[1] & 0xffff; vals[1] >>= 16; }
    if i < 3 { vals[3] = vals[2] & 0xff; vals[2] >>= 8; }
    for j in 0..4 {
        if vals[j] > 255 { return 0; }
        *a.add(j) = vals[j] as u8;
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn inet_addr(s: *const c_char) -> u32 {
    let mut a = [0u8; 4];
    if inet_aton(s, a.as_mut_ptr() as *mut c_void) == 1 {
        u32::from_ne_bytes(a)
    } else {
        !0u32 // INADDR_NONE
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct InAddr {
    pub s_addr: u32,
}

#[no_mangle]
pub unsafe extern "C" fn inet_ntoa(addr: InAddr) -> *mut c_char {
    static mut NTOA_BUF: [c_char; 16] = [0; 16];
    let b = addr.s_addr.to_ne_bytes();
    let mut pos = 0;
    for i in 0..4 {
        if i > 0 { NTOA_BUF[pos] = b'.' as c_char; pos += 1; }
        let (buf, len) = format_u64(b[i] as u64);
        for j in 0..len { NTOA_BUF[pos] = buf[j] as c_char; pos += 1; }
    }
    NTOA_BUF[pos] = 0;
    core::ptr::addr_of_mut!(NTOA_BUF).cast::<c_char>()
}

// inet_network/inet_makeaddr/inet_lnaof/inet_netof - simple implementations
#[no_mangle]
pub unsafe extern "C" fn inet_network(s: *const c_char) -> u32 {
    inet_addr(s).to_be()
}

#[no_mangle]
pub extern "C" fn inet_makeaddr(net: c_int, host: c_int) -> u32 {
    // musl's inet_legacy: class A when the network number fits in 8 bits,
    // class B when it fits in 16, otherwise the top 24 bits.
    let n = net as u32;
    let mut h = host as u32;
    if n < 256 {
        h |= n << 24;
    } else if n < 65536 {
        h |= n << 16;
    } else {
        h |= n << 8;
    }
    h
}

#[no_mangle]
pub extern "C" fn inet_lnaof(addr: u32) -> c_int {
    // s_addr already carries the network-order numeric value, exactly like
    // musl's inet_legacy.c; do not byte-swap it again.
    let h = addr;
    if h < 0x80000000 { (h & 0x00ffffff) as c_int }
    else if h < 0xc0000000 { (h & 0x0000ffff) as c_int }
    else { (h & 0x000000ff) as c_int }
}

#[no_mangle]
pub extern "C" fn inet_netof(addr: u32) -> c_int {
    let h = addr;
    if h < 0x80000000 { (h >> 24) as c_int }
    else if h < 0xc0000000 { (h >> 16) as c_int }
    else { (h >> 8) as c_int }
}

