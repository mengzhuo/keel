// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Resolver (UDP DNS)
// ============================================================

const MAX_NS: usize = 3;
const MAX_SEARCH: usize = 6;
const HOST_NOT_FOUND: c_int = 1;
const TRY_AGAIN: c_int = 2;
const NO_RECOVERY: c_int = 3;
const NO_DATA: c_int = 4;

static mut NS_COUNT: usize = 0;
static mut NS_ADDRS: [u32; MAX_NS] = [0; MAX_NS];
static mut SEARCH_COUNT: usize = 0;
static mut SEARCH_DOMAINS: [[u8; 256]; MAX_SEARCH] = [[0; 256]; MAX_SEARCH];
static RESOLV_PARSED: AtomicBool = AtomicBool::new(false);
static mut NAME_LOCK: c_int = 0;
static mut NAME_WAITERS: c_int = 0;
static mut RESOLV_INIT_LOCK: c_int = 0;
static mut RESOLV_INIT_WAITERS: c_int = 0;

unsafe fn name_lock() {
    spinlock_lock(&raw mut NAME_LOCK, &raw mut NAME_WAITERS);
}

unsafe fn name_unlock() {
    spinlock_unlock(&raw mut NAME_LOCK, &raw mut NAME_WAITERS);
}

unsafe fn resolv_ensure() {
    if RESOLV_PARSED.load(Ordering::Acquire) {
        return;
    }
    spinlock_lock(&raw mut RESOLV_INIT_LOCK, &raw mut RESOLV_INIT_WAITERS);
    if !RESOLV_PARSED.load(Ordering::Acquire) {
        let _ = resolv_parse_config();
        RESOLV_PARSED.store(true, Ordering::Release);
    }
    spinlock_unlock(&raw mut RESOLV_INIT_LOCK, &raw mut RESOLV_INIT_WAITERS);
}

#[repr(C)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16,
    sin_addr: u32,
    sin_zero: [u8; 8],
}

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe fn resolv_skip_ws(line: &[u8], mut i: usize) -> usize {
    while i < line.len() && (line[i] == b' ' || line[i] == b'\t') { i += 1; }
    i
}

unsafe fn resolv_token<'a>(line: &'a [u8], mut i: usize) -> (&'a [u8], usize) {
    let start = i;
    while i < line.len() && line[i] != b' ' && line[i] != b'\t' && line[i] != b'#' { i += 1; }
    (&line[start..i], i)
}

unsafe fn resolv_parse_config() -> c_int {
    NS_COUNT = 0;
    SEARCH_COUNT = 0;
    let path = b"/etc/resolv.conf\0";
    let fd = crate::open(path.as_ptr() as *const c_char, 0, 0);
    if fd < 0 { return -1; }
    let mut buf = [0u8; 4096];
    let n = crate::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
    crate::close(fd);
    if n <= 0 { return -1; }
    let len = n as usize;
    let mut i = 0;
    while i < len {
        let start = i;
        while i < len && buf[i] != b'\n' { i += 1; }
        let line = &buf[start..i];
        i += 1;
        let mut j = resolv_skip_ws(line, 0);
        if j >= line.len() || line[j] == b'#' { continue; }
        let (key, next) = resolv_token(line, j);
        j = next;
        if key == b"nameserver" {
            j = resolv_skip_ws(line, j);
            let (tok, _) = resolv_token(line, j);
            if !tok.is_empty() && NS_COUNT < MAX_NS {
                let mut name = [0u8; 64];
                let l = core::cmp::min(tok.len(), 63);
                name[..l].copy_from_slice(&tok[..l]);
                let addr = crate::inet_addr(name.as_ptr() as *const c_char);
                if addr != u32::MAX {
                    NS_ADDRS[NS_COUNT] = addr;
                    NS_COUNT += 1;
                }
            }
        } else if key == b"search" || key == b"domain" {
            loop {
                j = resolv_skip_ws(line, j);
                if j >= line.len() || line[j] == b'#' { break; }
                let (tok, next) = resolv_token(line, j);
                j = next;
                if tok.is_empty() { break; }
                if SEARCH_COUNT < MAX_SEARCH {
                    let l = core::cmp::min(tok.len(), 255);
                    SEARCH_DOMAINS[SEARCH_COUNT][..l].copy_from_slice(&tok[..l]);
                    SEARCH_DOMAINS[SEARCH_COUNT][l] = 0;
                    SEARCH_COUNT += 1;
                }
                if key == b"domain" { break; }
            }
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn res_init() -> c_int {
    resolv_ensure();
    0
}

#[no_mangle]
pub unsafe extern "C" fn res_mkquery(
    op: c_int,
    dname: *const c_char,
    class: c_int,
    type_: c_int,
    _data: *const u8,
    _datalen: c_int,
    _newrr: *const u8,
    buf: *mut u8,
    buflen: c_int,
) -> c_int {
    if dname.is_null() || buf.is_null() { return -1; }
    let mut l = crate::strnlen(dname as *const u8, 255);
    if l > 0 && *dname.add(l - 1) == b'.' as c_char { l -= 1; }
    if l > 0 && *dname.add(l - 1) == b'.' as c_char { return -1; }
    let n = 17 + l + if l != 0 { 1 } else { 0 };
    if l > 253 || buflen < n as c_int || op < 0 || op > 15 || class < 0 || class > 255 || type_ < 0 || type_ > 255 {
        return -1;
    }
    let mut q = [0u8; 280];
    crate::memset(q.as_mut_ptr() as *mut c_void, 0, n);
    q[2] = (op * 8 + 1) as u8;
    q[3] = 32;
    q[5] = 1;
    core::ptr::copy_nonoverlapping(dname as *const u8, q.as_mut_ptr().add(13), l);
    let mut i = 13;
    while q[i] != 0 {
        let mut j = i;
        while q[j] != 0 && q[j] != b'.' { j += 1; }
        if j - i > 63 { return -1; }
        q[i - 1] = (j - i) as u8;
        i = j + 1;
    }
    q[i + 1] = type_ as u8;
    q[i + 3] = class as u8;
    let mut ts = timespec { tv_sec: 0, tv_nsec: 0 };
    let _ = clock_gettime(CLOCK_REALTIME, &mut ts);
    let id = (((ts.tv_nsec as u64) + (ts.tv_nsec as u64) / 65536) & 0xffff) as u16;
    q[0] = (id >> 8) as u8;
    q[1] = id as u8;
    core::ptr::copy_nonoverlapping(q.as_ptr(), buf, n);
    n as c_int
}

unsafe fn res_send_inner(msg: *const u8, msglen: c_int, answer: *mut u8, anslen: c_int) -> c_int {
    if msg.is_null() || answer.is_null() || msglen < 12 || anslen < 12 { return -1; }
    resolv_ensure();
    if NS_COUNT == 0 {
        H_ERRNO!() = TRY_AGAIN;
        return -1;
    }
    for n in 0..NS_COUNT {
        let fd = crate::socket(AF_INET, SOCK_DGRAM, 0);
        if fd < 0 { continue; }
        let sa = SockAddrIn {
            sin_family: AF_INET as u16,
            sin_port: 53u16.to_be(),
            sin_addr: NS_ADDRS[n],
            sin_zero: [0; 8],
        };
        let sent = crate::sendto(fd, msg as *const c_void, msglen as usize, 0, &sa as *const SockAddrIn as *const crate::sockaddr, 16);
        if sent != msglen as isize {
            crate::close(fd);
            continue;
        }
        let mut pfd = PollFd { fd, events: 1, revents: 0 };
        let pr = crate::poll(&mut pfd as *mut PollFd as *mut c_void, 1, 5000);
        if pr <= 0 {
            crate::close(fd);
            continue;
        }
        let r = crate::recvfrom(fd, answer as *mut c_void, anslen as usize, 0, core::ptr::null_mut(), core::ptr::null_mut());
        crate::close(fd);
        if r >= 12 {
            let id_ok = *answer == *msg && *answer.add(1) == *msg.add(1);
            let qr = (*answer.add(2) & 0x80) != 0;
            if id_ok && qr {
                return r as c_int;
            }
        }
    }
    H_ERRNO!() = TRY_AGAIN;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn res_send(msg: *const u8, msglen: c_int, answer: *mut u8, anslen: c_int) -> c_int {
    if anslen < 512 {
        let mut buf = [0u8; 512];
        let r = res_send_inner(msg, msglen, buf.as_mut_ptr(), 512);
        if r >= 0 {
            let copy = core::cmp::min(r, anslen) as usize;
            if copy > 0 { core::ptr::copy_nonoverlapping(buf.as_ptr(), answer, copy); }
        }
        return r;
    }
    res_send_inner(msg, msglen, answer, anslen)
}

#[no_mangle]
pub unsafe extern "C" fn res_query(
    dname: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    let mut q = [0u8; 280];
    let ql = res_mkquery(0, dname, class, type_, core::ptr::null(), 0, core::ptr::null(), q.as_mut_ptr(), q.len() as c_int);
    if ql < 0 { return ql; }
    let r = res_send(q.as_ptr(), ql, answer, anslen);
    if r < 12 {
        H_ERRNO!() = TRY_AGAIN;
        return -1;
    }
    let rcode = answer.add(3).read() & 0x0f;
    if rcode == 3 {
        H_ERRNO!() = HOST_NOT_FOUND;
        return -1;
    }
    if rcode == 0 && *answer.add(6) == 0 && *answer.add(7) == 0 {
        H_ERRNO!() = NO_DATA;
        return -1;
    }
    if rcode != 0 {
        H_ERRNO!() = NO_RECOVERY;
        return -1;
    }
    r
}

#[no_mangle]
pub unsafe extern "C" fn res_querydomain(
    name: *const c_char,
    domain: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    if domain.is_null() || *domain == 0 { return res_query(name, class, type_, answer, anslen); }
    let mut buf = [0u8; 512];
    let nl = crate::strlen(name);
    let dl = crate::strlen(domain);
    if nl + 1 + dl >= buf.len() { return -1; }
    core::ptr::copy_nonoverlapping(name, buf.as_mut_ptr() as *mut c_char, nl);
    buf[nl] = b'.';
    core::ptr::copy_nonoverlapping(domain, buf.as_mut_ptr().add(nl + 1) as *mut c_char, dl);
    buf[nl + 1 + dl] = 0;
    res_query(buf.as_ptr() as *const c_char, class, type_, answer, anslen)
}

#[no_mangle]
pub unsafe extern "C" fn res_search(
    name: *const c_char,
    class: c_int,
    type_: c_int,
    answer: *mut u8,
    anslen: c_int,
) -> c_int {
    if name.is_null() { return -1; }
    resolv_ensure();
    let len = crate::strlen(name);
    let mut dots = 0;
    for i in 0..len {
        if *name.add(i) == b'.' as c_char { dots += 1; }
    }
    let trailing_dot = len > 0 && *name.add(len - 1) == b'.' as c_char;
    if trailing_dot || dots >= 1 {
        let r = res_query(name, class, type_, answer, anslen);
        if r >= 0 { return r; }
        if trailing_dot { return -1; }
    }
    for i in 0..SEARCH_COUNT {
        let dom = SEARCH_DOMAINS[i].as_ptr() as *const c_char;
        let r = res_querydomain(name, dom, class, type_, answer, anslen);
        if r >= 0 { return r; }
        if H_ERRNO!() != HOST_NOT_FOUND && H_ERRNO!() != NO_DATA { return -1; }
    }
    if dots == 0 {
        return res_query(name, class, type_, answer, anslen);
    }
    -1
}

// ============================================================
// Host and address resolution
// ============================================================

#[repr(C)]
pub struct HostEnt {
    pub h_name: *mut c_char,
    pub h_aliases: *mut *mut c_char,
    pub h_addrtype: c_int,
    pub h_length: c_int,
    pub h_addr_list: *mut *mut c_char,
}

#[repr(C)]
pub struct AddrInfo {
    pub ai_flags: c_int,
    pub ai_family: c_int,
    pub ai_socktype: c_int,
    pub ai_protocol: c_int,
    pub ai_addrlen: u32,
    pub ai_addr: *mut c_void,
    pub ai_canonname: *mut c_char,
    pub ai_next: *mut AddrInfo,
}

static mut HOST_ENT: HostEnt = HostEnt {
    h_name: core::ptr::null_mut(),
    h_aliases: core::ptr::null_mut(),
    h_addrtype: 0,
    h_length: 0,
    h_addr_list: core::ptr::null_mut(),
};
static mut HOST_NAME: [c_char; 256] = [0; 256];
static mut HOST_ALIASES: [*mut c_char; 8] = [core::ptr::null_mut(); 8];
static mut HOST_ADDR: [u8; 16] = [0; 16];
static mut HOST_ADDR_PTRS: [*mut c_char; 2] = [core::ptr::null_mut(); 2];

// DNS wire-format offsets used by host_from_answer and friends.
const DNS_HDR_LEN: usize = 12;
const DNS_QDCOUNT_OFF: usize = 4;
const DNS_ANCOUNT_OFF: usize = 6;
const DNS_QUESTION_LEN: usize = 4;
const DNS_RR_TYPE_OFF: usize = 0;
const DNS_RR_CLASS_OFF: usize = 2;
const DNS_RR_RDLEN_OFF: usize = 8;
const DNS_RR_RDATA_OFF: usize = 10;
const DNS_CLASS_IN: u16 = 1;
const DNS_TYPE_A: u16 = 1;
const DNS_TYPE_AAAA: u16 = 28;

unsafe fn host_from_answer(answer: *const u8, len: c_int, af: c_int) -> *mut HostEnt {
    if (len as usize) < DNS_HDR_LEN { return core::ptr::null_mut(); }
    let msg = match InBuf::new(answer, len as usize) {
        Some(msg) => msg,
        None => return core::ptr::null_mut(),
    };
    let qd = match msg.peek_u16_be(DNS_QDCOUNT_OFF) {
        Some(v) => v as usize,
        None => return core::ptr::null_mut(),
    };
    let an = match msg.peek_u16_be(DNS_ANCOUNT_OFF) {
        Some(v) => v as usize,
        None => return core::ptr::null_mut(),
    };
    let mut p = DNS_HDR_LEN;
    for _ in 0..qd {
        p = match dns_skip_name(&msg, p) {
            Some(v) => v,
            None => return core::ptr::null_mut(),
        };
        p = match p.checked_add(DNS_QUESTION_LEN) {
            Some(v) if v <= msg.len() => v,
            _ => return core::ptr::null_mut(),
        };
    }
    let want_type: u16 = if af == AF_INET6 { DNS_TYPE_AAAA } else { DNS_TYPE_A };
    let want_len = if af == AF_INET6 { 16 } else { 4 };
    for _ in 0..an {
        p = match dns_skip_name(&msg, p) {
            Some(v) => v,
            None => break,
        };
        let typ = match msg.peek_u16_be(p + DNS_RR_TYPE_OFF) {
            Some(v) => v,
            None => break,
        };
        let class = match msg.peek_u16_be(p + DNS_RR_CLASS_OFF) {
            Some(v) => v,
            None => break,
        };
        let rdlen = match msg.peek_u16_be(p + DNS_RR_RDLEN_OFF) {
            Some(v) => v as usize,
            None => break,
        };
        let rdata_off = match p.checked_add(DNS_RR_RDATA_OFF) {
            Some(v) => v,
            None => break,
        };
        let rdata = match msg.slice_at(rdata_off, rdlen) {
            Some(v) => v,
            None => break,
        };
        if typ == want_type && class == DNS_CLASS_IN && rdata.len() == want_len {
            core::ptr::copy_nonoverlapping(rdata.as_ptr(), HOST_ADDR.as_mut_ptr(), want_len);
            HOST_ENT.h_addrtype = af;
            HOST_ENT.h_length = want_len as c_int;
            HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
            HOST_ADDR_PTRS[1] = core::ptr::null_mut();
            HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
            HOST_ALIASES[0] = core::ptr::null_mut();
            HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
            HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
            H_ERRNO!() = 0;
            return &raw mut HOST_ENT;
        }
        p = match rdata_off.checked_add(rdlen) {
            Some(v) => v,
            None => break,
        };
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyname(name: *const c_char) -> *mut HostEnt {
    gethostbyname2(name, AF_INET)
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyname2(name: *const c_char, af: c_int) -> *mut HostEnt {
    if name.is_null() { H_ERRNO!() = HOST_NOT_FOUND; return core::ptr::null_mut(); }
    let mut addr = [0u8; 16];
    let addrlen = if af == AF_INET6 { 16 } else { 4 };
    if inet_pton(af, name, addr.as_mut_ptr() as *mut c_void) == 1 {
        core::ptr::copy_nonoverlapping(addr.as_ptr(), HOST_ADDR.as_mut_ptr(), addrlen);
        let n = crate::strlen(name);
        let l = core::cmp::min(n, 255);
        core::ptr::copy_nonoverlapping(name, HOST_NAME.as_mut_ptr(), l);
        HOST_NAME[l] = 0;
        HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
        HOST_ALIASES[0] = core::ptr::null_mut();
        HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
        HOST_ENT.h_addrtype = af;
        HOST_ENT.h_length = addrlen as c_int;
        HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
        HOST_ADDR_PTRS[1] = core::ptr::null_mut();
        HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
        H_ERRNO!() = 0;
        return &raw mut HOST_ENT;
    }
    let mut answer = [0u8; 512];
    let qtype = if af == AF_INET6 { 28 } else { 1 };
    let r = res_query(name, 1, qtype, answer.as_mut_ptr(), answer.len() as c_int);
    if r < 0 {
        if H_ERRNO!() == 0 { H_ERRNO!() = TRY_AGAIN; }
        return core::ptr::null_mut();
    }
    let ent = host_from_answer(answer.as_ptr(), r, af);
    if !ent.is_null() {
        let n = crate::strlen(name);
        let l = core::cmp::min(n, 255);
        core::ptr::copy_nonoverlapping(name, HOST_NAME.as_mut_ptr(), l);
        HOST_NAME[l] = 0;
    } else {
        H_ERRNO!() = HOST_NOT_FOUND;
    }
    ent
}

#[no_mangle]
pub unsafe extern "C" fn gethostbyaddr(addr: *const c_void, len: u32, type_: c_int) -> *mut HostEnt {
    // musl implements gethostbyaddr() on top of getnameinfo(), so /etc/hosts
    // and the DNS reverse path behave identically for both APIs.
    let (addr_len, salen) = match type_ {
        AF_INET => (4usize, 16u32),
        AF_INET6 => (16usize, 28u32),
        _ => {
            H_ERRNO!() = HOST_NOT_FOUND;
            return core::ptr::null_mut();
        }
    };
    if addr.is_null() || len as usize != addr_len {
        H_ERRNO!() = HOST_NOT_FOUND;
        return core::ptr::null_mut();
    }
    let mut sa = [0u8; 28];
    *(sa.as_mut_ptr() as *mut u16) = type_ as u16;
    if type_ == AF_INET {
        core::ptr::copy_nonoverlapping(addr as *const u8, sa.as_mut_ptr().add(4), 4);
    } else {
        core::ptr::copy_nonoverlapping(addr as *const u8, sa.as_mut_ptr().add(8), 16);
    }
    let mut name = [0 as c_char; 256];
    let rc = getnameinfo(
        sa.as_ptr() as *const c_void,
        salen,
        name.as_mut_ptr(),
        name.len() as u32,
        core::ptr::null_mut(),
        0,
        0,
    );
    if rc != 0 {
        H_ERRNO!() = match rc {
            EAI_AGAIN_V => TRY_AGAIN,
            EAI_NONAME_V => HOST_NOT_FOUND,
            _ => NO_RECOVERY,
        };
        return core::ptr::null_mut();
    }
    let l = crate::strlen(name.as_ptr());
    if l >= HOST_NAME.len() {
        H_ERRNO!() = NO_RECOVERY;
        return core::ptr::null_mut();
    }
    core::ptr::copy_nonoverlapping(name.as_ptr(), HOST_NAME.as_mut_ptr(), l + 1);
    core::ptr::copy_nonoverlapping(addr as *const u8, HOST_ADDR.as_mut_ptr(), addr_len);
    HOST_ENT.h_name = HOST_NAME.as_mut_ptr();
    HOST_ALIASES[0] = HOST_NAME.as_mut_ptr();
    HOST_ALIASES[1] = core::ptr::null_mut();
    HOST_ENT.h_aliases = HOST_ALIASES.as_mut_ptr();
    HOST_ENT.h_addrtype = type_;
    HOST_ENT.h_length = addr_len as c_int;
    HOST_ADDR_PTRS[0] = HOST_ADDR.as_mut_ptr() as *mut c_char;
    HOST_ADDR_PTRS[1] = core::ptr::null_mut();
    HOST_ENT.h_addr_list = HOST_ADDR_PTRS.as_mut_ptr();
    H_ERRNO!() = 0;
    &raw mut HOST_ENT
}

