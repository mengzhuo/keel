// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// POSIX shared memory + named semaphores
// ============================================================

const NAME_MAX: usize = 255;
const SEM_NSEMS_MAX: usize = 256;

// global spinlock for semtab, upgrade to per-entry if contention matters
static SEMTAB_LOCK: AtomicI32 = AtomicI32::new(0);

struct SemEntry {
    ino: u64,
    sem: *mut sem_t,
    refcnt: u32,
}

// zero-init is fine for static mut (all-zero = free slots)
static mut SEMTAB: [SemEntry; SEM_NSEMS_MAX] = {
    const ZERO: SemEntry = SemEntry { ino: 0, sem: core::ptr::null_mut(), refcnt: 0 };
    [ZERO; SEM_NSEMS_MAX]
};

unsafe fn semtab_lock() {
    while SEMTAB_LOCK.compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed).is_err() {
        core::hint::spin_loop();
    }
}

unsafe fn semtab_unlock() {
    SEMTAB_LOCK.store(0, Ordering::Release);
}

// Validate shm/sem name and construct /dev/shm/<name> path.
// Returns path length or -1 with ERRNO set.
unsafe fn shm_mapname(name: *const c_char, buf: *mut u8) -> c_int {
    let mut p = name;
    // strip leading /
    while *p == b'/' as c_char { p = p.add(1); }
    let start = p;
    // find end of first component
    while *p != 0 && *p != b'/' as c_char { p = p.add(1); }
    let len = p.offset_from(start) as usize;
    // no trailing /, not empty, not "." or ".."
    if len == 0 || *p != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    if len == 1 && *start == b'.' as c_char {
        ERRNO!() = EINVAL;
        return -1;
    }
    if len == 2 && *start == b'.' as c_char && *start.add(1) == b'.' as c_char {
        ERRNO!() = EINVAL;
        return -1;
    }
    if len > NAME_MAX {
        ERRNO!() = ENAMETOOLONG;
        return -1;
    }
    // build /dev/shm/<name>
    let prefix = b"/dev/shm/\0";
    let mut i = 0;
    while prefix[i] != 0 {
        *buf.add(i) = prefix[i];
        i += 1;
    }
    let mut j = 0;
    while j < len {
        *buf.add(i + j) = *start.add(j) as u8;
        j += 1;
    }
    *buf.add(i + j) = 0;
    (i + j) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn shm_open(name: *const c_char, flags: c_int, mode: mode_t) -> c_int {
    let mut buf = [0u8; 16 + NAME_MAX + 1];
    if shm_mapname(name, buf.as_mut_ptr()) < 0 { return -1; }
    let fd = sys_open(buf.as_ptr(), (flags | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK) as i64, mode as i64);
    if fd < 0 { ERRNO!() = (-fd) as c_int; return -1; }
    fd as c_int
}

#[no_mangle]
pub unsafe extern "C" fn shm_unlink(name: *const c_char) -> c_int {
    let mut buf = [0u8; 16 + NAME_MAX + 1];
    if shm_mapname(name, buf.as_mut_ptr()) < 0 { return -1; }
    let r = sys_unlink(buf.as_ptr());
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn sem_open(name: *const c_char, flags: c_int, mut args: ...) -> *mut sem_t {
    let mut buf = [0u8; 16 + NAME_MAX + 1];
    if shm_mapname(name, buf.as_mut_ptr()).is_negative() {
        return core::ptr::null_mut(); // SEM_FAILED
    }

    let oflags = flags & (O_CREAT | O_EXCL);

    semtab_lock();

    // find a free slot
    let mut slot: c_int = -1;
    for i in 0..SEMTAB_LEN {
        if SEMTAB[i].sem.is_null() && slot < 0 { slot = i as c_int; }
    }
    if slot < 0 {
        ERRNO!() = EMFILE;
        semtab_unlock();
        return core::ptr::null_mut();
    }
    // reserve slot
    SEMTAB[slot as usize].sem = !0usize as *mut sem_t; // sentinel
    semtab_unlock();

    // extract va_args for O_CREAT
    let mut mode: mode_t = 0;
    let mut value: c_uint = 0;
    if oflags & O_CREAT != 0 {
        mode = args.next_arg::<mode_t>() & 0o666;
        value = args.next_arg::<c_uint>();
        if value > SEM_VALUE_MAX as c_uint {
            ERRNO!() = EINVAL;
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
    }

    // try opening existing first (unless O_CREAT|O_EXCL)
    if oflags != O_CREAT | O_EXCL {
        let fd = sys_open(buf.as_ptr(), (O_RDWR | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK) as i64, 0);
        if fd >= 0 {
            let map = sys_mmap(
                core::ptr::null_mut(),
                core::mem::size_of::<sem_t>(),
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd as i32,
                0,
            );
            sys_close(fd);
            if map != MMAP_FAILED {
                let st_ino = get_ino(fd as c_int, buf.as_ptr());
                return sem_register(map as *mut sem_t, st_ino, slot);
            }
        }
        if oflags == 0 {
            // no O_CREAT and file doesn't exist
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
    }

    // create: write sem_t to temp file, mmap, link
    let mut tmp = [0u8; 32];
    // Each attempt gets a fresh unpredictable name; reusing one name made
    // the retry loop below useless and let a local user pre-create it.
    unsafe fn make_tmp_name(tmp: &mut [u8; 32]) {
        let prefix = b"/dev/shm/tmp-";
        tmp[..prefix.len()].copy_from_slice(prefix);
        let mut r: u64 = 0;
        let got = crate::getrandom(
            &mut r as *mut u64 as *mut u8,
            core::mem::size_of::<u64>(),
            0,
        );
        if got != core::mem::size_of::<u64>() as isize {
            let mut ts: timespec = core::mem::zeroed();
            sys_clock_gettime(0, &mut ts);
            r = (ts.tv_nsec as u64) ^ ((crate::getpid() as u64) << 32);
        }
        let hex = b"0123456789abcdef";
        for i in 0..16 {
            tmp[prefix.len() + i] = hex[((r >> (4 * (15 - i))) & 0xf) as usize];
        }
        tmp[prefix.len() + 16] = 0;
    }

    let mut newsem: sem_t = core::mem::zeroed();
    newsem.__val[0] = value as c_int;
    newsem.__val[1] = 0;
    newsem.__val[2] = 128; // pshared=0 flag

    let mut attempts = 0usize;
    loop {
        make_tmp_name(&mut tmp);
        let tmp_path = tmp.as_ptr();
        let fd = sys_open(tmp_path as *const u8, (O_RDWR | O_CREAT | O_EXCL | O_CLOEXEC | O_NONBLOCK) as i64, mode as i64);
        if fd < 0 {
            if (-fd) as c_int == EEXIST {
                attempts += 1;
                if attempts >= 16 {
                    ERRNO!() = EEXIST;
                    semtab_lock();
                    SEMTAB[slot as usize].sem = core::ptr::null_mut();
                    semtab_unlock();
                    return core::ptr::null_mut();
                }
                continue;
            }
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
        let written = sys_write(fd, &newsem as *const sem_t as *const u8, core::mem::size_of::<sem_t>());
        if written != core::mem::size_of::<sem_t>() as i64 {
            sys_close(fd);
            sys_unlink(tmp_path);
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
        let map = sys_mmap(
            core::ptr::null_mut(),
            core::mem::size_of::<sem_t>(),
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            fd as i32,
            0,
        );
        sys_close(fd);
        if map == MMAP_FAILED {
            sys_unlink(tmp_path);
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
        // link to final name
        let r = sys_linkat(AT_FDCWD, tmp_path, AT_FDCWD, buf.as_ptr(), 0);
        let e = if r < 0 { (-r) as c_int } else { 0 };
        sys_unlink(tmp_path);
        if e == 0 {
            let st_ino = get_ino(-1, buf.as_ptr());
            return sem_register(map as *mut sem_t, st_ino, slot);
        }
        sys_munmap(map, core::mem::size_of::<sem_t>());
        if e != EEXIST || oflags == O_CREAT | O_EXCL {
            ERRNO!() = e;
            semtab_lock();
            SEMTAB[slot as usize].sem = core::ptr::null_mut();
            semtab_unlock();
            return core::ptr::null_mut();
        }
        // EEXIST without O_EXCL: retry open existing
        break;
    }

    // fallthrough: try opening the existing file that won the race
    let fd = sys_open(buf.as_ptr(), (O_RDWR | O_NOFOLLOW | O_CLOEXEC | O_NONBLOCK) as i64, 0);
    if fd < 0 {
        semtab_lock();
        SEMTAB[slot as usize].sem = core::ptr::null_mut();
        semtab_unlock();
        return core::ptr::null_mut();
    }
    let map = sys_mmap(
        core::ptr::null_mut(),
        core::mem::size_of::<sem_t>(),
        PROT_READ | PROT_WRITE,
        MAP_SHARED,
        fd as i32,
        0,
    );
    sys_close(fd);
    if map == MMAP_FAILED {
        semtab_lock();
        SEMTAB[slot as usize].sem = core::ptr::null_mut();
        semtab_unlock();
        return core::ptr::null_mut();
    }
    let st_ino = get_ino(-1, buf.as_ptr());
    sem_register(map as *mut sem_t, st_ino, slot)
}

unsafe fn get_ino(_fd_hint: c_int, path: *const u8) -> u64 {
    let mut st: Stat = core::mem::zeroed();
    if sys_newfstatat(AT_FDCWD, path as *const c_char, &mut st as *mut Stat as *mut u8, 0) == 0 {
        st.st_ino
    } else {
        0
    }
}

unsafe fn sem_register(map: *mut sem_t, ino: u64, slot_hint: c_int) -> *mut sem_t {
    semtab_lock();
    // check if already mapped (by inode)
    for i in 0..SEMTAB_LEN {
        if SEMTAB[i].ino == ino && !SEMTAB[i].sem.is_null() && SEMTAB[i].sem != !0usize as *mut sem_t {
            // already mapped: unmap new, use existing, release hint slot
            sys_munmap(map as *mut u8, core::mem::size_of::<sem_t>());
            SEMTAB[slot_hint as usize].sem = core::ptr::null_mut();
            SEMTAB[i].refcnt += 1;
            let ret = SEMTAB[i].sem;
            semtab_unlock();
            return ret;
        }
    }
    // new entry
    SEMTAB[slot_hint as usize].sem = map;
    SEMTAB[slot_hint as usize].ino = ino;
    SEMTAB[slot_hint as usize].refcnt = 1;
    semtab_unlock();
    map
}

const SEMTAB_LEN: usize = SEM_NSEMS_MAX;

#[no_mangle]
pub unsafe extern "C" fn sem_close(sem: *mut sem_t) -> c_int {
    if sem.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    semtab_lock();
    for i in 0..SEMTAB_LEN {
        if SEMTAB[i].sem == sem {
            if SEMTAB[i].refcnt == 0 {
                semtab_unlock();
                ERRNO!() = EINVAL;
                return -1;
            }
            SEMTAB[i].refcnt -= 1;
            if SEMTAB[i].refcnt == 0 {
                SEMTAB[i].sem = core::ptr::null_mut();
                SEMTAB[i].ino = 0;
                semtab_unlock();
                sys_munmap(sem as *mut u8, core::mem::size_of::<sem_t>());
                return 0;
            }
            semtab_unlock();
            return 0;
        }
    }
    semtab_unlock();
    ERRNO!() = EINVAL;
    -1
}

#[no_mangle]
pub unsafe extern "C" fn sem_unlink(name: *const c_char) -> c_int {
    shm_unlink(name)
}

// --- pthread_once ---
#[no_mangle]
pub unsafe extern "C" fn pthread_once(control: *mut pthread_once_t, init_routine: Option<unsafe extern "C" fn()>) -> c_int {
    if a_load(control) == 2 { return 0; }
    loop {
        match a_cas(control, 0, 1) {
            0 => {
                if let Some(f) = init_routine { f(); }
                if a_swap(control, 2) == 3 { futex_wake(control, -1); }
                return 0;
            }
            1 => { a_cas(control, 1, 3); }
            3 => { futex_wait(control, 3); }
            _ => { return 0; }
        }
    }
}

// --- pthread_key_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_key_create(key: *mut pthread_key_t, dtor: Option<unsafe extern "C" fn(*mut c_void)>) -> c_int {
    let start = NEXT_KEY.load(Ordering::Relaxed);
    let mut j = start;
    loop {
        if KEY_DTORS[j].is_none() {
            KEY_DTORS[j] = dtor;
            NEXT_KEY.store((j + 1) % PTHREAD_KEYS_MAX, Ordering::Relaxed);
            *key = j as pthread_key_t;
            return 0;
        }
        j = (j + 1) % PTHREAD_KEYS_MAX;
        if j == start { return EAGAIN; }
    }
}
#[no_mangle]
pub unsafe extern "C" fn pthread_key_delete(key: pthread_key_t) -> c_int {
    if key as usize >= PTHREAD_KEYS_MAX {
        return EINVAL;
    }
    KEY_DTORS[key as usize] = None;
    let count = thread_slot_count();
    let mut i = 0usize;
    while i < count {
        let slot = thread_slot_at(i);
        if !slot.is_null() && core::ptr::read_volatile(&raw const (*slot).tid) > 0 {
            (*slot).tsd[key as usize] = core::ptr::null_mut();
        }
        i += 1;
    }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_getspecific(key: pthread_key_t) -> *mut c_void {
    if key as usize >= PTHREAD_KEYS_MAX {
        return core::ptr::null_mut();
    }
    if let Some(slot) = find_thread() { slot.tsd[key as usize] } else { core::ptr::null_mut() }
}
#[no_mangle]
pub unsafe extern "C" fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int {
    if key as usize >= PTHREAD_KEYS_MAX {
        ERRNO!() = EINVAL;
        return EINVAL;
    }
    if let Some(slot) = find_thread() {
        slot.tsd[key as usize] = value as *mut c_void;
        0
    } else { EINVAL }
}

// --- pthread_cancel/state ---
unsafe fn run_cleanup_handlers(slot: &mut Thread) {
    let mut cb = slot.cancelbuf;
    while !cb.is_null() {
        let cur = &*cb;
        let f = cur.__f;
        let x = cur.__x;
        cb = cur.__next;
        if let Some(func) = f {
            func(x);
        }
    }
    slot.cancelbuf = core::ptr::null_mut();
}

unsafe fn do_cancel() -> ! {
    if let Some(slot) = find_thread() {
        run_cleanup_handlers(slot);
        slot.result = !0usize as *mut c_void; // PTHREAD_CANCELED
        slot.detach_state = DT_EXITED;
        futex_wake(&raw mut slot.detach_state, 1);
    }
    sys_exit_thread(0);
}

extern "C" fn cancel_handler(_sig: c_int) {
    unsafe {
        if let Some(slot) = find_thread() {
            // In asynchronous mode, cancel immediately from the signal handler.
            // In deferred mode, the signal just interrupts blocking syscalls so
            // the next cancellation point can act on the pending request.
            if slot.cancel != 0
                && slot.cancel_state == PTHREAD_CANCEL_ENABLE
                && slot.cancel_type == PTHREAD_CANCEL_ASYNCHRONOUS
            {
                do_cancel();
            }
        }
    }
}

static CANCEL_INIT: AtomicI32 = AtomicI32::new(0);

unsafe fn ensure_cancel_handler() {
    if CANCEL_INIT.swap(1, Ordering::AcqRel) == 0 {
        let act = KSigAction {
            sa_handler: cancel_handler as *const () as usize,
            sa_flags: SA_RESTORER,
            sa_restorer: sig_restorer as *const () as usize,
            sa_mask: [!0u64],
        };
        sys_rt_sigaction(SIGCANCEL, &act as *const KSigAction, core::ptr::null_mut(), 8);
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_cancel(thread: PthreadT) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    ensure_cancel_handler();
    a_store(&raw mut (*slot).cancel, 1);
    let tid = (*slot).tid;
    if tid > 0 {
        sys_tgkill(sys_getpid() as c_int, tid, SIGCANCEL);
    }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_setcancelstate(state: c_int, oldstate: *mut c_int) -> c_int {
    if state != PTHREAD_CANCEL_ENABLE && state != PTHREAD_CANCEL_DISABLE { return EINVAL; }
    if let Some(slot) = find_thread() {
        if !oldstate.is_null() { *oldstate = slot.cancel_state; }
        slot.cancel_state = state;
        0
    } else { EINVAL }
}
#[no_mangle]
pub unsafe extern "C" fn pthread_setcanceltype(type_: c_int, oldtype: *mut c_int) -> c_int {
    if type_ != PTHREAD_CANCEL_DEFERRED && type_ != PTHREAD_CANCEL_ASYNCHRONOUS { return EINVAL; }
    if let Some(slot) = find_thread() {
        if !oldtype.is_null() { *oldtype = slot.cancel_type; }
        slot.cancel_type = type_;
        0
    } else { EINVAL }
}
#[no_mangle]
pub unsafe extern "C" fn pthread_testcancel() {
    if let Some(slot) = find_thread() {
        if slot.cancel != 0 && slot.cancel_state == PTHREAD_CANCEL_ENABLE {
            do_cancel();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn _pthread_cleanup_push(cb: *mut __ptcb, f: Option<unsafe extern "C" fn(*mut c_void)>, x: *mut c_void) {
    (*cb).__f = f;
    (*cb).__x = x;
    if let Some(slot) = find_thread() {
        (*cb).__next = slot.cancelbuf;
        slot.cancelbuf = cb;
    } else {
        (*cb).__next = core::ptr::null_mut();
    }
}

#[no_mangle]
pub unsafe extern "C" fn _pthread_cleanup_pop(cb: *mut __ptcb, run: c_int) {
    if let Some(slot) = find_thread() {
        slot.cancelbuf = (*cb).__next;
    }
    if run != 0 {
        if let Some(f) = (*cb).__f {
            f((*cb).__x);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_kill(thread: PthreadT, sig: c_int) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    let tid = (*slot).tid;
    if tid <= 0 { return EINVAL; }
    if sys_tgkill(sys_getpid() as c_int, tid, sig) < 0 { -1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_sigmask(how: c_int, set: *const SigSetT, oldset: *mut SigSetT) -> c_int {
    // POSIX pthread functions return the error number itself, not -1/errno.
    if sigprocmask(how, set, oldset) < 0 { ERRNO!() } else { 0 }
}


#[repr(C)]
pub struct lconv {
    pub decimal_point: *mut c_char,
    pub thousands_sep: *mut c_char,
    pub grouping: *mut c_char,
    pub int_curr_symbol: *mut c_char,
    pub currency_symbol: *mut c_char,
    pub mon_decimal_point: *mut c_char,
    pub mon_thousands_sep: *mut c_char,
    pub mon_grouping: *mut c_char,
    pub positive_sign: *mut c_char,
    pub negative_sign: *mut c_char,
    pub int_frac_digits: c_char,
    pub frac_digits: c_char,
    pub p_cs_precedes: c_char,
    pub p_sep_by_space: c_char,
    pub n_cs_precedes: c_char,
    pub n_sep_by_space: c_char,
    pub p_sign_posn: c_char,
    pub n_sign_posn: c_char,
    pub int_p_cs_precedes: c_char,
    pub int_p_sep_by_space: c_char,
    pub int_n_cs_precedes: c_char,
    pub int_n_sep_by_space: c_char,
    pub int_p_sign_posn: c_char,
    pub int_n_sign_posn: c_char,
}

static UTF8_NAME: [u8; 6] = [b'U', b'T', b'F', b'-', b'8', 0];
static C_NAME: [u8; 2] = [b'C', 0];
static mut LOCALE_NAME: *const c_char = C_NAME.as_ptr() as *const c_char;
static mut LOCALE_CTYPE_UTF8: bool = true;
static mut EMPTY_STR: [c_char; 1] = [0];
static mut DECIMAL_POINT: [c_char; 2] = [b'.' as c_char, 0];
static mut LCONV: lconv = lconv {
    decimal_point: core::ptr::null_mut(),
    thousands_sep: core::ptr::null_mut(),
    grouping: core::ptr::null_mut(),
    int_curr_symbol: core::ptr::null_mut(),
    currency_symbol: core::ptr::null_mut(),
    mon_decimal_point: core::ptr::null_mut(),
    mon_thousands_sep: core::ptr::null_mut(),
    mon_grouping: core::ptr::null_mut(),
    positive_sign: core::ptr::null_mut(),
    negative_sign: core::ptr::null_mut(),
    int_frac_digits: (-1i32) as c_char,
    frac_digits: (-1i32) as c_char,
    p_cs_precedes: (-1i32) as c_char,
    p_sep_by_space: (-1i32) as c_char,
    n_cs_precedes: (-1i32) as c_char,
    n_sep_by_space: (-1i32) as c_char,
    p_sign_posn: (-1i32) as c_char,
    n_sign_posn: (-1i32) as c_char,
    int_p_cs_precedes: (-1i32) as c_char,
    int_p_sep_by_space: (-1i32) as c_char,
    int_n_cs_precedes: (-1i32) as c_char,
    int_n_sep_by_space: (-1i32) as c_char,
    int_p_sign_posn: (-1i32) as c_char,
    int_n_sign_posn: (-1i32) as c_char,
};

#[no_mangle]
pub unsafe extern "C" fn setlocale(_category: c_int, locale: *const c_char) -> *mut c_char {
    if locale.is_null() || *locale == 0 {
        return LOCALE_NAME as *mut c_char;
    }
    let name = locale as *const u8;
    if crate::cstr_eq(name, b"C") || crate::cstr_eq(name, b"POSIX") {
        LOCALE_CTYPE_UTF8 = false;
        return LOCALE_NAME as *mut c_char;
    }
    if crate::cstr_contains_ci(name, b"UTF-8") || crate::cstr_contains_ci(name, b"UTF8") {
        LOCALE_CTYPE_UTF8 = true;
        return LOCALE_NAME as *mut c_char;
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn localeconv() -> *mut lconv {
    let empty = core::ptr::addr_of_mut!(EMPTY_STR) as *mut c_char;
    let lconv = core::ptr::addr_of_mut!(LCONV);
    (*lconv).decimal_point = core::ptr::addr_of_mut!(DECIMAL_POINT) as *mut c_char;
    (*lconv).thousands_sep = empty;
    (*lconv).grouping = empty;
    (*lconv).int_curr_symbol = empty;
    (*lconv).currency_symbol = empty;
    (*lconv).mon_decimal_point = empty;
    (*lconv).mon_thousands_sep = empty;
    (*lconv).mon_grouping = empty;
    (*lconv).positive_sign = empty;
    (*lconv).negative_sign = empty;
    lconv
}

pub type locale_t = *mut c_void;
static mut CURRENT_LOCALE: locale_t = LC_GLOBAL_LOCALE;

static mut C_LOCALE_STORAGE: c_int = 0;
const C_LOCALE: locale_t = core::ptr::addr_of_mut!(C_LOCALE_STORAGE) as locale_t;
const LC_GLOBAL_LOCALE: locale_t = usize::MAX as locale_t;

#[no_mangle]
pub unsafe extern "C" fn newlocale(_mask: c_int, name: *const c_char, base: locale_t) -> locale_t {
    if !name.is_null() && *name != 0 {
        let n = name as *const u8;
        if !crate::cstr_eq(n, b"C") && !crate::cstr_eq(n, b"POSIX") {
            return core::ptr::null_mut();
        }
    }
    if base.is_null() || base == C_LOCALE || base == LC_GLOBAL_LOCALE {
        C_LOCALE
    } else {
        core::ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn freelocale(_loc: locale_t) {}

#[no_mangle]
pub unsafe extern "C" fn uselocale(loc: locale_t) -> locale_t {
    let old = CURRENT_LOCALE;
    if !loc.is_null() {
        CURRENT_LOCALE = loc;
    }
    old
}

#[no_mangle]
pub unsafe extern "C" fn duplocale(loc: locale_t) -> locale_t {
    if loc.is_null() {
        return core::ptr::null_mut();
    }
    C_LOCALE
}

const NL_ITEM_CODESET: c_int = 14;
const NL_ITEM_RADIXCHAR: c_int = 0x10000;
const NL_ITEM_THOUSEP: c_int = 0x10001;
const NL_ITEM_YESEXPR: c_int = 0x50000;
const NL_ITEM_NOEXPR: c_int = 0x50001;

const C_TIME_STRINGS: &[u8] = b"Sun\0Mon\0Tue\0Wed\0Thu\0Fri\0Sat\0\
Sunday\0Monday\0Tuesday\0Wednesday\0Thursday\0Friday\0Saturday\0\
Jan\0Feb\0Mar\0Apr\0May\0Jun\0Jul\0Aug\0Sep\0Oct\0Nov\0Dec\0\
January\0February\0March\0April\0May\0June\0July\0August\0\
September\0October\0November\0December\0\
AM\0PM\0\
%a %b %e %T %Y\0%m/%d/%y\0%H:%M:%S\0%I:%M:%S %p\0\
\0\0%m/%d/%y\00123456789\0%a %b %e %T %Y\0%H:%M:%S\0";

const C_NUMERIC_STRINGS: &[u8] = b".\0\0";
const C_MESSAGES_STRINGS: &[u8] = b"^[yY]\0^[nN]\0";

unsafe fn langinfo_str(table: *const u8, mut idx: c_int) -> *mut c_char {
    let mut p = table;
    while idx > 0 {
        while *p != 0 { p = p.add(1); }
        p = p.add(1);
        idx -= 1;
    }
    p as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn nl_langinfo(item: c_int) -> *mut c_char {
    if item == NL_ITEM_CODESET {
        return if LOCALE_CTYPE_UTF8 {
            b"UTF-8\0".as_ptr() as *mut c_char
        } else {
            b"US-ASCII\0".as_ptr() as *mut c_char
        };
    }
    let cat = item >> 16;
    let idx = item & 0xFFFF;
    match cat {
        1 => {
            if idx > 1 { return b"\0".as_ptr() as *mut c_char; }
            langinfo_str(C_NUMERIC_STRINGS.as_ptr(), idx)
        }
        2 => {
            if idx > 49 { return b"\0".as_ptr() as *mut c_char; }
            langinfo_str(C_TIME_STRINGS.as_ptr(), idx)
        }
        5 => {
            if idx > 1 { return b"\0".as_ptr() as *mut c_char; }
            langinfo_str(C_MESSAGES_STRINGS.as_ptr(), idx)
        }
        _ => b"\0".as_ptr() as *mut c_char,
    }
}

pub type nl_catd = *mut c_void;

#[no_mangle]
pub unsafe extern "C" fn catopen(_name: *const c_char, _oflag: c_int) -> nl_catd {
    !0usize as nl_catd
}

#[no_mangle]
pub unsafe extern "C" fn catclose(_catd: nl_catd) -> c_int { 0 }

#[no_mangle]
pub unsafe extern "C" fn catgets(catd: nl_catd, _set_id: c_int, _msg_id: c_int, s: *const c_char) -> *mut c_char {
    if catd == !0usize as nl_catd {
        ERRNO!() = EBADF;
        return s as *mut c_char;
    }
    s as *mut c_char
}

pub type IconvT = *mut c_void;

const UNICODE_SURROGATE_START: u32 = 0xD800;
const UNICODE_HIGH_SURROGATE_END: u32 = 0xDBFF;
const UNICODE_LOW_SURROGATE_START: u32 = 0xDC00;
const UNICODE_SURROGATE_END: u32 = 0xDFFF;
const UNICODE_SURROGATE_END_EXCL: u32 = 0xE000;
const UNICODE_SUPPLEMENTARY_BASE: u32 = 0x10000;
const UNICODE_SUPPLEMENTARY_SIZE: u32 = 0x100000;
const UNICODE_MAX: u32 = 0x110000;

const ENC_UTF8: i32 = 1;
const ENC_UTF16LE: i32 = 2;
const ENC_UTF16BE: i32 = 3;
const ENC_UTF32LE: i32 = 4;
const ENC_UTF32BE: i32 = 5;
const ENC_WCHAR_T: i32 = 6;
const ENC_ASCII: i32 = 7;
const ENC_LATIN1: i32 = 8;
const ENC_WIN1252: i32 = 9;
const ENC_WIN1251: i32 = 10;
const ENC_KOI8R: i32 = 11;
const ENC_GBK: i32 = 12;
const ENC_GB2312: i32 = 13;
const ENC_BIG5: i32 = 14;
const ENC_EUCJP: i32 = 15;
const ENC_SHIFTJIS: i32 = 16;
const ENC_ISO8859_2: i32 = 17;
const ENC_ISO8859_3: i32 = 18;
const ENC_ISO8859_4: i32 = 19;
const ENC_ISO8859_5: i32 = 20;
const ENC_ISO8859_6: i32 = 21;
const ENC_ISO8859_7: i32 = 22;
const ENC_ISO8859_8: i32 = 23;
const ENC_ISO8859_9: i32 = 24;
const ENC_ISO8859_10: i32 = 25;
const ENC_ISO8859_11: i32 = 26;
const ENC_ISO8859_13: i32 = 27;
const ENC_ISO8859_14: i32 = 28;
const ENC_ISO8859_15: i32 = 29;
const ENC_ISO8859_16: i32 = 30;


fn make_cd(from: i32, to: i32) -> IconvT {
    ((from as usize) << 16 | (to as usize) << 1 | 1) as IconvT
}
fn extract_from(cd: IconvT) -> i32 {
    ((cd as usize) >> 16) as i32
}
fn extract_to(cd: IconvT) -> i32 {
    (((cd as usize) >> 1) & 0x7fff) as i32
}

unsafe fn match_name(input: *const u8, target: &[u8]) -> bool {
    let mut i = input;
    let mut j = 0;
    while *i != 0 && j < target.len() {
        let mut c = *i;
        if !c.is_ascii_alphanumeric() { i = i.add(1); continue; }
        c = c.to_ascii_lowercase();
        if c != target[j] { return false; }
        i = i.add(1);
        j += 1;
    }
    while *i != 0 && !(*i).is_ascii_alphanumeric() { i = i.add(1); }
    *i == 0 && j == target.len()
}

const ENCODING_ALIASES: &[(&[&[u8]], i32)] = &[
    (&[b"utf8", b"utf-8", b"char"], ENC_UTF8),
    (&[b"utf16le", b"utf-16le"], ENC_UTF16LE),
    (&[b"utf16be", b"utf-16be"], ENC_UTF16BE),
    (&[b"utf32le", b"utf-32le", b"ucs4le"], ENC_UTF32LE),
    (&[b"utf32be", b"utf-32be", b"ucs4be"], ENC_UTF32BE),
    (&[b"wchart", b"wchar-t"], ENC_WCHAR_T),
    (&[b"ascii", b"usascii", b"iso646"], ENC_ASCII),
    (&[b"iso88591", b"iso-8859-1", b"latin1"], ENC_LATIN1),
    (&[b"iso88592", b"iso-8859-2"], ENC_ISO8859_2),
    (&[b"iso88593", b"iso-8859-3"], ENC_ISO8859_3),
    (&[b"iso88594", b"iso-8859-4"], ENC_ISO8859_4),
    (&[b"iso88595", b"iso-8859-5"], ENC_ISO8859_5),
    (&[b"iso88596", b"iso-8859-6"], ENC_ISO8859_6),
    (&[b"iso88597", b"iso-8859-7"], ENC_ISO8859_7),
    (&[b"iso88598", b"iso-8859-8"], ENC_ISO8859_8),
    (&[b"iso88599", b"iso-8859-9"], ENC_ISO8859_9),
    (&[b"iso885910", b"iso-8859-10"], ENC_ISO8859_10),
    (&[b"iso885911", b"iso-8859-11", b"tis620"], ENC_ISO8859_11),
    (&[b"iso885913", b"iso-8859-13"], ENC_ISO8859_13),
    (&[b"iso885914", b"iso-8859-14"], ENC_ISO8859_14),
    (&[b"iso885915", b"iso-8859-15"], ENC_ISO8859_15),
    (&[b"iso885916", b"iso-8859-16"], ENC_ISO8859_16),
    (&[b"cp1252", b"windows1252", b"windows-1252"], ENC_WIN1252),
    (&[b"cp1251", b"windows1251", b"windows-1251"], ENC_WIN1251),
    (&[b"koi8r", b"koi8-r"], ENC_KOI8R),
    (&[b"gbk", b"cp936"], ENC_GBK),
    (&[b"gb2312"], ENC_GB2312),
    (&[b"big5", b"bigfive", b"cp950"], ENC_BIG5),
    (&[b"eucjp", b"euc-jp"], ENC_EUCJP),
    (&[b"shiftjis", b"sjis", b"cp932"], ENC_SHIFTJIS),
];

unsafe fn find_encoding(name: *const u8) -> i32 {
    if name.is_null() { return -1; }
    for &(aliases, enc) in ENCODING_ALIASES {
        for alias in aliases {
            if match_name(name, alias) {
                return enc;
            }
        }
    }
    -1
}

fn uni_to_jis(c: u32) -> u16 {
    let mut b = 0usize;
    let mut nel = REV_JIS.len();
    loop {
        let i = nel / 2;
        let j = REV_JIS[b + i] as usize;
        let row = j / 256;
        let col = j % 256;
        let d = JIS0208[row * 94 + col] as u32;
        if d == c { return (j as u16) + 0x2121; }
        if nel == 1 { return 0; }
        if c < d { nel /= 2; }
        else { b += i; nel -= nel / 2; }
    }
}

fn iso8859_decode_table(enc: i32) -> Option<&'static [u32; 128]> {
    match enc {
        ENC_ISO8859_2 => Some(&ISO8859_2_TO_U),
        ENC_ISO8859_3 => Some(&ISO8859_3_TO_U),
        ENC_ISO8859_4 => Some(&ISO8859_4_TO_U),
        ENC_ISO8859_5 => Some(&ISO8859_5_TO_U),
        ENC_ISO8859_6 => Some(&ISO8859_6_TO_U),
        ENC_ISO8859_7 => Some(&ISO8859_7_TO_U),
        ENC_ISO8859_8 => Some(&ISO8859_8_TO_U),
        ENC_ISO8859_9 => Some(&ISO8859_9_TO_U),
        ENC_ISO8859_10 => Some(&ISO8859_10_TO_U),
        ENC_ISO8859_11 => Some(&ISO8859_11_TO_U),
        ENC_ISO8859_13 => Some(&ISO8859_13_TO_U),
        ENC_ISO8859_14 => Some(&ISO8859_14_TO_U),
        ENC_ISO8859_15 => Some(&ISO8859_15_TO_U),
        ENC_ISO8859_16 => Some(&ISO8859_16_TO_U),
        _ => None,
    }
}

unsafe fn iconv_decode(enc: i32, src: *const u8, src_left: usize) -> (u32, usize, c_int) {
    if src_left == 0 { return (0, 0, EINVAL); }
    if let Some(table) = iso8859_decode_table(enc) {
        return (iso8859_to_u(table, *src), 1, 0);
    }
    match enc {
        ENC_UTF8 => {
            let c = *src;
            if c < 128 { return (c as u32, 1, 0); }
            if c < 0xC2 { return (0, 0, EILSEQ); }
            if c < 0xE0 {
                if src_left < 2 || (*src.add(1) & 0xC0) != 0x80 { return (0, 0, EILSEQ); }
                return (((c as u32 & 0x1F) << 6) | (*src.add(1) as u32 & 0x3F), 2, 0);
            }
            if c < 0xF0 {
                if src_left < 3 || (*src.add(1) & 0xC0) != 0x80 || (*src.add(2) & 0xC0) != 0x80 { return (0, 0, EILSEQ); }
                let cp = ((c as u32 & 0x0F) << 12) | ((*src.add(1) as u32 & 0x3F) << 6) | (*src.add(2) as u32 & 0x3F);
                if cp < 0x800 { return (0, 0, EILSEQ); }
                return (cp, 3, 0);
            }
            if c < 0xF8 {
                if src_left < 4 || (*src.add(1) & 0xC0) != 0x80 || (*src.add(2) & 0xC0) != 0x80 || (*src.add(3) & 0xC0) != 0x80 { return (0, 0, EILSEQ); }
                let cp = ((c as u32 & 0x07) << 18) | ((*src.add(1) as u32 & 0x3F) << 12) | ((*src.add(2) as u32 & 0x3F) << 6) | (*src.add(3) as u32 & 0x3F);
                if cp < UNICODE_SUPPLEMENTARY_BASE || cp >= UNICODE_MAX { return (0, 0, EILSEQ); }
                return (cp, 4, 0);
            }
            (0, 0, EILSEQ)
        }
        ENC_UTF16LE => {
            if src_left < 2 { return (0, 0, EINVAL); }
            let c = (*src as u32) | ((*src.add(1) as u32) << 8);
            if c >= UNICODE_SURROGATE_START && c <= UNICODE_HIGH_SURROGATE_END {
                if src_left < 4 { return (0, 0, EINVAL); }
                let d = (*src.add(2) as u32) | ((*src.add(3) as u32) << 8);
                if d < UNICODE_LOW_SURROGATE_START || d > UNICODE_SURROGATE_END { return (0, 0, EILSEQ); }
                return (((c - UNICODE_SURROGATE_START) << 10) + (d - UNICODE_LOW_SURROGATE_START) + UNICODE_SUPPLEMENTARY_BASE, 4, 0);
            }
            if c >= UNICODE_LOW_SURROGATE_START && c <= UNICODE_SURROGATE_END { return (0, 0, EILSEQ); }
            (c, 2, 0)
        }
        ENC_UTF16BE => {
            if src_left < 2 { return (0, 0, EINVAL); }
            let c = ((*src as u32) << 8) | (*src.add(1) as u32);
            if c >= UNICODE_SURROGATE_START && c <= UNICODE_HIGH_SURROGATE_END {
                if src_left < 4 { return (0, 0, EINVAL); }
                let d = ((*src.add(2) as u32) << 8) | (*src.add(3) as u32);
                if d < UNICODE_LOW_SURROGATE_START || d > UNICODE_SURROGATE_END { return (0, 0, EILSEQ); }
                return (((c - UNICODE_SURROGATE_START) << 10) + (d - UNICODE_LOW_SURROGATE_START) + UNICODE_SUPPLEMENTARY_BASE, 4, 0);
            }
            if c >= UNICODE_LOW_SURROGATE_START && c <= UNICODE_SURROGATE_END { return (0, 0, EILSEQ); }
            (c, 2, 0)
        }
        ENC_UTF32LE => {
            if src_left < 4 { return (0, 0, EINVAL); }
            let c = (*src as u32) | ((*src.add(1) as u32) << 8) | ((*src.add(2) as u32) << 16) | ((*src.add(3) as u32) << 24);
            if c >= UNICODE_SURROGATE_START && c < UNICODE_SURROGATE_END_EXCL { return (0, 0, EILSEQ); }
            if c >= UNICODE_MAX { return (0, 0, EILSEQ); }
            (c, 4, 0)
        }
        ENC_UTF32BE => {
            if src_left < 4 { return (0, 0, EINVAL); }
            let c = ((*src as u32) << 24) | ((*src.add(1) as u32) << 16) | ((*src.add(2) as u32) << 8) | (*src.add(3) as u32);
            if c >= UNICODE_SURROGATE_START && c < UNICODE_SURROGATE_END_EXCL { return (0, 0, EILSEQ); }
            if c >= UNICODE_MAX { return (0, 0, EILSEQ); }
            (c, 4, 0)
        }
        ENC_WCHAR_T => {
            if src_left < 4 { return (0, 0, EINVAL); }
            let c = core::ptr::read_unaligned(src as *const u32);
            if c >= UNICODE_SURROGATE_START && c < UNICODE_SURROGATE_END_EXCL { return (0, 0, EILSEQ); }
            if c >= UNICODE_MAX { return (0, 0, EILSEQ); }
            (c, 4, 0)
        }
        ENC_ASCII => {
            let c = *src as u32;
            if c >= 128 { return (0, 0, EILSEQ); }
            (c, 1, 0)
        }
        ENC_LATIN1 => (*src as u32, 1, 0),
        ENC_WIN1252 => {
            let b = *src;
            if b < 128 { return (b as u32, 1, 0); }
            (WIN1252_TO_U[b as usize - 128], 1, 0)
        }
        ENC_GBK | ENC_GB2312 => {
            let c = *src;
            if c < 128 { return (c as u32, 1, 0); }
            if c == 128 { return (0x20AC, 1, 0); }
            if src_left < 2 { return (0, 0, EINVAL); }
            let d = *src.add(1);
            if d < 0x40 || d == 0x7F || d > 0xFE { return (0, 0, EILSEQ); }
            let row = (c as usize).wrapping_sub(0x81);
            if row >= 126 { return (0, 0, EILSEQ); }
            let mut col = (d as usize) - 0x40;
            if col > 63 { col -= 1; }
            let cp = GB18030_TABLE[row * 190 + col] as u32;
            if cp == 0 { return (0, 0, EILSEQ); }
            (cp, 2, 0)
        }
        ENC_BIG5 => {
            let c = *src;
            if c < 128 { return (c as u32, 1, 0); }
            if src_left < 2 { return (0, 0, EINVAL); }
            let d = *src.add(1);
            if d < 0x40 || d == 0x7F || d > 0xFE { return (0, 0, EILSEQ); }
            let mut col = (d as usize) - 0x40;
            if col > 0x3E { col -= 0x22; }
            if c >= 0xA1 && c < 0xFA {
                let row = (c as usize) - 0xA1;
                if row >= 89 { return (0, 0, EILSEQ); }
                let cp = BIG5_TABLE[row * 157 + col] as u32;
                if cp == 0 { return (0, 0, EILSEQ); }
                return (cp, 2, 0);
            }
            (0, 0, EILSEQ)
        }
        ENC_EUCJP => {
            let c = *src;
            if c < 128 { return (c as u32, 1, 0); }
            if c == 0x8E {
                if src_left < 2 { return (0, 0, EINVAL); }
                let d = *src.add(1);
                if d < 0xA1 || d > 0xDF { return (0, 0, EILSEQ); }
                return ((d as u32) + 0xFF61 - 0xA1, 2, 0);
            }
            if c < 0xA1 { return (0, 0, EILSEQ); }
            if src_left < 2 { return (0, 0, EINVAL); }
            let d = *src.add(1);
            if d < 0xA1 || d > 0xFE { return (0, 0, EILSEQ); }
            let row = (c as usize) - 0xA1;
            let col = (d as usize) - 0xA1;
            if row >= 84 || col >= 94 { return (0, 0, EILSEQ); }
            let cp = JIS0208[row * 94 + col] as u32;
            if cp == 0 { return (0, 0, EILSEQ); }
            (cp, 2, 0)
        }
        ENC_SHIFTJIS => {
            let c = *src;
            if c < 128 { return (c as u32, 1, 0); }
            if c >= 0xA1 && c <= 0xDF { return ((c as u32) + 0xFF61 - 0xA1, 1, 0); }
            if src_left < 2 { return (0, 0, EINVAL); }
            let d = *src.add(1);
            if d < 0x40 || d == 0x7F || d > 0xFC { return (0, 0, EILSEQ); }
            let row = if c >= 129 && c <= 159 { (c as usize) - 129 }
                      else if c >= 224 && c <= 239 { (c as usize) - 193 }
                      else { return (0, 0, EILSEQ); };
            let (col, row_adj) = if d >= 64 && d <= 158 && d != 127 {
                let mut dd = d as usize;
                if dd > 127 { dd -= 1; }
                (dd - 64, row * 2)
            } else if d >= 159 && d <= 252 {
                ((d as usize) - 159, row * 2 + 1)
            } else {
                return (0, 0, EILSEQ);
            };
            if row_adj >= 84 { return (0, 0, EILSEQ); }
            let cp = JIS0208[row_adj * 94 + col] as u32;
            if cp == 0 { return (0, 0, EILSEQ); }
            (cp, 2, 0)
        }
        _ => (0, 0, EILSEQ),
    }
}

unsafe fn encode_single_byte(
    table: &[u32; 128],
    c: u32,
    dst: *mut u8,
    dst_left: usize,
) -> (usize, c_int) {
    if let Some(b) = u_to_iso8859(table, c) {
        if dst_left < 1 { return (0, E2BIG); }
        *dst = b;
        (1, 0)
    } else {
        (0, EILSEQ)
    }
}

unsafe fn iconv_encode(enc: i32, c: u32, dst: *mut u8, dst_left: usize) -> (usize, c_int) {
    // All ISO-8859 tables, Windows-1251 and KOI8-R share the same
    // ASCII-passthrough + reverse-table encoding.
    let single_byte_table = match enc {
        ENC_ISO8859_2 => Some(&ISO8859_2_TO_U),
        ENC_ISO8859_3 => Some(&ISO8859_3_TO_U),
        ENC_ISO8859_4 => Some(&ISO8859_4_TO_U),
        ENC_ISO8859_5 => Some(&ISO8859_5_TO_U),
        ENC_ISO8859_6 => Some(&ISO8859_6_TO_U),
        ENC_ISO8859_7 => Some(&ISO8859_7_TO_U),
        ENC_ISO8859_8 => Some(&ISO8859_8_TO_U),
        ENC_ISO8859_9 => Some(&ISO8859_9_TO_U),
        ENC_ISO8859_10 => Some(&ISO8859_10_TO_U),
        ENC_ISO8859_11 => Some(&ISO8859_11_TO_U),
        ENC_ISO8859_13 => Some(&ISO8859_13_TO_U),
        ENC_ISO8859_14 => Some(&ISO8859_14_TO_U),
        ENC_ISO8859_15 => Some(&ISO8859_15_TO_U),
        ENC_ISO8859_16 => Some(&ISO8859_16_TO_U),
        ENC_WIN1251 => Some(&WIN1251_TO_U),
        ENC_KOI8R => Some(&KOI8R_TO_U),
        _ => None,
    };
    if let Some(table) = single_byte_table {
        return encode_single_byte(table, c, dst, dst_left);
    }

    match enc {
        ENC_UTF8 => {
            if c < 0x80 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            if c < 0x800 {
                if dst_left < 2 { return (0, E2BIG); }
                *dst = (0xC0 | (c >> 6)) as u8;
                *dst.add(1) = (0x80 | (c & 0x3F)) as u8;
                return (2, 0);
            }
            if c < 0x10000 {
                if dst_left < 3 { return (0, E2BIG); }
                *dst = (0xE0 | (c >> 12)) as u8;
                *dst.add(1) = (0x80 | ((c >> 6) & 0x3F)) as u8;
                *dst.add(2) = (0x80 | (c & 0x3F)) as u8;
                return (3, 0);
            }
            if dst_left < 4 { return (0, E2BIG); }
            *dst = (0xF0 | (c >> 18)) as u8;
            *dst.add(1) = (0x80 | ((c >> 12) & 0x3F)) as u8;
            *dst.add(2) = (0x80 | ((c >> 6) & 0x3F)) as u8;
            *dst.add(3) = (0x80 | (c & 0x3F)) as u8;
            (4, 0)
        }
        ENC_UTF16LE => {
            if c < UNICODE_SUPPLEMENTARY_BASE {
                if dst_left < 2 { return (0, E2BIG); }
                *dst = (c & 0xFF) as u8;
                *dst.add(1) = (c >> 8) as u8;
                return (2, 0);
            }
            if dst_left < 4 { return (0, E2BIG); }
            let c = c - UNICODE_SUPPLEMENTARY_BASE;
            let hi = ((c >> 10) + UNICODE_SURROGATE_START) as u16;
            let lo = ((c & 0x3FF) + UNICODE_LOW_SURROGATE_START) as u16;
            *dst = (hi & 0xFF) as u8; *dst.add(1) = (hi >> 8) as u8;
            *dst.add(2) = (lo & 0xFF) as u8; *dst.add(3) = (lo >> 8) as u8;
            (4, 0)
        }
        ENC_UTF16BE => {
            if c < UNICODE_SUPPLEMENTARY_BASE {
                if dst_left < 2 { return (0, E2BIG); }
                *dst = (c >> 8) as u8;
                *dst.add(1) = (c & 0xFF) as u8;
                return (2, 0);
            }
            if dst_left < 4 { return (0, E2BIG); }
            let c = c - UNICODE_SUPPLEMENTARY_BASE;
            let hi = ((c >> 10) + UNICODE_SURROGATE_START) as u16;
            let lo = ((c & 0x3FF) + UNICODE_LOW_SURROGATE_START) as u16;
            *dst = (hi >> 8) as u8; *dst.add(1) = (hi & 0xFF) as u8;
            *dst.add(2) = (lo >> 8) as u8; *dst.add(3) = (lo & 0xFF) as u8;
            (4, 0)
        }
        ENC_UTF32LE => {
            if dst_left < 4 { return (0, E2BIG); }
            *dst = (c & 0xFF) as u8; *dst.add(1) = ((c >> 8) & 0xFF) as u8;
            *dst.add(2) = ((c >> 16) & 0xFF) as u8; *dst.add(3) = (c >> 24) as u8;
            (4, 0)
        }
        ENC_UTF32BE => {
            if dst_left < 4 { return (0, E2BIG); }
            *dst = (c >> 24) as u8; *dst.add(1) = ((c >> 16) & 0xFF) as u8;
            *dst.add(2) = ((c >> 8) & 0xFF) as u8; *dst.add(3) = (c & 0xFF) as u8;
            (4, 0)
        }
        ENC_WCHAR_T => {
            if dst_left < 4 { return (0, E2BIG); }
            core::ptr::write_unaligned(dst as *mut u32, c);
            (4, 0)
        }
        ENC_ASCII => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            (0, EILSEQ)
        }
        ENC_LATIN1 => {
            if c < 256 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            (0, EILSEQ)
        }
        ENC_WIN1252 => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            if c >= 0xA0 && c <= 0xFF && WIN1252_TO_U[c as usize - 128] == c {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            for i in 0..128usize {
                if WIN1252_TO_U[i] == c {
                    if dst_left < 1 { return (0, E2BIG); }
                    *dst = (i + 128) as u8; return (1, 0);
                }
            }
            (0, EILSEQ)
        }
        ENC_GBK | ENC_GB2312 => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            if c == 0x20AC {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = 128; return (1, 0);
            }
            for row in 0..126usize {
                for col in 0..190usize {
                    if GB18030_TABLE[row * 190 + col] as u32 == c {
                        if dst_left < 2 { return (0, E2BIG); }
                        *dst = (row + 0x81) as u8;
                        *dst.add(1) = if col <= 62 { (col + 0x40) as u8 } else { (col + 0x41) as u8 };
                        return (2, 0);
                    }
                }
            }
            (0, EILSEQ)
        }
        ENC_BIG5 => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            for row in 0..89usize {
                for col in 0..157usize {
                    if BIG5_TABLE[row * 157 + col] as u32 == c {
                        if dst_left < 2 { return (0, E2BIG); }
                        *dst = (row + 0xA1) as u8;
                        *dst.add(1) = if col <= 0x3E { (col + 0x40) as u8 } else { (col + 0x62) as u8 };
                        return (2, 0);
                    }
                }
            }
            (0, EILSEQ)
        }
        ENC_EUCJP => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            if c >= 0xFF61 && c <= 0xFF9F {
                if dst_left < 2 { return (0, E2BIG); }
                *dst = 0x8E;
                *dst.add(1) = (c - 0xFF61 + 0xA1) as u8;
                return (2, 0);
            }
            let jis = uni_to_jis(c);
            if jis == 0 { return (0, EILSEQ); }
            if dst_left < 2 { return (0, E2BIG); }
            *dst = ((jis >> 8) as u8).wrapping_add(0x80);
            *dst.add(1) = ((jis & 0xFF) as u8).wrapping_add(0x80);
            (2, 0)
        }
        ENC_SHIFTJIS => {
            if c < 128 {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = c as u8; return (1, 0);
            }
            if c >= 0xFF61 && c <= 0xFF9F {
                if dst_left < 1 { return (0, E2BIG); }
                *dst = (c - 0xFF61 + 0xA1) as u8;
                return (1, 0);
            }
            let jis = uni_to_jis(c);
            if jis == 0 { return (0, EILSEQ); }
            if dst_left < 2 { return (0, E2BIG); }
            let r = (jis >> 8) as usize;
            let d = (jis & 0xFF) as usize;
            *dst = ((r + 1) / 2 + if r < 95 { 112 } else { 176 }) as u8;
            *dst.add(1) = if r % 2 == 1 { (d + 31 + d / 96) as u8 } else { (d + 126) as u8 };
            (2, 0)
        }
        _ => (0, EILSEQ),
    }
}

#[no_mangle]
pub unsafe extern "C" fn iconv_open(tocode: *const c_char, fromcode: *const c_char) -> IconvT {
    let to = find_encoding(tocode as *const u8);
    let from = find_encoding(fromcode as *const u8);
    if to < 0 || from < 0 {
        ERRNO!() = EINVAL;
        return !0usize as IconvT;
    }
    make_cd(from, to)
}

#[no_mangle]
pub unsafe extern "C" fn iconv(
    cd: IconvT,
    inbuf: *mut *mut c_char,
    inbytesleft: *mut SizeT,
    outbuf: *mut *mut c_char,
    outbytesleft: *mut SizeT,
) -> SizeT {
    if cd == !0usize as IconvT {
        ERRNO!() = EINVAL;
        return !0usize;
    }
    // A null inbuf is the state-reset call; all converters are stateless.
    if inbuf.is_null() { return 0; }
    if inbytesleft.is_null() || outbuf.is_null() || outbytesleft.is_null() {
        ERRNO!() = EINVAL;
        return !0usize;
    }
    let from = extract_from(cd);
    let to = extract_to(cd);
    let mut src = *inbuf as *const u8;
    let mut dst = *outbuf as *mut u8;
    let mut src_left = *inbytesleft;
    let mut dst_left = *outbytesleft;
    let mut subst = 0usize;
    // Publish the conversion position before every error return; POSIX
    // requires *inbuf/*outbuf to point at the unconverted input/output.
    macro_rules! fail {
        ($e:expr) => {{
            *inbuf = src as *mut c_char;
            *outbuf = dst as *mut c_char;
            *inbytesleft = src_left;
            *outbytesleft = dst_left;
            ERRNO!() = $e;
            return !0usize;
        }};
    }
    while src_left > 0 {
        let (cp, consumed, err) = iconv_decode(from, src, src_left);
        if err != 0 {
            if err == EINVAL { break; }
            fail!(err);
        }
        let (written, err) = iconv_encode(to, cp, dst, dst_left);
        if err != 0 {
            if err == E2BIG {
                fail!(E2BIG);
            }
            if dst_left < 1 {
                fail!(E2BIG);
            }
            *dst = b'*';
            dst = dst.add(1);
            dst_left -= 1;
            subst += 1;
        } else {
            dst = dst.add(written);
            dst_left -= written;
        }
        src = src.add(consumed);
        src_left -= consumed;
    }
    *inbuf = src as *mut c_char;
    *outbuf = dst as *mut c_char;
    *inbytesleft = src_left;
    *outbytesleft = dst_left;
    subst
}

#[no_mangle]
pub unsafe extern "C" fn iconv_close(_cd: IconvT) -> c_int { 0 }

