// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// String/memory functions
// ============================================================

// strlen, memcpy, memset, memmove and memcmp live in string_fast.rs.

#[no_mangle]
pub unsafe extern "C" fn bcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int {
    memcmp(s1, s2, n)
}

// strcmp lives in string_fast.rs.

#[no_mangle]
pub unsafe extern "C" fn strncmp(s1: *const u8, s2: *const u8, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as c_int - b as c_int;
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    let n = strlen(src as *const c_char);
    core::ptr::copy_nonoverlapping(src, dst, n + 1);
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let l = strnlen(src, n);
    core::ptr::copy_nonoverlapping(src, dst, l);
    core::ptr::write_bytes(dst.add(l), 0, n - l);
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strcat(dst: *mut u8, src: *const u8) -> *mut u8 {
    let dl = strlen(dst as *const c_char);
    strcpy(dst.add(dl), src);
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strncat(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let dl = strlen(dst as *const c_char);
    let l = strnlen(src, n);
    core::ptr::copy_nonoverlapping(src, dst.add(dl), l);
    *dst.add(dl + l) = 0;
    dst
}

#[no_mangle]
pub unsafe extern "C" fn strlcpy(dst: *mut u8, src: *const u8, n: usize) -> usize {
    let l = strlen(src as *const c_char);
    if n > 0 {
        let c = if l < n { l } else { n - 1 };
        core::ptr::copy_nonoverlapping(src, dst, c);
        *dst.add(c) = 0;
    }
    l
}

#[no_mangle]
pub unsafe extern "C" fn strlcat(dst: *mut u8, src: *const u8, n: usize) -> usize {
    let dl = strlen(dst as *const c_char);
    let sl = strlen(src as *const c_char);
    if dl >= n { return n + sl; }
    let c = if sl < n - dl { sl } else { n - dl - 1 };
    core::ptr::copy_nonoverlapping(src, dst.add(dl), c);
    *dst.add(dl + c) = 0;
    dl + sl
}

#[no_mangle]
pub unsafe extern "C" fn bcopy(src: *const c_void, dst: *mut c_void, n: usize) {
    core::ptr::copy(src as *const u8, dst as *mut u8, n);
}

#[no_mangle]
pub unsafe extern "C" fn bzero(s: *mut c_void, n: usize) {
    core::ptr::write_bytes(s as *mut u8, 0, n);
}

#[no_mangle]
pub unsafe extern "C" fn strndup(s: *const c_char, n: usize) -> *mut c_char {
    let mut len = 0;
    while len < n && *s.add(len) != 0 {
        len += 1;
    }
    let p = malloc(len + 1) as *mut c_char;
    if p.is_null() {
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(s, p, len);
    *p.add(len) = 0;
    p
}

#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const u8, c: c_int) -> *mut u8 {
    let p = strchrnul(s as *const c_char, c) as *const u8;
    if *p != 0 || c as u8 == 0 {
        p as *mut u8
    } else {
        null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn strrchr(s: *const u8, c: c_int) -> *mut u8 {
    let len = strlen(s as *const c_char);
    memrchr(s, c, len + 1)
}

#[no_mangle]
pub unsafe extern "C" fn strnlen(s: *const u8, maxlen: usize) -> usize {
    let p = memchr(s, 0, maxlen);
    if p.is_null() {
        maxlen
    } else {
        p as usize - s as usize
    }
}

#[inline]
unsafe fn byte_set(members: *const u8) -> [u64; 4] {
    let mut set = [0u64; 4];
    let mut p = members;
    while *p != 0 {
        let b = *p as usize;
        set[b >> 6] |= 1u64 << (b & 63);
        p = p.add(1);
    }
    set
}

#[inline]
fn in_set(set: &[u64; 4], c: u8) -> bool {
    set[c as usize >> 6] & (1u64 << (c as usize & 63)) != 0
}

#[no_mangle]
pub unsafe extern "C" fn strcspn(s: *const u8, reject: *const u8) -> usize {
    let set = byte_set(reject);
    let mut i = 0usize;
    loop {
        let c = *s.add(i);
        if c == 0 || in_set(&set, c) {
            return i;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strspn(s: *const u8, accept: *const u8) -> usize {
    let set = byte_set(accept);
    let mut i = 0usize;
    loop {
        let c = *s.add(i);
        if c == 0 || !in_set(&set, c) {
            return i;
        }
        i += 1;
    }
}

#[no_mangle]
pub unsafe extern "C" fn strpbrk(s: *const u8, accept: *const u8) -> *mut u8 {
    let set = byte_set(accept);
    let mut i = 0usize;
    loop {
        let c = *s.add(i);
        if c == 0 {
            return null_mut();
        }
        if in_set(&set, c) {
            return s.add(i) as *mut u8;
        }
        i += 1;
    }
}

// ------------------------------------------------------------
// strstr / memmem (two-way, ported from musl)
// ------------------------------------------------------------

#[inline]
fn bitset_contains(set: &[usize; 4], b: usize) -> bool {
    set[b / 64] & (1usize << (b % 64)) != 0
}

unsafe fn twobyte_memmem(h0: *const u8, k0: usize, n: *const u8) -> *mut u8 {
    let nw = ((*n as u16) << 8) | *n.add(1) as u16;
    let mut hw = ((*h0 as u16) << 8) | *h0.add(1) as u16;
    let mut h = h0.add(2);
    let mut k = k0 - 2;
    while k != 0 {
        if hw == nw { return h.sub(2) as *mut u8; }
        k -= 1;
        hw = (hw << 8) | *h as u16;
        h = h.add(1);
    }
    if hw == nw { h.sub(2) as *mut u8 } else { null_mut() }
}

unsafe fn threebyte_memmem(h0: *const u8, k0: usize, n: *const u8) -> *mut u8 {
    let nw = ((*n as u32) << 24) | ((*n.add(1) as u32) << 16) | ((*n.add(2) as u32) << 8);
    let mut hw = ((*h0 as u32) << 24) | ((*h0.add(1) as u32) << 16) | ((*h0.add(2) as u32) << 8);
    let mut h = h0.add(3);
    let mut k = k0 - 3;
    while k != 0 {
        if hw == nw { return h.sub(3) as *mut u8; }
        k -= 1;
        hw = (hw | *h as u32) << 8;
        h = h.add(1);
    }
    if hw == nw { h.sub(3) as *mut u8 } else { null_mut() }
}

unsafe fn fourbyte_memmem(h0: *const u8, k0: usize, n: *const u8) -> *mut u8 {
    let nw = ((*n as u32) << 24)
        | ((*n.add(1) as u32) << 16)
        | ((*n.add(2) as u32) << 8)
        | *n.add(3) as u32;
    let mut hw = ((*h0 as u32) << 24)
        | ((*h0.add(1) as u32) << 16)
        | ((*h0.add(2) as u32) << 8)
        | *h0.add(3) as u32;
    let mut h = h0.add(4);
    let mut k = k0 - 4;
    while k != 0 {
        if hw == nw { return h.sub(4) as *mut u8; }
        k -= 1;
        hw = (hw << 8) | *h as u32;
        h = h.add(1);
    }
    if hw == nw { h.sub(4) as *mut u8 } else { null_mut() }
}

unsafe fn maximal_suffix(n: *const u8, l: usize, invert: bool) -> (usize, usize) {
    let mut ip = usize::MAX;
    let mut jp = 0usize;
    let mut k = 1usize;
    let mut p = 1usize;
    while jp.wrapping_add(k) < l {
        let a = *n.add(ip.wrapping_add(k));
        let b = *n.add(jp + k);
        let advance = if invert { a < b } else { a > b };
        if a == b {
            if k == p {
                jp += p;
                k = 1;
            } else {
                k += 1;
            }
        } else if advance {
            jp += k;
            k = 1;
            p = jp.wrapping_sub(ip);
        } else {
            ip = jp;
            jp += 1;
            k = 1;
            p = 1;
        }
    }
    (ip, p)
}

unsafe fn twoway_memmem(h0: *const u8, end: *const u8, n: *const u8, l: usize) -> *mut u8 {
    let mut byteset = [0usize; 4];
    let mut shift = [0usize; 256];
    for i in 0..l {
        let b = *n.add(i) as usize;
        byteset[b / 64] |= 1usize << (b % 64);
        shift[b] = i + 1;
    }

    let (ip1, p1) = maximal_suffix(n, l, false);
    let (ip2, p2) = maximal_suffix(n, l, true);
    let ms;
    let mut p;
    if ip2.wrapping_add(1) > ip1.wrapping_add(1) {
        ms = ip2;
        p = p2;
    } else {
        ms = ip1;
        p = p1;
    }

    // Periodic needle?
    let mem0;
    if memcmp(
        n as *const c_void,
        n.add(p) as *const c_void,
        ms.wrapping_add(1),
    ) != 0
    {
        mem0 = 0;
        let tail = l.wrapping_sub(ms).wrapping_sub(1);
        p = (if ms > tail { ms } else { tail }) + 1;
    } else {
        mem0 = l - p;
    }
    let mut mem = 0usize;
    let mut h = h0;
    loop {
        if (end as usize - h as usize) < l {
            return null_mut();
        }
        let last = *h.add(l - 1) as usize;
        if bitset_contains(&byteset, last) {
            let mut k = l - shift[last];
            if k != 0 {
                if k < mem {
                    k = mem;
                }
                h = h.add(k);
                mem = 0;
                continue;
            }
        } else {
            h = h.add(l);
            mem = 0;
            continue;
        }
        let mut k = if ms.wrapping_add(1) > mem {
            ms.wrapping_add(1)
        } else {
            mem
        };
        while k < l && *n.add(k) == *h.add(k) {
            k += 1;
        }
        if k < l {
            h = h.add(k.wrapping_sub(ms));
            mem = 0;
            continue;
        }
        let mut k = ms.wrapping_add(1);
        while k > mem && *n.add(k - 1) == *h.add(k - 1) {
            k -= 1;
        }
        if k <= mem {
            return h as *mut u8;
        }
        h = h.add(p);
        mem = mem0;
    }
}

// Same algorithm for a NUL-terminated haystack.  The end pointer starts at
// the candidate and grows incrementally, so an early match does not force a
// full strlen() of the haystack.
unsafe fn twoway_strstr(h0: *const u8, n: *const u8) -> *mut u8 {
    let mut byteset = [0usize; 4];
    let mut shift = [0usize; 256];
    let mut l = 0usize;
    while *n.add(l) != 0 && *h0.add(l) != 0 {
        let b = *n.add(l) as usize;
        byteset[b / 64] |= 1usize << (b % 64);
        shift[b] = l + 1;
        l += 1;
    }
    if *n.add(l) != 0 {
        return null_mut();
    }

    let (ip1, p1) = maximal_suffix(n, l, false);
    let (ip2, p2) = maximal_suffix(n, l, true);
    let ms;
    let mut p;
    if ip2.wrapping_add(1) > ip1.wrapping_add(1) {
        ms = ip2;
        p = p2;
    } else {
        ms = ip1;
        p = p1;
    }

    let mem0;
    if memcmp(
        n as *const c_void,
        n.add(p) as *const c_void,
        ms.wrapping_add(1),
    ) != 0
    {
        mem0 = 0;
        let tail = l.wrapping_sub(ms).wrapping_sub(1);
        p = (if ms > tail { ms } else { tail }) + 1;
    } else {
        mem0 = l - p;
    }
    let mut mem = 0usize;
    let mut h = h0;
    let mut z = h0;
    loop {
        if (z as usize - h as usize) < l {
            // Never scan past the end of the current page: a NUL-terminated
            // string does not promise bytes beyond its terminator.
            let room = 4096 - (z as usize & 4095);
            let grow = if (l | 63) < room { l | 63 } else { room };
            let z2 = memchr(z, 0, grow);
            if z2.is_null() {
                z = z.add(grow);
            } else {
                z = z2 as *const u8;
                if (z as usize - h as usize) < l {
                    return null_mut();
                }
            }
        }
        let last = *h.add(l - 1) as usize;
        if bitset_contains(&byteset, last) {
            let mut k = l - shift[last];
            if k != 0 {
                if k < mem {
                    k = mem;
                }
                h = h.add(k);
                mem = 0;
                continue;
            }
        } else {
            h = h.add(l);
            mem = 0;
            continue;
        }
        let mut k = if ms.wrapping_add(1) > mem {
            ms.wrapping_add(1)
        } else {
            mem
        };
        while k < l && *n.add(k) != 0 && *n.add(k) == *h.add(k) {
            k += 1;
        }
        if *n.add(k) != 0 {
            h = h.add(k.wrapping_sub(ms));
            mem = 0;
            continue;
        }
        let mut k = ms.wrapping_add(1);
        while k > mem && *n.add(k - 1) == *h.add(k - 1) {
            k -= 1;
        }
        if k <= mem {
            return h as *mut u8;
        }
        h = h.add(p);
        mem = mem0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn memmem(
    haystack: *const c_void, haystacklen: usize,
    needle: *const c_void, needlelen: usize,
) -> *mut c_void {
    if needlelen == 0 { return haystack as *mut c_void; }
    if haystacklen < needlelen { return null_mut(); }
    let h = haystack as *const u8;
    let n = needle as *const u8;
    let first = memchr(h, *n as c_int, haystacklen);
    if first.is_null() {
        return null_mut();
    }
    if needlelen == 1 {
        return first as *mut c_void;
    }
    let off = first as usize - h as usize;
    let h = h.add(off);
    let k = haystacklen - off;
    if k < needlelen {
        return null_mut();
    }
    match needlelen {
        2 => twobyte_memmem(h, k, n) as *mut c_void,
        3 => threebyte_memmem(h, k, n) as *mut c_void,
        4 => fourbyte_memmem(h, k, n) as *mut c_void,
        _ => twoway_memmem(h, h.add(k), n, needlelen) as *mut c_void,
    }
}

#[no_mangle]
pub unsafe extern "C" fn strstr(haystack: *const u8, needle: *const u8) -> *mut u8 {
    if *needle == 0 {
        return haystack as *mut u8;
    }
    let h = strchrnul(haystack as *const c_char, *needle as c_int) as *const u8;
    if *h == 0 {
        return null_mut();
    }
    if *needle.add(1) == 0 {
        return h as *mut u8;
    }
    twoway_strstr(h, needle)
}

#[no_mangle]
pub unsafe extern "C" fn strtok(s: *mut u8, delim: *const u8) -> *mut u8 {
    static mut STATE: *mut u8 = core::ptr::null_mut();
    let mut p = if s.is_null() { STATE } else { s };
    if p.is_null() {
        return core::ptr::null_mut();
    }
    loop {
        let c = *p;
        if c == 0 {
            STATE = core::ptr::null_mut();
            return core::ptr::null_mut();
        }
        let mut is_delim = false;
        let mut i = 0;
        loop {
            let d = *delim.add(i);
            if d == 0 {
                break;
            }
            if c == d {
                is_delim = true;
                break;
            }
            i += 1;
        }
        if !is_delim {
            break;
        }
        p = p.add(1);
    }
    let start = p;
    loop {
        let c = *p;
        if c == 0 {
            STATE = core::ptr::null_mut();
            return start;
        }
        let mut is_delim = false;
        let mut i = 0;
        loop {
            let d = *delim.add(i);
            if d == 0 {
                break;
            }
            if c == d {
                is_delim = true;
                break;
            }
            i += 1;
        }
        if is_delim {
            *p = 0;
            STATE = p.add(1);
            return start;
        }
        p = p.add(1);
    }
}

