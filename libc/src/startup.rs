// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// Startup: __libc_start_main
//
// musl crt1.o _start_c calls:
//   __libc_start_main(main, argc, argv, _init, _fini, rtld_fini, stack_end)
// ============================================================

// Auxiliary vector pointer, populated by the dynamic linker before
// constructors run so getauxval() works during early startup.
#[no_mangle]
pub static mut __auxv: *const usize = core::ptr::null();
#[no_mangle]
pub static mut __keel_static_base: usize = 0;
static mut RC_STATIC_EXEC: bool = false;

#[no_mangle]
pub static mut __progname: *mut c_char = core::ptr::null_mut();
#[no_mangle]
pub static mut __progname_full: *mut c_char = core::ptr::null_mut();
#[no_mangle]
pub static mut program_invocation_name: *mut c_char = core::ptr::null_mut();
#[no_mangle]
pub static mut program_invocation_short_name: *mut c_char = core::ptr::null_mut();

#[no_mangle]
pub static in6addr_any: [u32; 4] = [0; 4];
#[no_mangle]
pub static in6addr_loopback: [u32; 4] = [0, 0, 0, 0x0100_0000];

#[no_mangle]
pub static mut __daylight: c_int = 0;
#[no_mangle]
pub static mut __timezone: c_long = 0;
#[no_mangle]
pub static mut __tzname: [*mut c_char; 2] = [core::ptr::null_mut(); 2];

#[no_mangle]
pub unsafe extern "C" fn getauxval(type_: c_ulong) -> c_ulong {
    let mut p = __auxv;
    if p.is_null() {
        return 0;
    }
    loop {
        let tag = *p;
        if tag == 0 {
            return 0;
        }
        if tag == type_ as usize {
            return *p.add(1) as c_ulong;
        }
        p = p.add(2);
    }
}


unsafe fn run_static_init() {
    if !RC_STATIC_EXEC {
        return;
    }
    extern "C" {
        static __preinit_array_start: usize;
        static __preinit_array_end: usize;
        static __init_array_start: usize;
        static __init_array_end: usize;
    }
    let mut p = core::ptr::addr_of!(__preinit_array_start) as usize;
    let end = core::ptr::addr_of!(__preinit_array_end) as usize;
    while p < end {
        let f = core::ptr::read_unaligned(p as *const usize);
        if f != 0 {
            let f: unsafe extern "C" fn() = core::mem::transmute(f);
            f();
        }
        p += core::mem::size_of::<usize>();
    }
    p = core::ptr::addr_of!(__init_array_start) as usize;
    let end = core::ptr::addr_of!(__init_array_end) as usize;
    while p < end {
        let f = core::ptr::read_unaligned(p as *const usize);
        if f != 0 {
            let f: unsafe extern "C" fn() = core::mem::transmute(f);
            f();
        }
        p += core::mem::size_of::<usize>();
    }
}

type MainFn = unsafe extern "C" fn(c_int, *const *const c_char, *const *const c_char) -> c_int;
type InitFn = unsafe extern "C" fn();

#[cfg(target_arch = "x86_64")]
unsafe fn static_tls_tp() -> usize {
    let mut fs: u64 = 0;
    if arch_prctl(0x1003, &mut fs as *mut u64 as u64) != 0 {
        0
    } else {
        fs as usize
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn set_static_tls_tp(tp: usize) {
    let _ = arch_prctl(0x1002, tp as u64);
}

#[cfg(target_arch = "aarch64")]
unsafe fn static_tls_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mrs {}, tpidr_el0", out(reg) tp, options(nostack));
    tp
}

#[cfg(target_arch = "aarch64")]
unsafe fn set_static_tls_tp(tp: usize) {
    core::arch::asm!("msr tpidr_el0, {}", in(reg) tp, options(nostack));
}

#[cfg(target_arch = "riscv64")]
unsafe fn static_tls_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mv {}, tp", out(reg) tp, options(nostack));
    tp
}

#[cfg(target_arch = "powerpc64")]
unsafe fn static_tls_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mr {}, 13", out(reg) tp, options(nostack));
    tp
}

#[cfg(target_arch = "powerpc64")]
unsafe fn set_static_tls_tp(tp: usize) {
    core::arch::asm!("mr 13, {}", in(reg) tp, options(nostack));
}

#[cfg(target_arch = "loongarch64")]
unsafe fn static_tls_tp() -> usize {
    let tp: usize;
    core::arch::asm!("move {}, $tp", out(reg) tp, options(nostack));
    tp
}

#[cfg(target_arch = "loongarch64")]
unsafe fn set_static_tls_tp(tp: usize) {
    core::arch::asm!("move $tp, {}", in(reg) tp, options(nostack));
}

#[cfg(target_arch = "riscv64")]
unsafe fn set_static_tls_tp(tp: usize) {
    core::arch::asm!("mv tp, {}", in(reg) tp, options(nostack));
}

static mut STATIC_TLS_BLOCK_SIZE: usize = 0;
static mut STATIC_TLS_TP_OFFSET: usize = 0;
static mut STATIC_TLS_VAR_OFFSET: usize = 0;
static mut STATIC_TLS_TLS_SIZE: usize = 0;
static mut STATIC_TLS_IMAGE: *const u8 = core::ptr::null();
static mut STATIC_TLS_FILESZ: usize = 0;
static mut STATIC_TLS_CANARY: u64 = 0;

// Word indices in the 256-byte TCB. Keep in sync with libc/src/ldso/mod.rs.
const TCB_GENERATION: usize = 1;
const TCB_TLS_SIZE: usize = 2;
const TCB_MODULE_COUNT: usize = 3;
const TCB_TLSDESC: usize = 4;
const TCB_CANARY: usize = 5;
const TCB_ERRNO: usize = 6;
const TCB_GETRANDOM: usize = 7;
const TCB_THREAD: usize = 8;

unsafe fn static_tls_init_block(block: *mut u8) -> *mut u8 {
    if block.is_null() || STATIC_TLS_BLOCK_SIZE == 0 {
        return core::ptr::null_mut();
    }
    let var_base = block.add(STATIC_TLS_VAR_OFFSET);
    if STATIC_TLS_TLS_SIZE > 0 {
        core::ptr::write_bytes(var_base, 0, STATIC_TLS_TLS_SIZE);
    }
    if STATIC_TLS_FILESZ > 0 && !STATIC_TLS_IMAGE.is_null() {
        core::ptr::copy_nonoverlapping(STATIC_TLS_IMAGE, var_base, STATIC_TLS_FILESZ);
    }
    let tp = block.add(STATIC_TLS_TP_OFFSET);
    let tcb = if cfg!(target_arch = "x86_64") {
        tp as usize
    } else {
        block as usize
    };
    core::ptr::write_unaligned(tcb as *mut u64, tcb as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_GENERATION), 1);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_TLS_SIZE), STATIC_TLS_TLS_SIZE as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_MODULE_COUNT), 1);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_TLSDESC), 0);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_CANARY), STATIC_TLS_CANARY);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_ERRNO), 0);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_GETRANDOM), 0);
    core::ptr::write_unaligned((tcb as *mut u64).add(TCB_THREAD), 0);
    tp
}

unsafe fn static_tls_create_block() -> *mut u8 {
    if STATIC_TLS_BLOCK_SIZE == 0 {
        return core::ptr::null_mut();
    }
    let block = sys_mmap(
        core::ptr::null_mut(),
        STATIC_TLS_BLOCK_SIZE,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if block.is_null() || block == MMAP_FAILED {
        return core::ptr::null_mut();
    }
    static_tls_init_block(block)
}

#[repr(C)]
pub struct TlsIndex {
    pub ti_module: usize,
    pub ti_offset: usize,
}

/// Global-dynamic TLS access.  The dynamic loader intercepts this symbol for
/// dynamically linked programs and uses its own multi-module implementation.
/// A static executable has only module 0, so resolve directly against the
/// static TLS block created by `init_static_tls()`.
static mut TLS_GET_ADDR_IMPL: usize = 0;

/// The dynamic loader installs its multi-module TLS resolver here before
/// running constructors.  Keeping a dispatch pointer means a program that
/// obtains `__tls_get_addr` through `dlsym` still reaches the loader's
/// implementation rather than the static fallback below.
#[no_mangle]
pub unsafe extern "C" fn __keel_set_tls_get_addr(f: usize) {
    TLS_GET_ADDR_IMPL = f;
}

#[no_mangle]
pub unsafe extern "C" fn __tls_get_addr(ti: *const TlsIndex) -> *mut u8 {
    if TLS_GET_ADDR_IMPL != 0 {
        let f: unsafe extern "C" fn(*const TlsIndex) -> *mut u8 =
            core::mem::transmute(TLS_GET_ADDR_IMPL);
        return f(ti);
    }
    if ti.is_null() || (*ti).ti_module != 0 || !RC_STATIC_EXEC {
        return core::ptr::null_mut();
    }
    let tp = static_tls_tp();
    if tp == 0 {
        return core::ptr::null_mut();
    }
    #[cfg(target_arch = "powerpc64")]
    let dtp_bias = 0x8000usize;
    #[cfg(not(target_arch = "powerpc64"))]
    let dtp_bias = 0usize;
    (tp - STATIC_TLS_TP_OFFSET + STATIC_TLS_VAR_OFFSET + (*ti).ti_offset + dtp_bias) as *mut u8
}

/// Static executables have no dynamic loader to build a TCB.  Allocate one
/// from PT_TLS so that `%fs`/TP-relative TLS accesses and errno work.
unsafe fn init_static_tls() {
    if static_tls_tp() != 0 {
        return;
    }
    let phdr = getauxval(3) as usize; // AT_PHDR
    let phnum = getauxval(5) as usize; // AT_PHNUM
    let phent = getauxval(4) as usize; // AT_PHENT
    if phdr == 0 || phnum == 0 || phent < 56 {
        return;
    }

    let mut tls_vaddr = 0usize;
    let mut tls_filesz = 0usize;
    let mut tls_memsz = 0usize;
    let mut tls_align = 1usize;
    let mut exec_base = __keel_static_base;
    let mut i = 0usize;
    while i < phnum {
        let ph = phdr + i * phent;
        let p_type = core::ptr::read_unaligned(ph as *const u32);
        if p_type == 3 {
            // PT_INTERP means the dynamic loader already owns TLS.
            return;
        }
        if p_type == 6 {
            // PT_PHDR
            if exec_base == 0 {
                let p_vaddr = core::ptr::read_unaligned((ph + 16) as *const u64) as usize;
                exec_base = phdr.wrapping_sub(p_vaddr);
            }
        } else if p_type == 7 {
            // PT_TLS
            tls_vaddr = core::ptr::read_unaligned((ph + 16) as *const u64) as usize;
            tls_filesz = core::ptr::read_unaligned((ph + 32) as *const u64) as usize;
            tls_memsz = core::ptr::read_unaligned((ph + 40) as *const u64) as usize;
            let align = core::ptr::read_unaligned((ph + 48) as *const u64) as usize;
            if align > 1 {
                tls_align = align;
            }
        }
        i += 1;
    }

    let tcb_size = 256usize;
    #[cfg(target_arch = "x86_64")]
    let gap = 0usize;
    #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
    let gap = if cfg!(target_arch = "aarch64") { 16usize } else { 0usize };
    #[cfg(any(target_arch = "powerpc64", target_arch = "loongarch64"))]
    let gap = 0usize;

    let tls_size = if tls_align > 1 {
        (tls_memsz + tls_align - 1) & !(tls_align - 1)
    } else {
        tls_memsz
    };
    let total = tcb_size + gap + tls_size + tls_align;
    STATIC_TLS_BLOCK_SIZE = total;
    STATIC_TLS_TP_OFFSET = if cfg!(target_arch = "x86_64") {
        tls_size
    } else if cfg!(target_arch = "powerpc64") {
        // ELFv2 encodes TP-relative offsets with a -0x7000 bias, so TP sits
        // 0x7000 bytes above the TLS variable area.
        tcb_size + 0x7000
    } else {
        tcb_size
    };
    STATIC_TLS_VAR_OFFSET = if cfg!(target_arch = "x86_64") {
        0
    } else {
        tcb_size + gap
    };
    STATIC_TLS_TLS_SIZE = tls_size;
    STATIC_TLS_IMAGE = (exec_base + tls_vaddr) as *const u8;
    STATIC_TLS_FILESZ = tls_filesz;
    let random = getauxval(25) as *const u8; // AT_RANDOM
    STATIC_TLS_CANARY = if random.is_null() {
        0xdefaced_cafebeefu64
    } else {
        core::ptr::read_unaligned(random as *const u64)
            ^ core::ptr::read_unaligned(random.add(8) as *const u64)
    };

    let block = sys_mmap(
        core::ptr::null_mut(),
        total,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if block.is_null() || block == MMAP_FAILED {
        return;
    }
    let tp = static_tls_init_block(block);
    if tp.is_null() {
        return;
    }
    set_static_tls_tp(tp as usize);
    RC_STATIC_EXEC = true;
}


#[no_mangle]
pub unsafe extern "C" fn __libc_start_main(
    main: MainFn,
    argc: c_int,
    argv: *const *const c_char,
    _init: *const c_void,
    _fini: *const c_void,
    _rtld_fini: *const c_void,
    _stack_end: *const c_void,
) -> ! {
    let envp = argv.add((argc + 1) as usize);
    let mut aux = envp;
    while !(*aux).is_null() {
        aux = aux.add(1);
    }
    __auxv = aux.add(1) as *const usize;
    init_static_tls();
    errno_init();
    // Register the main thread during startup.  pthread_exit() uses the
    // thread table to decide whether it is the last thread; if the main
    // thread only registered lazily, a worker exiting first could call
    // exit() while the main thread was still alive.  Register slot 0
    // directly: pthread_self() would otherwise scan the whole static thread
    // table and fault in roughly 1 MiB on every process start.
    register_main_thread();

    __environ = envp as *mut *mut c_char;
    environ = __environ; // sync environ alias
    ___environ = __environ;
    _environ = __environ;

    // glibc-compatible program name globals.
    if !argv.is_null() && argc > 0 {
        let argv0 = *argv;
        __progname_full = argv0 as *mut c_char;
        program_invocation_name = argv0 as *mut c_char;
        let mut base = argv0;
        let mut p = argv0;
        while *p != 0 {
            if *p == b'/' as c_char { base = p.add(1); }
            p = p.add(1);
        }
        __progname = base as *mut c_char;
        program_invocation_short_name = base as *mut c_char;
    }

    __stdio_init();
    run_static_init();

    let mut set: SigSetT = 0;
    sigemptyset(&mut set);
    sigaddset(&mut set, SIGCANCEL);
    sigprocmask(SIG_UNBLOCK, &set, core::ptr::null_mut());

    if !_init.is_null() {
        let init_fn: InitFn = core::mem::transmute(_init);
        init_fn();
    }

    // aarch64/riscv64/loongarch64 GCC reads __stack_chk_guard as a global
    // (GOT-based for PIE) and powerpc64 through its TOC, so initialize it
    // from AT_RANDOM before calling main.  x86_64 uses %fs:0x28, which the
    // kernel already set up.
    #[cfg(any(
        target_arch = "aarch64",
        target_arch = "riscv64",
        target_arch = "powerpc64",
        target_arch = "loongarch64"
    ))]
    {
        let random_ptr = getauxval(25) as *const u8; // AT_RANDOM = 25
        if !random_ptr.is_null() {
            let a = core::ptr::read_unaligned(random_ptr as *const u64);
            let b = core::ptr::read_unaligned(random_ptr.add(8) as *const u64);
            __stack_chk_guard = (a ^ b) as usize;
        } else {
            // Linux always supplies AT_RANDOM; keep a nonzero fallback for
            // the unlikely case where the auxiliary vector is absent.
            __stack_chk_guard = 0xdefaced_cafebeef;
        }
    }

    let result = main(argc, argv, envp);

    if !_fini.is_null() {
        let fini_fn: InitFn = core::mem::transmute(_fini);
        fini_fn();
    }

    exit(result);
}

type LdsoDlopenFn = unsafe extern "C" fn(*const c_char, c_int) -> *mut c_void;
type LdsoDlsymFn = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void) -> *mut c_void;
type LdsoDlcloseFn = unsafe extern "C" fn(*mut c_void) -> c_int;
type LdsoDlerrorFn = unsafe extern "C" fn() -> *const c_char;

static mut LDSO_DLOPEN: Option<LdsoDlopenFn> = None;
static mut LDSO_DLSYM: Option<LdsoDlsymFn> = None;
static mut LDSO_DLCLOSE: Option<LdsoDlcloseFn> = None;
static mut LDSO_DLERROR: Option<LdsoDlerrorFn> = None;

#[no_mangle]
pub unsafe extern "C" fn __ldso_register_dlopen(f: LdsoDlopenFn) {
    LDSO_DLOPEN = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_register_dlsym(f: LdsoDlsymFn) {
    LDSO_DLSYM = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_register_dlclose(f: LdsoDlcloseFn) {
    LDSO_DLCLOSE = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_register_dlerror(f: LdsoDlerrorFn) {
    LDSO_DLERROR = Some(f);
}

#[no_mangle]
pub unsafe extern "C" fn dlopen(filename: *const c_char, flags: c_int) -> *mut c_void {
    if let Some(f) = LDSO_DLOPEN {
        f(filename, flags)
    } else {
        null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn dlsym_impl(
    handle: *mut c_void,
    symbol: *const c_char,
    ra: *mut c_void,
) -> *mut c_void {
    if let Some(f) = LDSO_DLSYM {
        f(handle, symbol, ra)
    } else {
        null_mut()
    }
}

// dlsym() must pass its own return address to the loader so that
// RTLD_NEXT can identify the calling object.  Keep the public entry
// point in assembly and tail-jump into the Rust implementation.
#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "x86_64")]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::arch::naked_asm!("mov rdx, [rsp]", "jmp dlsym_impl");
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::arch::naked_asm!("mov x2, x30", "b dlsym_impl");
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::arch::naked_asm!("mv a2, ra", "la t0, dlsym_impl", "jr t0");
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "powerpc64")]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::arch::naked_asm!(
        "addis 2, 12, .TOC.-dlsym@ha",
        "addi 2, 2, .TOC.-dlsym@l",
        "mflr 5",
        "b dlsym_impl",
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "loongarch64")]
pub unsafe extern "C" fn dlsym(_handle: *mut c_void, _symbol: *const c_char) -> *mut c_void {
    core::arch::naked_asm!("move $a2, $ra", "b dlsym_impl");
}

#[no_mangle]
pub unsafe extern "C" fn dlclose(handle: *mut c_void) -> c_int {
    if let Some(f) = LDSO_DLCLOSE {
        f(handle)
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn dlerror() -> *const c_char {
    if let Some(f) = LDSO_DLERROR {
        f()
    } else {
        core::ptr::null()
    }
}

