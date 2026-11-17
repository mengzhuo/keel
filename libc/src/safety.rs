// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Internal bounds-checked view over C buffers.
//
// The unsafe precondition is established once, when a view is constructed:
// the caller promises that `base..base+len` (InBuf) or `base..base+cap`
// (OutBuf) is valid for the duration of the view.  Every later access goes
// through checked arithmetic and returns None/false instead of reading or
// writing outside the declared range; the remaining unsafe blocks are
// justified by that invariant.
//
// These are internal helpers: do not add #[no_mangle] or public C symbols.

pub(crate) struct InBuf {
    base: *const u8,
    len: usize,
}

impl InBuf {
    /// # Safety
    /// `base` must be non-null and `base..base+len` must be readable and
    /// must not be mutated (through any other pointer) for as long as this
    /// value, or any slice borrowed from it, is used.
    pub(crate) unsafe fn new(base: *const u8, len: usize) -> Option<Self> {
        if base.is_null() || len > isize::MAX as usize {
            return None;
        }
        Some(Self { base, len })
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn peek_u8(&self, off: usize) -> Option<u8> {
        if off >= self.len {
            return None;
        }
        // SAFETY: off < len and the constructor established that the whole
        // range is readable.
        Some(unsafe { *self.base.add(off) })
    }

    pub(crate) fn peek_u16_be(&self, off: usize) -> Option<u16> {
        let end = off.checked_add(2)?;
        if end > self.len {
            return None;
        }
        // SAFETY: off..off+2 is inside the constructor's readable range.
        Some(unsafe { ((*self.base.add(off) as u16) << 8) | *self.base.add(off + 1) as u16 })
    }

    pub(crate) fn peek_u32_be(&self, off: usize) -> Option<u32> {
        let end = off.checked_add(4)?;
        if end > self.len {
            return None;
        }
        // SAFETY: off..off+4 is inside the constructor's readable range.
        Some(unsafe {
            ((*self.base.add(off) as u32) << 24)
                | ((*self.base.add(off + 1) as u32) << 16)
                | ((*self.base.add(off + 2) as u32) << 8)
                | *self.base.add(off + 3) as u32
        })
    }

    pub(crate) fn slice_at(&self, off: usize, n: usize) -> Option<&[u8]> {
        let end = off.checked_add(n)?;
        if end > self.len {
            return None;
        }
        if n == 0 {
            return Some(&[]);
        }
        // SAFETY: off..off+n is inside the constructor's readable range.
        Some(unsafe { core::slice::from_raw_parts(self.base.add(off), n) })
    }

    /// Returns a pointer to `n` readable bytes at `off`, without creating a
    /// Rust reference.  This is used when the destination buffer may alias
    /// the input: a `&[u8]` would make a simultaneous raw write undefined.
    pub(crate) fn ptr_at(&self, off: usize, n: usize) -> Option<*const u8> {
        let end = off.checked_add(n)?;
        if end > self.len {
            return None;
        }
        // SAFETY: off <= len; n == 0 may produce a one-past-end pointer,
        // which is valid as long as it is never dereferenced.
        Some(unsafe { self.base.add(off) })
    }
}

pub(crate) struct OutBuf {
    base: *mut u8,
    cap: usize,
    pos: usize,
}

impl OutBuf {
    /// # Safety
    /// `base` must be non-null and `base..base+cap` must be writable and
    /// must not be accessed (through any other pointer) for as long as this
    /// value is used.
    pub(crate) unsafe fn new(base: *mut u8, cap: usize) -> Option<Self> {
        if base.is_null() || cap > isize::MAX as usize {
            return None;
        }
        Some(Self { base, cap, pos: 0 })
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        // Invariant: pos <= cap, maintained by push() and extend().
        self.cap - self.pos
    }

    pub(crate) fn push(&mut self, b: u8) -> bool {
        if self.pos >= self.cap {
            return false;
        }
        // SAFETY: pos < cap and the constructor established that the whole
        // range is writable.
        unsafe {
            *self.base.add(self.pos) = b;
        }
        self.pos += 1;
        true
    }

    pub(crate) fn extend(&mut self, bytes: &[u8]) -> bool {
        self.extend_from(bytes.as_ptr(), bytes.len())
    }

    pub(crate) fn extend_from(&mut self, src: *const u8, len: usize) -> bool {
        if src.is_null() {
            return false;
        }
        let end = match self.pos.checked_add(len) {
            Some(end) => end,
            None => return false,
        };
        if end > self.cap {
            return false;
        }
        if len != 0 {
            // SAFETY: pos..pos+len is inside the constructor's writable
            // range.  `copy` (memmove semantics) is used so that an
            // overlapping source/destination is still defined.
            unsafe {
                core::ptr::copy(src, self.base.add(self.pos), len);
            }
        }
        self.pos = end;
        true
    }

    pub(crate) fn terminate(&mut self) -> bool {
        self.push(0)
    }
}

// Writes a DNS name as length-prefixed labels separated by dots and
// terminated by NUL.  This encodes the "space includes the NUL" rule once,
// instead of repeating the boundary arithmetic in every caller.
pub(crate) struct NameWriter<'a> {
    out: &'a mut OutBuf,
    wrote_label: bool,
}

impl<'a> NameWriter<'a> {
    pub(crate) fn new(out: &'a mut OutBuf) -> Self {
        Self {
            out,
            wrote_label: false,
        }
    }

    pub(crate) fn label(&mut self, label: &[u8]) -> bool {
        self.label_from(label.as_ptr(), label.len())
    }

    pub(crate) fn label_from(&mut self, src: *const u8, len: usize) -> bool {
        if len == 0 || len > 63 {
            return false;
        }
        if self.wrote_label && !self.out.push(b'.') {
            return false;
        }
        if !self.out.extend_from(src, len) {
            return false;
        }
        self.wrote_label = true;
        true
    }

    pub(crate) fn finish(&mut self) -> bool {
        self.out.terminate()
    }
}

// Skips an encoded DNS name.  Returns the offset just past the name
// (terminator or compression pointer), or None if it is truncated.
pub(crate) fn dns_skip_name(msg: &InBuf, mut p: usize) -> Option<usize> {
    while p < msg.len() {
        let c = msg.peek_u8(p)?;
        if c == 0 {
            return p.checked_add(1);
        }
        if c & 0xc0 == 0xc0 {
            return if msg.len() - p < 2 {
                None
            } else {
                p.checked_add(2)
            };
        }
        let step = 1 + c as usize;
        if step > msg.len() - p {
            return None;
        }
        p = p.checked_add(step)?;
    }
    None
}

// Expands the DNS name at `start` into `out`.
//
// Returns the number of bytes consumed from `start` in the original encoding
// (the value POSIX dn_expand returns), or None on a malformed/truncated name
// or an output buffer that is too small.
//
// `max_steps` bounds the total number of label/pointer steps (dn_expand uses
// the message length), and `max_jumps` bounds compression-pointer jumps.
pub(crate) fn dns_expand_name_at(
    msg: &InBuf,
    start: usize,
    out: &mut OutBuf,
    max_steps: usize,
    max_jumps: usize,
) -> Option<usize> {
    if start >= msg.len() {
        return None;
    }
    let mut p = start;
    let mut consumed: Option<usize> = None;
    let mut writer = NameWriter::new(out);
    let mut steps = 0usize;
    let mut jumps = 0usize;

    while steps < max_steps {
        let c = msg.peek_u8(p)?;
        if c & 0xc0 == 0xc0 {
            jumps = jumps.checked_add(1)?;
            if jumps > max_jumps {
                return None;
            }
            let lo = msg.peek_u8(p.checked_add(1)?)? as usize;
            let target = (((c & 0x3f) as usize) << 8) | lo;
            if consumed.is_none() {
                consumed = Some(p.checked_add(2)?.checked_sub(start)?);
            }
            if target >= msg.len() {
                return None;
            }
            p = target;
        } else if c != 0 {
            let l = c as usize;
            if l > 63 {
                return None;
            }
            let label = msg.ptr_at(p.checked_add(1)?, l)?;
            if !writer.label_from(label, l) {
                return None;
            }
            p = p.checked_add(1 + l)?;
        } else {
            if !writer.finish() {
                return None;
            }
            return Some(match consumed {
                Some(n) => n,
                None => p.checked_add(1)?.checked_sub(start)?,
            });
        }
        steps += 1;
    }
    None
}
