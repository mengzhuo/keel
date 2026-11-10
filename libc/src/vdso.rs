// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// vDSO fast paths
// ============================================================
//
// The kernel maps a small ELF image (AT_SYSINFO_EHDR) into every process.
// It exports clock_gettime, gettimeofday, getcpu and, on recent kernels,
// getrandom.  Calling them avoids a ring transition; musl does the same for
// clock_gettime and getcpu.  Every entry point here falls back to the real
// syscall when the symbol or the kernel support is missing.
//
// The vDSO uses two naming schemes.  x86_64 and riscv64 export
// __vdso_<name>; aarch64 exports __kernel_<name>.  Look both up and cache
// whichever one the current kernel provides.

const AT_SYSINFO_EHDR: c_ulong = 33;
const AT_PAGESZ: c_ulong = 6;

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const DT_HASH: u64 = 4;
const DT_STRTAB: u64 = 5;
const DT_SYMTAB: u64 = 6;

const STT_FUNC: u8 = 2;
const STB_GLOBAL: u8 = 1;
const STB_WEAK: u8 = 2;

const MADV_WIPEONFORK: i32 = 18;

// Opaque per-thread state for vDSO getrandom.  One pointer lives in the
// otherwise unused part of the 256-byte TCB (0x38; errno ends at 0x34).
const GETRANDOM_TCB_OFFSET: usize = 0x38;

// The state pointer uses 1 as an "allocation in progress" marker so a signal
// handler that re-enters getrandom falls back to the syscall instead of
// allocating a second state.
const STATE_ALLOCATING: usize = 1;

// vgetrandom_opaque_params from linux/random.h.
#[repr(C)]
struct VgetrandomParams {
    size_of_opaque_state: u32,
    mmap_prot: u32,
    mmap_flags: u32,
    reserved: [u32; 13],
}

const VGETRANDOM_UNINIT: usize = 0;
const VGETRANDOM_INITIALIZING: usize = 1;
const VGETRANDOM_READY: usize = 2;
const VGETRANDOM_DISABLED: usize = 3;

static VGETRANDOM_STATE: AtomicUsize = AtomicUsize::new(VGETRANDOM_UNINIT);
static VGETRANDOM_FN: AtomicUsize = AtomicUsize::new(0);
static VGETRANDOM_SIZE: AtomicUsize = AtomicUsize::new(0);
static VGETRANDOM_PROT: AtomicUsize = AtomicUsize::new(0);
static VGETRANDOM_FLAGS: AtomicUsize = AtomicUsize::new(0);
static VGETRANDOM_PAGE: AtomicUsize = AtomicUsize::new(0);

// Serializes first-time state allocation.  try_lock semantics are deliberate:
// a signal handler that re-enters getrandom while its own thread is
// allocating must fall back to the syscall, not spin forever.
static VGETRANDOM_ALLOC: AtomicUsize = AtomicUsize::new(0);

// usize::MAX means "not resolved yet"; 0 means "no vDSO".
static CLOCK_GETTIME_VDSO: AtomicUsize = AtomicUsize::new(usize::MAX);
static GETCPU_VDSO: AtomicUsize = AtomicUsize::new(usize::MAX);

#[inline]
unsafe fn read_u16(p: *const u8) -> u16 {
    u16::from_le(core::ptr::read_unaligned(p as *const u16))
}

#[inline]
unsafe fn read_u32(p: *const u8) -> u32 {
    u32::from_le(core::ptr::read_unaligned(p as *const u32))
}

#[inline]
unsafe fn read_u64(p: *const u8) -> u64 {
    u64::from_le(core::ptr::read_unaligned(p as *const u64))
}

unsafe fn cstr_eq(s: *const u8, want: &[u8]) -> bool {
    let mut i = 0usize;
    while i < want.len() {
        if *s.add(i) != want[i] {
            return false;
        }
        i += 1;
    }
    *s.add(want.len()) == 0
}

/// Look up a symbol in the vDSO by name.  Versions are ignored: the vDSO
/// exports exactly one definition of each entry point.
unsafe fn vdso_lookup(want: &[u8]) -> usize {
    let eh = crate::getauxval(AT_SYSINFO_EHDR) as usize;
    if eh == 0 {
        return 0;
    }
    let image = eh as *const u8;
    if read_u32(image) != 0x464c_457f || *image.add(4) != 2 {
        return 0; // not a 64-bit ELF image
    }
    let phoff = read_u64(image.add(32)) as usize;
    let phentsize = read_u16(image.add(54)) as usize;
    let phnum = read_u16(image.add(56)) as usize;

    let mut load_base = usize::MAX;
    let mut dynamic = 0usize;
    let mut i = 0usize;
    while i < phnum {
        let ph = image.add(phoff + i * phentsize);
        match read_u32(ph) {
            PT_LOAD => {
                let off = read_u64(ph.add(8)) as usize;
                let vaddr = read_u64(ph.add(16)) as usize;
                load_base = eh.wrapping_add(off).wrapping_sub(vaddr);
            }
            PT_DYNAMIC => {
                dynamic = eh.wrapping_add(read_u64(ph.add(8)) as usize);
            }
            _ => {}
        }
        i += 1;
    }
    if load_base == usize::MAX || dynamic == 0 {
        return 0;
    }

    let mut strtab = 0usize;
    let mut symtab = 0usize;
    let mut hash = 0usize;
    let mut dp = dynamic as *const u8;
    loop {
        let tag = read_u64(dp);
        if tag == 0 {
            break;
        }
        let val = read_u64(dp.add(8)) as usize;
        match tag {
            DT_STRTAB => strtab = load_base.wrapping_add(val),
            DT_SYMTAB => symtab = load_base.wrapping_add(val),
            DT_HASH => hash = load_base.wrapping_add(val),
            _ => {}
        }
        dp = dp.add(16);
    }
    if strtab == 0 || symtab == 0 || hash == 0 {
        return 0;
    }

    // DT_HASH starts with nbucket/nchain; the vDSO always carries one.
    let nsym = read_u32((hash + 4) as *const u8) as usize;
    let mut i = 1usize;
    while i < nsym {
        let sym = (symtab + i * 24) as *const u8;
        let info = *sym.add(4);
        if info & 0xf != STT_FUNC {
            i += 1;
            continue;
        }
        let bind = info >> 4;
        if bind != STB_GLOBAL && bind != STB_WEAK {
            i += 1;
            continue;
        }
        if read_u16(sym.add(6)) == 0 {
            i += 1;
            continue;
        }
        let name = strtab.wrapping_add(read_u32(sym) as usize);
        if cstr_eq(name as *const u8, want) {
            return load_base.wrapping_add(read_u64(sym.add(8)) as usize);
        }
        i += 1;
    }
    0
}

/// Try the two names the kernel architectures use for the same entry point.
unsafe fn vdso_lookup_either(vdso_name: &[u8], kernel_name: &[u8]) -> usize {
    let p = vdso_lookup(vdso_name);
    if p != 0 {
        return p;
    }
    vdso_lookup(kernel_name)
}

#[inline]
unsafe fn vdso_clock_gettime() -> usize {
    let p = CLOCK_GETTIME_VDSO.load(Ordering::Relaxed);
    if p != usize::MAX {
        return p;
    }
    let p = vdso_lookup_either(b"__vdso_clock_gettime", b"__kernel_clock_gettime");
    CLOCK_GETTIME_VDSO.store(p, Ordering::Relaxed);
    p
}

#[no_mangle]
pub unsafe extern "C" fn clock_gettime(clockid: c_int, ts: *mut timespec) -> c_int {
    let p = vdso_clock_gettime();
    if p != 0 {
        let f: unsafe extern "C" fn(c_int, *mut timespec) -> c_int = core::mem::transmute(p);
        let r = f(clockid, ts);
        if r == 0 {
            return 0;
        }
        if r == -EINVAL {
            ERRNO!() = EINVAL;
            return -1;
        }
        // Unsupported clock or an old vDSO: fall back to the syscall.
    }
    let r = sys_clock_gettime(clockid, ts);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        -1
    } else {
        0
    }
}

/// sched_getcpu() fast path.  Returns false when the vDSO is unavailable or
/// declines this CPU, in which case the caller issues the real syscall.
#[inline]
unsafe fn vdso_getcpu(cpu: *mut u32) -> bool {
    let p = GETCPU_VDSO.load(Ordering::Relaxed);
    let p = if p == usize::MAX {
        let p = vdso_lookup_either(b"__vdso_getcpu", b"__kernel_getcpu");
        GETCPU_VDSO.store(p, Ordering::Relaxed);
        p
    } else {
        p
    };
    if p == 0 {
        return false;
    }
    let f: unsafe extern "C" fn(*mut u32, *mut u32, *mut c_void) -> c_long =
        core::mem::transmute(p);
    f(cpu, core::ptr::null_mut(), core::ptr::null_mut()) == 0
}

// ============================================================
// vDSO getrandom
// ============================================================

#[inline]
unsafe fn tcb_slot(offset: usize) -> *mut usize {
    let tp = arch_get_tp_fast();
    if tp == 0 {
        return core::ptr::null_mut();
    }
    #[cfg(target_arch = "x86_64")]
    let tcb = tp;
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    let tcb = tp - 256;
    if core::ptr::read_unaligned(tcb as *const usize) != tcb {
        return core::ptr::null_mut();
    }
    (tcb + offset) as *mut usize
}

#[inline]
unsafe fn getrandom_slot() -> *mut usize {
    tcb_slot(GETRANDOM_TCB_OFFSET)
}

/// Query the vDSO once.  Returns true when getrandom() may use the vDSO and
/// the per-thread state layout has been recorded.
unsafe fn vgetrandom_init() -> bool {
    match VGETRANDOM_STATE.load(Ordering::Acquire) {
        VGETRANDOM_READY => return true,
        VGETRANDOM_DISABLED => return false,
        VGETRANDOM_UNINIT => {}
        // Another thread is querying right now; use the syscall rather than
        // waiting, so getrandom() always makes progress.
        _ => return false,
    }
    if VGETRANDOM_STATE
        .compare_exchange(
            VGETRANDOM_UNINIT,
            VGETRANDOM_INITIALIZING,
            Ordering::AcqRel,
            Ordering::Acquire,
        )
        .is_err()
    {
        return false;
    }

    let p = vdso_lookup_either(b"__vdso_getrandom", b"__kernel_getrandom");
    let mut ready = false;
    if p != 0 {
        let query: unsafe extern "C" fn(
            *mut u8,
            usize,
            u32,
            *mut VgetrandomParams,
            usize,
        ) -> isize = core::mem::transmute(p);
        let mut params: VgetrandomParams = core::mem::zeroed();
        if query(core::ptr::null_mut(), 0, 0, &mut params, usize::MAX) == 0 {
            let page = crate::getauxval(AT_PAGESZ) as usize;
            let page = if page == 0 { 4096 } else { page };
            let size = params.size_of_opaque_state as usize;
            if size > 0 && size <= page {
                VGETRANDOM_FN.store(p, Ordering::Relaxed);
                VGETRANDOM_SIZE.store(size, Ordering::Relaxed);
                VGETRANDOM_PROT.store(params.mmap_prot as usize, Ordering::Relaxed);
                VGETRANDOM_FLAGS.store(params.mmap_flags as usize, Ordering::Relaxed);
                VGETRANDOM_PAGE.store(page, Ordering::Relaxed);
                ready = true;
            }
        }
    }
    VGETRANDOM_STATE.store(
        if ready {
            VGETRANDOM_READY
        } else {
            VGETRANDOM_DISABLED
        },
        Ordering::Release,
    );
    ready
}

/// Get (or allocate) this thread's opaque vDSO state.  Returns null when the
/// syscall must be used instead.
unsafe fn vgetrandom_thread_state() -> *mut u8 {
    let slot = getrandom_slot();
    if slot.is_null() {
        return core::ptr::null_mut();
    }
    // The TCB is only 4-byte aligned when a loaded DSO's TLS alignment is 4,
    // so the slot must not be touched through an &AtomicUsize (unaligned
    // atomic references panic in debug builds and are not atomic on all
    // architectures).  Only this thread, including its signal handlers, uses
    // the slot; unaligned loads/stores plus a try-lock are sufficient.
    let mut state = core::ptr::read_unaligned(slot);
    if state == 0 {
        if VGETRANDOM_ALLOC
            .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return core::ptr::null_mut();
        }
        // A previous signal handler may have completed the allocation while
        // this invocation was interrupted before taking the lock.
        state = core::ptr::read_unaligned(slot);
        if state != 0 {
            VGETRANDOM_ALLOC.store(0, Ordering::Release);
            return if state == STATE_ALLOCATING {
                core::ptr::null_mut()
            } else {
                state as *mut u8
            };
        }
        core::ptr::write_unaligned(slot, STATE_ALLOCATING);
        let saved_errno = ERRNO!();
        let page = VGETRANDOM_PAGE.load(Ordering::Relaxed);
        let prot = VGETRANDOM_PROT.load(Ordering::Relaxed) as i32;
        let flags = VGETRANDOM_FLAGS.load(Ordering::Relaxed) as i32;
        let mapped = sys_mmap(core::ptr::null_mut(), page, prot, flags, -1, 0);
        if mapped.is_null() || mapped == MMAP_FAILED {
            ERRNO!() = saved_errno;
            core::ptr::write_unaligned(slot, 0);
            VGETRANDOM_ALLOC.store(0, Ordering::Release);
            return core::ptr::null_mut();
        }
        // fork() copies the state page.  Wiping it in the child forces the
        // kernel's generation check to reseed instead of reusing the parent's
        // buffered bytes.  Without this guarantee, do not use the vDSO.
        let advice = <Arch as Syscalls>::syscall3(
            sysno(28, 233),
            mapped as i64,
            page as i64,
            MADV_WIPEONFORK as i64,
        );
        if advice < 0 {
            ERRNO!() = saved_errno;
            sys_munmap(mapped, page);
            core::ptr::write_unaligned(slot, 0);
            VGETRANDOM_ALLOC.store(0, Ordering::Release);
            VGETRANDOM_STATE.store(VGETRANDOM_DISABLED, Ordering::Release);
            return core::ptr::null_mut();
        }
        // Keep the state from straddling a page boundary; mmap returns a
        // page-aligned block and vgetrandom_init() rejected oversized states.
        core::ptr::write_unaligned(slot, mapped as usize);
        VGETRANDOM_ALLOC.store(0, Ordering::Release);
        state = mapped as usize;
    }
    if state == STATE_ALLOCATING {
        return core::ptr::null_mut();
    }
    state as *mut u8
}

/// vDSO getrandom().  Returns None when the caller must issue the syscall.
unsafe fn vgetrandom(buf: *mut u8, len: usize, flags: u32) -> Option<isize> {
    if len == 0 || !vgetrandom_init() {
        return None;
    }
    let state = vgetrandom_thread_state();
    if state.is_null() {
        return None;
    }
    let p = VGETRANDOM_FN.load(Ordering::Relaxed);
    if p == 0 {
        return None;
    }
    let size = VGETRANDOM_SIZE.load(Ordering::Relaxed);
    let f: unsafe extern "C" fn(*mut u8, usize, u32, *mut u8, usize) -> isize =
        core::mem::transmute(p);
    Some(f(buf, len, flags, state, size))
}

/// Release a thread's getrandom state when it exits.
pub unsafe fn vdso_getrandom_thread_exit() {
    let slot = getrandom_slot();
    if slot.is_null() {
        return;
    }
    let state = core::ptr::read_unaligned(slot);
    if state > STATE_ALLOCATING {
        core::ptr::write_unaligned(slot, 0);
        let page = VGETRANDOM_PAGE.load(Ordering::Relaxed);
        sys_munmap(state as *mut u8, page);
    }
}
