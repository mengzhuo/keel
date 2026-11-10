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
    n_brackets: usize,
    brackets: *mut BracketData,
    brackets_cap: usize,
}

struct RegexCContext {
    code: *mut CompiledRegex,
    pos: usize,
    last_atom_start: usize,
    err: c_int,
    extended: bool,
    icase: bool,
}

#[derive(Copy, Clone)]
struct MatchFrame {
    pc: usize,
    sp: usize,
    at_bol: bool,
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

fn case_fold_byte(b: u8) -> u8 {
    if b >= b'a' && b <= b'z' { b - 32 }
    else if b >= b'A' && b <= b'Z' { b + 32 }
    else { b }
}



fn compile_expr(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    start: usize,
    end: usize,
    in_group: bool,
) -> bool {
    let mut i = start;
    let mut alt_start = start;
    let extended = ctx.extended;

    while i < end {
        let b = unsafe { *pat.add(i) as u8 };

        if b == b'|' && extended {
            if !compile_sequence(ctx, pat, plen, alt_start, i) { return false; }
            let jump_pos = ctx.pos;
            if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
            let cur = ctx.pos;
            unsafe { patch32((*ctx.code).code, jump_pos + 1, cur as u32); }
            alt_start = i + 1;
            i += 1;
            continue;
        }

        if b == b')' && extended && in_group {
            if !compile_sequence(ctx, pat, plen, alt_start, i) { return false; }
            return true;
        }

        i += 1;
    }

    compile_sequence(ctx, pat, plen, alt_start, end)
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

        if b == b'|' && extended { return true; }
        if b == b')' && in_group_check(start, end, extended) { return true; }

        if b == b'\\' && i + 1 < end {
            let next = unsafe { *pat.add(i + 1) as u8 };
            if next >= 0x80 { return false; }
            if !extended && next == b'(' {
                i = i.wrapping_add(2);
                if !compile_group(ctx, pat, plen, &mut i, end) { return false; }
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
            i += 1;
            if !compile_group(ctx, pat, plen, &mut i, end) { return false; }
            continue;
        }

        if b == b'[' {
            let atom = ctx.pos;
            if !compile_bracket(ctx, pat, plen, &mut i, end) { return false; }
            ctx.last_atom_start = atom;
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
            let prev_end = ctx.pos;
            let op = unsafe { *(*ctx.code).code.add(prev_start) };
            if op == OP_CHAR || op == OP_BRACKET || op == OP_BOL || op == OP_EOL {
                let prev_len = prev_end - prev_start;
                let mut prev = [0u8; 32];
                for k in 0..prev_len {
                    prev[k] = unsafe { *(*ctx.code).code.add(prev_start + k) };
                }
                ctx.pos = prev_start;

                if b == b'*' {
                    let sp = ctx.pos;
                    if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
                    for k in 0..prev_len { if !emit(ctx, prev[k]) { return false; } }
                    let jmp = ctx.pos;
                    if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
                    let after = ctx.pos;
                    unsafe {
                        patch32((*ctx.code).code, sp + 1, after as u32);
                        patch32((*ctx.code).code, jmp + 1, sp as u32);
                    }
                } else if b == b'+' {
                    let jmp = ctx.pos;
                    if !emit(ctx, OP_JUMP) || !emit32(ctx, 0) { return false; }
                    let sp = ctx.pos;
                    if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
                    for k in 0..prev_len { if !emit(ctx, prev[k]) { return false; } }
                    let after = ctx.pos;
                    unsafe {
                        patch32((*ctx.code).code, jmp + 1, sp as u32);
                        patch32((*ctx.code).code, sp + 1, after as u32);
                    }
                } else {
                    let sp = ctx.pos;
                    if !emit(ctx, OP_SPLIT) || !emit32(ctx, 0) { return false; }
                    for k in 0..prev_len { if !emit(ctx, prev[k]) { return false; } }
                    let after = ctx.pos;
                    unsafe { patch32((*ctx.code).code, sp + 1, after as u32); }
                }
                ctx.last_atom_start = prev_start;
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

fn in_group_check(start: usize, end: usize, extended: bool) -> bool {
    extended && end > start
}

fn compile_group(
    ctx: &mut RegexCContext,
    pat: *const c_char,
    plen: usize,
    i: &mut usize,
    end: usize,
) -> bool {
    let start = *i;
    let mut depth = 1;
    let mut j = start;
    while j < end && depth > 0 {
        let b = unsafe { *pat.add(j) as u8 };
        if b == b'(' { depth += 1; }
        else if b == b')' { depth -= 1; }
        if depth > 0 { j += 1; }
    }
    if depth != 0 { return false; }
    if !compile_expr(ctx, pat, plen, start, j, true) { return false; }
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

fn match_regex_impl(
    code: &CompiledRegex,
    input: *const c_char,
    nmatch: usize,
    pmatch: *mut crate::regmatch_t,
    eflags: c_int,
) -> c_int {
    let frames = match (code.ncode + 1).checked_mul(core::mem::size_of::<MatchFrame>()) {
        Some(n) => n,
        None => return REG_ESPACE,
    };
    let stack = unsafe { crate::malloc(frames) } as *mut MatchFrame;
    if stack.is_null() {
        return REG_ESPACE;
    }
    let r = match_regex_inner(code, input, nmatch, pmatch, eflags, stack);
    unsafe { crate::free(stack as *mut c_void) };
    r
}

fn match_regex_inner(
    code: &CompiledRegex,
    input: *const c_char,
    nmatch: usize,
    pmatch: *mut crate::regmatch_t,
    eflags: c_int,
    stack: *mut MatchFrame,
) -> c_int {
    let slen = unsafe { crate::strlen(input) };
    let notbol = (eflags & REG_NOTBOL) != 0;
    let noteol = (eflags & REG_NOTEOL) != 0;
    let bytecode = code.code;
    let ncode = code.ncode;
    let stack_cap = ncode + 1;

    for start in 0..=slen {
        let mut top: usize = 0;
        unsafe { *stack.add(top) = MatchFrame { pc: 0, sp: start, at_bol: start == 0 }; }
        top += 1;
        let mut found = false;
        let mut match_end = 0usize;

        while top > 0 {
            top -= 1;
            let frame = unsafe { *stack.add(top) };
            let mut pc = frame.pc;
            let mut sp = frame.sp;
            let mut at_bol = frame.at_bol;

            loop {
                if pc >= ncode { break; }
                let op = unsafe { *bytecode.add(pc) };
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
                        if top + 2 <= stack_cap {
                            unsafe {
                                *stack.add(top) = MatchFrame { pc: alt, sp, at_bol };
                                top += 1;
                                *stack.add(top) = MatchFrame { pc: next, sp, at_bol };
                            }
                            top += 1;
                        }
                        break;
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
            if !pmatch.is_null() && nmatch > 0 {
                unsafe {
                    (*pmatch).rm_so = start as crate::regoff_t;
                    (*pmatch).rm_eo = match_end as crate::regoff_t;
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

        let mut ctx = RegexCContext {
            code,
            pos: 0,
            last_atom_start: 0,
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
