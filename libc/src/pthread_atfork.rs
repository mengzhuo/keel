// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// pthread_atfork() handler registry and fork hook, musl-compatible.
// Included from libc/src/lib.rs; fork() must call __fork_handler(-1/0/1).

#[derive(Copy, Clone)]
struct atfork_funcs {
    prepare: Option<unsafe extern "C" fn()>,
    parent: Option<unsafe extern "C" fn()>,
    child: Option<unsafe extern "C" fn()>,
}

// The registry grows with realloc; musl also allocates one node per handler.
static mut ATFORK_FUNCS: *mut atfork_funcs = core::ptr::null_mut();
static mut ATFORK_CAP: usize = 0;
static ATFORK_NFUNCS: AtomicUsize = AtomicUsize::new(0);
static ATFORK_LOCK: AtomicI32 = AtomicI32::new(0);

#[inline]
unsafe fn atfork_lock() {
    while ATFORK_LOCK
        .compare_exchange_weak(0, 1, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
}

#[inline]
unsafe fn atfork_unlock() {
    ATFORK_LOCK.store(0, Ordering::Release);
}

// who < 0: prepare (reverse order, acquires lock)
// who == 0: parent (forward order, releases lock)
// who != 0: child (forward order, releases lock)
#[no_mangle]
pub unsafe extern "C" fn __fork_handler(who: c_int) {
    if who < 0 {
        atfork_lock();
        let n = ATFORK_NFUNCS.load(Ordering::Acquire);
        let base = ATFORK_FUNCS;
        if n == 0 || base.is_null() {
            return;
        }
        let mut i = n;
        while i > 0 {
            i -= 1;
            let entry = &*base.add(i);
            if let Some(f) = entry.prepare {
                f();
            }
        }
    } else {
        let n = ATFORK_NFUNCS.load(Ordering::Acquire);
        let base = ATFORK_FUNCS;
        if n != 0 && !base.is_null() {
            for i in 0..n {
                let entry = &*base.add(i);
                if who == 0 {
                    if let Some(f) = entry.parent {
                        f();
                    }
                } else if let Some(f) = entry.child {
                    f();
                }
            }
        }
        atfork_unlock();
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_atfork(
    prepare: Option<unsafe extern "C" fn()>,
    parent: Option<unsafe extern "C" fn()>,
    child: Option<unsafe extern "C" fn()>,
) -> c_int {
    atfork_lock();
    let n = ATFORK_NFUNCS.load(Ordering::Relaxed);
    if n == ATFORK_CAP {
        let new_cap = if ATFORK_CAP == 0 { 8 } else { ATFORK_CAP * 2 };
        let new = crate::realloc(
            ATFORK_FUNCS as *mut c_void,
            new_cap * core::mem::size_of::<atfork_funcs>(),
        ) as *mut atfork_funcs;
        if new.is_null() {
            atfork_unlock();
            return ENOMEM;
        }
        ATFORK_FUNCS = new;
        ATFORK_CAP = new_cap;
    }
    *ATFORK_FUNCS.add(n) = atfork_funcs { prepare, parent, child };
    ATFORK_NFUNCS.store(n + 1, Ordering::Release);
    atfork_unlock();
    0
}
