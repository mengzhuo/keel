// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Search: hcreate/hdestroy/hsearch (hash table)
// ============================================================

const HSEARCH_MINSIZE: usize = 8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct HSearchEntry {
    key: *mut c_char,
    data: *mut c_void,
}

#[repr(C)]
struct HTab {
    entries: *mut HSearchEntry,
    mask: usize,
    used: usize,
}

#[repr(C)]
pub struct HSearchData {
    tab: *mut HTab,
    __unused1: c_uint,
    __unused2: c_uint,
}

// Single static hash table for the non-_r variants.
static mut HTAB_DATA: HSearchData = HSearchData {
    tab: core::ptr::null_mut(),
    __unused1: 0,
    __unused2: 0,
};

unsafe fn keyhash(k: *const c_char) -> usize {
    let mut h: usize = 0;
    let mut p = k;
    while *p != 0 {
        h = h.wrapping_mul(31).wrapping_add(*p as usize);
        p = p.add(1);
    }
    h
}

unsafe fn htab_lookup(key: *const c_char, hash: usize, htab: *mut HSearchData) -> *mut HSearchEntry {
    let tab = (*htab).tab;
    let mask = (*tab).mask;
    let entries = (*tab).entries;
    let mut i = hash;
    let mut j: usize = 1;
    loop {
        let e = entries.add(i & mask);
        if (*e).key.is_null() || strcmp((*e).key as *const u8, key as *const u8) == 0 {
            return e;
        }
        i = i.wrapping_add(j);
        j += 1;
    }
}

unsafe fn htab_resize(nel: usize, htab: *mut HSearchData) -> c_int {
    let tab = (*htab).tab;
    let old_entries = (*tab).entries;
    let old_size = (*tab).mask + 1;

    let mut newsize: usize = HSEARCH_MINSIZE;
    while newsize < nel {
        let Some(next) = newsize.checked_mul(2) else {
            ERRNO!() = ENOMEM;
            return 0;
        };
        newsize = next;
    }

    let new_entries = calloc(newsize, core::mem::size_of::<HSearchEntry>()) as *mut HSearchEntry;
    if new_entries.is_null() {
        ERRNO!() = ENOMEM;
        return 0;
    }
    (*tab).entries = new_entries;
    (*tab).mask = newsize - 1;

    if !old_entries.is_null() {
        for idx in 0..old_size {
            let old_e = old_entries.add(idx);
            if !(*old_e).key.is_null() {
                let h = keyhash((*old_e).key);
                let mut i = h;
                let mut j: usize = 1;
                loop {
                    let e = new_entries.add(i & (*tab).mask);
                    if (*e).key.is_null() {
                        *e = *old_e;
                        break;
                    }
                    i = i.wrapping_add(j);
                    j += 1;
                }
            }
        }
        free(old_entries as *mut c_void);
    }
    1
}

unsafe fn hcreate_r_impl(nel: usize, htab: *mut HSearchData) -> c_int {
    if nel == 0 || nel > (1usize << 30) {
        ERRNO!() = ENOMEM;
        return 0;
    }
    (*htab).tab = calloc(1, core::mem::size_of::<HTab>()) as *mut HTab;
    if (*htab).tab.is_null() {
        ERRNO!() = ENOMEM;
        return 0;
    }
    if htab_resize(nel, htab) == 0 {
        free((*htab).tab as *mut c_void);
        (*htab).tab = core::ptr::null_mut();
        return 0;
    }
    1
}

unsafe fn hdestroy_r_impl(htab: *mut HSearchData) {
    if !(*htab).tab.is_null() {
        if !(*(*htab).tab).entries.is_null() {
            free((*(*htab).tab).entries as *mut c_void);
        }
        free((*htab).tab as *mut c_void);
        (*htab).tab = core::ptr::null_mut();
    }
}

unsafe fn hsearch_r_impl(item: HSearchEntry, action: c_int, retval: *mut *mut HSearchEntry, htab: *mut HSearchData) -> c_int {
    let hash = keyhash(item.key);
    let e = htab_lookup(item.key, hash, htab);

    if !(*e).key.is_null() {
        *retval = e;
        return 1;
    }
    if action == 0 { // FIND
        *retval = core::ptr::null_mut();
        return 0;
    }
    // ENTER
    let item_key = item.key;
    *e = item;
    (*(*htab).tab).used += 1;
    if (*(*htab).tab).used > (*(*htab).tab).mask - (*(*htab).tab).mask / 4 {
        if htab_resize(2 * (*(*htab).tab).used, htab) == 0 {
            (*(*htab).tab).used -= 1;
            (*e).key = core::ptr::null_mut();
            *retval = core::ptr::null_mut();
            return 0;
        }
        let e2 = htab_lookup(item_key, hash, htab);
        *retval = e2;
        return 1;
    }
    *retval = e;
    1
}

#[no_mangle]
pub unsafe extern "C" fn hcreate(nel: usize) -> c_int {
    hcreate_r_impl(nel, core::ptr::addr_of_mut!(HTAB_DATA))
}

#[no_mangle]
pub unsafe extern "C" fn hdestroy() {
    hdestroy_r_impl(core::ptr::addr_of_mut!(HTAB_DATA));
}

#[no_mangle]
pub unsafe extern "C" fn hsearch(item: HSearchEntry, action: c_int) -> *mut HSearchEntry {
    let mut e: *mut HSearchEntry = core::ptr::null_mut();
    hsearch_r_impl(item, action, &mut e, core::ptr::addr_of_mut!(HTAB_DATA));
    e
}

#[no_mangle]
pub unsafe extern "C" fn hcreate_r(nel: usize, htab: *mut HSearchData) -> c_int {
    hcreate_r_impl(nel, htab)
}

#[no_mangle]
pub unsafe extern "C" fn hdestroy_r(htab: *mut HSearchData) {
    hdestroy_r_impl(htab);
}

#[no_mangle]
pub unsafe extern "C" fn hsearch_r(item: HSearchEntry, action: c_int, retval: *mut *mut HSearchEntry, htab: *mut HSearchData) -> c_int {
    hsearch_r_impl(item, action, retval, htab)
}

// ============================================================
// Search: insque / remque
// ============================================================

#[repr(C)]
struct QueNode {
    next: *mut QueNode,
    prev: *mut QueNode,
}

#[no_mangle]
pub unsafe extern "C" fn insque(element: *mut c_void, pred: *mut c_void) {
    let e = element as *mut QueNode;
    let p = pred as *mut QueNode;
    if p.is_null() {
        (*e).next = core::ptr::null_mut();
        (*e).prev = core::ptr::null_mut();
        return;
    }
    (*e).next = (*p).next;
    (*e).prev = p;
    (*p).next = e;
    if !(*e).next.is_null() {
        (*(*e).next).prev = e;
    }
}

#[no_mangle]
pub unsafe extern "C" fn remque(element: *mut c_void) {
    let e = element as *mut QueNode;
    if !(*e).next.is_null() {
        (*(*e).next).prev = (*e).prev;
    }
    if !(*e).prev.is_null() {
        (*(*e).prev).next = (*e).next;
    }
}

// ============================================================
// Search: lsearch / lfind
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn lsearch(
    key: *const c_void,
    base: *mut c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) -> *mut c_void {
    let n = *nelp;
    let p = base as *mut u8;
    for i in 0..n {
        let elem = p.add(i * width);
        if compar.unwrap()(key, elem as *const c_void) == 0 {
            return elem as *mut c_void;
        }
    }
    let dest = p.add(n * width);
    memcpy(dest as *mut c_void, key as *const c_void, width);
    *nelp = n + 1;
    dest as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn lfind(
    key: *const c_void,
    base: *const c_void,
    nelp: *mut usize,
    width: usize,
    compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
) -> *mut c_void {
    let n = *nelp;
    let p = base as *const u8;
    for i in 0..n {
        let elem = p.add(i * width);
        if compar.unwrap()(key, elem as *const c_void) == 0 {
            return elem as *mut c_void;
        }
    }
    core::ptr::null_mut()
}

// ============================================================
// Search: tsearch / tfind / tdelete / twalk / tdestroy (AVL)
// ============================================================

// AVL tree height < 1.44*log2(nodes+2)-0.3; this is musl's safe upper
// bound for the explicit ancestor stack.
const MAXH: usize = core::mem::size_of::<*mut c_void>() * 8 * 3 / 2;

#[repr(C)]
pub struct TreeNode {
    key: *const c_void,
    a: [*mut TreeNode; 2],
    h: c_int,
}

type ComparFn = Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>;

#[inline]
unsafe fn node_height(n: *const TreeNode) -> c_int {
    if n.is_null() { 0 } else { (*n).h }
}

unsafe fn rotate(p: *mut *mut TreeNode, x: *mut TreeNode, dir: usize) -> c_int {
    let y = (*x).a[dir];
    let z = (*y).a[1 - dir];
    let hx = (*x).h;
    let hz = node_height(z);
    let new_root;
    if hz > node_height((*y).a[dir]) {
        (*x).a[dir] = (*z).a[1 - dir];
        (*y).a[1 - dir] = (*z).a[dir];
        (*z).a[1 - dir] = x;
        (*z).a[dir] = y;
        (*x).h = hz;
        (*y).h = hz;
        (*z).h = hz + 1;
        new_root = z;
    } else {
        (*x).a[dir] = z;
        (*y).a[1 - dir] = x;
        (*x).h = hz + 1;
        (*y).h = hz + 2;
        new_root = y;
    }
    *p = new_root;
    (*new_root).h - hx
}

unsafe fn balance(p: *mut *mut TreeNode) -> c_int {
    let n = *p;
    let h0 = node_height((*n).a[0]);
    let h1 = node_height((*n).a[1]);
    if (h0 - h1).abs() <= 1 {
        let old = (*n).h;
        (*n).h = if h0 < h1 { h1 + 1 } else { h0 + 1 };
        (*n).h - old
    } else {
        rotate(p, n, if h0 < h1 { 1 } else { 0 })
    }
}

unsafe fn avl_new(key: *const c_void) -> *mut TreeNode {
    let n = calloc(1, core::mem::size_of::<TreeNode>()) as *mut TreeNode;
    if !n.is_null() {
        (*n).key = key;
        (*n).a = [core::ptr::null_mut(); 2];
        (*n).h = 1;
    }
    n
}

#[no_mangle]
pub unsafe extern "C" fn tsearch(
    key: *const c_void,
    rootp: *mut *mut TreeNode,
    compar: ComparFn,
) -> *mut c_void {
    if rootp.is_null() {
        return core::ptr::null_mut();
    }
    let mut a = [core::ptr::null_mut::<*mut TreeNode>(); MAXH];
    let mut n = *rootp;
    let mut i = 0usize;
    a[i] = rootp;
    i += 1;
    loop {
        if n.is_null() {
            break;
        }
        let c = compar.unwrap()(key, (*n).key);
        if c == 0 {
            return n as *mut c_void;
        }
        let dir = (c > 0) as usize;
        if i >= MAXH {
            return core::ptr::null_mut();
        }
        a[i] = &mut (*n).a[dir] as *mut *mut TreeNode;
        i += 1;
        n = (*n).a[dir];
    }
    let r = avl_new(key);
    if r.is_null() {
        return core::ptr::null_mut();
    }
    i -= 1;
    *a[i] = r;
    while i > 0 {
        i -= 1;
        if balance(a[i]) == 0 {
            break;
        }
    }
    r as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn tfind(
    key: *const c_void,
    rootp: *const *mut TreeNode,
    compar: ComparFn,
) -> *mut c_void {
    if rootp.is_null() {
        return core::ptr::null_mut();
    }
    let mut n = *rootp;
    loop {
        if n.is_null() {
            return core::ptr::null_mut();
        }
        let c = compar.unwrap()(key, (*n).key);
        if c == 0 {
            return n as *mut c_void;
        }
        n = (*n).a[(c > 0) as usize];
    }
}

#[no_mangle]
pub unsafe extern "C" fn tdelete(
    key: *const c_void,
    rootp: *mut *mut TreeNode,
    compar: ComparFn,
) -> *mut c_void {
    if rootp.is_null() {
        return core::ptr::null_mut();
    }
    // a[0] and a[1] both point at rootp; *a[0] is the non-null token returned
    // when the root itself is deleted (musl does the same).
    let mut a = [core::ptr::null_mut::<*mut TreeNode>(); MAXH + 1];
    let mut n = *rootp;
    let mut i = 0usize;
    a[i] = rootp;
    i += 1;
    a[i] = rootp;
    i += 1;
    loop {
        if n.is_null() {
            return core::ptr::null_mut();
        }
        let c = compar.unwrap()(key, (*n).key);
        if c == 0 {
            break;
        }
        let dir = (c > 0) as usize;
        if i >= MAXH {
            return core::ptr::null_mut();
        }
        a[i] = &mut (*n).a[dir] as *mut *mut TreeNode;
        i += 1;
        n = (*n).a[dir];
    }
    let parent = *a[i - 2];
    let child;
    if !(*n).a[0].is_null() {
        // Free the in-order predecessor and move its key into this node.
        let deleted = n;
        if i >= MAXH {
            return core::ptr::null_mut();
        }
        a[i] = &mut (*n).a[0] as *mut *mut TreeNode;
        i += 1;
        n = (*n).a[0];
        while !(*n).a[1].is_null() {
            if i >= MAXH {
                return core::ptr::null_mut();
            }
            a[i] = &mut (*n).a[1] as *mut *mut TreeNode;
            i += 1;
            n = (*n).a[1];
        }
        (*deleted).key = (*n).key;
        child = (*n).a[0];
    } else {
        child = (*n).a[1];
    }
    free(n as *mut c_void);
    i -= 1;
    *a[i] = child;
    while i > 1 {
        i -= 1;
        if balance(a[i]) == 0 {
            break;
        }
    }
    parent as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn twalk(
    root: *const TreeNode,
    action: Option<unsafe extern "C" fn(*const c_void, c_int, c_int)>,
) {
    unsafe fn walk(
        node: *const TreeNode,
        action: Option<unsafe extern "C" fn(*const c_void, c_int, c_int)>,
        depth: c_int,
    ) {
        if node.is_null() {
            return;
        }
        if (*node).h == 1 {
            action.unwrap()(node as *const c_void, 3, depth); // leaf
        } else {
            action.unwrap()(node as *const c_void, 0, depth); // preorder
            walk((*node).a[0], action, depth + 1);
            action.unwrap()(node as *const c_void, 1, depth); // postorder (left done)
            walk((*node).a[1], action, depth + 1);
            action.unwrap()(node as *const c_void, 2, depth); // endorder
        }
    }
    walk(root, action, 0);
}

#[no_mangle]
pub unsafe extern "C" fn tdestroy(root: *mut TreeNode, freekey: Option<unsafe extern "C" fn(*mut c_void)>) {
    if root.is_null() {
        return;
    }
    tdestroy((*root).a[0], freekey);
    tdestroy((*root).a[1], freekey);
    if let Some(f) = freekey {
        f((*root).key as *mut c_void);
    }
    free(root as *mut c_void);
}

// ============================================================
// fnmatch: full POSIX pattern matching

const FNMATCH_COMPONENT_MAX: usize = 1024;
const FNMATCH_PREFIX_MAX: usize = 4096;
// ============================================================

const FNM_PATHNAME: c_int = 0x1;
const FNM_NOESCAPE: c_int = 0x2;
const FNM_PERIOD: c_int = 0x4;
const FNM_LEADING_DIR: c_int = 0x8;
const FNM_CASEFOLD: c_int = 0x10;
const FNM_NOMATCH: c_int = 1;

const FNM_END: c_int = 0;
const FNM_STAR: c_int = -5;
const FNM_QUESTION: c_int = -4;
const FNM_BRACKET: c_int = -3;
const FNM_UNMATCHABLE: c_int = -2;

fn ascii_tolower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' { c + 32 } else { c }
}

fn ascii_toupper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' { c - 32 } else { c }
}

fn ascii_casefold(k: u8) -> u8 {
    let upper = ascii_toupper(k);
    if upper != k { upper } else { ascii_tolower(k) }
}

/// Get next byte from pattern, classifying it.
/// Returns (token_type, bytes_consumed).
unsafe fn pat_next(pat: *const u8, m: usize, flags: c_int) -> (c_int, usize) {
    if m == 0 || *pat == 0 {
        return (FNM_END, 0);
    }
    let mut p = pat;
    let mut step = 1usize;
    let mut esc = false;
    if *p == b'\\' && !(flags & FNM_NOESCAPE != 0) && *p.add(1) != 0 {
        p = p.add(1);
        step = 2;
        esc = true;
    }
    if *p == b'[' && !esc {
        // scan for matching ]
        let mut k = 1usize;
        if k < m && (*pat.add(k) == b'^' || *pat.add(k) == b'!') { k += 1; }
        if k < m && *pat.add(k) == b']' { k += 1; }
        while k < m && *pat.add(k) != 0 && *pat.add(k) != b']' {
            if k + 1 < m && *pat.add(k) == b'[' &&
               (*pat.add(k+1) == b':' || *pat.add(k+1) == b'.' || *pat.add(k+1) == b'=') {
                let z = *pat.add(k+1);
                k += 2;
                if k < m { k += 1; }
                while k < m && *pat.add(k) != 0 && (*pat.add(k-1) != z || *pat.add(k) != b']') { k += 1; }
                if k == m || *pat.add(k) == 0 { break; }
            }
            k += 1;
        }
        if k == m || *pat.add(k) == 0 {
            // unmatched [ is literal
            return (b'[' as c_int, 1);
        }
        return (FNM_BRACKET, k + 1);
    }
    if *p == b'*' && !esc {
        return (FNM_STAR, 1);
    }
    if *p == b'?' && !esc {
        return (FNM_QUESTION, 1);
    }
    (*p as c_int, step)
}

unsafe fn match_class(name: &[u8], k: u8, kfold: u8) -> bool {
    let eq = |c: u8| k == c || kfold == c;
    let in_range = |lo: u8, hi: u8| (k >= lo && k <= hi) || (kfold >= lo && kfold <= hi);
    match name {
        b"alnum" => in_range(b'0', b'9') || in_range(b'A', b'Z') || in_range(b'a', b'z'),
        b"alpha" => in_range(b'A', b'Z') || in_range(b'a', b'z'),
        b"blank" => eq(b' ') || eq(b'\t'),
        b"cntrl" => k < 0x20 || k == 0x7f,
        b"digit" => in_range(b'0', b'9'),
        b"graph" => k >= 0x21 && k <= 0x7e,
        b"lower" => in_range(b'a', b'z'),
        b"print" => k >= 0x20 && k <= 0x7e,
        b"punct" =>
            (k >= 0x21 && k <= 0x2f) ||
            (k >= 0x3a && k <= 0x40) ||
            (k >= 0x5b && k <= 0x60) ||
            (k >= 0x7b && k <= 0x7e),
        b"space" => eq(b' ') || eq(b'\t') || eq(b'\n') || eq(b'\r') || eq(0x0c) || eq(0x0b),
        b"upper" => in_range(b'A', b'Z'),
        b"xdigit" => in_range(b'0', b'9') || in_range(b'A', b'F') || in_range(b'a', b'f'),
        _ => false,
    }
}

/// Match a bracket expression [class] against char k.
unsafe fn match_bracket(mut p: *const u8, k: u8, kfold: u8) -> bool {
    let mut inv = false;
    p = p.add(1); // skip [
    if *p == b'^' || *p == b'!' {
        inv = true;
        p = p.add(1);
    }
    // handle ] as first char
    if *p == b']' {
        if k == b']' { return !inv; }
        p = p.add(1);
    } else if *p == b'-' {
        if k == b'-' { return !inv; }
        p = p.add(1);
    }
    while *p != b']' && *p != 0 {
        if *p == b'-' && *p.add(1) != b']' && *p.add(1) != 0 {
            // range: previous char to next char
            let lo = *p.sub(1);
            let hi = *p.add(1);
            if lo <= hi && (k >= lo && k <= hi || kfold >= lo && kfold <= hi) {
                return !inv;
            }
            p = p.add(2);
        } else if *p == b'[' && (*p.add(1) == b':' || *p.add(1) == b'.' || *p.add(1) == b'=') {
            let z = *p.add(1);
            let start = p.add(2);
            let mut q = start;
            while *q != 0 && (*q != z || *q.add(1) != b']') { q = q.add(1); }
            if *q != 0 {
                let len = (q as usize) - (start as usize);
                if len > 0 {
                    let inner = core::slice::from_raw_parts(start, len);
                    let matched =
                        if z == b':' { match_class(inner, k, kfold) }
                        else { len == 1 && (inner[0] == k || inner[0] == kfold) };
                    if matched { return !inv; }
                }
                p = q.add(2);
                continue;
            }
        } else {
            if *p == k || *p == kfold {
                return !inv;
            }
            p = p.add(1);
        }
    }
    inv
}

/// Core fnmatch for a single path component (no FNM_PATHNAME splitting).
/// Match one non-star, non-end pattern token against one input byte.
unsafe fn fnmatch_token(
    tok: c_int,
    pat: *const u8,
    s: *const u8,
    flags: c_int,
) -> bool {
    if *s == 0 {
        return false;
    }
    let k = *s;
    let kfold = if flags & FNM_CASEFOLD != 0 { ascii_casefold(k) } else { k };
    match tok {
        FNM_QUESTION => true,
        FNM_BRACKET => match_bracket(pat, k, kfold),
        c => k as c_int == c || kfold as c_int == c,
    }
}

unsafe fn fnmatch_internal(pat: *const u8, str: *const u8, flags: c_int) -> c_int {
    // Handle leading .
    if flags & FNM_PERIOD != 0 && *str == b'.' && *pat != b'.' {
        return FNM_NOMATCH;
    }

    let mut p = pat;
    let mut s = str;

    // Consume pattern up to first *
    loop {
        let (tok, step) = pat_next(p, usize::MAX, flags);
        match tok {
            FNM_END => {
                return if *s == 0 { 0 } else { FNM_NOMATCH };
            }
            FNM_STAR => {
                p = p.add(1);
                break;
            }
            _ => {
                if !fnmatch_token(tok, p, s, flags) { return FNM_NOMATCH; }
                p = p.add(step);
                s = s.add(1);
            }
        }
    }

    // Now p points after first *, rest of pattern is variable
    // Factor: find tail after last * and count fixed chars needed
    let mut ptail = p;
    let mut tailcnt: usize = 0;
    let mut pp = p;
    loop {
        let (tok, step) = pat_next(pp, usize::MAX, flags);
        if tok == FNM_END { break; }
        if tok == FNM_STAR {
            tailcnt = 0;
            ptail = pp.add(1);
        } else {
            tailcnt += 1;
        }
        pp = pp.add(step);
    }

    // str must have at least tailcnt chars
    let slen = strlen(s as *const c_char);
    if slen < tailcnt { return FNM_NOMATCH; }

    // Match tail: ptail..end against str[tailcnt from end]
    let stail = s.add(slen - tailcnt);
    let mut tp = ptail;
    let mut ts = stail;
    loop {
        let (tok, step) = pat_next(tp, usize::MAX, flags);
        if tok == FNM_END {
            break;
        }
        if !fnmatch_token(tok, tp, ts, flags) { return FNM_NOMATCH; }
        tp = tp.add(step);
        ts = ts.add(1);
    }

    let endstr = stail;
    let endpat = ptail;

    let mut cp = p;
    let mut cs = s;
    while cp < endpat {
        let (tok, step) = pat_next(cp, usize::MAX, flags);
        if tok == FNM_STAR {
            cp = cp.add(step);
            cs = s;
            continue;
        }
        let comp_start = cp;
        let mut ok = false;
        let mut try_s = cs;
        while try_s < endstr {
            let mut tmp_p = comp_start;
            let mut tmp_s = try_s;
            let mut matched = true;
            loop {
                let (t, st) = pat_next(tmp_p, usize::MAX, flags);
                if t == FNM_STAR || t == FNM_END {
                    cp = tmp_p;
                    cs = tmp_s;
                    ok = true;
                    break;
                }
                if !fnmatch_token(t, tmp_p, tmp_s, flags) {
                    matched = false;
                    break;
                }
                tmp_s = tmp_s.add(1);
                tmp_p = tmp_p.add(st);
            }
            if ok { break; }
            if !matched {
                try_s = try_s.add(1);
            }
        }
        if !ok {
            return FNM_NOMATCH;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn fnmatch(pat: *const c_char, str: *const c_char, flags: c_int) -> c_int {
    let pat = pat as *const u8;
    let str = str as *const u8;

    if flags & FNM_PATHNAME != 0 {
        // Split on /
        let mut s = str;
        let mut p = pat;
        loop {
            // find next / in str
            let mut send = s;
            while *send != 0 && *send != b'/' { send = send.add(1); }
            // find next / in pat
            let mut pend = p;
            loop {
                let (tok, step) = pat_next(pend, usize::MAX, flags);
                if tok == FNM_END || (tok >= 0 && tok as u8 == b'/') {
                    break;
                }
                pend = pend.add(step);
            }
            let (ptok, pstep) = pat_next(pend, usize::MAX, flags);
            let pat_is_slash = ptok >= 0 && ptok as u8 == b'/';
            let str_is_slash = *send == b'/';
            if pat_is_slash != str_is_slash && (*send == 0 || !(flags & FNM_LEADING_DIR != 0)) {
                return FNM_NOMATCH;
            }
            // match component
            // We need a null-terminated copy of the component
            let plen = (pend as usize) - (p as usize);
            let slen = (send as usize) - (s as usize);
            // Components longer than 1024 bytes are rejected rather than
            // matched with a heap buffer.
            if plen > FNMATCH_COMPONENT_MAX || slen > FNMATCH_COMPONENT_MAX {
                return FNM_NOMATCH;
            }
            let mut pbuf = [0u8; FNMATCH_COMPONENT_MAX + 1];
            let mut sbuf = [0u8; FNMATCH_COMPONENT_MAX + 1];
            core::ptr::copy_nonoverlapping(p, pbuf.as_mut_ptr(), plen);
            pbuf[plen] = 0;
            core::ptr::copy_nonoverlapping(s, sbuf.as_mut_ptr(), slen);
            sbuf[slen] = 0;
            if fnmatch_internal(pbuf.as_ptr(), sbuf.as_ptr(), flags & !FNM_LEADING_DIR) != 0 {
                return FNM_NOMATCH;
            }
            if !pat_is_slash {
                return 0;
            }
            s = send.add(1);
            p = pend.add(pstep);
        }
    }

    if flags & FNM_LEADING_DIR != 0 {
        // Try matching at each /
        let mut s = str;
        while *s != 0 {
            if *s == b'/' {
                // temporarily null-terminate
                // The prefix must be NUL-terminated for fnmatch_internal().
                let len = (s as usize) - (str as usize);
                // Just call fnmatch_internal with a temp buffer
                if len < FNMATCH_PREFIX_MAX {
                    let mut buf = [0u8; FNMATCH_PREFIX_MAX];
                    core::ptr::copy_nonoverlapping(str, buf.as_mut_ptr(), len);
                    buf[len] = 0;
                    if fnmatch_internal(pat, buf.as_ptr(), flags) == 0 {
                        return 0;
                    }
                }
            }
            s = s.add(1);
        }
    }

    fnmatch_internal(pat, str, flags)
}

