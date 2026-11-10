// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// crabc dynamic linker.
//
// Loads the executable and DT_NEEDED objects, resolves symbols, applies
// relocations, sets up TLS and transfers control to the program entry point.
// The architecture/syscall layer is in arch.rs.

#![no_std]
#![no_main]
#![allow(dead_code, deref_nullptr, static_mut_refs)]

use core::ffi::{c_char, c_void};
use core::sync::atomic::{AtomicBool, Ordering};

include!("arch.rs");

#[cfg(not(test))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ============================================================
// Constants
// ============================================================

const PT_LOAD: u32 = 1;
const PT_DYNAMIC: u32 = 2;
const PT_PHDR: u32 = 6;
const PT_TLS: u32 = 7;
const PF_R: u32 = 4;
const PF_W: u32 = 2;
const PF_X: u32 = 1;

const DT_NULL: u64 = 0;
const DT_NEEDED: u64 = 1;
const DT_PLTRELSZ: u64 = 2;
const DT_HASH: u64 = 4;
const DT_STRTAB: u64 = 5;
const DT_SYMTAB: u64 = 6;
const DT_RELA: u64 = 7;
const DT_RELASZ: u64 = 8;
const DT_STRSZ: u64 = 10;
const DT_INIT: u64 = 12;
const DT_FINI: u64 = 13;
const DT_RPATH: u64 = 15;
const DT_JMPREL: u64 = 23;
const DT_INIT_ARRAY: u64 = 25;
const DT_FINI_ARRAY: u64 = 26;
const DT_INIT_ARRAYSZ: u64 = 27;
const DT_FINI_ARRAYSZ: u64 = 28;
const DT_RUNPATH: u64 = 29;
const DT_GNU_HASH: u64 = 0x6ffffef5;
const DT_RELRSZ: u64 = 35;
const DT_RELR: u64 = 36;
const DT_RELRENT: u64 = 37;

const R_X86_64_64: u64 = 1;
const R_X86_64_COPY: u64 = 5;
const R_X86_64_GLOB_DAT: u64 = 6;
const R_X86_64_JUMP_SLOT: u64 = 7;
const R_X86_64_RELATIVE: u64 = 8;
const R_X86_64_DTPMOD64: u64 = 16;
const R_X86_64_DTPOFF64: u64 = 17;
const R_X86_64_TPOFF64: u64 = 18;

const R_AARCH64_NONE: u64 = 0;
const R_AARCH64_ABS64: u64 = 257;
const R_AARCH64_GLOB_DAT: u64 = 1025;
const R_AARCH64_JUMP_SLOT: u64 = 1026;
const R_AARCH64_RELATIVE: u64 = 1027;
const R_AARCH64_TLS_DTPMOD64: u64 = 1028;
const R_AARCH64_TLS_DTPREL64: u64 = 1029;
const R_AARCH64_TLS_TPREL64: u64 = 1030;
const R_AARCH64_TLSLE_ADD_TPREL_HI12: u64 = 549;
const R_AARCH64_TLSLE_ADD_TPREL_LO12: u64 = 550;
const R_AARCH64_TLSLE_ADD_TPREL_LO12_NC: u64 = 551;
const R_AARCH64_TLSDESC: u64 = 1031;

const R_RISCV_RELATIVE: u64 = 3;
const R_RISCV_64: u64 = 2;
const R_RISCV_GLOB_DAT: u64 = 5;  // skipped, RISC-V uses R_RISCV_64=2 as GLOB_DAT
const R_RISCV_JUMP_SLOT: u64 = 5;
const R_RISCV_TLS_DTPMOD64: u64 = 7;
const R_RISCV_TLS_DTPREL64: u64 = 9;
const R_RISCV_TLS_TPREL64: u64 = 11;
const R_RISCV_TLSDESC: u64 = 12;
const R_RISCV_COPY: u64 = 4;

const RTLD_LAZY: i32 = 1;
const RTLD_NOW: i32 = 2;
const RTLD_LOCAL: i32 = 0;
const RTLD_NOLOAD: i32 = 4;
const RTLD_NODELETE: i32 = 0x1000;
const RTLD_GLOBAL: i32 = 0x100;

const AT_NULL: u64 = 0;
const AT_PHDR: u64 = 3;
const AT_PHENT: u64 = 4;
const AT_PHNUM: u64 = 5;
const AT_PAGESZ: u64 = 6;
const AT_BASE: u64 = 7;
const AT_ENTRY: u64 = 9;
const AT_UID: u64 = 11;
const AT_EUID: u64 = 12;
const AT_GID: u64 = 13;
const AT_EGID: u64 = 14;
const AT_SECURE: u64 = 23;
const AT_RANDOM: u64 = 25;
const AT_SYSINFO_EHDR: u64 = 33;

const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const PROT_EXEC: i32 = 4;
const PROT_NONE: i32 = 0;
const MAP_PRIVATE: i32 = 0x02;
const MAP_FIXED: i32 = 0x10;
const MAP_ANONYMOUS: i32 = 0x20;
const MAP_FAILED: usize = !0usize;

const PHDR_SIZE: usize = 56;
const PH_TYPE: usize = 0;
const PH_FLAGS: usize = 4;
const PH_OFFSET: usize = 8;
const PH_VADDR: usize = 16;
const PH_FILESZ: usize = 32;
const PH_MEMSZ: usize = 40;
const PH_ALIGN: usize = 48;

const SYMTAB_ENT_SIZE: usize = 24;
const NO_IDX: usize = usize::MAX;
const TCB_SIZE: usize = 256;
const DSO_BASE_START: u64 = 0x200000000;
const DSO_BASE_STRIDE: u64 = 0x100000000;

// ============================================================
// Loaded object tracking
// ============================================================

struct LoadedObject {
    base: u64,
    symtab: *const u8,
    sym_count: usize,
    strtab: *const u8,
    strsz: usize,
    gnu_hash: usize,
    hash: usize,
    versym: usize,
    dyn_addr: usize,
    dyn_memsz: usize,
    tls_image: *const u8,
    tls_filesz: u64,
    tls_memsz: u64,
    tls_align: u64,
    init: u64,
    init_array: u64,
    init_array_sz: u64,
    init_present: bool,
    init_array_present: bool,
    global: bool,
    is_self: bool,
    // Identity of the file backing this object. Different names (libc.so vs
    // libc.musl-x86_64.so.1, or symlinks in LD_LIBRARY_PATH) must not load
    // the same library twice.
    dev: u64,
    ino: u64,
    map_start: u64,
    map_end: u64,
    needed_by: usize,
    dep_count: usize,
    dep_cap: usize,
    deps: *mut usize,
    rpath: *const u8,
    rpath_len: usize,
    nodelete: bool,
    name: *mut u8,
    name_cap: usize,
}

const EMPTY_OBJ: LoadedObject = LoadedObject {
    base: 0,
    symtab: core::ptr::null(),
    sym_count: 0,
    strtab: core::ptr::null(),
    strsz: 0,
    gnu_hash: 0,
    hash: 0,
    versym: 0,
    dyn_addr: 0,
    dyn_memsz: 0,
    tls_image: core::ptr::null(),
    tls_filesz: 0,
    tls_memsz: 0,
    tls_align: 0,
    init: 0,
    init_array: 0,
    init_array_sz: 0,
    init_present: false,
    init_array_present: false,
    global: false,
    is_self: false,
    dev: 0,
    ino: 0,
    map_start: 0,
    map_end: 0,
    needed_by: NO_IDX,
    dep_count: 0,
    dep_cap: 0,
    deps: core::ptr::null_mut(),
    rpath: core::ptr::null(),
    rpath_len: 0,
    nodelete: false,
    name: core::ptr::null_mut(),
    name_cap: 0,
};

// The object table keeps stable indices but grows by mmap'ing a larger
// array and copying; dependencies use the same stable indices.
static mut LOADED: *mut *mut LoadedObject = core::ptr::null_mut();
static mut LOADED_CAP: usize = 0;
static mut LOADED_COUNT: usize = 0;

static mut TLS_TOTAL_SIZE: usize = 0;
static mut TLS_LAYOUT_OFFSET: *mut usize = core::ptr::null_mut();
static mut TLS_FILESZ: *mut u64 = core::ptr::null_mut();
static mut TLS_MEMSZ: *mut u64 = core::ptr::null_mut();
static mut TLS_IMAGE: *mut *const u8 = core::ptr::null_mut();
static mut TLS_MODULE_COUNT: usize = 0;
// Modules whose offsets were fixed before the initial TLS block was built can
// use the register-only static descriptor.  Modules loaded later may not fit
// in every thread's block (threads created before the dlopen keep their old
// block), so their descriptors resolve through the per-thread dynamic path.
static mut STATIC_TLS_COUNT: usize = 0;
static mut TLS_GENERATION: u64 = 1;
static TLS_LOCK: AtomicBool = AtomicBool::new(false);
static mut LD_LIBRARY_PATH: *const u8 = core::ptr::null();
static mut RUNPATH: *const u8 = core::ptr::null();
static mut RUNPATH_LEN: usize = 0;
static mut ORIGIN_DIR: *mut u8 = core::ptr::null_mut();
static mut ORIGIN_CAP: usize = 0;
static mut ORIGIN_LEN: usize = 0;
static mut ORIGIN_READY: bool = false;
static mut LOADED_PATH: *mut u8 = core::ptr::null_mut();
static mut LOADED_PATH_CAP: usize = 0;
static mut LOADED_PATH_LEN: usize = 0;

unsafe fn map_zeroed(bytes: usize) -> *mut u8 {
    if bytes == 0 {
        return core::ptr::null_mut();
    }
    let p = sys_mmap(
        core::ptr::null_mut(),
        bytes,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if p as usize == MAP_FAILED {
        core::ptr::null_mut()
    } else {
        p
    }
}

unsafe fn ensure_loaded_capacity(need: usize) -> bool {
    if need <= LOADED_CAP {
        return true;
    }
    let mut cap = if LOADED_CAP == 0 { 16 } else { LOADED_CAP };
    while cap < need {
        cap = match cap.checked_mul(2) {
            Some(n) => n,
            None => return false,
        };
    }
    let obj_bytes = match cap.checked_mul(core::mem::size_of::<*mut LoadedObject>()) {
        Some(n) => n,
        None => return false,
    };
    let new_loaded = map_zeroed(obj_bytes) as *mut *mut LoadedObject;
    let new_off = map_zeroed(cap * core::mem::size_of::<usize>()) as *mut usize;
    let new_filesz = map_zeroed(cap * core::mem::size_of::<u64>()) as *mut u64;
    let new_memsz = map_zeroed(cap * core::mem::size_of::<u64>()) as *mut u64;
    let new_image = map_zeroed(cap * core::mem::size_of::<*const u8>()) as *mut *const u8;
    if new_loaded.is_null() || new_off.is_null() || new_filesz.is_null()
        || new_memsz.is_null() || new_image.is_null()
    {
        if !new_loaded.is_null() { sys_munmap(new_loaded as *mut u8, obj_bytes); }
        if !new_off.is_null() { sys_munmap(new_off as *mut u8, cap * core::mem::size_of::<usize>()); }
        if !new_filesz.is_null() { sys_munmap(new_filesz as *mut u8, cap * core::mem::size_of::<u64>()); }
        if !new_memsz.is_null() { sys_munmap(new_memsz as *mut u8, cap * core::mem::size_of::<u64>()); }
        if !new_image.is_null() { sys_munmap(new_image as *mut u8, cap * core::mem::size_of::<*const u8>()); }
        return false;
    }
    if !LOADED.is_null() {
        core::ptr::copy_nonoverlapping(LOADED, new_loaded, LOADED_COUNT);
        sys_munmap(
            LOADED as *mut u8,
            LOADED_CAP * core::mem::size_of::<*mut LoadedObject>(),
        );
        core::ptr::copy_nonoverlapping(TLS_LAYOUT_OFFSET, new_off, LOADED_COUNT);
        core::ptr::copy_nonoverlapping(TLS_FILESZ, new_filesz, LOADED_COUNT);
        core::ptr::copy_nonoverlapping(TLS_MEMSZ, new_memsz, LOADED_COUNT);
        core::ptr::copy_nonoverlapping(TLS_IMAGE, new_image, LOADED_COUNT);
        sys_munmap(TLS_LAYOUT_OFFSET as *mut u8, LOADED_CAP * core::mem::size_of::<usize>());
        sys_munmap(TLS_FILESZ as *mut u8, LOADED_CAP * core::mem::size_of::<u64>());
        sys_munmap(TLS_MEMSZ as *mut u8, LOADED_CAP * core::mem::size_of::<u64>());
        sys_munmap(TLS_IMAGE as *mut u8, LOADED_CAP * core::mem::size_of::<*const u8>());
    }
    LOADED = new_loaded;
    TLS_LAYOUT_OFFSET = new_off;
    TLS_FILESZ = new_filesz;
    TLS_MEMSZ = new_memsz;
    TLS_IMAGE = new_image;
    LOADED_CAP = cap;
    true
}

unsafe fn ensure_dep_capacity(obj: *mut LoadedObject, need: usize) -> bool {
    if need <= (*obj).dep_cap {
        return true;
    }
    let mut cap = if (*obj).dep_cap == 0 { 8 } else { (*obj).dep_cap };
    while cap < need {
        cap = match cap.checked_mul(2) {
            Some(n) => n,
            None => return false,
        };
    }
    let bytes = cap * core::mem::size_of::<usize>();
    let deps = map_zeroed(bytes) as *mut usize;
    if deps.is_null() {
        return false;
    }
    if !(*obj).deps.is_null() {
        core::ptr::copy_nonoverlapping((*obj).deps, deps, (*obj).dep_count);
        sys_munmap((*obj).deps as *mut u8, (*obj).dep_cap * core::mem::size_of::<usize>());
    }
    (*obj).deps = deps;
    (*obj).dep_cap = cap;
    true
}

unsafe fn ensure_name_capacity(obj: *mut LoadedObject, need: usize) -> bool {
    if need <= (*obj).name_cap {
        return true;
    }
    let mut cap = if (*obj).name_cap == 0 { 64 } else { (*obj).name_cap };
    while cap < need {
        cap = match cap.checked_mul(2) {
            Some(n) => n,
            None => return false,
        };
    }
    let name = map_zeroed(cap);
    if name.is_null() {
        return false;
    }
    if !(*obj).name.is_null() {
        core::ptr::copy_nonoverlapping((*obj).name, name, (*obj).name_cap);
        sys_munmap((*obj).name, (*obj).name_cap);
    }
    (*obj).name = name;
    (*obj).name_cap = cap;
    true
}

unsafe fn alloc_object() -> *mut LoadedObject {
    let obj = map_zeroed(core::mem::size_of::<LoadedObject>()) as *mut LoadedObject;
    if !obj.is_null() {
        *obj = EMPTY_OBJ;
    }
    obj
}

unsafe fn free_object(obj: *mut LoadedObject) {
    if obj.is_null() {
        return;
    }
    if !(*obj).deps.is_null() {
        sys_munmap((*obj).deps as *mut u8, (*obj).dep_cap * core::mem::size_of::<usize>());
    }
    if !(*obj).name.is_null() {
        sys_munmap((*obj).name, (*obj).name_cap);
    }
    sys_munmap(obj as *mut u8, core::mem::size_of::<LoadedObject>());
}

unsafe fn push_object(value: LoadedObject) -> Option<usize> {
    if !ensure_loaded_capacity(LOADED_COUNT + 1) {
        return None;
    }
    let obj = alloc_object();
    if obj.is_null() {
        return None;
    }
    *obj = value;
    *LOADED.add(LOADED_COUNT) = obj;
    LOADED_COUNT += 1;
    Some(LOADED_COUNT - 1)
}

// ============================================================
// _start: self-relocate ldso, then call run_main(sp)
// ============================================================

#[cfg(not(test))]
#[cfg(target_arch = "x86_64")]
core::arch::global_asm!(
    ".global _start",
    ".type _start, @function",
    "_start:",
    "mov rdi, rsp",
    "mov rax, [rsp]",
    "lea rbx, [rsp + 8]",
    "lea rcx, [rbx + rax*8]",
    "add rcx, 8",
    "2:",
    "cmp qword ptr [rcx], 0",
    "je 3f",
    "add rcx, 8",
    "jmp 2b",
    "3:",
    "add rcx, 8",
    "xor rsi, rsi",
    "4:",
    "mov rax, [rcx]",
    "cmp rax, 0",
    "je 5f",
    "cmp rax, 7",
    "jne 6f",
    "mov rsi, [rcx + 8]",
    "6:",
    "add rcx, 16",
    "jmp 4b",
    "5:",
    "mov rax, [rsi + 32]",
    "movzx rcx, word ptr [rsi + 56]",
    "lea r8, [rsi + rax]",
    "xor r9, r9",
    "7:",
    "cmp r9, rcx",
    "jge 8f",
    "mov eax, [r8]",
    "cmp eax, 2",
    "je 9f",
    "add r8, 56",
    "inc r9",
    "jmp 7b",
    "9:",
    "mov r10, [r8 + 16]",
    "mov r11, [r8 + 40]",
    "add r10, rsi",
    "xor rax, rax",
    "xor rbx, rbx",
    "mov rcx, r10",
    "lea rdx, [r10 + r11]",
    "10:",
    "cmp rcx, rdx",
    "jge 11f",
    "mov r12, [rcx]",
    "mov r13, [rcx + 8]",
    "cmp r12, 0",
    "je 11f",
    "cmp r12, 7",
    "jne 12f",
    "lea rax, [rsi + r13]",
    "12:",
    "cmp r12, 8",
    "jne 13f",
    "mov rbx, r13",
    "13:",
    "add rcx, 16",
    "jmp 10b",
    "11:",
    "test rbx, rbx",
    "jz 8f",
    "test rax, rax",
    "jz 8f",
    "xor rcx, rcx",
    "14:",
    "cmp rcx, rbx",
    "jge 8f",
    "mov r12, [rax + rcx]",
    "mov r13, [rax + rcx + 8]",
    "mov r14, [rax + rcx + 16]",
    "and r13d, 0xffffffff",
    "cmp r13d, 8",
    "jne 15f",
    "add r12, rsi",
    "add r14, rsi",
    "mov [r12], r14",
    "15:",
    "add rcx, 24",
    "jmp 14b",
    "8:",
    ".hidden {run_main}",
    "call {run_main}",
    "ud2",
    run_main = sym run_main,
);

// aarch64 _start: self-relocate ldso, then call run_main(sp, ldso_base)
#[cfg(not(test))]
#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".global _start",
    ".type _start, @function",
    "_start:",
    // Save sp into x29 (frame pointer, callee-saved)
    "mov x29, sp",
    // Walk stack: argc, argv[], NULL, envp[], NULL, auxv[]
    "ldr x0, [sp]",              // argc
    "add x1, sp, #8",            // &argv[0]
    "add x2, x1, x0, lsl #3",   // skip argv[]
    "add x2, x2, #8",            // skip NULL after argv -> &envp[0]
    "2:",
    "ldr x3, [x2]",
    "cbz x3, 3f",
    "add x2, x2, #8",
    "b 2b",
    "3:",
    "add x2, x2, #8",            // &auxv[0]
    "mov x20, #0",                // ldso_base = 0
    "4:",
    "ldr x3, [x2]",              // auxv tag
    "cbz x3, 5f",                // AT_NULL -> done
    "cmp x3, #7",                // AT_BASE
    "bne 6f",
    "ldr x20, [x2, #8]",         // ldso_base
    "6:",
    "add x2, x2, #16",
    "b 4b",
    "5:",
    // x20 = ldso_base. Walk ldso's ELF phdrs to find PT_DYNAMIC.
    "ldr x0, [x20, #32]",        // e_phoff
    "ldrh w1, [x20, #56]",       // e_phnum
    "add x2, x20, x0",           // phdr table
    "mov x3, #0",                 // i
    "7:",
    "cmp x3, x1",
    "bge 8f",
    "ldr w4, [x2]",              // p_type
    "cmp w4, #2",                // PT_DYNAMIC
    "beq 9f",
    "add x2, x2, #56",           // next phdr (PHDR_SIZE=56)
    "add x3, x3, #1",
    "b 7b",
    "9:",
    // Found PT_DYNAMIC. Read DT_RELA and DT_RELASZ from dynamic section.
    "ldr x4, [x2, #16]",         // p_vaddr
    "ldr x5, [x2, #40]",         // p_memsz
    "add x4, x4, x20",           // dyn_addr = base + p_vaddr
    "add x5, x4, x5",            // dyn_end
    "mov x6, #0",                 // rela = 0
    "mov x7, #0",                 // relasz = 0
    "10:",
    "cmp x4, x5",
    "bge 11f",
    "ldr x8, [x4]",              // d_tag
    "ldr x9, [x4, #8]",          // d_val
    "cbz x8, 11f",               // DT_NULL
    "cmp x8, #7",                // DT_RELA
    "bne 12f",
    "add x6, x20, x9",           // rela = base + d_val
    "12:",
    "cmp x8, #8",                // DT_RELASZ
    "bne 13f",
    "mov x7, x9",                // relasz = d_val
    "13:",
    "add x4, x4, #16",
    "b 10b",
    "11:",
    // Apply R_AARCH64_RELATIVE (type 1027) relocations.
    "cbz x7, 8f",
    "cbz x6, 8f",
    "add x8, x6, x7",            // table_end
    "14:",
    "cmp x6, x8",
    "bge 8f",
    "ldr x9, [x6]",              // r_offset
    "ldr x10, [x6, #8]",         // r_info
    "ldr x11, [x6, #16]",        // r_addend
    "cmp w10, #1027",             // R_AARCH64_RELATIVE
    "bne 15f",
    "add x9, x9, x20",           // slot = base + r_offset
    "add x11, x11, x20",         // val = base + r_addend
    "str x11, [x9]",
    "15:",
    "add x6, x6, #24",
    "b 14b",
    "8:",
    ".hidden {run_main}",
    "mov x0, x29",               // sp
    "mov x1, x20",               // ldso_base
    "bl {run_main}",
    "brk #1",
    run_main = sym run_main,
);

// riscv64 _start: self-relocate ldso, then call run_main(sp, ldso_base)
#[cfg(not(test))]
#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".global _start",
    ".type _start, @function",
    "_start:",
    // Save sp into s0 (frame pointer, callee-saved)
    "mv s0, sp",
    // Walk stack: argc, argv[], NULL, envp[], NULL, auxv[]
    "ld a0, 0(sp)",              // argc
    "add a1, sp, 8",            // &argv[0]
    "slli a2, a0, 3",
    "add a2, a1, a2",           // skip argv[]
    "addi a2, a2, 8",           // skip NULL after argv -> &envp[0]
    "2:",
    "ld a3, 0(a2)",
    "beqz a3, 3f",
    "addi a2, a2, 8",
    "j 2b",
    "3:",
    "addi a2, a2, 8",           // &auxv[0]
    "li s1, 0",                 // ldso_base = 0
    "4:",
    "ld a3, 0(a2)",             // auxv tag
    "beqz a3, 5f",              // AT_NULL -> done
    "li a4, 7",                 // AT_BASE
    "bne a3, a4, 6f",
    "ld s1, 8(a2)",             // ldso_base
    "6:",
    "addi a2, a2, 16",
    "j 4b",
    "5:",
    // s1 = ldso_base. Walk ldso's ELF phdrs to find PT_DYNAMIC.
    "ld a0, 32(s1)",            // e_phoff
    "lhu a1, 56(s1)",           // e_phnum
    "add a2, s1, a0",           // phdr table
    "li a3, 0",                 // i
    "7:",
    "bgeu a3, a1, 8f",
    "lw a4, 0(a2)",             // p_type
    "li a5, 2",                 // PT_DYNAMIC
    "beq a4, a5, 9f",
    "addi a2, a2, 56",          // next phdr (PHDR_SIZE=56)
    "addi a3, a3, 1",
    "j 7b",
    "9:",
    // Found PT_DYNAMIC. Read DT_RELA and DT_RELASZ from dynamic section.
    "ld a4, 16(a2)",            // p_vaddr
    "ld a5, 40(a2)",            // p_memsz
    "add a4, a4, s1",           // dyn_addr = base + p_vaddr
    "add a5, a4, a5",           // dyn_end
    "li a6, 0",                 // rela = 0
    "li a7, 0",                 // relasz = 0
    "10:",
    "bgeu a4, a5, 11f",
    "ld t0, 0(a4)",             // d_tag
    "ld t1, 8(a4)",             // d_val
    "beqz t0, 11f",             // DT_NULL
    "li t2, 7",                 // DT_RELA
    "bne t0, t2, 12f",
    "add a6, s1, t1",           // rela = base + d_val
    "12:",
    "li t2, 8",                 // DT_RELASZ
    "bne t0, t2, 13f",
    "mv a7, t1",                // relasz = d_val
    "13:",
    "addi a4, a4, 16",
    "j 10b",
    "11:",
    // Apply R_RISCV_RELATIVE (type 3) relocations.
    "beqz a7, 8f",
    "beqz a6, 8f",
    "add t3, a6, a7",           // table_end
    "14:",
    "bgeu a6, t3, 8f",
    "ld t4, 0(a6)",             // r_offset
    "ld t5, 8(a6)",             // r_info
    "ld t6, 16(a6)",            // r_addend
    "li t0, 3",                 // R_RISCV_RELATIVE
    "slli t1, t5, 32",
    "srli t1, t1, 32", // r_type = r_info & 0xffffffff
    "bne t1, t0, 15f",
    "add t4, t4, s1",           // slot = base + r_offset
    "add t6, t6, s1",           // val = base + r_addend
    "sd t6, 0(t4)",
    "15:",
    "addi a6, a6, 24",
    "j 14b",
    "8:",
    ".hidden {run_main}",
    "mv a0, s0",               // sp
    "mv a1, s1",               // ldso_base
    "call {run_main}",
    "unimp",
    run_main = sym run_main,
);

// ============================================================
// Entry point
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn run_main(sp: usize, ldso_base: u64) -> ! {
    unsafe { load_and_jump(sp, ldso_base) }
}

// ============================================================
// String helpers (no_std)
// ============================================================

unsafe fn str_len(s: *const u8) -> usize {
    let start = s as usize;
    let mut p = s;
    // Volatile keeps the byte loop from being turned back into a call to
    // strlen(), which would recurse into this function.
    while (p as usize) & 7 != 0 {
        if core::ptr::read_volatile(p) == 0 {
            return p as usize - start;
        }
        p = p.add(1);
    }
    const ONES: u64 = 0x0101_0101_0101_0101;
    const HIGHS: u64 = 0x8080_8080_8080_8080;
    loop {
        let w = core::ptr::read(p as *const u64);
        let z = w.wrapping_sub(ONES) & !w & HIGHS;
        if z != 0 {
            return (p as usize - start) + (z.trailing_zeros() as usize / 8);
        }
        p = p.add(8);
    }
}

unsafe fn sym_count_from_gnu_hash(gh: usize) -> usize {
    let nb = u32::from_le_bytes(core::ptr::read_unaligned(gh as *const [u8; 4])) as usize;
    let symoffset = u32::from_le_bytes(core::ptr::read_unaligned((gh + 4) as *const [u8; 4])) as usize;
    let bloom_size = u32::from_le_bytes(core::ptr::read_unaligned((gh + 8) as *const [u8; 4])) as usize;
    let buckets = gh + 16 + bloom_size * 8;
    let chain = buckets + nb * 4;
    let mut max_idx = 0usize;
    let mut has_any = false;
    for i in 0..nb {
        let symidx = u32::from_le_bytes(core::ptr::read_unaligned((buckets + i * 4) as *const [u8; 4])) as usize;
        if symidx == 0 || symidx < symoffset {
            continue;
        }
        let mut idx = symidx;
        loop {
            let cidx = idx - symoffset;
            if cidx > max_idx {
                max_idx = cidx;
            }
            has_any = true;
            let entry = u32::from_le_bytes(core::ptr::read_unaligned((chain + cidx * 4) as *const [u8; 4]));
            if entry & 1 != 0 {
                break;
            }
            idx += 1;
        }
    }
    if has_any {
        symoffset + max_idx + 1
    } else {
        symoffset
    }
}

unsafe fn sym_count_from_hash(h: usize) -> usize {
    let nchain = u32::from_le_bytes(core::ptr::read_unaligned((h + 4) as *const [u8; 4])) as usize;
    nchain
}

#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const c_char) -> usize {
    str_len(s as *const u8)
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    let a = a as *const u8;
    let b = b as *const u8;
    let mut i = 0usize;
    while i + 8 <= n {
        let wa = core::ptr::read_unaligned(a.add(i) as *const u64);
        let wb = core::ptr::read_unaligned(b.add(i) as *const u64);
        if wa != wb {
            break;
        }
        i += 8;
    }
    while i < n {
        let va = core::ptr::read_volatile(a.add(i));
        let vb = core::ptr::read_volatile(b.add(i));
        if va != vb {
            return va as i32 - vb as i32;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn bcmp(a: *const c_void, b: *const c_void, n: usize) -> i32 {
    memcmp(a, b, n)
}

/// Compare null-terminated `a` (with known length) against null-terminated `b`.
unsafe fn str_eq(a: *const u8, a_len: usize, b: *const u8) -> bool {
    let mut i = 0;
    while i < a_len {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    *b.add(a_len) == 0
}

/// Walk kernel-stack envp for a var starting with `prefix` (e.g. b"LD_LIBRARY_PATH=").
/// Returns pointer to the value part (after the '=') or None.
unsafe fn find_env(sp: usize, prefix: &[u8]) -> Option<*const u8> {
    let argc = *(sp as *const u64) as usize;
    // skip: argc + argv[0..argc] + NULL
    let mut p = sp + 8 + (argc + 1) * 8;
    loop {
        let env_ptr = *(p as *const u64) as *const u8;
        if env_ptr.is_null() {
            break;
        }
        let mut matches = true;
        let mut i = 0;
        while i < prefix.len() {
            if *env_ptr.add(i) != prefix[i] {
                matches = false;
                break;
            }
            i += 1;
        }
        if matches {
            return Some(env_ptr.add(prefix.len()));
        }
        p += 8;
    }
    None
}

const AT_FDCWD: i64 = -100;

// ============================================================
// Syscall wrappers (raw, no_std)
// ============================================================

fn sys_open(path: *const u8) -> i64 {
    unsafe { <Arch as Syscalls>::syscall3(SYS_OPENAT, AT_FDCWD, path as i64, 0) }
}

fn sys_readlink(path: *const u8, buf: *mut u8, bufsz: usize) -> i64 {
    unsafe { <Arch as Syscalls>::syscall4(SYS_READLINKAT, AT_FDCWD, path as i64, buf as i64, bufsz as i64) }
}

fn sys_read(fd: i64, buf: *mut u8, count: usize) -> i64 {
    unsafe { <Arch as Syscalls>::syscall3(SYS_READ, fd, buf as i64, count as i64) }
}

fn sys_write(fd: i64, buf: *const u8, count: usize) -> i64 {
    unsafe { <Arch as Syscalls>::syscall3(SYS_WRITE, fd, buf as i64, count as i64) }
}

fn sys_close(fd: i64) {
    unsafe { <Arch as Syscalls>::syscall1(SYS_CLOSE, fd); }
}

fn sys_mmap(
    addr: *mut u8,
    length: usize,
    prot: i32,
    flags: i32,
    fd: i32,
    offset: i64,
) -> *mut u8 {
    let result = unsafe { <Arch as Syscalls>::syscall6(SYS_MMAP, addr as i64, length as i64, prot as i64, flags as i64, fd as i64, offset) };
    if result < 0 && result > -4096 {
        return MAP_FAILED as *mut u8;
    }
    result as *mut u8
}

fn sys_exit(code: i32) -> ! {
    unsafe { <Arch as Syscalls>::syscall_noreturn1(SYS_EXIT, code as i64) }
}

fn sys_lseek(fd: i64, offset: i64) -> i64 {
    unsafe { <Arch as Syscalls>::syscall3(SYS_LSEEK, fd, offset, 0) }
}

#[cfg(target_arch = "x86_64")]
fn sys_arch_prctl(code: i64, addr: u64) -> i64 {
    unsafe { <Arch as Syscalls>::syscall2(SYS_ARCH_PRCTL, code, addr as i64) }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn read_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mov {}, fs:[0]", out(reg) tp);
    tp
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn read_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mrs {}, tpidr_el0", out(reg) tp);
    tp
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
unsafe fn read_tp() -> usize {
    let tp: usize;
    core::arch::asm!("mv {}, tp", out(reg) tp);
    tp
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn write_tp(addr: usize) {
    sys_arch_prctl(0x1002, addr as u64);
}

#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn write_tp(addr: usize) {
    core::arch::asm!("msr tpidr_el0, {}", in(reg) addr);
}

#[cfg(target_arch = "riscv64")]
#[inline(always)]
unsafe fn write_tp(addr: usize) {
    core::arch::asm!("mv tp, {}", in(reg) addr);
}

unsafe fn write_stderr(msg: &[u8]) {
    let _ = sys_write(2, msg.as_ptr(), msg.len());
}

unsafe fn write_hex_stderr(v: usize) {
    let mut buf = [0u8; 18];
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        let nibble = ((v >> (60 - i * 4)) & 0xf) as u8;
        buf[2 + i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
    }
    write_stderr(&buf);
}


unsafe fn die(code: i32, label: &[u8], detail: usize) -> ! {
    write_stderr(b"[ldso fatal ");
    write_stderr(label);
    write_stderr(b" ");
    write_hex_stderr(detail);
    write_stderr(b"]\n");
    sys_exit(code)
}

// ============================================================
// ELF helpers
// ============================================================

fn prot_from_flags(flags: u32) -> i32 {
    let mut prot = 0;
    if flags & PF_R != 0 {
        prot |= PROT_READ;
    }
    if flags & PF_W != 0 {
        prot |= PROT_WRITE;
    }
    if flags & PF_X != 0 {
        prot |= PROT_EXEC;
    }
    prot
}

// ============================================================
// Library search
// ============================================================

unsafe fn try_open(
    dir: *const u8,
    dir_len: usize,
    lib_name: *const u8,
    lib_name_len: usize,
) -> i64 {
    let bytes = match dir_len
        .checked_add(lib_name_len)
        .and_then(|n| n.checked_add(2))
    {
        Some(n) => n,
        None => return -1,
    };
    let buf = map_zeroed(bytes);
    if buf.is_null() {
        return -1;
    }
    core::ptr::copy_nonoverlapping(dir, buf, dir_len);
    *buf.add(dir_len) = b'/';
    core::ptr::copy_nonoverlapping(lib_name, buf.add(dir_len + 1), lib_name_len);
    *buf.add(dir_len + 1 + lib_name_len) = 0;
    let fd = sys_open(buf);
    sys_munmap(buf, bytes);
    fd
}

/// Resolve $ORIGIN lazily: only binaries whose RUNPATH uses it need the
/// /proc/self/exe readlink, and skipping it keeps startup lean.
unsafe fn ensure_origin() {
    if ORIGIN_READY {
        return;
    }
    ORIGIN_READY = true;
    let proc_exe = b"/proc/self/exe\0";
    let mut cap = 256usize;
    let mut exe_path = map_zeroed(cap);
    if exe_path.is_null() {
        return;
    }
    let len = loop {
        let r = sys_readlink(proc_exe.as_ptr(), exe_path, cap);
        if r <= 0 {
            sys_munmap(exe_path, cap);
            return;
        }
        if (r as usize) < cap {
            break r as usize;
        }
        let next = match cap.checked_mul(2) {
            Some(n) => n,
            None => {
                sys_munmap(exe_path, cap);
                return;
            }
        };
        let grown = map_zeroed(next);
        if grown.is_null() {
            sys_munmap(exe_path, cap);
            return;
        }
        sys_munmap(exe_path, cap);
        exe_path = grown;
        cap = next;
    };
    let mut slash = len;
    while slash > 0 {
        slash -= 1;
        if *exe_path.add(slash) == b'/' {
            break;
        }
    }
    ORIGIN_LEN = slash;
    if slash > ORIGIN_CAP {
        let grown = map_zeroed(slash);
        if grown.is_null() {
            sys_munmap(exe_path, cap);
            ORIGIN_LEN = 0;
            return;
        }
        if !ORIGIN_DIR.is_null() {
            sys_munmap(ORIGIN_DIR, ORIGIN_CAP);
        }
        ORIGIN_DIR = grown;
        ORIGIN_CAP = slash;
    }
    if slash > 0 {
        core::ptr::copy_nonoverlapping(exe_path, ORIGIN_DIR, slash);
    }
    sys_munmap(exe_path, cap);
}

unsafe fn try_open_expanded_origin(
    dir: *const u8,
    dir_len: usize,
    lib_name: *const u8,
    lib_name_len: usize,
    origin: *const u8,
    origin_len: usize,
) -> i64 {
    if dir_len >= 7 {
        let pat = b"$ORIGIN";
        let mut matches = true;
        let mut i = 0;
        while i < 7 {
            if *dir.add(i) != pat[i] {
                matches = false;
                break;
            }
            i += 1;
        }
        if matches {
            let rem_len = dir_len - 7;
            let expanded_len = match origin_len.checked_add(rem_len) {
                Some(n) => n,
                None => return -1,
            };
            let expanded = map_zeroed(expanded_len);
            if expanded.is_null() {
                return -1;
            }
            if origin_len > 0 {
                core::ptr::copy_nonoverlapping(origin, expanded, origin_len);
            }
            if rem_len > 0 {
                core::ptr::copy_nonoverlapping(dir.add(7), expanded.add(origin_len), rem_len);
            }
            let fd = try_open(expanded, expanded_len, lib_name, lib_name_len);
            sys_munmap(expanded, expanded_len);
            return fd;
        }
    }
    try_open(dir, dir_len, lib_name, lib_name_len)
}

unsafe fn try_open_expanded(
    dir: *const u8,
    dir_len: usize,
    lib_name: *const u8,
    lib_name_len: usize,
) -> i64 {
    ensure_origin();
    try_open_expanded_origin(
        dir, dir_len, lib_name, lib_name_len,
        ORIGIN_DIR, ORIGIN_LEN,
    )
}

unsafe fn object_origin(idx: usize) -> (*const u8, usize) {
    let obj = &(**LOADED.add(idx));
    let mut slash = 0usize;
    let mut found = false;
    if !obj.name.is_null() {
        let mut i = 0usize;
        while i < obj.name_cap && *obj.name.add(i) != 0 {
            if *obj.name.add(i) == b'/' {
                slash = i;
                found = true;
            }
            i += 1;
        }
    }
    if found {
        (obj.name, slash)
    } else {
        ensure_origin();
        (ORIGIN_DIR, ORIGIN_LEN)
    }
}

unsafe fn find_library_fd(
    lib_name: *const u8,
    lib_name_len: usize,
    ld_path: Option<*const u8>,
    needed_by: Option<usize>,
) -> i64 {
    // A name with a slash is a path, not something to search for.
    if lib_name_len > 0 {
        let mut i = 0usize;
        let mut has_slash = false;
        while i < lib_name_len {
            if *lib_name.add(i) == b'/' {
                has_slash = true;
                break;
            }
            i += 1;
        }
        if has_slash {
            let fd = sys_open(lib_name);
            return if fd >= 0 { fd } else { -1 };
        }
    }

    if let Some(ldp) = ld_path {
        let ldp_len = str_len(ldp);
        let mut start = 0usize;
        while start < ldp_len {
            let mut end = start;
            while end < ldp_len && *ldp.add(end) != b':' {
                end += 1;
            }
            if end > start {
                let fd = try_open(ldp.add(start), end - start, lib_name, lib_name_len);
                if fd >= 0 {
                    return fd;
                }
            }
            if end >= ldp_len {
                break;
            }
            start = end + 1;
        }
    }

    // Walk the requesting object's RUNPATH/RPATH, then its loader chain.
    let mut cur = needed_by;
    let mut guard = 0usize;
    while let Some(idx) = cur {
        if idx >= LOADED_COUNT || guard >= LOADED_COUNT {
            break;
        }
        guard += 1;
        let (rp, rp_len) = ((**LOADED.add(idx)).rpath, (**LOADED.add(idx)).rpath_len);
        if !rp.is_null() && rp_len > 0 {
            let (origin, origin_len) = object_origin(idx);
            let mut start = 0usize;
            while start < rp_len {
                let mut end = start;
                while end < rp_len && *rp.add(end) != b':' {
                    end += 1;
                }
                if end > start {
                    let fd = try_open_expanded_origin(
                        rp.add(start), end - start, lib_name, lib_name_len,
                        origin, origin_len,
                    );
                    if fd >= 0 {
                        return fd;
                    }
                }
                if end >= rp_len {
                    break;
                }
                start = end + 1;
            }
        }
        let parent = (**LOADED.add(idx)).needed_by;
        cur = if parent == NO_IDX { None } else { Some(parent as usize) };
    }

    let defaults: &[(&[u8], usize)] = &[
        (b"/lib", 4),
        (b"/usr/lib", 8),
        (b"/usr/local/lib", 14),
    ];
    for &(dir_bytes, dir_len) in defaults {
        let fd = try_open(dir_bytes.as_ptr(), dir_len, lib_name, lib_name_len);
        if fd >= 0 {
            return fd;
        }
    }

    -1
}

// ============================================================
// DSO loading
// ============================================================

/// Load a shared object from an already-open fd at the given base address.
/// Registers it in the LOADED array. Returns true on success.
fn sys_munmap(addr: *mut u8, length: usize) -> i64 {
    unsafe { <Arch as Syscalls>::syscall2(SYS_MUNMAP, addr as i64, length as i64) }
}

unsafe fn fd_identity(fd: i64) -> (u64, u64) {
    // st_dev and st_ino are the first two 64-bit words of struct stat on all
    // supported architectures.
    let mut st = [0u64; 32];
    let r = <Arch as Syscalls>::syscall2(SYS_FSTAT, fd, st.as_mut_ptr() as i64);
    if r < 0 {
        (0, 0)
    } else {
        (st[0], st[1])
    }
}

unsafe fn loaded_object_by_identity(dev: u64, ino: u64) -> Option<usize> {
    if dev == 0 && ino == 0 {
        return None;
    }
    for i in 1..LOADED_COUNT {
        if (**LOADED.add(i)).dev == dev && (**LOADED.add(i)).ino == ino {
            return Some(i);
        }
    }
    None
}

unsafe fn resolve_fd_path(fd: i64) -> usize {
    // Keep the /proc path in its own buffer: readlinkat reads the path while
    // writing the result, so aliasing the two would be undefined.
    let mut proc_path = [0u8; 64];
    let prefix = b"/proc/self/fd/";
    let mut pos = 0usize;
    for &b in prefix {
        if pos + 1 >= proc_path.len() { return 0; }
        proc_path[pos] = b;
        pos += 1;
    }
    let mut n = fd as u64;
    let mut digits = [0u8; 20];
    let mut d = 0usize;
    loop {
        digits[d] = b'0' + (n % 10) as u8;
        d += 1;
        n /= 10;
        if n == 0 { break; }
    }
    while d > 0 {
        d -= 1;
        if pos + 1 >= proc_path.len() { return 0; }
        proc_path[pos] = digits[d];
        pos += 1;
    }
    proc_path[pos] = 0;
    let mut cap = if LOADED_PATH_CAP == 0 { 256 } else { LOADED_PATH_CAP };
    loop {
        if cap > LOADED_PATH_CAP {
            let next = map_zeroed(cap);
            if next.is_null() {
                return 0;
            }
            if !LOADED_PATH.is_null() {
                sys_munmap(LOADED_PATH, LOADED_PATH_CAP);
            }
            LOADED_PATH = next;
            LOADED_PATH_CAP = cap;
        }
        let r = sys_readlink(proc_path.as_ptr(), LOADED_PATH, LOADED_PATH_CAP);
        if r <= 0 {
            return 0;
        }
        if (r as usize) < LOADED_PATH_CAP {
            return r as usize;
        }
        cap = match cap.checked_mul(2) {
            Some(n) => n,
            None => return 0,
        };
    }
}

unsafe fn load_dso_from_fd(fd: i64, desired_base: u64) -> Option<u64> {
    LOADED_PATH_LEN = resolve_fd_path(fd);
    let object_dev_ino = fd_identity(fd);
    let object_dev = object_dev_ino.0;
    let object_ino = object_dev_ino.1;
    let mut buf = [0u8; 4096];
    let n = sys_read(fd, buf.as_mut_ptr(), buf.len());
    if n < 64 {
        return None;
    }
    if buf[0] != 0x7f || buf[1] != b'E' {
        return None;
    }

    let e_phoff = u64::from_le_bytes(buf[32..40].try_into().unwrap());
    let e_phnum = u16::from_le_bytes(buf[56..58].try_into().unwrap()) as usize;
    let phdr_end = e_phoff as usize + e_phnum * PHDR_SIZE;
    if phdr_end > n as usize {
        return None;
    }

    #[derive(Copy, Clone)]
    struct LoadSeg {
        p_offset: u64,
        p_vaddr: u64,
        p_filesz: u64,
        p_memsz: u64,
        p_flags: u32,
    }

    let mut segs: [LoadSeg; 8] = [LoadSeg { p_offset: 0, p_vaddr: 0, p_filesz: 0, p_memsz: 0, p_flags: 0 }; 8];
    let mut seg_count: usize = 0;
    let mut min_vaddr = u64::MAX;
    let mut max_vaddr_end = 0u64;

    let mut tls_vaddr: u64 = 0;
    let mut tls_filesz: u64 = 0;
    let mut tls_memsz: u64 = 0;
    let mut tls_align: u64 = 0;

    for i in 0..e_phnum {
        let ph = buf.as_ptr().add(e_phoff as usize + i * PHDR_SIZE);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_TLS {
            tls_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            tls_filesz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_FILESZ) as *const [u8; 8]));
            tls_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
            tls_align = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_ALIGN) as *const [u8; 8]));
            continue;
        }
        if p_type != PT_LOAD {
            continue;
        }
        if seg_count >= segs.len() {
            return None;
        }
        let p_flags = u32::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_FLAGS) as *const [u8; 4]));
        let p_offset = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_OFFSET) as *const [u8; 8]));
        let p_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
        let p_filesz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_FILESZ) as *const [u8; 8]));
        let p_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
        segs[seg_count] = LoadSeg { p_offset, p_vaddr, p_filesz, p_memsz, p_flags };
        seg_count += 1;
        if p_vaddr < min_vaddr { min_vaddr = p_vaddr; }
        let end = p_vaddr + p_memsz;
        if end > max_vaddr_end { max_vaddr_end = end; }
    }

    if seg_count == 0 || min_vaddr == u64::MAX {
        return None;
    }

    const PAGE: u64 = 4096;
    let total_size = ((max_vaddr_end + PAGE - 1) & !(PAGE - 1)) as usize;

    // ponytail: ASLR may place our own stack or the executable at the
    // desired base. Probe for a free span before committing with MAP_FIXED.
    let mut base = desired_base;
    let actual_base = loop {
        let probe = sys_mmap(
            base as *mut u8,
            total_size,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        if probe as usize == base as usize {
            sys_munmap(probe, total_size);
            break base;
        }
        if probe as usize != MAP_FAILED {
            sys_munmap(probe, total_size);
        }
        base += DSO_BASE_STRIDE;
        if base > desired_base + DSO_BASE_STRIDE * 16 {
            return None;
        }
    };

    let tls_image = (actual_base + tls_vaddr) as *const u8;

    for i in 0..seg_count {
        let seg = segs[i];
        let adj = seg.p_vaddr & (PAGE - 1);
        let map_addr = actual_base + seg.p_vaddr - adj;
        let map_off = seg.p_offset - adj;
        let mem_len = ((seg.p_memsz + adj + PAGE - 1) & !(PAGE - 1)) as usize;
        let file_len = ((seg.p_filesz + adj + PAGE - 1) & !(PAGE - 1)) as usize;
        let prot = prot_from_flags(seg.p_flags);

        if file_len > 0 {
            let fp = sys_mmap(
                map_addr as *mut u8,
                file_len,
                prot,
                MAP_PRIVATE | MAP_FIXED,
                fd as i32,
                map_off as i64,
            );
            if fp as usize == MAP_FAILED {
                return None;
            }
        }

        if seg.p_memsz > seg.p_filesz {
            let vseg = actual_base + seg.p_vaddr;
            // The last file page can contain bytes beyond p_filesz; zero just
            // that part. The remaining BSS is fresh anonymous zero pages.
            let bss_start = vseg + seg.p_filesz;
            let bss_end = vseg + seg.p_memsz;
            let file_end = map_addr + file_len as u64;
            let zero_end = if file_end < bss_end { file_end } else { bss_end };
            if zero_end > bss_start {
                core::ptr::write_bytes(bss_start as *mut u8, 0, (zero_end - bss_start) as usize);
            }
            if mem_len > file_len {
                let tail = map_addr + file_len as u64;
                let tp = sys_mmap(
                    tail as *mut u8,
                    mem_len - file_len,
                    prot,
                    MAP_PRIVATE | MAP_FIXED | MAP_ANONYMOUS,
                    -1,
                    0,
                );
                if tp as usize == MAP_FAILED {
                    return None;
                }
            }
        }
    }

    // Find PT_DYNAMIC
    let mut dyn_vaddr: u64 = 0;
    let mut dyn_memsz: u64 = 0;
    for i in 0..e_phnum {
        let ph = buf.as_ptr().add(e_phoff as usize + i * PHDR_SIZE);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_DYNAMIC {
            dyn_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            dyn_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
            break;
        }
    }
    if dyn_vaddr == 0 {
        return None;
    }

    let dyn_addr = (actual_base + dyn_vaddr) as usize;
    let dyn_end = dyn_addr + dyn_memsz as usize;

    // Parse DT_SYMTAB, DT_STRTAB, DT_STRSZ
    let mut dt_symtab: u64 = 0;
    let mut dt_strtab: u64 = 0;
    let mut dt_strsz: u64 = 0;
    let mut dt_init: u64 = 0;
    let mut dt_init_array: u64 = 0;
    let mut dt_init_array_sz: u64 = 0;
    let mut dt_init_present = false;
    let mut dt_init_array_present = false;
    let mut dt_gnu_hash: u64 = 0;
    let mut dt_hash: u64 = 0;
    let mut dt_versym: u64 = 0;
    let mut dt_rpath_off: u64 = 0;
    let mut dt_rpath_present = false;
    let mut dt_runpath_off: u64 = 0;
    let mut dt_runpath_present = false;
    let mut dp = dyn_addr;
    while dp + 16 <= dyn_end {
        let d_tag = u64::from_le_bytes(core::ptr::read_unaligned(dp as *const [u8; 8]));
        let d_val = u64::from_le_bytes(core::ptr::read_unaligned((dp + 8) as *const [u8; 8]));
        if d_tag == DT_NULL {
            break;
        }
        match d_tag {
            DT_SYMTAB => dt_symtab = d_val,
            DT_STRTAB => dt_strtab = d_val,
            DT_STRSZ => dt_strsz = d_val,
            DT_GNU_HASH => dt_gnu_hash = d_val,
            DT_HASH => dt_hash = d_val,
            0x6ffffff0 => dt_versym = d_val, // DT_VERSYM
            DT_RUNPATH => { dt_runpath_off = d_val; dt_runpath_present = true; }
            DT_RPATH => { dt_rpath_off = d_val; dt_rpath_present = true; }
            DT_INIT => { dt_init = d_val; dt_init_present = true; }
            DT_INIT_ARRAY => { dt_init_array = d_val; dt_init_array_present = true; }
            DT_INIT_ARRAYSZ => dt_init_array_sz = d_val,
            _ => {}
        }
        dp += 16;
    }

    let symtab_ptr = (actual_base + dt_symtab) as *const u8;
    let strtab_ptr = (actual_base + dt_strtab) as *const u8;
    let strsz = dt_strsz as usize;
    let (rpath, rpath_len) = if dt_runpath_present {
        let p = (actual_base + dt_strtab + dt_runpath_off) as *const u8;
        (p, str_len(p))
    } else if dt_rpath_present {
        let p = (actual_base + dt_strtab + dt_rpath_off) as *const u8;
        (p, str_len(p))
    } else {
        (core::ptr::null(), 0)
    };

    let mut sym_count: usize = 0;
    if dt_gnu_hash != 0 {
        sym_count = sym_count_from_gnu_hash((actual_base + dt_gnu_hash) as usize);
    } else if dt_hash != 0 {
        sym_count = sym_count_from_hash((actual_base + dt_hash) as usize);
    } else if dt_strtab > dt_symtab && dt_strtab - dt_symtab >= SYMTAB_ENT_SIZE as u64 {
        sym_count = ((dt_strtab - dt_symtab) / SYMTAB_ENT_SIZE as u64) as usize;
    }

    {
        let value = LoadedObject {
            base: actual_base,
            symtab: symtab_ptr,
            sym_count,
            strtab: strtab_ptr,
            strsz,
            gnu_hash: if dt_gnu_hash != 0 { (actual_base + dt_gnu_hash) as usize } else { 0 },
            hash: if dt_hash != 0 { (actual_base + dt_hash) as usize } else { 0 },
            versym: if dt_versym != 0 { (actual_base + dt_versym) as usize } else { 0 },
            dyn_addr,
            dyn_memsz: dyn_memsz as usize,
            tls_image,
            tls_filesz,
            tls_memsz,
            tls_align,
            init: actual_base + dt_init,
            init_array: actual_base + dt_init_array,
            init_array_sz: dt_init_array_sz,
            init_present: dt_init_present,
            init_array_present: dt_init_array_present,
            global: false,
            is_self: false,
            dev: object_dev,
            ino: object_ino,
            map_start: actual_base + min_vaddr,
            map_end: actual_base + max_vaddr_end,
            needed_by: NO_IDX,
            dep_count: 0,
            dep_cap: 0,
            deps: core::ptr::null_mut(),
            rpath,
            rpath_len,
            nodelete: false,
            name: core::ptr::null_mut(),
            name_cap: 0,
        };
        if push_object(value).is_none() {
            return None;
        }
    }

    Some(actual_base)
}

// ============================================================
// Symbol resolution
// ============================================================

/// Look up symbol name from object's own symtab, then search all loaded objects.
unsafe fn resolve_symbol_from_index(obj_idx: usize, sym_idx: usize) -> u64 {
    let obj = &(**LOADED.add(obj_idx));
    if sym_idx == 0 || obj.symtab.is_null() || obj.strtab.is_null() {
        return 0;
    }
    let sym_entry = obj.symtab.add(sym_idx * SYMTAB_ENT_SIZE);
    let st_name = u32::from_le_bytes(core::ptr::read_unaligned(sym_entry as *const [u8; 4])) as usize;
    if st_name >= obj.strsz {
        return 0;
    }
    let name = obj.strtab.add(st_name);
    resolve_symbol(name)
}

/// Read a little-endian 32-bit word from an unaligned address.
#[inline]
unsafe fn ld_u32(p: *const u8) -> u32 {
    u32::from_le(core::ptr::read_unaligned(p as *const u32))
}

/// Read a little-endian 64-bit word from an unaligned address.
#[inline]
unsafe fn ld_u64(p: *const u8) -> u64 {
    u64::from_le(core::ptr::read_unaligned(p as *const u64))
}

#[inline]
unsafe fn hash_gnu(name: *const u8, len: usize) -> u32 {
    let mut h: u32 = 5381;
    let mut i = 0usize;
    while i < len {
        h = h.wrapping_mul(33).wrapping_add(*name.add(i) as u32);
        i += 1;
    }
    h
}

#[inline]
unsafe fn hash_sysv(name: *const u8, len: usize) -> u32 {
    let mut h: u32 = 0;
    let mut i = 0usize;
    while i < len {
        h = h.wrapping_shl(4).wrapping_add(*name.add(i) as u32);
        let g = h & 0xf000_0000;
        if g != 0 {
            h ^= g >> 24;
            h &= !g;
        }
        i += 1;
    }
    h
}

/// Return the symbol at `idx` if it is a defined global/weak symbol named
/// `name`, preserving the filtering the linear scan used to do.
unsafe fn match_sym(
    obj: &LoadedObject,
    idx: usize,
    name: *const u8,
    name_len: usize,
) -> Option<(u64, usize)> {
    if idx == 0 || idx >= obj.sym_count {
        return None;
    }
    let sym = obj.symtab.add(idx * SYMTAB_ENT_SIZE);
    let st_name = ld_u32(sym) as usize;
    let info = *sym.add(4);
    if info >> 4 == 0 {
        return None;
    }
    let shndx = u16::from_le_bytes(core::ptr::read_unaligned(sym.add(6) as *const [u8; 2]));
    let value = ld_u64(sym.add(8));
    // An undefined symbol must not satisfy a lookup.  ET_EXEC objects can
    // carry a non-zero st_value (the PLT stub) for an undefined function;
    // returning it makes the PLT jump to itself forever.
    if shndx == 0 || value == 0 || st_name >= obj.strsz {
        return None;
    }
    if !str_eq(name, name_len, obj.strtab.add(st_name)) {
        return None;
    }
    Some((obj.base + value, ld_u64(sym.add(16)) as usize))
}

/// A symbol is "default" when its .gnu.version entry does not have the hidden
/// bit set (i.e. it is exported as `foo@@VER`, not `foo@VER`). Symbol lookup
/// must prefer the default version; libnuma, for example, exports an old 1.1
/// ABI and a 1.2 ABI under the same name.
unsafe fn sym_is_default(obj: &LoadedObject, idx: usize) -> bool {
    if obj.versym == 0 || idx >= obj.sym_count {
        return true;
    }
    let v = u16::from_le_bytes(core::ptr::read_unaligned(
        (obj.versym as *const u8).add(idx * 2) as *const [u8; 2],
    ));
    v & 0x8000 == 0
}

unsafe fn lookup_gnu(
    obj: &LoadedObject,
    name: *const u8,
    name_len: usize,
) -> Option<(u64, usize)> {
    let gh = obj.gnu_hash as *const u8;
    let nbuckets = ld_u32(gh) as usize;
    let symoffset = ld_u32(gh.add(4)) as usize;
    let bloom_size = ld_u32(gh.add(8)) as usize;
    let bloom_shift = ld_u32(gh.add(12));
    if nbuckets == 0 || bloom_size == 0 || symoffset >= obj.sym_count {
        return None;
    }
    let bloom = (gh as usize + 16) as *const u8;
    let buckets = bloom.add(bloom_size * 8);
    let chain = buckets.add(nbuckets * 4);
    let h = hash_gnu(name, name_len);
    let word = ld_u64(bloom.add(((h as usize) / 64) % bloom_size * 8));
    let mask = (1u64 << (h % 64)) | (1u64 << ((h >> bloom_shift) % 64));
    if word & mask != mask {
        return None;
    }
    let mut idx = ld_u32(buckets.add((h as usize % nbuckets) * 4)) as usize;
    if idx < symoffset || idx >= obj.sym_count {
        return None;
    }
    let mut fallback = None;
    loop {
        let c = ld_u32(chain.add((idx - symoffset) * 4));
        if (c | 1) == (h | 1) {
            if let Some(r) = match_sym(obj, idx, name, name_len) {
                if sym_is_default(obj, idx) {
                    return Some(r);
                }
                if fallback.is_none() {
                    fallback = Some(r);
                }
            }
        }
        if c & 1 != 0 {
            break;
        }
        idx += 1;
        if idx >= obj.sym_count {
            break;
        }
    }
    fallback
}

unsafe fn lookup_sysv(
    obj: &LoadedObject,
    name: *const u8,
    name_len: usize,
) -> Option<(u64, usize)> {
    let h = obj.hash as *const u8;
    let nbucket = ld_u32(h) as usize;
    let nchain = ld_u32(h.add(4)) as usize;
    if nbucket == 0 || nchain == 0 {
        return None;
    }
    let buckets = h.add(8);
    let chains = buckets.add(nbucket * 4);
    let mut idx = ld_u32(buckets.add((hash_sysv(name, name_len) as usize % nbucket) * 4)) as usize;
    let mut fallback = None;
    while idx != 0 && idx < nchain {
        if let Some(r) = match_sym(obj, idx, name, name_len) {
            if sym_is_default(obj, idx) {
                return Some(r);
            }
            if fallback.is_none() {
                fallback = Some(r);
            }
        }
        idx = ld_u32(chains.add(idx * 4)) as usize;
    }
    fallback
}

unsafe fn lookup_linear(
    obj: &LoadedObject,
    name: *const u8,
    name_len: usize,
) -> Option<(u64, usize)> {
    let mut j = 0usize;
    let mut fallback = None;
    while j < obj.sym_count {
        if let Some(r) = match_sym(obj, j, name, name_len) {
            if sym_is_default(obj, j) {
                return Some(r);
            }
            if fallback.is_none() {
                fallback = Some(r);
            }
        }
        j += 1;
    }
    fallback
}

/// Search all loaded objects for a symbol with the given null-terminated name.
/// Returns the resolved address (base + st_value) or 0 if not found.
unsafe fn resolve_symbol(name: *const u8) -> u64 {
    resolve_symbol_with_size(name, usize::MAX).0
}

/// Same as resolve_symbol but also returns the defining symbol's st_size.
/// `exclude` is an object index to skip (use usize::MAX to skip nothing).
unsafe fn resolve_symbol_with_size(name: *const u8, exclude: usize) -> (u64, usize) {
    let name_len = str_len(name);
    if name_len == 0 {
        return (0, 0);
    }
    if str_eq(name, name_len, b"__tls_get_addr\0".as_ptr()) {
        return ((__tls_get_addr as *const () as usize) as u64, 0);
    }
    if str_eq(name, name_len, b"__rc_create_thread_tls\0".as_ptr()) {
        return ((__rc_create_thread_tls as *const () as usize) as u64, 0);
    }
    if str_eq(name, name_len, b"__rc_tls_block_size\0".as_ptr()) {
        return ((__rc_tls_block_size as *const () as usize) as u64, 0);
    }
    if str_eq(name, name_len, b"__rc_init_thread_tls_at\0".as_ptr()) {
        return ((__rc_init_thread_tls_at as *const () as usize) as u64, 0);
    }

    for i in 0..LOADED_COUNT {
        if i == exclude {
            continue;
        }
        let obj = &(**LOADED.add(i));
        if obj.symtab.is_null() || obj.strtab.is_null() {
            continue;
        }
        // Use the object's own hash table; only objects without one need
        // the linear walk.
        if obj.gnu_hash != 0 {
            if let Some(r) = lookup_gnu(obj, name, name_len) {
                return r;
            }
        } else if obj.hash != 0 {
            if let Some(r) = lookup_sysv(obj, name, name_len) {
                return r;
            }
        } else if let Some(r) = lookup_linear(obj, name, name_len) {
            return r;
        }
    }
    (0, 0)
}

unsafe fn resolve_copy_source(obj_idx: usize, sym_idx: usize) -> (u64, usize) {
    let obj = &(**LOADED.add(obj_idx));
    if sym_idx == 0 || obj.symtab.is_null() || obj.strtab.is_null() {
        return (0, 0);
    }
    if sym_idx * SYMTAB_ENT_SIZE >= obj.sym_count * SYMTAB_ENT_SIZE {
        return (0, 0);
    }
    let sym_entry = obj.symtab.add(sym_idx * SYMTAB_ENT_SIZE);
    let st_name =
        u32::from_le_bytes(core::ptr::read_unaligned(sym_entry as *const [u8; 4])) as usize;
    if st_name >= obj.strsz {
        return (0, 0);
    }
    let name = obj.strtab.add(st_name);
    resolve_symbol_with_size(name, obj_idx)
}

/// Resolve a (possibly undefined) TLS symbol to the module that defines it
/// and that module's TLS offset. Undefined TLS symbols have `st_value == 0`
/// in the referencing object; the real offset lives in the defining DSO.
unsafe fn resolve_tls_symbol(obj_idx: usize, sym_idx: usize) -> (usize, u64) {
    let obj = &(**LOADED.add(obj_idx));
    if sym_idx == 0 || obj.symtab.is_null() || obj.strtab.is_null() {
        return (obj_idx, 0);
    }
    if sym_idx * SYMTAB_ENT_SIZE >= obj.sym_count * SYMTAB_ENT_SIZE {
        return (obj_idx, 0);
    }
    let sym_entry = obj.symtab.add(sym_idx * SYMTAB_ENT_SIZE);
    let st_name =
        u32::from_le_bytes(core::ptr::read_unaligned(sym_entry as *const [u8; 4])) as usize;
    let st_shndx =
        u16::from_le_bytes(core::ptr::read_unaligned(sym_entry.add(6) as *const [u8; 2]));
    let st_value = u64::from_le_bytes(core::ptr::read_unaligned(sym_entry.add(8) as *const [u8; 8]));
    if st_shndx != 0 || st_name == 0 || st_name >= obj.strsz {
        // Defined here (or anonymous): use the local symbol.
        return (obj_idx, st_value);
    }
    let name = obj.strtab.add(st_name);
    let name_len = str_len(name);
    for i in 0..LOADED_COUNT {
        let o = &(**LOADED.add(i));
        if o.symtab.is_null() || o.strtab.is_null() {
            continue;
        }
        for j in 0..o.sym_count {
            let se = o.symtab.add(j * SYMTAB_ENT_SIZE);
            let s_name =
                u32::from_le_bytes(core::ptr::read_unaligned(se as *const [u8; 4])) as usize;
            let s_shndx =
                u16::from_le_bytes(core::ptr::read_unaligned(se.add(6) as *const [u8; 2]));
            if s_shndx == 0 || s_name >= o.strsz {
                continue;
            }
            let sym_name = o.strtab.add(s_name);
            if str_eq(name, name_len, sym_name) {
                let value =
                    u64::from_le_bytes(core::ptr::read_unaligned(se.add(8) as *const [u8; 8]));
                return (i, value);
            }
        }
    }
    (obj_idx, st_value)
}

unsafe fn resolve_symbol_module(obj_idx: usize, sym_idx: usize) -> usize {
    resolve_tls_symbol(obj_idx, sym_idx).0
}

unsafe fn tls_sym_offset(obj_idx: usize, sym_idx: usize) -> u64 {
    let obj = &(**LOADED.add(obj_idx));
    if sym_idx == 0 || obj.symtab.is_null() {
        return 0;
    }
    if sym_idx * SYMTAB_ENT_SIZE >= obj.sym_count * SYMTAB_ENT_SIZE {
        return 0;
    }
    let sym_entry = obj.symtab.add(sym_idx * SYMTAB_ENT_SIZE);
    u64::from_le_bytes(core::ptr::read_unaligned(sym_entry.add(8) as *const [u8; 8]))
}

unsafe fn tls_tprel_offset(obj_idx: usize, sym_idx: usize, addend: i64) -> i64 {
    let (module, sym_off) = resolve_tls_symbol(obj_idx, sym_idx);
    let off_in_mod = sym_off as i64 + addend;
    (*TLS_LAYOUT_OFFSET.add(module) as i64) + off_in_mod - (tls_var_area_offset_from_tp() as i64)
}

unsafe fn tls_var_area_offset_from_block() -> usize {
    #[cfg(target_arch = "x86_64")]
    { 0 }
    #[cfg(target_arch = "aarch64")]
    { TCB_SIZE }
    #[cfg(target_arch = "riscv64")]
    { TCB_SIZE }
}

unsafe fn tls_tcb_offset_from_block() -> usize {
    #[cfg(target_arch = "x86_64")]
    { TLS_TOTAL_SIZE }
    #[cfg(target_arch = "aarch64")]
    { 0 }
    #[cfg(target_arch = "riscv64")]
    { 0 }
}

unsafe fn tls_tp_offset_from_block() -> usize {
    #[cfg(target_arch = "x86_64")]
    { TLS_TOTAL_SIZE }
    #[cfg(target_arch = "aarch64")]
    { TCB_SIZE }
    #[cfg(target_arch = "riscv64")]
    { TCB_SIZE }
}

unsafe fn tls_var_area_offset_from_tp() -> usize {
    #[cfg(target_arch = "x86_64")]
    { TLS_TOTAL_SIZE }
    #[cfg(target_arch = "aarch64")]
    { 0 }
    #[cfg(target_arch = "riscv64")]
    { 0 }
}

unsafe fn tls_tcb_offset_from_tp() -> isize {
    #[cfg(target_arch = "x86_64")]
    { 0 }
    #[cfg(target_arch = "aarch64")]
    { -(TCB_SIZE as isize) }
    #[cfg(target_arch = "riscv64")]
    { -(TCB_SIZE as isize) }
}

// ============================================================
// Relocation processing
// ============================================================

/// Process all relocations for every loaded object.
unsafe fn process_all_relocations() {
    process_relocations_range(0);
}

/// Relocate the objects in `start..LOADED_COUNT`.
///
/// DT_RELR entries are applied as `*slot += base`, so re-running them over
/// objects that were already relocated adds the base twice and corrupts
/// every pointer. dlopen therefore relocates only the objects it just added.
unsafe fn process_relocations_range(start: usize) {
    // First pass: non-COPY relocations so source symbols have final values.
    for i in start..LOADED_COUNT {
        let (base, rela_off, rela_sz, jmprel_off, jmprel_sz, relr_off, relr_sz) = relocation_info(i);
        apply_relr(base, relr_off, relr_sz);
        apply_rela_table(i, base, rela_off, rela_sz, false);
        apply_rela_table(i, base, jmprel_off, jmprel_sz, false);
    }
    // Second pass: COPY relocations copy initialized data into the executable.
    for i in start..LOADED_COUNT {
        let (base, rela_off, rela_sz, _, _, _, _) = relocation_info(i);
        apply_rela_table(i, base, rela_off, rela_sz, true);
    }
}

unsafe fn relocation_info(i: usize) -> (u64, u64, u64, u64, u64, u64, u64) {
    let obj = &(**LOADED.add(i));
    let base = obj.base;
    let dp = obj.dyn_addr;
    let dyn_end = dp + obj.dyn_memsz;

    let mut rela_off: u64 = 0;
    let mut rela_sz: u64 = 0;
    let mut jmprel_off: u64 = 0;
    let mut jmprel_sz: u64 = 0;
    let mut relr_off: u64 = 0;
    let mut relr_sz: u64 = 0;

    let mut pos = dp;
    while pos + 16 <= dyn_end {
        let d_tag = u64::from_le_bytes(core::ptr::read_unaligned(pos as *const [u8; 8]));
        let d_val = u64::from_le_bytes(core::ptr::read_unaligned((pos + 8) as *const [u8; 8]));
        if d_tag == DT_NULL {
            break;
        }
        match d_tag {
            DT_RELA => rela_off = d_val,
            DT_RELASZ => rela_sz = d_val,
            DT_JMPREL => jmprel_off = d_val,
            DT_PLTRELSZ => jmprel_sz = d_val,
            DT_RELR => relr_off = d_val,
            DT_RELRSZ => relr_sz = d_val,
            _ => {}
        }
        pos += 16;
    }
    (base, rela_off, rela_sz, jmprel_off, jmprel_sz, relr_off, relr_sz)
}

/// Apply packed relative relocations (DT_RELR): entries are either an
/// address or a bitmap of the 63 words following the previous address.
unsafe fn apply_relr(base: u64, relr_off: u64, relr_sz: u64) {
    if relr_sz == 0 {
        return;
    }
    let table = (base + relr_off) as *const u8;
    let count = (relr_sz / 8) as usize;
    let mut where_: u64 = 0;
    let mut i = 0usize;
    while i < count {
        let e = u64::from_le_bytes(core::ptr::read_unaligned(table.add(i * 8) as *const [u8; 8]));
        if e & 1 == 0 {
            where_ = e;
            let slot = (base + where_) as *mut u64;
            *slot = (*slot).wrapping_add(base);
            where_ += 8;
        } else {
            let mut bits = e >> 1;
            let mut off = where_;
            while bits != 0 {
                if bits & 1 != 0 {
                    let slot = (base + off) as *mut u64;
                    *slot = (*slot).wrapping_add(base);
                }
                bits >>= 1;
                off += 8;
            }
            where_ += 63 * 8;
        }
        i += 1;
    }
}

/// Apply entries from one relocation table.
unsafe fn apply_rela_table(
    obj_idx: usize,
    base: u64,
    table_off: u64,
    table_sz: u64,
    copy_only: bool,
) {
    if table_sz == 0 {
        return;
    }
    let table = (base + table_off) as *const u8;
    let count = table_sz as usize / 24;

    for i in 0..count {
        let entry = table.add(i * 24);
        let r_offset = u64::from_le_bytes(core::ptr::read_unaligned(entry as *const [u8; 8]));
        let r_info = u64::from_le_bytes(core::ptr::read_unaligned(entry.add(8) as *const [u8; 8]));
        let r_addend =
            i64::from_le_bytes(core::ptr::read_unaligned(entry.add(16) as *const [u8; 8]));

        let r_type = r_info & 0xffffffff;
        let r_sym_idx = (r_info >> 32) as usize;
        let slot = (base + r_offset) as *mut u64;

        #[cfg(target_arch = "riscv64")]
        if r_type == R_RISCV_COPY {
            if !copy_only {
                continue;
            }
            let (src, sym_size) = resolve_copy_source(obj_idx, r_sym_idx);
            if src != 0 && sym_size != 0 {
                let dst = (base + r_offset) as *mut u8;
                core::ptr::copy_nonoverlapping(src as *const u8, dst, sym_size);
            }
            continue;
        }
        #[cfg(not(target_arch = "riscv64"))]
        if r_type == R_X86_64_COPY {
            if !copy_only {
                continue;
            }
            let (src, sym_size) = resolve_copy_source(obj_idx, r_sym_idx);
            if src != 0 && sym_size != 0 {
                let dst = (base + r_offset) as *mut u8;
                core::ptr::copy_nonoverlapping(src as *const u8, dst, sym_size);
            }
            continue;
        }
        if copy_only {
            continue;
        }

        match r_type {
            R_X86_64_RELATIVE | R_AARCH64_RELATIVE | R_RISCV_RELATIVE => {
                *slot = (base as i64 + r_addend) as u64;
            }
            R_X86_64_64 | R_AARCH64_ABS64 | R_RISCV_64 => {
                let sym_value = resolve_symbol_from_index(obj_idx, r_sym_idx);
                *slot = (sym_value as i64 + r_addend) as u64;
            }
            #[cfg(target_arch = "riscv64")]
            R_RISCV_JUMP_SLOT => {
                let sym_value = resolve_symbol_from_index(obj_idx, r_sym_idx);
                *slot = sym_value;
            }
            #[cfg(target_arch = "riscv64")]
            R_RISCV_TLS_DTPMOD64 => {
                let (module, _) = resolve_tls_symbol(obj_idx, r_sym_idx);
                *slot = module as u64;
            }
            #[cfg(target_arch = "riscv64")]
            R_RISCV_TLS_DTPREL64 => {
                let (_, sym_off) = resolve_tls_symbol(obj_idx, r_sym_idx);
                *slot = (sym_off as i64 + r_addend) as u64;
            }
            #[cfg(target_arch = "riscv64")]
            R_RISCV_TLS_TPREL64 => {
                let fs_off = tls_tprel_offset(obj_idx, r_sym_idx, r_addend);
                *slot = fs_off as u64;
            }
            #[cfg(not(target_arch = "riscv64"))]
            R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
                let sym_value = resolve_symbol_from_index(obj_idx, r_sym_idx);
                *slot = sym_value;
            }
            #[cfg(not(target_arch = "riscv64"))]
            R_AARCH64_GLOB_DAT | R_AARCH64_JUMP_SLOT => {
                let sym_value = resolve_symbol_from_index(obj_idx, r_sym_idx);
                *slot = sym_value;
            }
            R_X86_64_DTPMOD64 | R_AARCH64_TLS_DTPMOD64 => {
                let (module, _) = resolve_tls_symbol(obj_idx, r_sym_idx);
                *slot = module as u64;
            }
            R_X86_64_DTPOFF64 | R_AARCH64_TLS_DTPREL64 => {
                let (_, sym_off) = resolve_tls_symbol(obj_idx, r_sym_idx);
                *slot = (sym_off as i64 + r_addend) as u64;
            }
            R_X86_64_TPOFF64 | R_AARCH64_TLS_TPREL64 => {
                let fs_off = tls_tprel_offset(obj_idx, r_sym_idx, r_addend);
                *slot = fs_off as u64;
            }
            _ => {}
        }
        if r_type == R_AARCH64_TLSLE_ADD_TPREL_HI12 {
            let fs_off = tls_tprel_offset(obj_idx, r_sym_idx, r_addend);
            let insn = core::ptr::read_unaligned(slot as *const u32);
            let imm = ((fs_off >> 12) & 0xFFF) as u32;
            let new_insn = (insn & !(0xFFFu32 << 10)) | (imm << 10);
            core::ptr::write_unaligned(slot as *mut u32, new_insn);
        } else if r_type == R_AARCH64_TLSLE_ADD_TPREL_LO12 || r_type == R_AARCH64_TLSLE_ADD_TPREL_LO12_NC {
            let fs_off = tls_tprel_offset(obj_idx, r_sym_idx, r_addend);
            let insn = core::ptr::read_unaligned(slot as *const u32);
            let imm = (fs_off & 0xFFF) as u32;
            let new_insn = (insn & !(0xFFFu32 << 10)) | (imm << 10);
            core::ptr::write_unaligned(slot as *mut u32, new_insn);
        } else if r_type == R_AARCH64_TLSDESC || r_type == R_RISCV_TLSDESC {
            let (module, sym_off) = resolve_tls_symbol(obj_idx, r_sym_idx);
            let off_in_mod = sym_off as i64 + r_addend;
            let desc = slot as *mut [u64; 2];
            if module >= STATIC_TLS_COUNT && off_in_mod >= 0
                && (off_in_mod as u64) <= 0xffff_ffff && module <= 0xffff_ffff
            {
                // [module:32][offset:32]; the resolver maps this thread's
                // copy lazily, so a thread created before the dlopen still
                // sees the module's initial .tdata and its own writes.
                (*desc)[0] = __tlsdesc_dynamic as *const () as u64;
                (*desc)[1] = ((module as u64) << 32) | off_in_mod as u64;
            } else {
                let fs_off = tls_tprel_offset(obj_idx, r_sym_idx, r_addend);
                (*desc)[0] = __tlsdesc_static as *const () as u64;
                (*desc)[1] = fs_off as u64;
            }
            if cfg!(debug_assertions) {
                write_stderr(b"ldso: TLSDESC slot=");
                write_hex_stderr(slot as usize);
                write_stderr(b" resolver=");
                write_hex_stderr((*desc)[0] as usize);
                write_stderr(b" arg=");
                write_hex_stderr((*desc)[1] as usize);
                write_stderr(b"\n");
            }
        }
    }
}

unsafe fn run_constructors() {
    // Dependencies must be initialized before their dependents, and the
    // executable last; the load list is breadth-first, so walk it backwards.
    let mut i = LOADED_COUNT;
    while i > 0 {
        i -= 1;
        run_constructors_for(i);
    }
}

unsafe fn run_constructors_for(idx: usize) {
    let obj = &(**LOADED.add(idx));
    if obj.init_present && obj.init != 0 && obj.init != obj.base {
        let f: extern "C" fn() = core::mem::transmute(obj.init);
        f();
    }
    if obj.init_array_present && obj.init_array != 0 && obj.init_array_sz >= 8 {
        let count = (obj.init_array_sz / 8) as usize;
        for j in 0..count {
            let entry = (obj.init_array as *const u8).add(j * 8);
            let fp = u64::from_le_bytes(core::ptr::read_unaligned(entry as *const [u8; 8]));
            if fp != 0 {
                let f: extern "C" fn() = core::mem::transmute(fp);
                f();
            }
        }
    }
}

// TLS descriptor resolver for modules whose layout is fixed at startup.
// The descriptor is a two-word structure: [0] = resolver, [1] = precomputed
// TP offset.  The AArch64/RISC-V TLS descriptor ABI requires the resolver to
// preserve every register except the return register and the condition flags:
// the caller keeps TP in a caller-saved register across the call.  A normal
// Rust function clobbers those registers (the generated aarch64 sequence keeps
// TP in x2), so implement it in assembly exactly like musl.
#[cfg(not(target_arch = "x86_64"))]
extern "C" {
    fn __tlsdesc_static(desc: *const u64) -> u64;
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
unsafe extern "C" fn __tlsdesc_static(desc: *const u64) -> u64 {
    // x86_64 does not use TLSDESC in this toolchain; keep a referenceable
    // definition for the shared relocation path.
    core::ptr::read_unaligned(desc.add(1))
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".globl __tlsdesc_static",
    ".hidden __tlsdesc_static",
    ".type __tlsdesc_static,%function",
    "__tlsdesc_static:",
    "ldr x0, [x0, #8]",
    "ret",
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".globl __tlsdesc_static",
    ".hidden __tlsdesc_static",
    ".type __tlsdesc_static,%function",
    "__tlsdesc_static:",
    "ld a0, 8(a0)",
    "jr t0",
);

// Dynamic descriptor: descriptor[1] packs (module << 32) | offset.  The
// resolver must preserve every register except the result, so a small
// assembly shim saves the caller-saved set and calls the Rust helper.
#[cfg(not(target_arch = "x86_64"))]
extern "C" {
    fn __tlsdesc_dynamic(desc: *const u64) -> u64;
}

#[cfg(target_arch = "x86_64")]
#[no_mangle]
unsafe extern "C" fn __tlsdesc_dynamic(desc: *const u64) -> u64 {
    __rc_tlsdesc_dynamic(desc) as u64
}

#[cfg(target_arch = "aarch64")]
core::arch::global_asm!(
    ".globl __tlsdesc_dynamic",
    ".hidden __tlsdesc_dynamic",
    ".type __tlsdesc_dynamic,%function",
    "__tlsdesc_dynamic:",
    "stp x1, x2, [sp, #-288]!",
    "stp x3, x4, [sp, #16]",
    "stp x5, x6, [sp, #32]",
    "stp x7, x8, [sp, #48]",
    "stp x9, x10, [sp, #64]",
    "stp x11, x12, [sp, #80]",
    "stp x13, x14, [sp, #96]",
    "stp x15, x16, [sp, #112]",
    "stp x17, x18, [sp, #128]",
    "str x30, [sp, #144]",
    "stp q0, q1, [sp, #160]",
    "stp q2, q3, [sp, #192]",
    "stp q4, q5, [sp, #224]",
    "stp q6, q7, [sp, #256]",
    "bl __rc_tlsdesc_dynamic",
    "ldp q6, q7, [sp, #256]",
    "ldp q4, q5, [sp, #224]",
    "ldp q2, q3, [sp, #192]",
    "ldp q0, q1, [sp, #160]",
    "ldr x30, [sp, #144]",
    "ldp x17, x18, [sp, #128]",
    "ldp x15, x16, [sp, #112]",
    "ldp x13, x14, [sp, #96]",
    "ldp x11, x12, [sp, #80]",
    "ldp x9, x10, [sp, #64]",
    "ldp x7, x8, [sp, #48]",
    "ldp x5, x6, [sp, #32]",
    "ldp x3, x4, [sp, #16]",
    "ldp x1, x2, [sp], #288",
    "ret",
);

#[cfg(target_arch = "riscv64")]
core::arch::global_asm!(
    ".globl __tlsdesc_dynamic",
    ".hidden __tlsdesc_dynamic",
    ".type __tlsdesc_dynamic,%function",
    "__tlsdesc_dynamic:",
    "addi sp, sp, -224",
    "sd t0, 0(sp)",
    "sd t1, 8(sp)",
    "sd t2, 16(sp)",
    "sd t3, 24(sp)",
    "sd t4, 32(sp)",
    "sd t5, 40(sp)",
    "sd t6, 48(sp)",
    "sd a1, 56(sp)",
    "sd a2, 64(sp)",
    "sd a3, 72(sp)",
    "sd a4, 80(sp)",
    "sd a5, 88(sp)",
    "sd a6, 96(sp)",
    "sd a7, 104(sp)",
    "sd ra, 112(sp)",
    "fsd ft0, 120(sp)",
    "fsd ft1, 128(sp)",
    "fsd ft2, 136(sp)",
    "fsd ft3, 144(sp)",
    "fsd ft4, 152(sp)",
    "fsd ft5, 160(sp)",
    "fsd ft6, 168(sp)",
    "fsd ft7, 176(sp)",
    "fsd ft8, 184(sp)",
    "fsd ft9, 192(sp)",
    "fsd ft10, 200(sp)",
    "fsd ft11, 208(sp)",
    "call __rc_tlsdesc_dynamic",
    "fld ft11, 208(sp)",
    "fld ft10, 200(sp)",
    "fld ft9, 192(sp)",
    "fld ft8, 184(sp)",
    "fld ft7, 176(sp)",
    "fld ft6, 168(sp)",
    "fld ft5, 160(sp)",
    "fld ft4, 152(sp)",
    "fld ft3, 144(sp)",
    "fld ft2, 136(sp)",
    "fld ft1, 128(sp)",
    "fld ft0, 120(sp)",
    "ld ra, 112(sp)",
    "ld a7, 104(sp)",
    "ld a6, 96(sp)",
    "ld a5, 88(sp)",
    "ld a4, 80(sp)",
    "ld a3, 72(sp)",
    "ld a2, 64(sp)",
    "ld a1, 56(sp)",
    "ld t6, 48(sp)",
    "ld t5, 40(sp)",
    "ld t4, 32(sp)",
    "ld t3, 24(sp)",
    "ld t2, 16(sp)",
    "ld t1, 8(sp)",
    "ld t0, 0(sp)",
    "addi sp, sp, 224",
    "jr t0",
);

#[repr(C)]
struct TlsdescExt {
    next: *mut TlsdescExt,
    addr: *mut u8,
    module: usize,
}

/// Resolve a TLSDESC access for a module that was loaded after startup.
///
/// Each thread keeps its own extension list at TCB+0x20 (the reserved slot).
/// The list is per-thread, so no lock is needed; a signal handler that races
/// with itself can only create a second, already initialized copy.
#[no_mangle]
pub unsafe extern "C" fn __rc_tlsdesc_dynamic(desc: *const u64) -> usize {
    let encoded = core::ptr::read_unaligned(desc.add(1));
    let module = (encoded >> 32) as usize;
    let off = (encoded & 0xffff_ffff) as usize;
    let tp = read_tp();
    let tcb = (tp as isize + tls_tcb_offset_from_tp()) as *mut u64;
    let mut node = core::ptr::read_unaligned(tcb.add(4)) as *mut TlsdescExt;
    while !node.is_null() {
        if (*node).module == module {
            return ((*node).addr as usize).wrapping_add(off).wrapping_sub(tp);
        }
        node = (*node).next;
    }
    let memsz = *TLS_MEMSZ.add(module) as usize;
    let filesz = *TLS_FILESZ.add(module) as usize;
    let mut align = (**LOADED.add(module)).tls_align as usize;
    if align == 0 {
        align = 1;
    }
    if align > 1 << 20 {
        align = 1 << 20;
    }
    // TLS offsets are relative to a module base aligned to tls_align, so the
    // private copy must keep that alignment too.
    let bytes = memsz.max(1).checked_add(align).unwrap_or(memsz.max(1));
    let raw = map_zeroed(bytes);
    let data = if raw.is_null() {
        core::ptr::null_mut()
    } else if align > 1 {
        ((raw as usize + align - 1) & !(align - 1)) as *mut u8
    } else {
        raw
    };
    let node_ptr = map_zeroed(core::mem::size_of::<TlsdescExt>()) as *mut TlsdescExt;
    if data.is_null() || node_ptr.is_null() {
        // Out of address space: fall back to the layout-based offset.
        return (*TLS_LAYOUT_OFFSET.add(module))
            .wrapping_add(off)
            .wrapping_sub(tls_var_area_offset_from_tp());
    }
    if filesz != 0 {
        let copy = if filesz < memsz { filesz } else { memsz };
        core::ptr::copy_nonoverlapping(*TLS_IMAGE.add(module), data, copy);
    }
    (*node_ptr).addr = data;
    (*node_ptr).module = module;
    (*node_ptr).next = core::ptr::read_unaligned(tcb.add(4)) as *mut TlsdescExt;
    core::ptr::write_unaligned(tcb.add(4), node_ptr as u64);
    (data as usize).wrapping_add(off).wrapping_sub(tp)
}

unsafe fn tls_lock() {
    while TLS_LOCK.swap(true, Ordering::Acquire) {}
}

unsafe fn tls_unlock() {
    TLS_LOCK.store(false, Ordering::Release);
}

unsafe fn expand_thread_tls(old_total: usize, old_module_count: usize) {
    let total = TLS_TOTAL_SIZE + TCB_SIZE;
    let block = sys_mmap(
        core::ptr::null_mut(),
        total,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if block as usize == MAP_FAILED {
        return;
    }
    let old_fs = read_tp();
    // The old block was laid out with `old_total`, which may be smaller than
    // the current TLS_TOTAL_SIZE. Using the new size here shifts the source
    // and silently corrupts every thread-local variable during dlopen.
    let old_var_base = if cfg!(target_arch = "x86_64") {
        (old_fs as isize - old_total as isize) as *mut u8
    } else {
        old_fs as *mut u8
    };
    let old_tcb = (old_fs as isize + tls_tcb_offset_from_tp()) as *const u64;
    let old_tlsdesc = core::ptr::read_unaligned(old_tcb.add(4));
    let old_getrandom = core::ptr::read_unaligned(old_tcb.add(7));
    let old_thread = core::ptr::read_unaligned(old_tcb.add(8));
    if old_total > 0 {
        core::ptr::copy_nonoverlapping(old_var_base, block.add(tls_var_area_offset_from_block()), old_total);
    }
    for i in old_module_count..TLS_MODULE_COUNT {
        if *TLS_MEMSZ.add(i) == 0 {
            continue;
        }
        let dst = block.add(tls_var_area_offset_from_block()).add(*TLS_LAYOUT_OFFSET.add(i));
        let src = *TLS_IMAGE.add(i);
        let filesz = *TLS_FILESZ.add(i) as usize;
        let memsz = *TLS_MEMSZ.add(i) as usize;
        if filesz > 0 {
            core::ptr::copy_nonoverlapping(src, dst, filesz);
        }
        if memsz > filesz {
            core::ptr::write_bytes(dst.add(filesz), 0, memsz - filesz);
        }
    }
    let tcb = block.add(tls_tcb_offset_from_block());
    core::ptr::write_unaligned(tcb as *mut u64, tcb as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(1), TLS_GENERATION);
    core::ptr::write_unaligned((tcb as *mut u64).add(2), TLS_TOTAL_SIZE as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(3), TLS_MODULE_COUNT as u64);
    // 0x38 is libc's per-thread vDSO getrandom state pointer.  Keep the
    // state this thread already allocated when growing its block; dropping
    // it would leak the page and force a fresh kernel reseed.  0x40 is the
    // pointer to the thread's Thread record and moves with the block.
    core::ptr::write_unaligned((tcb as *mut u64).add(4), old_tlsdesc);
    core::ptr::write_unaligned((tcb as *mut u64).add(7), old_getrandom);
    core::ptr::write_unaligned((tcb as *mut u64).add(8), old_thread);
    write_tp(block.add(tls_tp_offset_from_block()) as usize);
}

unsafe fn assign_tls_for_new_modules(first_new: usize) -> bool {
    if first_new >= LOADED_COUNT {
        return false;
    }
    let mut base_end = TLS_TOTAL_SIZE;
    let mut placed = false;
    for i in first_new..LOADED_COUNT {
        let obj = &(**LOADED.add(i));
        if obj.tls_memsz == 0 {
            *TLS_LAYOUT_OFFSET.add(i) = 0;
            *TLS_FILESZ.add(i) = 0;
            *TLS_MEMSZ.add(i) = 0;
            *TLS_IMAGE.add(i) = core::ptr::null();
            continue;
        }
        let align = if obj.tls_align > 0 { obj.tls_align as usize } else { 1 };
        let new_offset = (base_end + align - 1) & !(align - 1);
        *TLS_LAYOUT_OFFSET.add(i) = new_offset;
        *TLS_FILESZ.add(i) = obj.tls_filesz;
        *TLS_MEMSZ.add(i) = obj.tls_memsz;
        *TLS_IMAGE.add(i) = obj.tls_image;
        base_end = new_offset + obj.tls_memsz as usize;
        placed = true;
    }
    TLS_MODULE_COUNT = LOADED_COUNT;
    if placed {
        TLS_TOTAL_SIZE = base_end;
    }
    placed
}

unsafe fn install_new_tls(old_total: usize, old_module_count: usize) {
    TLS_GENERATION = TLS_GENERATION.wrapping_add(1);
    if TLS_GENERATION == 0 {
        TLS_GENERATION = 1;
    }
    tls_lock();
    expand_thread_tls(old_total, old_module_count);
    tls_unlock();
}



unsafe fn lookup_symbol_in_object(obj_idx: usize, name: *const u8, name_len: usize) -> u64 {
    let obj = &(**LOADED.add(obj_idx));
    if obj.symtab.is_null() || obj.strtab.is_null() {
        return 0;
    }
    let mut fallback = 0u64;
    for j in 0..obj.sym_count {
        let sym_entry = obj.symtab.add(j * SYMTAB_ENT_SIZE);
        let st_name_off = u32::from_le_bytes(core::ptr::read_unaligned(sym_entry as *const [u8; 4])) as usize;
        let st_value = u64::from_le_bytes(core::ptr::read_unaligned(sym_entry.add(8) as *const [u8; 8]));
        if st_value == 0 {
            continue;
        }
        if st_name_off >= obj.strsz {
            continue;
        }
        let sym_name = obj.strtab.add(st_name_off);
        if str_eq(name, name_len, sym_name) {
            if sym_is_default(obj, j) {
                return obj.base + st_value;
            }
            if fallback == 0 {
                fallback = obj.base + st_value;
            }
        }
    }
    fallback
}

static mut DLERROR_BUF: [u8; 4096] = [0; 4096];
static mut DLERROR_SET: bool = false;
const DLERROR_BUF_SIZE: usize = 4096;

unsafe fn set_dlerror(msg: &[u8]) {
    let len = if msg.len() >= DLERROR_BUF_SIZE {
        DLERROR_BUF_SIZE - 1
    } else {
        msg.len()
    };
    core::ptr::copy_nonoverlapping(
        msg.as_ptr(),
        core::ptr::addr_of_mut!(DLERROR_BUF).cast::<u8>(),
        len,
    );
    DLERROR_BUF[len] = 0;
    DLERROR_SET = true;
}

const DL_GLOBAL_SENTINEL: *mut u8 = 1usize as *mut u8;

type LdsoDlopenFn = unsafe extern "C" fn(*const u8, i32) -> *mut u8;
type LdsoDlsymFn = unsafe extern "C" fn(*mut u8, *const u8, *mut u8) -> *mut u8;
type LdsoDlcloseFn = unsafe extern "C" fn(*mut u8) -> i32;
type LdsoDlerrorFn = unsafe extern "C" fn() -> *const u8;

#[repr(C)]
pub struct DlPhdrInfo {
    addr: u64,
    name: *const u8,
    phdr: *const u8,
    phnum: u16,
    __pad0: u16,
    __pad1: u32,
    adds: u64,
    subs: u64,
    tls_modid: usize,
    tls_data: *mut u8,
}

type LdsoIterateCb = unsafe extern "C" fn(*mut DlPhdrInfo, usize, *mut u8) -> i32;
type LdsoIterateFn = unsafe extern "C" fn(Option<LdsoIterateCb>, *mut u8) -> i32;
type LdsoDlinfoFn = unsafe extern "C" fn(*mut u8, i32, *mut u8) -> i32;

#[repr(C)]
struct LinkMap {
    l_addr: u64,
    l_name: *mut u8,
    l_ld: *mut u8,
    l_next: *mut LinkMap,
    l_prev: *mut LinkMap,
}

const EMPTY_LINK_MAP: LinkMap = LinkMap {
    l_addr: 0,
    l_name: core::ptr::null_mut(),
    l_ld: core::ptr::null_mut(),
    l_next: core::ptr::null_mut(),
    l_prev: core::ptr::null_mut(),
};
static mut LINK_MAPS: *mut LinkMap = core::ptr::null_mut();
static mut LINK_MAPS_CAP: usize = 0;

unsafe fn object_index(handle: *mut u8) -> Option<usize> {
    if handle == DL_GLOBAL_SENTINEL {
        return Some(0);
    }
    handle_index(handle)
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dl_iterate_phdr(cb: Option<LdsoIterateCb>, data: *mut u8) -> i32 {
    let cb = match cb { Some(c) => c, None => return 0 };
    for i in 0..LOADED_COUNT {
        let base = (**LOADED.add(i)).base;
        let eh = base as *const u8;
        let mut phdr: *const u8 = core::ptr::null();
        let mut phnum: u16 = 0;
        if *eh == 0x7f && *eh.add(1) == b'E' {
            let e_phoff = u64::from_le_bytes(core::ptr::read_unaligned(eh.add(32) as *const [u8; 8]));
            let e_phentsize = u16::from_le_bytes(core::ptr::read_unaligned(eh.add(54) as *const [u8; 2]));
            let e_phnum = u16::from_le_bytes(core::ptr::read_unaligned(eh.add(56) as *const [u8; 2]));
            if e_phentsize as usize == PHDR_SIZE {
                phdr = (base + e_phoff) as *const u8;
                phnum = e_phnum;
            }
        }
        let mut name = (**LOADED.add(i)).name;
        if name.is_null() {
            name = b"\0".as_ptr() as *mut u8;
        }
        let mut info = DlPhdrInfo {
            addr: base,
            name,
            phdr,
            phnum,
            __pad0: 0,
            __pad1: 0,
            adds: 0,
            subs: 0,
            tls_modid: i + 1,
            tls_data: core::ptr::null_mut(),
        };
        let r = cb(&mut info as *mut DlPhdrInfo, core::mem::size_of::<DlPhdrInfo>(), data);
        if r != 0 { return r; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dlinfo(handle: *mut u8, request: i32, info: *mut u8) -> i32 {
    if info.is_null() { return -1; }
    let idx = match object_index(handle) {
        Some(i) => i,
        None => {
            set_dlerror(b"dlinfo: invalid handle\0");
            return -1;
        }
    };
    match request {
        2 => {
            // RTLD_DI_LINKMAP
            if LINK_MAPS_CAP < LOADED_COUNT {
                let bytes = LOADED_COUNT * core::mem::size_of::<LinkMap>();
                let maps = map_zeroed(bytes) as *mut LinkMap;
                if maps.is_null() {
                    return -1;
                }
                // Keep old link-map arrays mapped: RTLD_DI_LINKMAP returns
                // pointers into them and callers may retain the pointer.
                LINK_MAPS = maps;
                LINK_MAPS_CAP = LOADED_COUNT;
            }
            for i in 0..LOADED_COUNT {
                let map = LINK_MAPS.add(i);
                (*map) = LinkMap {
                    l_addr: (**LOADED.add(i)).base,
                    l_name: (**LOADED.add(i)).name,
                    l_ld: (**LOADED.add(i)).dyn_addr as *mut u8,
                    l_next: core::ptr::null_mut(),
                    l_prev: core::ptr::null_mut(),
                };
            }
            for i in 0..LOADED_COUNT {
                let map = LINK_MAPS.add(i);
                if i + 1 < LOADED_COUNT {
                    (*map).l_next = LINK_MAPS.add(i + 1);
                }
                if i > 0 {
                    (*map).l_prev = LINK_MAPS.add(i - 1);
                }
            }
            *(info as *mut *mut LinkMap) = LINK_MAPS.add(idx);
            0
        }
        1 => {
            // RTLD_DI_ORIGIN
            let dst = info;
            let name = (**LOADED.add(idx)).name;
            if name.is_null() {
                return -1;
            }
            let mut end = 0usize;
            while end < (**LOADED.add(idx)).name_cap && *name.add(end) != 0 { end += 1; }
            let mut slash = end;
            while slash > 0 {
                if *name.add(slash - 1) == b'/' { break; }
                slash -= 1;
            }
            if slash == 0 {
                *dst = b'.' as u8;
                *dst.add(1) = 0;
            } else {
                for i in 0..(slash - 1) {
                    *dst.add(i) = *name.add(i);
                }
                *dst.add(slash - 1) = 0;
            }
            0
        }
        _ => {
            set_dlerror(b"dlinfo: unsupported request\0");
            -1
        }
    }
}

unsafe fn register_dlopen_callbacks() {
    let reg_open = resolve_symbol(b"__ldso_register_dlopen\0".as_ptr());
    if reg_open != 0 {
        let f: extern "C" fn(LdsoDlopenFn) = core::mem::transmute(reg_open);
        f(__ldso_dlopen as LdsoDlopenFn);
    }
    let reg_sym = resolve_symbol(b"__ldso_register_dlsym\0".as_ptr());
    if reg_sym != 0 {
        let f: extern "C" fn(LdsoDlsymFn) = core::mem::transmute(reg_sym);
        f(__ldso_dlsym as LdsoDlsymFn);
    }
    let reg_close = resolve_symbol(b"__ldso_register_dlclose\0".as_ptr());
    if reg_close != 0 {
        let f: extern "C" fn(LdsoDlcloseFn) = core::mem::transmute(reg_close);
        f(__ldso_dlclose as LdsoDlcloseFn);
    }
    let reg_error = resolve_symbol(b"__ldso_register_dlerror\0".as_ptr());
    if reg_error != 0 {
        let f: extern "C" fn(LdsoDlerrorFn) = core::mem::transmute(reg_error);
        f(__ldso_dlerror as LdsoDlerrorFn);
    }
    let reg_iter = resolve_symbol(b"__ldso_register_dl_iterate\0".as_ptr());
    if reg_iter != 0 {
        let f: extern "C" fn(LdsoIterateFn) = core::mem::transmute(reg_iter);
        f(__ldso_dl_iterate_phdr as LdsoIterateFn);
    }
    let reg_info = resolve_symbol(b"__ldso_register_dlinfo\0".as_ptr());
    if reg_info != 0 {
        let f: extern "C" fn(LdsoDlinfoFn) = core::mem::transmute(reg_info);
        f(__ldso_dlinfo as LdsoDlinfoFn);
    }
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dlopen(filename: *const u8, flags: i32) -> *mut u8 {
    DLERROR_SET = false;
    if filename.is_null() {
        return DL_GLOBAL_SENTINEL;
    }
    let noload = (flags & RTLD_NOLOAD) != 0;
    let make_global = (flags & RTLD_GLOBAL) != 0;
    let nodelete = (flags & RTLD_NODELETE) != 0;
    let name_len = str_len(filename);
    if let Some(idx) = loaded_object_by_name(filename, name_len) {
        if make_global {
            promote_global(idx);
        }
        if nodelete {
            (**LOADED.add(idx)).nodelete = true;
        }
        return &mut (**LOADED.add(idx)) as *mut LoadedObject as *mut u8;
    }
    let ld = if LD_LIBRARY_PATH.is_null() {
        None
    } else {
        Some(LD_LIBRARY_PATH)
    };
    let fd = find_library_fd(filename, name_len, ld, Some(0));
    if fd < 0 {
        if noload {
            set_dlerror(b"dlopen: library is not already loaded\0");
        } else {
            set_dlerror(b"dlopen: cannot open file\0");
        }
        return core::ptr::null_mut();
    }
    let (dev, ino) = fd_identity(fd);
    if let Some(idx) = loaded_object_by_identity(dev, ino) {
        sys_close(fd);
        if make_global {
            promote_global(idx);
        }
        if nodelete {
            (**LOADED.add(idx)).nodelete = true;
        }
        return &mut (**LOADED.add(idx)) as *mut LoadedObject as *mut u8;
    }
    if noload {
        sys_close(fd);
        set_dlerror(b"dlopen: library is not already loaded\0");
        return core::ptr::null_mut();
    }
    let first_new = LOADED_COUNT;
    let desired = DSO_BASE_START + (LOADED_COUNT as u64) * DSO_BASE_STRIDE;
    let _base = match load_dso_from_fd(fd, desired) {
        Some(b) => b,
        None => {
            sys_close(fd);
            set_dlerror(b"dlopen: failed to load\0");
            return core::ptr::null_mut();
        }
    };
    sys_close(fd);
    let idx = LOADED_COUNT - 1;
    if LOADED_PATH_LEN > 0 {
        set_loaded_name(idx, LOADED_PATH, LOADED_PATH_LEN);
        LOADED_PATH_LEN = 0;
    } else {
        set_loaded_name(idx, filename, name_len);
    }
    (**LOADED.add(idx)).needed_by = 0;
    (**LOADED.add(idx)).global = make_global;
    (**LOADED.add(idx)).nodelete = nodelete;

    // Pull in the module's own DT_NEEDED closure before relocating it.
    // On failure undo the objects loaded by this dlopen instead of exiting.
    let mut scan = first_new;
    let mut deps_ok = true;
    while scan < LOADED_COUNT {
        if !load_object_needed(scan, ld, false) {
            deps_ok = false;
            break;
        }
        scan += 1;
    }
    if !deps_ok {
        rollback_loaded(first_new);
        set_dlerror(b"dlopen: failed to load dependency\0");
        return core::ptr::null_mut();
    }
    if make_global {
        promote_global(idx);
    }

    // TLS layout must be installed before relocations: aarch64/riscv TPOFF
    // and TLSDESC relocations embed layout-dependent offsets. The TLS image
    // itself is copied only after relocations, because .tdata may contain
    // pointers that need relocation first.
    let old_tls_total = TLS_TOTAL_SIZE;
    let old_tls_count = TLS_MODULE_COUNT;
    let placed_tls = assign_tls_for_new_modules(first_new);
    process_relocations_range(first_new);
    if placed_tls {
        install_new_tls(old_tls_total, old_tls_count);
    }
    // Dependencies first, then the module that requested them.
    let mut i = LOADED_COUNT;
    while i > first_new {
        i -= 1;
        run_constructors_for(i);
    }
    &mut (**LOADED.add(idx)) as *mut LoadedObject as *mut u8
}

unsafe fn handle_index(handle: *mut u8) -> Option<usize> {
    if handle.is_null() || handle == DL_GLOBAL_SENTINEL {
        return None;
    }
    for i in 0..LOADED_COUNT {
        if *LOADED.add(i) as *mut u8 == handle {
            return Some(i);
        }
    }
    None
}

unsafe fn object_for_addr(addr: usize) -> Option<usize> {
    for i in 0..LOADED_COUNT {
        if addr >= (**LOADED.add(i)).map_start as usize && addr < (**LOADED.add(i)).map_end as usize {
            return Some(i);
        }
    }
    None
}

unsafe fn dlsym_object(obj_idx: usize, name: *const u8, name_len: usize) -> Option<(usize, usize)> {
    let obj = &(**LOADED.add(obj_idx));
    if obj.symtab.is_null() || obj.strtab.is_null() {
        return None;
    }
    let mut fallback = None;
    for j in 0..obj.sym_count {
        let ent = obj.symtab.add(j * SYMTAB_ENT_SIZE);
        let st_name = u32::from_le_bytes(core::ptr::read_unaligned(ent as *const [u8; 4])) as usize;
        if st_name >= obj.strsz {
            continue;
        }
        if !str_eq(name, name_len, obj.strtab.add(st_name)) {
            continue;
        }
        let st_value = u64::from_le_bytes(core::ptr::read_unaligned(ent.add(8) as *const [u8; 8]));
        let st_info = *ent.add(4);
        let st_shndx = u16::from_le_bytes(core::ptr::read_unaligned(ent.add(6) as *const [u8; 2]));
        if st_shndx == 0 {
            continue;
        }
        let st_type = st_info & 0xf;
        let st_bind = st_info >> 4;
        if !(st_type == 0 || st_type == 1 || st_type == 2 || st_type == 5 || st_type == 6) {
            continue;
        }
        if !(st_bind == 1 || st_bind == 2 || st_bind == 10) {
            continue;
        }
        if st_value == 0 && st_type != 6 {
            continue;
        }
        if sym_is_default(obj, j) {
            return Some((obj_idx, j));
        }
        if fallback.is_none() {
            fallback = Some((obj_idx, j));
        }
    }
    fallback
}

/// Breadth-first search of an object and its DT_NEEDED closure.
unsafe fn dlsym_scope(idx: usize, name: *const u8, name_len: usize) -> Option<(usize, usize)> {
    let count = LOADED_COUNT;
    if idx >= count || count == 0 {
        return None;
    }
    let qbytes = count * core::mem::size_of::<usize>();
    let queue = map_zeroed(qbytes) as *mut usize;
    let seen = map_zeroed(count);
    if queue.is_null() || seen.is_null() {
        if !queue.is_null() { sys_munmap(queue as *mut u8, qbytes); }
        if !seen.is_null() { sys_munmap(seen, count); }
        return None;
    }
    let mut head = 0usize;
    let mut tail = 0usize;
    *seen.add(idx) = 1;
    *queue.add(tail) = idx;
    tail += 1;
    let mut result = None;
    while head < tail {
        let i = *queue.add(head);
        head += 1;
        if i >= count {
            continue;
        }
        if let Some(r) = dlsym_object(i, name, name_len) {
            result = Some(r);
            break;
        }
        for d in 0..(**LOADED.add(i)).dep_count {
            let dep = *(**LOADED.add(i)).deps.add(d);
            if dep < count && *seen.add(dep) == 0 && tail < count {
                *seen.add(dep) = 1;
                *queue.add(tail) = dep;
                tail += 1;
            }
        }
    }
    sys_munmap(queue as *mut u8, qbytes);
    sys_munmap(seen, count);
    result
}

unsafe fn symbol_value(obj_idx: usize, sym_idx: usize) -> *mut u8 {
    let obj = &(**LOADED.add(obj_idx));
    let ent = obj.symtab.add(sym_idx * SYMTAB_ENT_SIZE);
    let st_value = u64::from_le_bytes(core::ptr::read_unaligned(ent.add(8) as *const [u8; 8]));
    let st_info = *ent.add(4);
    if (st_info & 0xf) == 6 {
        let ti = TlsIndex {
            ti_module: obj_idx,
            ti_offset: st_value as usize,
        };
        return __tls_get_addr(&ti);
    }
    (obj.base + st_value) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dlsym(handle: *mut u8, symbol: *const u8, ra: *mut u8) -> *mut u8 {
    DLERROR_SET = false;
    if symbol.is_null() {
        set_dlerror(b"dlsym: null symbol\0");
        return core::ptr::null_mut();
    }
    let name_len = str_len(symbol);
    let mut found: Option<(usize, usize)> = None;

    if handle.is_null() || handle == DL_GLOBAL_SENTINEL {
        for i in 0..LOADED_COUNT {
            if i == 0 || (**LOADED.add(i)).global {
                if let Some(r) = dlsym_object(i, symbol, name_len) {
                    found = Some(r);
                    break;
                }
            }
        }
    } else if handle as usize == usize::MAX {
        // RTLD_NEXT: start after the object that called dlsym.
        let start = match object_for_addr(ra as usize) {
            Some(i) => i + 1,
            None => 1,
        };
        for i in start..LOADED_COUNT {
            if i == 0 || (**LOADED.add(i)).global {
                if let Some(r) = dlsym_object(i, symbol, name_len) {
                    found = Some(r);
                    break;
                }
            }
        }
    } else if let Some(idx) = handle_index(handle) {
        found = dlsym_scope(idx, symbol, name_len);
    } else {
        set_dlerror(b"dlsym: invalid handle\0");
        return core::ptr::null_mut();
    }

    match found {
        Some((obj_idx, sym_idx)) => symbol_value(obj_idx, sym_idx),
        None => {
            set_dlerror(b"dlsym: symbol not found\0");
            core::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dlclose(handle: *mut u8) -> i32 {
    DLERROR_SET = false;
    // musl never unloads libraries or runs destructors from dlclose(); it
    // only validates the handle. Keep the same semantics so that repeated
    // dlopen/dlclose cycles preserve static state and TLS pointers.
    if handle == DL_GLOBAL_SENTINEL || handle_index(handle).is_some() {
        0
    } else {
        set_dlerror(b"dlclose: invalid library handle\0");
        1
    }
}

#[no_mangle]
pub unsafe extern "C" fn __ldso_dlerror() -> *const u8 {
    if DLERROR_SET {
        DLERROR_SET = false;
        core::ptr::addr_of_mut!(DLERROR_BUF).cast::<u8>()
    } else {
        core::ptr::null()
    }
}

unsafe fn compute_tls_layout() {
    let mut total: usize = 0;
    for i in 0..LOADED_COUNT {
        let obj = &(**LOADED.add(i));
        let align = if obj.tls_align > 0 { obj.tls_align as usize } else { 1 };
        let block_size = ((obj.tls_memsz as usize + align - 1) / align) * align;
        total += block_size;
    }
    if total < 4096 {
        total = 4096;
    }
    total += total;
    TLS_TOTAL_SIZE = (total + 4095) & !4095;

    #[cfg(target_arch = "aarch64")]
    {
        // AArch64 uses the TLS_ABOVE_TP layout: the variable area is at positive
        // offsets from TP, with a fixed gap at the start. The linker expects
        // module offsets to maintain the same alignment congruence as the image
        // address in the file so that each TLS symbol is correctly aligned.
        const GAP_ABOVE_TP: usize = 16;
        let mut offset = GAP_ABOVE_TP;
        for i in 0..LOADED_COUNT {
            let obj = &(**LOADED.add(i));
            if obj.tls_memsz == 0 {
                *TLS_LAYOUT_OFFSET.add(i) = 0;
                *TLS_FILESZ.add(i) = 0;
                *TLS_MEMSZ.add(i) = 0;
                *TLS_IMAGE.add(i) = core::ptr::null();
                continue;
            }
            let align = if obj.tls_align > 0 { obj.tls_align as usize } else { 1 };
            let image = obj.tls_image as usize;
            // The variable area base is block + TCB_SIZE. Since the block is
            // page-aligned, its residue modulo align is TCB_SIZE % align.
            let var_base_mod = TCB_SIZE % align;
            let desired = image.wrapping_sub(var_base_mod).wrapping_sub(offset) & (align - 1);
            offset += desired;
            *TLS_LAYOUT_OFFSET.add(i) = offset;
            if cfg!(debug_assertions) {
                write_stderr(b"ldso: TLS module ");
                write_hex_stderr(i);
                write_stderr(b" offset=");
                write_hex_stderr(offset);
                write_stderr(b"\n");
            }
            *TLS_FILESZ.add(i) = obj.tls_filesz;
            *TLS_MEMSZ.add(i) = obj.tls_memsz;
            *TLS_IMAGE.add(i) = obj.tls_image;
            let block_size = ((obj.tls_memsz as usize + align - 1) / align) * align;
            offset += block_size;
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        // x86_64 places the variable area below TP; modules are laid out from the
        // end of the variable area (closest to TP) backwards.
        let mut end = TLS_TOTAL_SIZE;
        for i in 0..LOADED_COUNT {
            let obj = &(**LOADED.add(i));
            if obj.tls_memsz == 0 {
                *TLS_LAYOUT_OFFSET.add(i) = 0;
                *TLS_FILESZ.add(i) = 0;
                *TLS_MEMSZ.add(i) = 0;
                *TLS_IMAGE.add(i) = core::ptr::null();
                continue;
            }
            let align = if obj.tls_align > 0 { obj.tls_align as usize } else { 1 };
            let block_size = ((obj.tls_memsz as usize + align - 1) / align) * align;
            end -= block_size;
            end &= !(align - 1);
            *TLS_LAYOUT_OFFSET.add(i) = end;
            *TLS_FILESZ.add(i) = obj.tls_filesz;
            *TLS_MEMSZ.add(i) = obj.tls_memsz;
            *TLS_IMAGE.add(i) = obj.tls_image;
        }
    }

    #[cfg(target_arch = "riscv64")]
    {
        // RISC-V uses TLS_ABOVE_TP like aarch64 but with GAP_ABOVE_TP=0.
        let mut offset: usize = 0;
        for i in 0..LOADED_COUNT {
            let obj = &(**LOADED.add(i));
            if obj.tls_memsz == 0 {
                *TLS_LAYOUT_OFFSET.add(i) = 0;
                *TLS_FILESZ.add(i) = 0;
                *TLS_MEMSZ.add(i) = 0;
                *TLS_IMAGE.add(i) = core::ptr::null();
                continue;
            }
            let align = if obj.tls_align > 0 { obj.tls_align as usize } else { 1 };
            let image = obj.tls_image as usize;
            let var_base_mod = TCB_SIZE % align;
            let desired = image.wrapping_sub(var_base_mod).wrapping_sub(offset) & (align - 1);
            offset += desired;
            *TLS_LAYOUT_OFFSET.add(i) = offset;
            if cfg!(debug_assertions) {
                write_stderr(b"ldso: TLS module ");
                write_hex_stderr(i);
                write_stderr(b" offset=");
                write_hex_stderr(offset);
                write_stderr(b"\n");
            }
            *TLS_FILESZ.add(i) = obj.tls_filesz;
            *TLS_MEMSZ.add(i) = obj.tls_memsz;
            *TLS_IMAGE.add(i) = obj.tls_image;
            let block_size = ((obj.tls_memsz as usize + align - 1) / align) * align;
            offset += block_size;
        }
    }

    TLS_MODULE_COUNT = LOADED_COUNT;
}

unsafe fn init_tls_block(block: *mut u8) -> *mut u8 {
    let var_base = block.add(tls_var_area_offset_from_block());
    for i in 0..TLS_MODULE_COUNT {
        if *TLS_MEMSZ.add(i) == 0 {
            continue;
        }
        let dst = var_base.add(*TLS_LAYOUT_OFFSET.add(i));
        let src = *TLS_IMAGE.add(i);
        let filesz = *TLS_FILESZ.add(i) as usize;
        let memsz = *TLS_MEMSZ.add(i) as usize;
        if filesz > 0 {
            core::ptr::copy_nonoverlapping(src, dst, filesz);
        }
        if memsz > filesz {
            core::ptr::write_bytes(dst.add(filesz), 0, memsz - filesz);
        }
    }
    let tcb = block.add(tls_tcb_offset_from_block());
    core::ptr::write_unaligned(tcb as *mut u64, tcb as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(1), TLS_GENERATION);
    core::ptr::write_unaligned((tcb as *mut u64).add(2), TLS_TOTAL_SIZE as u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(3), TLS_MODULE_COUNT as u64);
    // 0x20 is the per-thread dynamic TLSDESC list.  0x38 is libc's per-thread
    // vDSO getrandom state pointer; 0x40 is the Thread record pointer.  A
    // block carved out of an application stack is not zero-filled, so clear
    // all three.
    core::ptr::write_unaligned((tcb as *mut u64).add(4), 0u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(7), 0u64);
    core::ptr::write_unaligned((tcb as *mut u64).add(8), 0u64);
    block.add(tls_tp_offset_from_block())
}

#[repr(C)]
pub struct TlsIndex {
    ti_module: usize,
    ti_offset: usize,
}

#[no_mangle]
pub unsafe extern "C" fn __tls_get_addr(ti: *const TlsIndex) -> *mut u8 {
    let module = (*ti).ti_module;
    let offset = (*ti).ti_offset;
    let fs_base = read_tp();
    let tcb = (fs_base as isize + tls_tcb_offset_from_tp()) as *mut u8;
    let thread_gen = core::ptr::read_unaligned((tcb as *const u64).add(1));
    if thread_gen != TLS_GENERATION {
        // Use this thread's own block size/module count: it may have missed
        // more than one dlopen and the global "old" values would be wrong.
        let thread_total = core::ptr::read_unaligned((tcb as *const u64).add(2)) as usize;
        let thread_modules = core::ptr::read_unaligned((tcb as *const u64).add(3)) as usize;
        tls_lock();
        let thread_gen2 = core::ptr::read_unaligned((tcb as *const u64).add(1));
        if thread_gen2 != TLS_GENERATION {
            expand_thread_tls(thread_total, thread_modules);
        }
        tls_unlock();
    }
    let fs_base2 = read_tp();
    let tls_base = fs_base2 - tls_var_area_offset_from_tp();
    (tls_base as *mut u8).add(*TLS_LAYOUT_OFFSET.add(module)).add(offset) as *mut u8
}

#[no_mangle]
pub unsafe extern "C" fn __rc_create_thread_tls() -> *mut u8 {
    let total = TLS_TOTAL_SIZE + TCB_SIZE;
    if total == 0 {
        return core::ptr::null_mut();
    }
    let block = sys_mmap(
        core::ptr::null_mut(),
        total,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if block as usize == MAP_FAILED {
        return core::ptr::null_mut();
    }
    init_tls_block(block)
}

#[no_mangle]
pub unsafe extern "C" fn __rc_init_thread_tls_at(block: *mut u8) -> *mut u8 {
    // Lay out the TLS block inside an application-provided thread stack.
    // musl does the same: keeping the loader's separate mmap away from the
    // stack leaves MAP_GROWSDOWN free to grow into the pages below it.
    init_tls_block(block)
}

#[no_mangle]
pub unsafe extern "C" fn __rc_tls_block_size() -> usize {
    TLS_TOTAL_SIZE + TCB_SIZE
}

#[no_mangle]
pub unsafe extern "C" fn __rc_tls_base_offset() -> usize {
    tls_var_area_offset_from_tp() + tls_var_area_offset_from_block()
}

unsafe fn register_self(ldso_base: u64) {
    let ehdr = ldso_base as *const u8;
    if *ehdr != 0x7f || *ehdr.add(1) != b'E' {
        return;
    }
    let e_phoff = u64::from_le_bytes(core::ptr::read_unaligned(ehdr.add(32) as *const [u8; 8]));
    let e_phnum = u16::from_le_bytes(core::ptr::read_unaligned(ehdr.add(56) as *const [u8; 2])) as usize;
    let mut dyn_vaddr: u64 = 0;
    let mut dyn_memsz: u64 = 0;
    for i in 0..e_phnum {
        let ph = ehdr.add(e_phoff as usize + i * PHDR_SIZE);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_DYNAMIC {
            dyn_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            dyn_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
            break;
        }
    }
    if dyn_vaddr == 0 {
        return;
    }
    let dyn_addr = (ldso_base + dyn_vaddr) as usize;
    let dyn_end = dyn_addr + dyn_memsz as usize;
    let mut dt_symtab: u64 = 0;
    let mut dt_strtab: u64 = 0;
    let mut dt_strsz: u64 = 0;
    let mut dt_gnu_hash: u64 = 0;
    let mut dt_hash: u64 = 0;
    let mut pos = dyn_addr;
    while pos + 16 <= dyn_end {
        let d_tag = u64::from_le_bytes(core::ptr::read_unaligned(pos as *const [u8; 8]));
        let d_val = u64::from_le_bytes(core::ptr::read_unaligned((pos + 8) as *const [u8; 8]));
        if d_tag == DT_NULL {
            break;
        }
        match d_tag {
            DT_SYMTAB => dt_symtab = d_val,
            DT_STRTAB => dt_strtab = d_val,
            DT_STRSZ => dt_strsz = d_val,
            DT_GNU_HASH => dt_gnu_hash = d_val,
            DT_HASH => dt_hash = d_val,
            _ => {}
        }
        pos += 16;
    }
    if dt_symtab == 0 || dt_strtab == 0 {
        return;
    }
    let symtab_ptr = (ldso_base + dt_symtab) as *const u8;
    let strtab_ptr = (ldso_base + dt_strtab) as *const u8;
    let sym_count = ((dt_strtab - dt_symtab) / SYMTAB_ENT_SIZE as u64) as usize;
    let mut self_map_start = u64::MAX;
    let mut self_map_end = 0u64;
    for i in 0..e_phnum {
        let ph = ehdr.add(e_phoff as usize + i * PHDR_SIZE);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type != PT_LOAD { continue; }
        let p_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
        let p_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
        if p_vaddr < self_map_start { self_map_start = p_vaddr; }
        if p_vaddr + p_memsz > self_map_end { self_map_end = p_vaddr + p_memsz; }
    }
    if self_map_start == u64::MAX { self_map_start = 0; }
    {
        let value = LoadedObject {
            base: ldso_base,
            symtab: symtab_ptr,
            sym_count,
            strtab: strtab_ptr,
            strsz: dt_strsz as usize,
            gnu_hash: if dt_gnu_hash != 0 { (ldso_base + dt_gnu_hash) as usize } else { 0 },
            hash: if dt_hash != 0 { (ldso_base + dt_hash) as usize } else { 0 },
            versym: 0,
            is_self: true,
            dyn_addr,
            dyn_memsz: dyn_memsz as usize,
            tls_image: core::ptr::null(),
            tls_filesz: 0,
            tls_memsz: 0,
            tls_align: 0,
            init: 0,
            init_array: 0,
            init_array_sz: 0,
            init_present: false,
            init_array_present: false,
            global: false,
            dev: 0,
            ino: 0,
            map_start: ldso_base + self_map_start,
            map_end: ldso_base + self_map_end,
            needed_by: NO_IDX,
            dep_count: 0,
            dep_cap: 0,
            deps: core::ptr::null_mut(),
            rpath: core::ptr::null(),
            rpath_len: 0,
            nodelete: false,
            name: core::ptr::null_mut(),
            name_cap: 0,
        };
        if push_object(value).is_none() {
            die(99, b"loaded_mmap", 0);
        }
    }
}

unsafe fn set_loaded_name(idx: usize, name: *const u8, name_len: usize) {
    if idx >= LOADED_COUNT {
        return;
    }
    let obj = *LOADED.add(idx);
    if !ensure_name_capacity(obj, name_len + 1) {
        return;
    }
    core::ptr::copy_nonoverlapping(name, (*obj).name, name_len);
    *(*obj).name.add(name_len) = 0;
}

unsafe fn loaded_object_by_name(name: *const u8, name_len: usize) -> Option<usize> {
    if name_len == 0 {
        return None;
    }
    for i in 0..LOADED_COUNT {
        let obj_name = (**LOADED.add(i)).name;
        if obj_name.is_null() || *obj_name == 0 {
            continue;
        }
        if str_eq(name, name_len, obj_name) {
            return Some(i);
        }
    }
    // A library loaded by path is also found by its basename, as musl's
    // shortname matching does.
    let mut has_slash = false;
    for i in 0..name_len {
        if *name.add(i) == b'/' { has_slash = true; break; }
    }
    if has_slash {
        return None;
    }
    for i in 0..LOADED_COUNT {
        let obj_name = (**LOADED.add(i)).name;
        if obj_name.is_null() {
            continue;
        }
        let mut base = obj_name;
        let mut p = obj_name;
        while *p != 0 {
            if *p == b'/' { base = p.add(1); }
            p = p.add(1);
        }
        if base != obj_name && str_eq(name, name_len, base) {
            return Some(i);
        }
    }
    None
}

// ============================================================
// Dependency loading
// ============================================================

/// Load one DT_NEEDED entry if it is not already in the object list.
unsafe fn load_needed_dso(
    name: *const u8,
    name_len: usize,
    ld_path: Option<*const u8>,
    make_global: bool,
    needed_by: usize,
) -> Option<usize> {
    if let Some(i) = loaded_object_by_name(name, name_len) {
        if make_global {
            (**LOADED.add(i)).global = true;
        }
        return Some(i);
    }
    if !ensure_loaded_capacity(LOADED_COUNT + 1) {
        return None;
    }
    let fd = find_library_fd(name, name_len, ld_path, Some(needed_by));
    if fd < 0 {
        return None;
    }
    let (dev, ino) = fd_identity(fd);
    if let Some(idx) = loaded_object_by_identity(dev, ino) {
        sys_close(fd);
        if make_global {
            (**LOADED.add(idx)).global = true;
        }
        return Some(idx);
    }
    let desired_base = DSO_BASE_START + (LOADED_COUNT as u64) * DSO_BASE_STRIDE;
    if load_dso_from_fd(fd, desired_base).is_none() {
        sys_close(fd);
        return None;
    }
    sys_close(fd);
    let idx = LOADED_COUNT - 1;
    if LOADED_PATH_LEN > 0 {
        set_loaded_name(idx, LOADED_PATH, LOADED_PATH_LEN);
        LOADED_PATH_LEN = 0;
    } else {
        set_loaded_name(idx, name, name_len);
    }
    (**LOADED.add(idx)).needed_by = needed_by;
    if make_global {
        (**LOADED.add(idx)).global = true;
    }
    Some(idx)
}

/// Walk one loaded object's dynamic section and load its DT_NEEDED entries.
unsafe fn rollback_loaded(first_new: usize) {
    let mut i = first_new;
    while i < LOADED_COUNT {
        let obj = *LOADED.add(i);
        if !obj.is_null() {
            let start = (*obj).map_start & !4095u64;
            let end = ((*obj).map_end + 4095) & !4095u64;
            if end > start {
                sys_munmap(start as *mut u8, (end - start) as usize);
            }
            free_object(obj);
            *LOADED.add(i) = core::ptr::null_mut();
        }
        i += 1;
    }
    if first_new < LOADED_COUNT {
        LOADED_COUNT = first_new;
    }
}

unsafe fn add_dependency(obj_idx: usize, dep_idx: usize) {
    if obj_idx >= LOADED_COUNT || dep_idx >= LOADED_COUNT {
        return;
    }
    let obj = *LOADED.add(obj_idx);
    for i in 0..(*obj).dep_count {
        if *(*obj).deps.add(i) == dep_idx {
            return;
        }
    }
    if !ensure_dep_capacity(obj, (*obj).dep_count + 1) {
        return;
    }
    *(*obj).deps.add((*obj).dep_count) = dep_idx;
    (*obj).dep_count += 1;
}

/// Mark an object and its reachable DT_NEEDED closure as global.
unsafe fn promote_global(idx: usize) {
    let count = LOADED_COUNT;
    if idx >= count || count == 0 {
        return;
    }
    let sbytes = count * core::mem::size_of::<usize>();
    let stack = map_zeroed(sbytes) as *mut usize;
    let seen = map_zeroed(count);
    if stack.is_null() || seen.is_null() {
        if !stack.is_null() { sys_munmap(stack as *mut u8, sbytes); }
        if !seen.is_null() { sys_munmap(seen, count); }
        return;
    }
    let mut top = 0usize;
    *seen.add(idx) = 1;
    *stack.add(top) = idx;
    top += 1;
    while top > 0 {
        top -= 1;
        let i = *stack.add(top);
        if i >= count {
            continue;
        }
        (**LOADED.add(i)).global = true;
        for d in 0..(**LOADED.add(i)).dep_count {
            let dep = *(**LOADED.add(i)).deps.add(d);
            if dep < count && *seen.add(dep) == 0 && top < count {
                *seen.add(dep) = 1;
                *stack.add(top) = dep;
                top += 1;
            }
        }
    }
    sys_munmap(stack as *mut u8, sbytes);
    sys_munmap(seen, count);
}

unsafe fn load_object_needed(obj_idx: usize, ld_path: Option<*const u8>, make_global: bool) -> bool {
    let (dyn_addr, dyn_memsz, strtab, strsz) = {
        let obj = &(**LOADED.add(obj_idx));
        (obj.dyn_addr, obj.dyn_memsz, obj.strtab, obj.strsz)
    };
    let end = dyn_addr + dyn_memsz;
    let mut pos = dyn_addr;
    while pos + 16 <= end {
        let tag = u64::from_le_bytes(core::ptr::read_unaligned(pos as *const [u8; 8]));
        if tag == DT_NULL {
            break;
        }
        if tag == DT_NEEDED {
            let off =
                u64::from_le_bytes(core::ptr::read_unaligned((pos + 8) as *const [u8; 8])) as usize;
            if off < strsz {
                let name = strtab.add(off);
                let len = str_len(name);
                if len > 0 {
                    let dep = match load_needed_dso(name, len, ld_path, make_global, obj_idx) {
                        Some(d) => d,
                        None => return false,
                    };
                    add_dependency(obj_idx, dep);
                }
            }
        }
        pos += 16;
    }
    true
}

// ============================================================
// Main flow: load executable + dependencies, relocate, jump
// ============================================================

unsafe fn load_and_jump(sp: usize, ldso_base: u64) -> ! {

    // 1. Find LD_LIBRARY_PATH from kernel envp
    let ld_path = find_env(sp, b"LD_LIBRARY_PATH=");
    LD_LIBRARY_PATH = ld_path.unwrap_or(core::ptr::null());

    // 2. The kernel has already mapped the executable and placed its program
    //    headers at AT_PHDR. Use that mapping instead of re-reading
    //    /proc/self/exe and remapping every segment: the old path cost a
    //    procfs open+read plus a second set of page faults on every exec.
    let argc0 = *(sp as *const u64) as usize;
    let argv0 = sp + 8;
    let envp0 = argv0 + (argc0 + 1) * 8;
    let mut envc0 = 0usize;
    while *((envp0 + envc0 * 8) as *const u64) != 0 {
        envc0 += 1;
    }
    let auxv0 = (envp0 + (envc0 + 1) * 8) as *const u64;

    let mut at_phdr: u64 = 0;
    let mut at_phent: u64 = PHDR_SIZE as u64;
    let mut at_phnum: u64 = 0;
    let mut at_entry: u64 = 0;
    let mut ai = 0usize;
    loop {
        let tag = *auxv0.add(ai * 2);
        let val = *auxv0.add(ai * 2 + 1);
        if tag == AT_NULL {
            break;
        }
        match tag {
            AT_PHDR => at_phdr = val,
            AT_PHENT => at_phent = val,
            AT_PHNUM => at_phnum = val,
            AT_ENTRY => at_entry = val,
            _ => {}
        }
        ai += 1;
    }
    if at_phdr == 0 || at_phnum == 0 || at_entry == 0 || at_phent < PHDR_SIZE as u64 {
        die(96, b"auxv_phdr", at_phdr as usize);
    }

    let phdr = at_phdr as *const u8;
    let phnum = at_phnum as usize;
    let phent = at_phent as usize;

    // PT_PHDR's p_vaddr is the load bias of the program headers.
    let mut exec_base: u64 = u64::MAX;
    let mut i = 0usize;
    while i < phnum {
        let ph = phdr.add(i * phent);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_PHDR {
            let p_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            exec_base = at_phdr.wrapping_sub(p_vaddr);
            break;
        }
        i += 1;
    }
    if exec_base == u64::MAX {
        die(95, b"no_pt_phdr", at_phdr as usize);
    }

    let mut exec_tls_image: *const u8 = core::ptr::null();
    let mut exec_tls_filesz: u64 = 0;
    let mut exec_tls_memsz: u64 = 0;
    let mut exec_tls_align: u64 = 0;
    let mut i = 0usize;
    while i < phnum {
        let ph = phdr.add(i * phent);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_TLS {
            let p_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            exec_tls_filesz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_FILESZ) as *const [u8; 8]));
            exec_tls_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
            exec_tls_align = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_ALIGN) as *const [u8; 8]));
            exec_tls_image = (exec_base + p_vaddr) as *const u8;
            break;
        }
        i += 1;
    }
    let e_entry = at_entry;
    let e_phnum = at_phnum as u16;

    // 4. Parse executable's PT_DYNAMIC (base = 0)
    let mut dyn_vaddr: u64 = 0;
    let mut dyn_memsz: u64 = 0;
    for i in 0..e_phnum as usize {
        let ph = phdr.add(i * phent);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type == PT_DYNAMIC {
            dyn_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
            dyn_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
            break;
        }
    }

    let mut dt_symtab: u64 = 0;
    let mut dt_strtab: u64 = 0;
    let mut dt_strsz: u64 = 0;
    let mut dt_init: u64 = 0;
    let mut dt_init_array: u64 = 0;
    let mut dt_init_array_sz: u64 = 0;
    let mut dt_init_present = false;
    let mut dt_init_array_present = false;
    let mut dt_runpath_off: u64 = 0;
    let mut dt_runpath_present = false;
    let mut dt_rpath_off: u64 = 0;
    let mut dt_rpath_present = false;
    let mut dt_gnu_hash: u64 = 0;
    let mut dt_hash: u64 = 0;
    let mut dt_versym: u64 = 0;

    // ponytail: max 32 DT_NEEDED, enough for any realistic binary
    let mut needed_offsets: [u64; 32] = [0; 32];
    let mut needed_count: usize = 0;

    if dyn_vaddr != 0 {
        let dyn_start = (exec_base + dyn_vaddr) as usize;
        let dyn_end = dyn_start + dyn_memsz as usize;
        let mut dp = dyn_start;
        while dp + 16 <= dyn_end {
            let d_tag = u64::from_le_bytes(core::ptr::read_unaligned(dp as *const [u8; 8]));
            let d_val = u64::from_le_bytes(core::ptr::read_unaligned((dp + 8) as *const [u8; 8]));
            if d_tag == DT_NULL {
                break;
            }
            match d_tag {
                DT_NEEDED => {
                    if needed_count < 32 {
                        needed_offsets[needed_count] = d_val;
                        needed_count += 1;
                    }
                }
                DT_SYMTAB => dt_symtab = d_val,
                DT_STRTAB => dt_strtab = d_val,
                DT_STRSZ => dt_strsz = d_val,
                DT_GNU_HASH => dt_gnu_hash = d_val,
                DT_HASH => dt_hash = d_val,
                0x6ffffff0 => dt_versym = d_val, // DT_VERSYM
                DT_INIT => { dt_init = d_val; dt_init_present = true; }
                DT_INIT_ARRAY => { dt_init_array = d_val; dt_init_array_present = true; }
                DT_INIT_ARRAYSZ => dt_init_array_sz = d_val,
                DT_RUNPATH => { dt_runpath_off = d_val; dt_runpath_present = true; }
                DT_RPATH => { dt_rpath_off = d_val; dt_rpath_present = true; }
                _ => {}
            }
            dp += 16;
        }
    }

    // Resolve DT_NEEDED name pointers (offsets into strtab)
    let mut needed_names: [(*const u8, usize); 32] = [(core::ptr::null(), 0); 32];
    for i in 0..needed_count {
        let name_ptr = (exec_base + dt_strtab + needed_offsets[i]) as *const u8;
        let name_len = str_len(name_ptr);
        needed_names[i] = (name_ptr, name_len);
    }

    if dt_runpath_present {
        RUNPATH = (exec_base + dt_strtab + dt_runpath_off) as *const u8;
        RUNPATH_LEN = str_len(RUNPATH);
    } else if dt_rpath_present {
        RUNPATH = (exec_base + dt_strtab + dt_rpath_off) as *const u8;
        RUNPATH_LEN = str_len(RUNPATH);
    }


    // Register executable as (**LOADED.add(0))
    let mut exec_map_start = u64::MAX;
    let mut exec_map_end = 0u64;
    for i in 0..phnum {
        let ph = phdr.add(i * phent);
        let p_type = u32::from_le_bytes(core::ptr::read_unaligned(ph as *const [u8; 4]));
        if p_type != PT_LOAD { continue; }
        let p_vaddr = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_VADDR) as *const [u8; 8]));
        let p_memsz = u64::from_le_bytes(core::ptr::read_unaligned(ph.add(PH_MEMSZ) as *const [u8; 8]));
        if p_vaddr < exec_map_start { exec_map_start = p_vaddr; }
        if p_vaddr + p_memsz > exec_map_end { exec_map_end = p_vaddr + p_memsz; }
    }
    if exec_map_start == u64::MAX { exec_map_start = 0; }
    let mut exec_sym_count: usize = 0;
    if dt_gnu_hash != 0 {
        exec_sym_count = sym_count_from_gnu_hash((exec_base + dt_gnu_hash) as usize);
    } else if dt_hash != 0 {
        exec_sym_count = sym_count_from_hash((exec_base + dt_hash) as usize);
    } else if dt_strtab > dt_symtab && dt_strtab - dt_symtab >= SYMTAB_ENT_SIZE as u64 {
        exec_sym_count = ((dt_strtab - dt_symtab) / SYMTAB_ENT_SIZE as u64) as usize;
    }
    {
        let value = LoadedObject {
        base: exec_base,
        symtab: (exec_base + dt_symtab) as *const u8,
        sym_count: exec_sym_count,
        strtab: (exec_base + dt_strtab) as *const u8,
        strsz: dt_strsz as usize,
        gnu_hash: if dt_gnu_hash != 0 { (exec_base + dt_gnu_hash) as usize } else { 0 },
        hash: if dt_hash != 0 { (exec_base + dt_hash) as usize } else { 0 },
        versym: if dt_versym != 0 { (exec_base + dt_versym) as usize } else { 0 },
        is_self: false,
        dyn_addr: (exec_base + dyn_vaddr) as usize,
        dyn_memsz: dyn_memsz as usize,
        tls_image: exec_tls_image,
        tls_filesz: exec_tls_filesz,
        tls_memsz: exec_tls_memsz,
        tls_align: exec_tls_align,
        init: exec_base + dt_init,
        init_array: exec_base + dt_init_array,
        init_array_sz: dt_init_array_sz,
        init_present: dt_init_present,
        init_array_present: dt_init_array_present,
        global: true,
        dev: 0,
        ino: 0,
        map_start: exec_base + exec_map_start,
        map_end: exec_base + exec_map_end,
        needed_by: NO_IDX,
        dep_count: 0,
        dep_cap: 0,
        deps: core::ptr::null_mut(),
        rpath: RUNPATH,
        rpath_len: RUNPATH_LEN,
        nodelete: false,
        name: core::ptr::null_mut(),
        name_cap: 0,
        };
        if push_object(value).is_none() {
            die(99, b"loaded_mmap", 0);
        }
    }

    // 5. Load the executable's DT_NEEDED entries, then every loaded
    // object's dependencies, breadth-first. DSO dependencies are not
    // optional: real programs (curl, git, python, ...) rely on them.
    for i in 0..needed_count {
        let (name_ptr, name_len) = needed_names[i];
        let dep = match load_needed_dso(name_ptr, name_len, ld_path, true, 0) {
            Some(d) => d,
            None => die(89, b"needed_fd", LOADED_COUNT),
        };
        add_dependency(0, dep);
    }
    let mut scan = 1usize;
    while scan < LOADED_COUNT {
        if !load_object_needed(scan, ld_path, true) {
            die(89, b"needed_fd", LOADED_COUNT);
        }
        scan += 1;
    }

    // Register the interpreter last: symbol lookup walks LOADED in order, and
    // the loader's private memcpy/strlen/etc. must never shadow libc.
    register_self(ldso_base);

    compute_tls_layout();
    STATIC_TLS_COUNT = LOADED_COUNT;

    process_all_relocations();
    // libc.so exports a link-time `__tls_get_addr` for static C++ runtimes.
    // Point it at the loader's multi-module resolver before constructors or
    // main can call it (including via dlsym).
    let set_tls_get_addr = resolve_symbol(b"__rc_set_tls_get_addr\0".as_ptr());
    if set_tls_get_addr != 0 {
        let setter: unsafe extern "C" fn(usize) = core::mem::transmute(set_tls_get_addr);
        setter(__tls_get_addr as *const () as usize);
    }
    register_dlopen_callbacks();

    // Always allocate a TCB so that %fs-relative accesses (e.g. stack canary
    // at %fs:0x28) work even when there is no TLS data in the binary.
    {
        let alloc_size = TLS_TOTAL_SIZE + TCB_SIZE;
        let tls_block = sys_mmap(
            core::ptr::null_mut(),
            alloc_size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        if tls_block as usize == MAP_FAILED {
            die(93, b"tls_mmap", alloc_size);
        }
        let _tcb = init_tls_block(tls_block);
        write_tp(_tcb as usize);
    }

    // Set libc.so's __auxv so constructors (e.g. compiler_builtins CPU feature
    // detection) can call getauxval before __libc_start_main runs.
    let argc = *(sp as *const u64) as usize;
    let argv_start = sp + 8;
    let envp_start = argv_start + (argc + 1) * 8;
    let mut envc = 0usize;
    while *((envp_start + envc * 8) as *const u64) != 0 {
        envc += 1;
    }
    let auxv = (envp_start + (envc + 1) * 8) as *const usize;
    let auxv_sym = resolve_symbol(b"__auxv\0".as_ptr());
    if auxv_sym != 0 {
        core::ptr::write(auxv_sym as *mut *const usize, auxv);
    }

    run_constructors();

    build_and_jump(e_entry, at_phdr, e_phnum, sp)
}

// ============================================================
// Build a fresh stack for the target program and jump
// ============================================================

struct OrigAuxv {
    at_base: u64,
    at_random: *const u8,
    at_secure: u64,
    at_uid: u64,
    at_euid: u64,
    at_gid: u64,
    at_egid: u64,
    at_sysinfo_ehdr: u64,
}

unsafe fn read_orig_auxv(orig_sp: usize, envc: usize) -> OrigAuxv {
    let mut out = OrigAuxv {
        at_base: 0,
        at_random: core::ptr::null(),
        at_secure: 0,
        at_uid: 0,
        at_euid: 0,
        at_gid: 0,
        at_egid: 0,
        at_sysinfo_ehdr: 0,
    };
    let argc = *(orig_sp as *const u64) as usize;
    let argv_start = orig_sp + 8;
    let envp_start = argv_start + (argc + 1) * 8;
    let auxv = (envp_start + (envc + 1) * 8) as *const u64;
    let mut i = 0;
    loop {
        let tag = *auxv.add(i * 2);
        let val = *auxv.add(i * 2 + 1);
        if tag == AT_NULL {
            break;
        }
        match tag {
            AT_BASE => out.at_base = val,
            AT_RANDOM => out.at_random = val as *const u8,
            AT_SECURE => out.at_secure = val,
            AT_UID => out.at_uid = val,
            AT_EUID => out.at_euid = val,
            AT_GID => out.at_gid = val,
            AT_EGID => out.at_egid = val,
            AT_SYSINFO_EHDR => out.at_sysinfo_ehdr = val,
            _ => {}
        }
        i += 1;
    }
    out
}

unsafe fn build_and_jump(entry: u64, phdr_addr: u64, phnum: u16, orig_sp: usize) -> ! {
    let argc = *(orig_sp as *const u64) as usize;
    let argv_start = orig_sp + 8;
    let envp_start = argv_start + (argc + 1) * 8;

    let mut envc: usize = 0;
    while *((envp_start + envc * 8) as *const u64) != 0 {
        envc += 1;
    }

    let orig_auxv = read_orig_auxv(orig_sp, envc);

    // Give the program a usable main stack. The previous 256 KiB was small
    // enough that runtimes like V8 (which derives its JS stack limit from
    // pthread_getattr_np) report "Maximum call stack size exceeded".
    let mut rlim = [0u64; 2];
    let mut stack_size = 8 * 1024 * 1024usize;
    if <Arch as Syscalls>::syscall2(SYS_GETRLIMIT, 3, &mut rlim as *mut [u64; 2] as i64) == 0 {
        let cur = rlim[0] as usize;
        if cur >= 1024 * 1024 && cur <= 64 * 1024 * 1024 {
            stack_size = cur;
        }
    }
    let stack_base = sys_mmap(
        core::ptr::null_mut(),
        stack_size,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if stack_base as usize == MAP_FAILED {
        die(94, b"stack_mmap", stack_size);
    }
    let mut sp = stack_base as usize + stack_size;

    // Do not cap argv/envp at a fixed array size.  The temporary pointer
    // array lives in an anonymous mapping and is unmapped before the jump.
    let ptr_words = match argc.checked_add(envc).and_then(|n| n.checked_add(2)) {
        Some(n) => n,
        None => die(95, b"argv_env_overflow", 0),
    };
    let ptr_bytes = match ptr_words.checked_mul(8) {
        Some(n) => n,
        None => die(95, b"argv_env_overflow", 0),
    };
    let ptr_map = sys_mmap(
        core::ptr::null_mut(),
        ptr_bytes,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANONYMOUS,
        -1,
        0,
    );
    if ptr_map as usize == MAP_FAILED {
        die(96, b"argv_env_mmap", ptr_bytes);
    }
    let ptrs = ptr_map as *mut usize;
    let new_argv = ptrs;
    let new_envp = ptrs.add(argc);

    for i in 0..argc {
        let s = *((argv_start + i * 8) as *const u64) as *const u8;
        let len = str_len(s);
        if sp < stack_base as usize + len + 1 + 4096 {
            die(97, b"argv_env_stack", 0);
        }
        sp -= len + 1;
        core::ptr::copy_nonoverlapping(s, sp as *mut u8, len + 1);
        *new_argv.add(i) = sp;
    }

    for i in 0..envc {
        let s = *((envp_start + i * 8) as *const u64) as *const u8;
        let len = str_len(s);
        if sp < stack_base as usize + len + 1 + 4096 {
            die(97, b"argv_env_stack", 0);
        }
        sp -= len + 1;
        core::ptr::copy_nonoverlapping(s, sp as *mut u8, len + 1);
        *new_envp.add(i) = sp;
    }

    // musl's crt1 needs AT_RANDOM for the stack canary; preserve it on the new stack.
    let mut random_bytes = [0u8; 16];
    if !orig_auxv.at_random.is_null() {
        core::ptr::copy_nonoverlapping(orig_auxv.at_random, random_bytes.as_mut_ptr(), 16);
    } else {
        for i in 0..16 {
            random_bytes[i] = (i as u8).wrapping_add(1);
        }
    }
    sp -= 16;
    sp &= !15usize;
    let random_ptr = sp;
    core::ptr::copy_nonoverlapping(random_bytes.as_ptr(), random_ptr as *mut u8, 16);

    // 16-byte align before structured data so argc lands on a boundary
    sp &= !15usize;

    // Pad before auxv so argc lands on a 16-byte boundary (no gap between
    // argv[] and argc).  envp/argv arrays plus argc are (envc+argc+3) words.
    if (envc + argc) % 2 == 0 {
        sp -= 8;
        *(sp as *mut u64) = 0;
    }

    const AUXV_ENTRIES: usize = 14;
    sp -= AUXV_ENTRIES * 16;
    let aux = sp as *mut u64;
    *aux.add(0) = AT_PHDR;
    *aux.add(1) = phdr_addr;
    *aux.add(2) = AT_PHENT;
    *aux.add(3) = PHDR_SIZE as u64;
    *aux.add(4) = AT_PHNUM;
    *aux.add(5) = phnum as u64;
    *aux.add(6) = AT_PAGESZ;
    *aux.add(7) = 4096;
    *aux.add(8) = AT_ENTRY;
    *aux.add(9) = entry;
    *aux.add(10) = AT_BASE;
    *aux.add(11) = orig_auxv.at_base;
    *aux.add(12) = AT_SECURE;
    *aux.add(13) = orig_auxv.at_secure;
    *aux.add(14) = AT_RANDOM;
    *aux.add(15) = random_ptr as u64;
    *aux.add(16) = AT_UID;
    *aux.add(17) = orig_auxv.at_uid;
    *aux.add(18) = AT_EUID;
    *aux.add(19) = orig_auxv.at_euid;
    *aux.add(20) = AT_GID;
    *aux.add(21) = orig_auxv.at_gid;
    *aux.add(22) = AT_EGID;
    *aux.add(23) = orig_auxv.at_egid;
    *aux.add(24) = AT_SYSINFO_EHDR;
    *aux.add(25) = orig_auxv.at_sysinfo_ehdr;
    *aux.add(26) = AT_NULL;
    *aux.add(27) = 0;

    sp -= (envc + 1) * 8;
    for i in 0..envc {
        *((sp + i * 8) as *mut u64) = *new_envp.add(i) as u64;
    }
    *((sp + envc * 8) as *mut u64) = 0;

    sp -= (argc + 1) * 8;
    for i in 0..argc {
        *((sp + i * 8) as *mut u64) = *new_argv.add(i) as u64;
    }
    *((sp + argc * 8) as *mut u64) = 0;

    sp -= 8;
    *(sp as *mut u64) = argc as u64;

    let _ = sys_munmap(ptr_map, ptr_bytes);

    #[cfg(target_arch = "x86_64")]
    core::arch::asm!(
        "mov rsp, {sp}",
        "jmp {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "aarch64")]
    core::arch::asm!(
        "mov sp, {sp}",
        "br {entry}",
        sp = in(reg) sp,
        entry = in(reg) entry,
        options(noreturn)
    );

    #[cfg(target_arch = "riscv64")]
    {
        // Workaround: riscv64 compiler reuses entry register for other
        // calculations. Force a volatile reload right before the asm block.
        let entry_ref = &entry as *const u64;
        let entry_val = unsafe { core::ptr::read_volatile(entry_ref) };
        core::arch::asm!(
            "mv sp, {sp}",
            "jr {entry}",
            sp = in(reg) sp,
            entry = in(reg) entry_val,
            options(noreturn)
        );
    }
}

// ============================================================
// Memory functions (required by no_std linker)
// ============================================================

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let d = s as *mut u8;
    let byte = c as u8;
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::asm!("rep stosb", inout("rdi") d => _, inout("rcx") n => _, in("al") byte,
                         options(nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let word = 0x0101_0101_0101_0101u64 * byte as u64;
        let mut i = 0usize;
        while i + 8 <= n {
            core::ptr::write_unaligned(d.add(i) as *mut u64, word);
            i += 8;
        }
        while i < n {
            *d.add(i) = byte;
            i += 1;
        }
    }
    s
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let d = dst as *mut u8;
    let s = src as *const u8;
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::asm!("rep movsb", inout("rdi") d => _, inout("rsi") s => _,
                         inout("rcx") n => _, options(nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut i = 0usize;
        while i + 8 <= n {
            core::ptr::write_unaligned(d.add(i) as *mut u64,
                core::ptr::read_unaligned(s.add(i) as *const u64));
            i += 8;
        }
        while i < n {
            *d.add(i) = *s.add(i);
            i += 1;
        }
    }
    dst
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    let dst = dst as *mut u8;
    let src = src as *const u8;
    if (dst as usize) < (src as usize) {
        let mut i = 0;
        while i < n {
            unsafe {
                *dst.add(i) = *src.add(i);
            }
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            unsafe {
                *dst.add(i) = *src.add(i);
            }
        }
    }
    dst as *mut c_void
}
