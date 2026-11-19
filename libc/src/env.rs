// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// getenv / setenv / putenv / unsetenv / clearenv
// ============================================================

#[no_mangle]
pub static mut __environ: *mut *mut c_char = core::ptr::null_mut();

// Growable tracking of strings allocated by setenv/putenv.  Realloc keeps
// this unbounded; if it fails the new string is simply left untracked.
static mut ENV_ALLOCED: *mut *mut c_char = core::ptr::null_mut();
static mut ENV_ALLOCED_N: usize = 0;
static mut ENV_ALLOCED_CAP: usize = 0;

unsafe fn env_rm_add(old: *mut c_char, mut new: *mut c_char) {
    for i in 0..ENV_ALLOCED_N {
        if (*ENV_ALLOCED.add(i)) == old {
            *ENV_ALLOCED.add(i) = new;
            if !old.is_null() {
                free(old as *mut c_void);
            }
            return;
        } else if (*ENV_ALLOCED.add(i)).is_null() && !new.is_null() {
            *ENV_ALLOCED.add(i) = new;
            new = core::ptr::null_mut();
        }
    }
    if !new.is_null() {
        if ENV_ALLOCED_N == ENV_ALLOCED_CAP {
            let new_cap = if ENV_ALLOCED_CAP == 0 { 16 } else { ENV_ALLOCED_CAP * 2 };
            let grown = realloc(
                ENV_ALLOCED as *mut c_void,
                new_cap * core::mem::size_of::<*mut c_char>(),
            ) as *mut *mut c_char;
            if grown.is_null() {
                return;
            }
            ENV_ALLOCED = grown;
            ENV_ALLOCED_CAP = new_cap;
        }
        *ENV_ALLOCED.add(ENV_ALLOCED_N) = new;
        ENV_ALLOCED_N += 1;
    }
}

unsafe fn strchrnul_u8(s: *const u8, c: u8) -> *const u8 {
    let mut p = s;
    while *p != 0 && *p != c {
        p = p.add(1);
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn getenv(name: *const c_char) -> *mut c_char {
    if __environ.is_null() || name.is_null() {
        return core::ptr::null_mut();
    }
    let name = name as *const u8;
    let l = strchrnul_u8(name, b'=') as usize - name as usize;
    if l == 0 || *name.add(l) != 0 {
        return core::ptr::null_mut();
    }
    let mut e = __environ;
    while !(*e).is_null() {
        let entry = *e as *const u8;
        if strncmp(entry, name, l) == 0 && *entry.add(l) == b'=' {
            return entry.add(l + 1) as *mut c_char;
        }
        e = e.add(1);
    }
    core::ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn setenv(
    var: *const c_char,
    value: *const c_char,
    overwrite: c_int,
) -> c_int {
    if var.is_null() || value.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let var = var as *const u8;
    let l1 = strchrnul_u8(var, b'=') as usize - var as usize;
    if l1 == 0 || *var.add(l1) != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    if overwrite == 0 && !getenv(var as *const c_char).is_null() {
        return 0;
    }
    let l2 = strlen(value as *const c_char);
    let s = malloc(l1 + l2 + 2) as *mut u8;
    if s.is_null() {
        return -1;
    }
    core::ptr::copy_nonoverlapping(var, s, l1);
    *s.add(l1) = b'=';
    core::ptr::copy_nonoverlapping(value as *const u8, s.add(l1 + 1), l2 + 1);
    putenv_internal(s as *mut c_char, l1, s as *mut c_char)
}

unsafe fn putenv_internal(s: *mut c_char, l: usize, r: *mut c_char) -> c_int {
    let mut i = 0;
    if !__environ.is_null() {
        let mut e = __environ;
        while !(*e).is_null() {
            if strncmp(s as *const u8, *e as *const u8, l + 1) == 0 {
                let tmp = *e;
                // putenv() was handed the very string already stored in
                // environ (a setenv-allocated entry).  It is still live and
                // tracked, so it must not be freed as a replaced allocation:
                // environ would keep pointing at freed storage.
                if tmp == s {
                    return 0;
                }
                *e = s;
                env_rm_add(tmp, r);
                return 0;
            }
            e = e.add(1);
            i += 1;
        }
    }
    let newenv = malloc(core::mem::size_of::<*mut c_char>() * (i + 2)) as *mut *mut c_char;
    if newenv.is_null() {
        if !r.is_null() { free(r as *mut c_void); }
        return -1;
    }
    if i > 0 && !__environ.is_null() {
        core::ptr::copy_nonoverlapping(__environ, newenv, i);
    }
    *newenv.add(i) = s;
    *newenv.add(i + 1) = core::ptr::null_mut();
    // don't free old __environ (may not be from our malloc)
    __environ = newenv;
    environ = __environ;
    if !r.is_null() {
        env_rm_add(core::ptr::null_mut(), r);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn putenv(s: *mut c_char) -> c_int {
    if s.is_null() { return -1; }
    let l = strchrnul_u8(s as *const u8, b'=') as usize - s as *const u8 as usize;
    if l == 0 || *s.add(l) as u8 == 0 {
        return unsetenv(s);
    }
    putenv_internal(s, l, core::ptr::null_mut())
}

#[no_mangle]
pub unsafe extern "C" fn unsetenv(name: *const c_char) -> c_int {
    if name.is_null() {
        ERRNO!() = EINVAL;
        return -1;
    }
    let name = name as *const u8;
    let l = strchrnul_u8(name, b'=') as usize - name as usize;
    if l == 0 || *name.add(l) != 0 {
        ERRNO!() = EINVAL;
        return -1;
    }
    if __environ.is_null() {
        return 0;
    }
    let mut e = __environ;
    let mut eo = e;
    while !(*e).is_null() {
        if strncmp(name, *e as *const u8, l) == 0 && *(*e as *const u8).add(l) == b'=' {
            env_rm_add(*e, core::ptr::null_mut());
        } else {
            if eo != e {
                *eo = *e;
            }
            eo = eo.add(1);
        }
        e = e.add(1);
    }
    if eo != e {
        *eo = core::ptr::null_mut();
    }
    environ = __environ;
    0
}

#[no_mangle]
pub unsafe extern "C" fn clearenv() -> c_int {
    let e = __environ;
    __environ = core::ptr::null_mut();
    environ = core::ptr::null_mut();
    if !e.is_null() {
        let mut p = e;
        while !(*p).is_null() {
            env_rm_add(*p, core::ptr::null_mut());
            p = p.add(1);
        }
    }
    0
}

