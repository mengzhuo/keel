// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Minimal POSIX regex engine (bytecode VM).
// Supports BRE and ERE: literals, bracket expressions, groups, *, +, ?, |.
// REG_ICASE, REG_EXTENDED, REG_NOSUB, REG_NOTBOL, REG_NOTEOL.
// Unknown backslash sequences are treated as literal next char.
// Bytes >= 0x80 outside brackets return REG_BADPAT.

const REG_EXTENDED: c_int = 1;
const REG_ICASE: c_int = 2;
const REG_NOSUB: c_int = 8;
const REG_NOTBOL: c_int = 1;
const REG_NOTEOL: c_int = 2;

const REG_OK: c_int = 0;
const REG_NOMATCH: c_int = 1;
const REG_BADPAT: c_int = 2;
const REG_ESPACE: c_int = 12;

const OP_CHAR: u8 = 1;
const OP_BRACKET: u8 = 2;
const OP_MATCH: u8 = 3;
const OP_JUMP: u8 = 4;
const OP_SPLIT: u8 = 5;
const OP_BOL: u8 = 6;
const OP_EOL: u8 = 7;
const OP_ANY: u8 = 8;
const OP_SAVE: u8 = 9;

// regex_t layout: re_nsub(8) + __opaque(8) + __padding[4](32) + __nsub2(8) + __padding2(1) = 57 bytes
// __opaque is at offset 8

#[derive(Copy, Clone)]
struct BracketRange {
    min: u8,
    max: u8,
}

struct BracketData {
    negated: bool,
    icase: bool,
    nranges: usize,
    ranges: *mut BracketRange,
    ranges_cap: usize,
}

struct CompiledRegex {
    ncode: usize,
    code: *mut u8,
    code_cap: usize,
    icase: bool,
    extended: bool,
    nosub: bool,
    nsub: usize,
    n_brackets: usize,
    brackets: *mut BracketData,
    brackets_cap: usize,
}

struct RegexCContext {
    code: *mut CompiledRegex,
    pos: usize,
    last_atom_start: usize,
    depth: usize,
    groups: usize,
    err: c_int,
    extended: bool,
    icase: bool,
}

#[derive(Copy, Clone)]
struct MatchFrame {
    pc: usize,
    sp: usize,
    at_bol: bool,
    undo: usize,
}

#[derive(Copy, Clone)]
struct CaptureUndo {
    slot: usize,
    old: usize,
}

struct CaptureState {
    slots: *mut usize,
    nslots: usize,
    undo: *mut CaptureUndo,
    nundo: usize,
    undo_cap: usize,
}

#[inline]
fn is_ascii_letter(b: u8) -> bool {
    (b >= b'a' && b <= b'z') || (b >= b'A' && b <= b'Z')
}

fn bracket_contains(br: &BracketData, b: u8) -> bool {
    let c = if br.icase { case_fold_byte(b) } else { b };
    for i in 0..br.nranges {
        let r = unsafe { &*br.ranges.add(i) };
        if c >= r.min && c <= r.max {
            return !br.negated;
        }
    }
    br.negated
}

fn grow_code(ctx: &mut RegexCContext, need: usize) -> bool {
    let code = unsafe { &mut *ctx.code };
    if need <= code.code_cap {
        return true;
    }
    let mut cap = if code.code_cap == 0 { 128 } else { code.code_cap };
    while cap < need {
        cap *= 2;
    }
    let p = unsafe { crate::realloc(code.code as *mut c_void, cap) } as *mut u8;
    if p.is_null() {
        ctx.err = REG_ESPACE;
        return false;
    }
    code.code = p;
    code.code_cap = cap;
    true
}

fn emit(ctx: &mut RegexCContext, byte: u8) -> bool {
    if !grow_code(ctx, ctx.pos + 1) {
        return false;
    }
    unsafe { *(*ctx.code).code.add(ctx.pos) = byte; }
    ctx.pos += 1;
    true
}

fn emit32(ctx: &mut RegexCContext, val: u32) -> bool {
    emit(ctx, (val & 0xff) as u8)
        && emit(ctx, ((val >> 8) & 0xff) as u8)
        && emit(ctx, ((val >> 16) & 0xff) as u8)
        && emit(ctx, ((val >> 24) & 0xff) as u8)
}

unsafe fn patch32(code: *mut u8, at: usize, val: u32) {
    *code.add(at) = (val & 0xff) as u8;
    *code.add(at + 1) = ((val >> 8) & 0xff) as u8;
    *code.add(at + 2) = ((val >> 16) & 0xff) as u8;
    *code.add(at + 3) = ((val >> 24) & 0xff) as u8;
}

fn emit_char(ctx: &mut RegexCContext, b: u8) -> bool {
    emit(ctx, OP_CHAR) && emit(ctx, b)
}

fn emit_save(ctx: &mut RegexCContext, slot: usize) -> bool {
    emit(ctx, OP_SAVE) && emit32(ctx, slot as u32)
}

fn case_fold_byte(b: u8) -> u8 {
    if b >= b'a' && b <= b'z' { b - 32 }
    else if b >= b'A' && b <= b'Z' { b + 32 }
    else { b }
}



unsafe fn find_top_level_bar(pat: *const c_char, start: usize, end: usize) -> Option<usize> {
    let mut i = start;
    let mut depth = 0i32;
    let mut in_bracket = false;
    while i < end {
        let c = *pat.add(i) as u8;
        if c == b'\\' { i += 2; continue; }
        if in_bracket {
            if c == b']' { in_bracket = false; }
            i += 1;
            continue;
        }
        match c {
            b'[' => in_bracket = true,
            b'(' => depth += 1,
            b')' => { if depth > 0 { depth -= 1; } }
            b'|' if depth == 0 => return Some(i),
            _ => {}
        }
        i += 1;
    }
    None
}

fn compile_expr(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    start: usize,
    end: usize,
    in_group: bool,
) -> bool {
    // Nesting/alternative recursion is caller-controlled; cap it so a pattern
    // like "((((...a...))))" cannot exhaust the C stack in regcomp().
    const MAX_DEPTH: usize = 256;
    if ctx.depth >= MAX_DEPTH {
        ctx.err = REG_ESPACE;
        return false;
    }
    ctx.depth += 1;
    let ok = compile_expr_inner(ctx, pat, plen, start, end, in_group);
    ctx.depth -= 1;
    ok
}

fn compile_expr_inner(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    start: usize,
    end: usize,
    _in_group: bool,
) -> bool {
    // A quantifier may only apply to an atom compiled in this branch, never
    // to code left over from an earlier alternative or an enclosing group.
    ctx.last_atom_start = ctx.pos;
    if ctx.extended {
        if let Some(bar) = unsafe { find_top_level_bar(pat, start, end) } {
            let split = ctx.pos;
            if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
            if !compile_sequence(ctx, pat, plen, start, bar) { return false; }
            let jump = ctx.pos;
            if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
            let alt = ctx.pos;
            unsafe { patch32((*ctx.code).code, split + 1, alt as u32); }
            if !compile_expr(ctx, pat, plen, bar + 1, end, false) { return false; }
            let after = ctx.pos;
            unsafe { patch32((*ctx.code).code, jump + 1, after as u32); }
            return true;
        }
    }
    compile_sequence(ctx, pat, plen, start, end)
}

unsafe fn emit_bytes(ctx: &mut RegexCContext, src: *const u8, len: usize) -> bool {
    if !grow_code(ctx, ctx.pos + len) { return false; }
    core::ptr::copy_nonoverlapping(src, (*ctx.code).code.add(ctx.pos), len);
    ctx.pos += len;
    true
}

/// Adjust absolute jump/split targets inside a copied atom.  Quantifiers move
/// the atom by emitting a split in front of it; without this, patterns like
/// `(a|b)*` keep the old targets and match the wrong code (or loop forever).
unsafe fn relocate_ops(code: *mut u8, start: usize, len: usize, delta: u32) {
    if delta == 0 { return; }
    let end = start + len;
    let mut at = start;
    while at < end {
        let op = *code.add(at);
        let step = match op {
            OP_CHAR => 2,
            OP_BRACKET | OP_JUMP | OP_SPLIT | OP_SAVE => 5,
            _ => 1,
        };
        if (op == OP_JUMP || op == OP_SPLIT) && at + 5 <= end {
            let old = read32(code, at + 1);
            patch32(code, at + 1, old.wrapping_add(delta as usize) as u32);
        }
        at += step;
    }
}

fn emit_quantifier(
    ctx: &mut RegexCContext,
    prev: *const u8,
    len: usize,
    kind: u8,
    atom_start: usize,
) -> bool {
    unsafe {
        if kind == b'*' {
            let split = ctx.pos;
            if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
            let new_start = ctx.pos;
            if !emit_bytes(ctx, prev, len) { return false; }
            relocate_ops((*ctx.code).code, new_start, len, (new_start - atom_start) as u32);
            let jump = ctx.pos;
            if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
            let after = ctx.pos;
            patch32((*ctx.code).code, split + 1, after as u32);
            patch32((*ctx.code).code, jump + 1, split as u32);
        } else if kind == b'+' {
            let atom = ctx.pos;
            if !emit_bytes(ctx, prev, len) { return false; }
            relocate_ops((*ctx.code).code, atom, len, (atom - atom_start) as u32);
            let split = ctx.pos;
            if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
            let jump = ctx.pos;
            if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
            let after = ctx.pos;
            patch32((*ctx.code).code, split + 1, after as u32);
            patch32((*ctx.code).code, jump + 1, atom as u32);
        } else {
            let split = ctx.pos;
            if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
            let new_start = ctx.pos;
            if !emit_bytes(ctx, prev, len) { return false; }
            relocate_ops((*ctx.code).code, new_start, len, (new_start - atom_start) as u32);
            let after = ctx.pos;
            patch32((*ctx.code).code, split + 1, after as u32);
        }
    }
    true
}

fn compile_sequence(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    start: usize,
    end: usize,
) -> bool {
    let mut i = start;
    let extended = ctx.extended;

    while i < end {
        let b = unsafe { *pat.add(i) as u8 };

        // compile_expr() splits top-level alternatives into separate ranges;
        // a bar that reaches here is inside a bracket or a malformed branch.
        if b == b'|' && extended { return true; }

        if b == b'\\' && i + 1 < end {
            let next = unsafe { *pat.add(i + 1) as u8 };
            if next >= 0x80 { return false; }
            if !extended && next == b'(' {
                let atom = ctx.pos;
                i = i.wrapping_add(2);
                if !compile_group(ctx, pat, plen, &mut i, end) { return false; }
                ctx.last_atom_start = atom;
                continue;
            }
            if !extended && next == b')' { return true; }
            let atom = ctx.pos;
            if !emit_char(ctx, next) { return false; }
            ctx.last_atom_start = atom;
            i = i.wrapping_add(2);
            continue;
        }

        if b == b'(' && extended {
            let atom = ctx.pos;
            i += 1;
            if !compile_group(ctx, pat, plen, &mut i, end) { return false; }
            ctx.last_atom_start = atom;
            continue;
        }

        if b == b'[' {
            let atom = ctx.pos;
            if !compile_bracket(ctx, pat, plen, &mut i, end) { return false; }
            ctx.last_atom_start = atom;
            continue;
        }

        if b == b'.' {
            let atom = ctx.pos;
            if !emit(ctx, OP_ANY) { return false; }
            ctx.last_atom_start = atom;
            i += 1;
            continue;
        }

        if b == b'^' && i == start {
            let atom = ctx.pos;
            if !emit(ctx, OP_BOL) { return false; }
            ctx.last_atom_start = atom;
            i += 1;
            continue;
        }
        if b == b'$' && i + 1 == end {
            let atom = ctx.pos;
            if !emit(ctx, OP_EOL) { return false; }
            ctx.last_atom_start = atom;
            i += 1;
            continue;
        }

        if (b == b'*' || b == b'+' || b == b'?') && ctx.last_atom_start < ctx.pos {
            let prev_start = ctx.last_atom_start;
            let prev_len = ctx.pos - prev_start;
            let op = unsafe { *(*ctx.code).code.add(prev_start) };
            if op == OP_CHAR || op == OP_BRACKET || op == OP_ANY || op == OP_BOL
                || op == OP_EOL || op == OP_SAVE
            {
                let prev = unsafe { crate::malloc(prev_len) as *mut u8 };
                if prev.is_null() { return false; }
                unsafe {
                    core::ptr::copy_nonoverlapping(
                        (*ctx.code).code.add(prev_start), prev, prev_len);
                }
                ctx.pos = prev_start;
                let ok = emit_quantifier(ctx, prev, prev_len, b, prev_start);
                unsafe { crate::free(prev as *mut c_void); }
                if !ok { return false; }
                // A second quantifier applies to nothing (a** / a*?), so make
                // the last-atom marker invalid and let it be a literal.
                ctx.last_atom_start = ctx.pos;
                i += 1;
                continue;
            }
        }

        if b >= 0x80 { return false; }
        let atom = ctx.pos;
        if !emit_char(ctx, b) { return false; }
        ctx.last_atom_start = atom;
        i += 1;
    }
    true
}

fn compile_group(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    i: &mut usize,
    end: usize,
) -> bool {
    let start = *i;
    let mut depth = 1i32;
    let mut in_bracket = false;
    let mut j = start;
    while j < end && depth > 0 {
        let b = unsafe { *pat.add(j) as u8 };
        if b == b'\\' { j += 2; continue; }
        if in_bracket {
            if b == b']' { in_bracket = false; }
            j += 1;
            continue;
        }
        if b == b'[' { in_bracket = true; }
        else if b == b'(' { depth += 1; }
        else if b == b')' { depth -= 1; }
        if depth > 0 { j += 1; }
    }
    if depth != 0 { return false; }
    ctx.groups += 1;
    let group = ctx.groups;
    if !emit_save(ctx, group * 2) { return false; }
    if !compile_expr(ctx, pat, plen, start, j, true) { return false; }
    if !emit_save(ctx, group * 2 + 1) { return false; }
    *i = j + 1;
    true
}

fn add_range(ctx: &mut RegexCContext, bi: usize, min: u8, max: u8) -> bool {
    let bd = unsafe { &mut *(*ctx.code).brackets.add(bi) };
    if bd.nranges == bd.ranges_cap {
        let cap = if bd.ranges_cap == 0 { 16 } else { bd.ranges_cap * 2 };
        let p = unsafe {
            crate::realloc(
                bd.ranges as *mut c_void,
                cap * core::mem::size_of::<BracketRange>(),
            )
        } as *mut BracketRange;
        if p.is_null() {
            ctx.err = REG_ESPACE;
            return false;
        }
        bd.ranges = p;
        bd.ranges_cap = cap;
    }
    unsafe { *bd.ranges.add(bd.nranges) = BracketRange { min, max }; }
    bd.nranges += 1;
    true
}

fn compile_bracket(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    _plen: usize,
    i: &mut usize,
    end: usize,
) -> bool {
    let bi = unsafe { (*ctx.code).n_brackets };
    unsafe {
        let code = &mut *ctx.code;
        if bi == code.brackets_cap {
            let cap = if code.brackets_cap == 0 { 8 } else { code.brackets_cap * 2 };
            let p = crate::realloc(
                code.brackets as *mut c_void,
                cap * core::mem::size_of::<BracketData>(),
            ) as *mut BracketData;
            if p.is_null() {
                ctx.err = REG_ESPACE;
                return false;
            }
            code.brackets = p;
            code.brackets_cap = cap;
        }
        let bd = &mut *code.brackets.add(bi);
        bd.negated = false;
        bd.icase = ctx.icase;
        bd.nranges = 0;
        bd.ranges = core::ptr::null_mut();
        bd.ranges_cap = 0;
        code.n_brackets += 1;
    }

    let mut negated = false;
    let mut j = *i + 1;
    if j >= end { return false; }
    let first = unsafe { *pat.add(j) as u8 };
    if first == b'^' { negated = true; j += 1; }

    while j < end {
        let cb = unsafe { *pat.add(j) as u8 };
        if cb == b']' && j > *i + 1 + (if negated { 1 } else { 0 }) { break; }
        if cb >= 0x80 { return false; }
        let lo = cb;
        j += 1;

        if j + 1 < end && unsafe { *pat.add(j) as u8 } == b'-' {
            let dash_next = unsafe { *pat.add(j + 1) as u8 };
            if dash_next == b']' {
                if !add_range(ctx, bi, lo, lo) { return false; }
                if ctx.icase && is_ascii_letter(lo) {
                    let fc = case_fold_byte(lo);
                    if !add_range(ctx, bi, fc, fc) { return false; }
                }
                continue;
            }
            if dash_next >= 0x80 { return false; }
            let hi = dash_next;
            if lo > hi { return false; }
            j += 2;
            if !add_range(ctx, bi, lo, hi) { return false; }
            if ctx.icase {
                let flo = case_fold_byte(lo);
                let fhi = case_fold_byte(hi);
                if flo != lo || fhi != hi {
                    let (rmin, rmax) = if flo < fhi { (flo, fhi) } else { (fhi, flo) };
                    if !add_range(ctx, bi, rmin, rmax) { return false; }
                }
            }
        } else {
            if !add_range(ctx, bi, lo, lo) { return false; }
            if ctx.icase && is_ascii_letter(lo) {
                let f = case_fold_byte(lo);
                if !add_range(ctx, bi, f, f) { return false; }
            }
        }
    }
    if j >= end || unsafe { *pat.add(j) as u8 } != b']' { return false; }
    *i = j + 1;

    unsafe {
        let bd = &mut *(*ctx.code).brackets.add(bi);
        bd.negated = negated;
        bd.icase = ctx.icase;
    }

    if !emit(ctx, OP_BRACKET) { return false; }
    if !emit32(ctx, bi as u32) { return false; }
    true
}

#[inline]
unsafe fn read32(code: *const u8, at: usize) -> usize {
    (*code.add(at) as usize)
        | ((*code.add(at + 1) as usize) << 8)
        | ((*code.add(at + 2) as usize) << 16)
        | ((*code.add(at + 3) as usize) << 24)
}

const CAPTURE_NONE: usize = usize::MAX;

impl CaptureState {
    unsafe fn save(&mut self, slot: usize, value: usize) -> bool {
        if slot >= self.nslots { return true; }
        if self.nundo == self.undo_cap {
            let cap = if self.undo_cap == 0 { 32 } else { self.undo_cap * 2 };
            let p = crate::realloc(
                self.undo as *mut c_void,
                cap * core::mem::size_of::<CaptureUndo>(),
            ) as *mut CaptureUndo;
            if p.is_null() { return false; }
            self.undo = p;
            self.undo_cap = cap;
        }
        *self.undo.add(self.nundo) = CaptureUndo { slot, old: *self.slots.add(slot) };
        self.nundo += 1;
        *self.slots.add(slot) = value;
        true
    }

    unsafe fn restore(&mut self, target: usize) {
        while self.nundo > target {
            self.nundo -= 1;
            let u = *self.undo.add(self.nundo);
            *self.slots.add(u.slot) = u.old;
        }
    }

    unsafe fn reset(&mut self) {
        self.nundo = 0;
        for i in 0..self.nslots {
            *self.slots.add(i) = CAPTURE_NONE;
        }
    }
}

impl Drop for CaptureState {
    fn drop(&mut self) {
        unsafe {
            if !self.slots.is_null() {
                crate::free(self.slots as *mut c_void);
            }
            if !self.undo.is_null() {
                crate::free(self.undo as *mut c_void);
            }
        }
    }
}

struct MatchStack {
    frames: *mut MatchFrame,
    cap: usize,
}

/// Per-bytecode "last input position" table used to break empty-match loops
/// (`()*`, `(a*)*`, `^*`): re-entering the same split without consuming input
/// cannot discover a new match.
struct Visited {
    p: *mut usize,
}

impl Drop for Visited {
    fn drop(&mut self) {
        unsafe {
            if !self.p.is_null() {
                crate::free(self.p as *mut c_void);
            }
        }
    }
}

fn match_regex_impl(
    code: &CompiledRegex,
    input: *const c_char,
    nmatch: usize,
    pmatch: *mut crate::regmatch_t,
    eflags: c_int,
) -> c_int {
    let initial = (code.ncode + 1).max(8);
    let bytes = match initial.checked_mul(core::mem::size_of::<MatchFrame>()) {
        Some(n) => n,
        None => return REG_ESPACE,
    };
    let frames = unsafe { crate::malloc(bytes) } as *mut MatchFrame;
    if frames.is_null() {
        return REG_ESPACE;
    }
    let mut stack = MatchStack { frames, cap: initial };
    let r = match_regex_inner(code, input, nmatch, pmatch, eflags, &mut stack);
    unsafe { crate::free(stack.frames as *mut c_void) };
    r
}

fn match_regex_inner(
    code: &CompiledRegex,
    input: *const c_char,
    nmatch: usize,
    pmatch: *mut crate::regmatch_t,
    eflags: c_int,
    stack: &mut MatchStack,
) -> c_int {
    let slen = unsafe { crate::strlen(input) };
    let notbol = (eflags & REG_NOTBOL) != 0;
    let noteol = (eflags & REG_NOTEOL) != 0;
    let bytecode = code.code;
    let ncode = code.ncode;

    let vbytes = match ncode.checked_mul(core::mem::size_of::<usize>()) {
        Some(n) => n,
        None => return REG_ESPACE,
    };
    let vp = unsafe { crate::malloc(vbytes) } as *mut usize;
    if vp.is_null() {
        return REG_ESPACE;
    }
    let visited = Visited { p: vp };

    let nslots = match code.nsub.checked_add(1).and_then(|n| n.checked_mul(2)) {
        Some(n) => n,
        None => return REG_ESPACE,
    };
    let slots = unsafe { crate::malloc(nslots * core::mem::size_of::<usize>()) } as *mut usize;
    if slots.is_null() { return REG_ESPACE; }
    let undo_cap = 32usize;
    let undo = unsafe {
        crate::malloc(undo_cap * core::mem::size_of::<CaptureUndo>())
    } as *mut CaptureUndo;
    if undo.is_null() {
        unsafe { crate::free(slots as *mut c_void) };
        return REG_ESPACE;
    }
    let mut caps = CaptureState { slots, nslots, undo, nundo: 0, undo_cap };

    for start in 0..=slen {
        let mut top: usize = 0;
        unsafe { caps.reset() };
        for i in 0..ncode {
            unsafe { *visited.p.add(i) = CAPTURE_NONE };
        }
        unsafe {
            *stack.frames.add(top) = MatchFrame {
                pc: 0,
                sp: start,
                at_bol: start == 0,
                undo: 0,
            };
        }
        top += 1;
        let mut found = false;
        let mut match_end = 0usize;

        while top > 0 {
            top -= 1;
            let frame = unsafe { *stack.frames.add(top) };
            unsafe { caps.restore(frame.undo) };
            let mut pc = frame.pc;
            let mut sp = frame.sp;
            let mut at_bol = frame.at_bol;

            loop {
                if pc >= ncode { break; }
                let op = unsafe { *bytecode.add(pc) };
                // Never read an opcode's operand past the bytecode buffer.
                match op {
                    OP_CHAR if pc + 2 > ncode => break,
                    OP_BRACKET | OP_JUMP | OP_SPLIT | OP_SAVE if pc + 5 > ncode => break,
                    _ => {}
                }
                match op {
                    OP_CHAR => {
                        if sp >= slen { break; }
                        let expected = unsafe { *bytecode.add(pc + 1) };
                        let actual = unsafe { *input.add(sp) as u8 };
                        let m = if code.icase && is_ascii_letter(expected) && is_ascii_letter(actual) {
                            case_fold_byte(expected) == case_fold_byte(actual)
                        } else {
                            expected == actual
                        };
                        if !m { break; }
                        sp += 1;
                        at_bol = false;
                        pc += 2;
                    }
                    OP_BRACKET => {
                        if sp >= slen { break; }
                        let bi = unsafe { read32(bytecode, pc + 1) };
                        let actual = unsafe { *input.add(sp) as u8 };
                        if bi >= code.n_brackets { break; }
                        if !bracket_contains(unsafe { &*code.brackets.add(bi) }, actual) { break; }
                        sp += 1;
                        at_bol = false;
                        pc += 5;
                    }
                    OP_ANY => {
                        if sp >= slen { break; }
                        sp += 1;
                        at_bol = false;
                        pc += 1;
                    }
                    OP_MATCH => {
                        if !noteol || sp == slen {
                            found = true;
                            match_end = sp;
                        }
                        break;
                    }
                    OP_JUMP => {
                        pc = unsafe { read32(bytecode, pc + 1) };
                    }
                    OP_SPLIT => {
                        let alt = unsafe { read32(bytecode, pc + 1) };
                        let next = pc + 5;
                        // Empty-iteration guard: the same split at the same
                        // input position would repeat forever.
                        if unsafe { *visited.p.add(pc) } == sp {
                            break;
                        }
                        unsafe { *visited.p.add(pc) = sp };
                        if top + 2 > stack.cap {
                            let mut cap = stack.cap;
                            while cap < top + 2 {
                                cap = match cap.checked_mul(2) {
                                    Some(n) => n,
                                    None => return REG_ESPACE,
                                };
                            }
                            let bytes = match cap.checked_mul(core::mem::size_of::<MatchFrame>()) {
                                Some(n) => n,
                                None => return REG_ESPACE,
                            };
                            let p = unsafe {
                                crate::realloc(stack.frames as *mut c_void, bytes)
                            } as *mut MatchFrame;
                            if p.is_null() {
                                return REG_ESPACE;
                            }
                            stack.frames = p;
                            stack.cap = cap;
                        }
                        unsafe {
                            *stack.frames.add(top) = MatchFrame {
                                pc: alt,
                                sp,
                                at_bol,
                                undo: caps.nundo,
                            };
                            top += 1;
                            *stack.frames.add(top) = MatchFrame {
                                pc: next,
                                sp,
                                at_bol,
                                undo: caps.nundo,
                            };
                        }
                        top += 1;
                        break;
                    }
                    OP_SAVE => {
                        let slot = unsafe { read32(bytecode, pc + 1) };
                        if !unsafe { caps.save(slot, sp) } {
                            return REG_ESPACE;
                        }
                        pc += 5;
                    }
                    OP_BOL => {
                        if !at_bol && !(start == 0 && !notbol) { break; }
                        pc += 1;
                    }
                    OP_EOL => {
                        if sp != slen { break; }
                        pc += 1;
                    }
                    _ => { break; }
                }
            }
            if found { break; }
        }

        if found {
            if !code.nosub && !pmatch.is_null() && nmatch > 0 {
                let count = nmatch.min(code.nsub + 1);
                unsafe {
                    *caps.slots.add(0) = start;
                    *caps.slots.add(1) = match_end;
                    for i in 0..count {
                        let so = *caps.slots.add(i * 2);
                        let eo = *caps.slots.add(i * 2 + 1);
                        let m = pmatch.add(i);
                        if so == CAPTURE_NONE || eo == CAPTURE_NONE {
                            (*m).rm_so = -1;
                            (*m).rm_eo = -1;
                        } else {
                            (*m).rm_so = so as crate::regoff_t;
                            (*m).rm_eo = eo as crate::regoff_t;
                        }
                    }
                }
            }
            return REG_OK;
        }
    }
    REG_NOMATCH
}

unsafe fn free_compiled(code: *mut CompiledRegex) {
    if code.is_null() { return; }
    let c = &*code;
    if !c.brackets.is_null() {
        let mut i = 0usize;
        while i < c.n_brackets {
            let bd = &*c.brackets.add(i);
            if !bd.ranges.is_null() {
                crate::free(bd.ranges as *mut c_void);
            }
            i += 1;
        }
        crate::free(c.brackets as *mut c_void);
    }
    if !c.code.is_null() {
        crate::free(c.code as *mut c_void);
    }
    crate::free(code as *mut c_void);
}

#[no_mangle]
pub unsafe extern "C" fn regcomp(preg: *mut crate::regex_t, pattern: *const c_char, cflags: c_int) -> c_int {
    if preg.is_null() || pattern.is_null() { return REG_BADPAT; }
    let code = crate::malloc(core::mem::size_of::<CompiledRegex>()) as *mut CompiledRegex;
    if code.is_null() { return REG_ESPACE; }
    crate::memset(code as *mut c_void, 0, core::mem::size_of::<CompiledRegex>());
    let rc = compile_regex_into(pattern, cflags, code);
    if rc != REG_OK {
        free_compiled(code);
        return rc;
    }
    crate::memset(preg as *mut c_void, 0, core::mem::size_of::<crate::regex_t>());
    (*preg).re_nsub = (*code).nsub;
    (*preg).__nsub2 = (*code).nsub;
    (*preg).__opaque = code as *mut c_void;
    REG_OK
}

fn compile_regex_into(pat: *const c_char, flags: c_int, code: *mut CompiledRegex) -> c_int {
    unsafe {
        let plen = crate::strlen(pat);
        if plen == 0 { return REG_BADPAT; }

        crate::memset(code as *mut c_void, 0, core::mem::size_of::<CompiledRegex>());
        (*code).icase = (flags & REG_ICASE) != 0;
        (*code).extended = (flags & REG_EXTENDED) != 0;
        (*code).nosub = (flags & REG_NOSUB) != 0;

        let mut ctx = RegexCContext {
            code,
            pos: 0,
            last_atom_start: 0,
            depth: 0,
            groups: 0,
            err: REG_OK,
            extended: (*code).extended,
            icase: (*code).icase,
        };

        if !compile_expr(&mut ctx, pat, plen, 0, plen, false) {
            return if ctx.err != 0 { ctx.err } else { REG_BADPAT };
        }

        if !emit(&mut ctx, OP_MATCH) {
            return ctx.err;
        }

        (*code).ncode = ctx.pos;
        (*code).nsub = ctx.groups;
        REG_OK
    }
}

#[no_mangle]
pub unsafe extern "C" fn regexec(
    preg: *const crate::regex_t,
    string: *const c_char,
    nmatch: usize,
    pmatch: *mut crate::regmatch_t,
    eflags: c_int,
) -> c_int {
    if preg.is_null() || string.is_null() { return REG_BADPAT; }
    let code = (*preg).__opaque as *const CompiledRegex;
    if code.is_null() { return REG_BADPAT; }
    match_regex_impl(&*code, string, nmatch, pmatch, eflags)
}

const ERR_MSG: &[u8] = b"unknown error\0";
const PAT_MSG: &[u8] = b"invalid pattern\0";
const OK_MSG: &[u8] = b"no error\0";

#[no_mangle]
pub unsafe extern "C" fn regerror(
    errcode: c_int,
    _preg: *const crate::regex_t,
    errbuf: *mut c_char,
    errbuf_size: usize,
) -> usize {
    let msg = match errcode {
        0 => OK_MSG,
        2 | 4 | 5 | 6 | 7 | 8 | 9 | 10 | 11 | 13 => PAT_MSG,
        12 => b"out of memory\0" as &[u8],
        _ => ERR_MSG,
    };
    let len = crate::strlen(msg.as_ptr() as *const c_char);
    if !errbuf.is_null() && errbuf_size > 0 {
        let copy = if len < errbuf_size { len } else { errbuf_size - 1 };
        crate::memcpy(errbuf as *mut c_void, msg.as_ptr() as *const c_void, copy);
        *errbuf.add(copy) = 0;
    }
    len + 1
}

#[no_mangle]
pub unsafe extern "C" fn regfree(preg: *mut crate::regex_t) {
    if preg.is_null() { return; }
    let code = (*preg).__opaque as *mut CompiledRegex;
    if !code.is_null() {
        free_compiled(code);
        (*preg).__opaque = core::ptr::null_mut();
    }
}
