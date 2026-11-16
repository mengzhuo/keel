// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Parsing of DNS messages for the ns_* API from <arpa/nameser.h>.

pub const NS_MAXDNAME_V: usize = 1025;
const NS_INT16SZ_V: usize = 2;
const NS_INT32SZ_V: usize = 4;
const NS_S_MAX_V: usize = 4;

#[repr(C)]
pub struct NsMsg {
    pub _msg: *const u8,
    pub _eom: *const u8,
    pub _id: u16,
    pub _flags: u16,
    pub _counts: [u16; NS_S_MAX_V],
    pub _sections: [*const u8; NS_S_MAX_V],
    pub _sect: c_int,
    pub _rrnum: c_int,
    pub _msg_ptr: *const u8,
}

#[repr(C)]
pub struct NsRr {
    pub name: [c_char; NS_MAXDNAME_V],
    pub type_: u16,
    pub rr_class: u16,
    pub ttl: u32,
    pub rdlength: u16,
    __pad: u16,
    pub rdata: *const u8,
}

#[no_mangle]
pub static _ns_flagdata: [[c_int; 2]; 16] = [
    [0x8000, 15],
    [0x7800, 11],
    [0x0400, 10],
    [0x0200, 9],
    [0x0100, 8],
    [0x0080, 7],
    [0x0040, 6],
    [0x0020, 5],
    [0x0010, 4],
    [0x000f, 0],
    [0x0000, 0],
    [0x0000, 0],
    [0x0000, 0],
    [0x0000, 0],
    [0x0000, 0],
    [0x0000, 0],
];

fn ns_offset(base: *const u8, len: usize, p: *const u8) -> Option<usize> {
    let off = (p as usize).checked_sub(base as usize)?;
    if off <= len { Some(off) } else { None }
}

// Skips `count` resource records starting at `start`, returning the number
// of bytes advanced.  All reads go through InBuf, so a truncated message
// turns into None instead of an out-of-bounds access.
fn ns_skiprr_inner(msg: &InBuf, start: usize, section: c_int, mut count: c_int) -> Option<usize> {
    let mut p = start;
    while count > 0 {
        count -= 1;
        p = dns_skip_name(msg, p)?;
        // type + class
        if msg.len() - p < 2 * NS_INT16SZ_V {
            return None;
        }
        p += 2 * NS_INT16SZ_V;
        if section != 0 {
            // ttl + rdlength
            if msg.len() - p < NS_INT32SZ_V + NS_INT16SZ_V {
                return None;
            }
            let rdlen = msg.peek_u16_be(p + NS_INT32SZ_V)? as usize;
            p += NS_INT32SZ_V + NS_INT16SZ_V;
            if rdlen > msg.len() - p {
                return None;
            }
            p += rdlen;
        }
    }
    p.checked_sub(start)
}

#[no_mangle]
pub unsafe extern "C" fn ns_skiprr(ptr: *const u8, eom: *const u8, section: c_int, count: c_int) -> c_int {
    let len = match (eom as usize).checked_sub(ptr as usize) {
        Some(len) => len,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    let msg = match InBuf::new(ptr, len) {
        Some(msg) => msg,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    match ns_skiprr_inner(&msg, 0, section, count) {
        Some(r) if r <= c_int::MAX as usize => r as c_int,
        _ => {
            ERRNO!() = EMSGSIZE;
            -1
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn ns_initparse(msg: *const u8, msglen: c_int, handle: *mut NsMsg) -> c_int {
    if msg.is_null() || handle.is_null() || msglen < 0 {
        ERRNO!() = EMSGSIZE;
        return -1;
    }
    let len = msglen as usize;
    let view = match InBuf::new(msg, len) {
        Some(view) => view,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    (*handle)._msg = msg;
    (*handle)._eom = msg.add(len);
    if len < (2 + NS_S_MAX_V) * NS_INT16SZ_V {
        ERRNO!() = EMSGSIZE;
        return -1;
    }
    (*handle)._id = match view.peek_u16_be(0) {
        Some(v) => v,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    (*handle)._flags = match view.peek_u16_be(2) {
        Some(v) => v,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    for i in 0..NS_S_MAX_V {
        (*handle)._counts[i] = match view.peek_u16_be(4 + 2 * i) {
            Some(v) => v,
            None => {
                ERRNO!() = EMSGSIZE;
                return -1;
            }
        };
    }
    let mut p = (2 + NS_S_MAX_V) * NS_INT16SZ_V;
    for i in 0..NS_S_MAX_V {
        let count = (*handle)._counts[i] as c_int;
        if count != 0 {
            (*handle)._sections[i] = msg.add(p);
            let r = match ns_skiprr_inner(&view, p, i as c_int, count) {
                Some(r) => r,
                None => {
                    ERRNO!() = EMSGSIZE;
                    return -1;
                }
            };
            p = match p.checked_add(r) {
                Some(p) => p,
                None => {
                    ERRNO!() = EMSGSIZE;
                    return -1;
                }
            };
        } else {
            (*handle)._sections[i] = core::ptr::null();
        }
    }
    if p != len {
        ERRNO!() = EMSGSIZE;
        return -1;
    }
    (*handle)._sect = NS_S_MAX_V as c_int;
    (*handle)._rrnum = -1;
    (*handle)._msg_ptr = core::ptr::null();
    0
}

#[no_mangle]
pub unsafe extern "C" fn ns_name_uncompress(
    msg: *const u8,
    eom: *const u8,
    src: *const u8,
    dst: *mut c_char,
    dstsiz: usize,
) -> c_int {
    let r = crate::dn_expand(msg, eom, src, dst, dstsiz as c_int);
    if r < 0 { ERRNO!() = EMSGSIZE; }
    r
}

#[no_mangle]
pub unsafe extern "C" fn ns_parserr(handle: *mut NsMsg, section: c_int, rrnum_in: c_int, rr: *mut NsRr) -> c_int {
    if handle.is_null() || rr.is_null() || section < 0 || section >= NS_S_MAX_V as c_int {
        ERRNO!() = ENODEV;
        return -1;
    }
    let base = (*handle)._msg;
    let len = match ((*handle)._eom as usize).checked_sub(base as usize) {
        Some(len) => len,
        None => {
            ERRNO!() = ENODEV;
            return -1;
        }
    };
    let view = match InBuf::new(base, len) {
        Some(view) => view,
        None => {
            ERRNO!() = ENODEV;
            return -1;
        }
    };
    let sec = section as usize;
    if section != (*handle)._sect {
        (*handle)._sect = section;
        (*handle)._rrnum = 0;
        (*handle)._msg_ptr = (*handle)._sections[sec];
    }
    let mut rrnum = rrnum_in;
    if rrnum == -1 { rrnum = (*handle)._rrnum; }
    if rrnum < 0 || rrnum >= (*handle)._counts[sec] as c_int {
        ERRNO!() = ENODEV;
        return -1;
    }
    if rrnum < (*handle)._rrnum {
        (*handle)._rrnum = 0;
        (*handle)._msg_ptr = (*handle)._sections[sec];
    }
    if rrnum > (*handle)._rrnum {
        let off = match ns_offset(base, len, (*handle)._msg_ptr) {
            Some(off) => off,
            None => {
                ERRNO!() = ENODEV;
                return -1;
            }
        };
        let skip = match ns_skiprr_inner(&view, off, section, rrnum - (*handle)._rrnum) {
            Some(skip) => skip,
            None => {
                ERRNO!() = EMSGSIZE;
                return -1;
            }
        };
        let next = match off.checked_add(skip) {
            Some(next) if next <= len => next,
            _ => {
                ERRNO!() = EMSGSIZE;
                return -1;
            }
        };
        (*handle)._msg_ptr = base.add(next);
        (*handle)._rrnum = rrnum;
    }
    let r = ns_name_uncompress(
        (*handle)._msg,
        (*handle)._eom,
        (*handle)._msg_ptr,
        (*rr).name.as_mut_ptr(),
        NS_MAXDNAME_V,
    );
    if r < 0 { return -1; }
    let mut off = match ns_offset(base, len, (*handle)._msg_ptr) {
        Some(off) => off,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    off = match off.checked_add(r as usize) {
        Some(off) if off <= len => off,
        _ => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    (*handle)._msg_ptr = base.add(off);
    if len - off < 2 * NS_INT16SZ_V {
        ERRNO!() = EMSGSIZE;
        return -1;
    }
    (*rr).type_ = match view.peek_u16_be(off) {
        Some(v) => v,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    (*rr).rr_class = match view.peek_u16_be(off + NS_INT16SZ_V) {
        Some(v) => v,
        None => {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
    };
    off += 2 * NS_INT16SZ_V;
    if section != 0 {
        if len - off < NS_INT32SZ_V + NS_INT16SZ_V {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
        (*rr).ttl = match view.peek_u32_be(off) {
            Some(v) => v,
            None => {
                ERRNO!() = EMSGSIZE;
                return -1;
            }
        };
        (*rr).rdlength = match view.peek_u16_be(off + NS_INT32SZ_V) {
            Some(v) => v,
            None => {
                ERRNO!() = EMSGSIZE;
                return -1;
            }
        };
        off += NS_INT32SZ_V + NS_INT16SZ_V;
        if (*rr).rdlength as usize > len - off {
            ERRNO!() = EMSGSIZE;
            return -1;
        }
        (*rr).rdata = base.add(off);
        off += (*rr).rdlength as usize;
    } else {
        (*rr).ttl = 0;
        (*rr).rdlength = 0;
        (*rr).rdata = core::ptr::null();
    }
    (*handle)._msg_ptr = base.add(off);
    (*handle)._rrnum += 1;
    if (*handle)._rrnum > (*handle)._counts[sec] as c_int {
        (*handle)._sect = section + 1;
        if (*handle)._sect == NS_S_MAX_V as c_int {
            (*handle)._rrnum = -1;
            (*handle)._msg_ptr = core::ptr::null();
        } else {
            (*handle)._rrnum = 0;
        }
    }
    0
}
