// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Standalone mapper for static ELF and static-PIE binaries.
//
// ELF parsing, segment mapping and relocation live in loader_core.rs; the
// architecture/syscall layer lives in arch.rs.

mod arch;
mod loader_core;

use arch::sys_mmap;

use loader_core::*;
use std::env;
use std::ffi::CString;
use std::fs;
use std::os::unix::io::AsRawFd;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: loader <elf-binary> [args...]");
        process::exit(1);
    }
    if let Err(e) = load_and_run(&args[1], &args[1..]) {
        eprintln!("loader: {e}");
        process::exit(1);
    }
}

/// Read the kernel's vDSO address from our own auxv.  The target inherits
/// the same address space, so the same pointer stays valid after the jump.
fn read_sysinfo_ehdr() -> u64 {
    let Ok(data) = fs::read("/proc/self/auxv") else {
        return 0;
    };
    let mut i = 0;
    while i + 16 <= data.len() {
        let tag = u64::from_ne_bytes(data[i..i + 8].try_into().unwrap());
        let val = u64::from_ne_bytes(data[i + 8..i + 16].try_into().unwrap());
        if tag == AT_NULL {
            break;
        }
        if tag == AT_SYSINFO_EHDR {
            return val;
        }
        i += 16;
    }
    0
}

fn load_and_run(path: &str, argv: &[String]) -> Result<(), String> {
    let file = fs::File::open(path).map_err(|e| format!("open {path}: {e}"))?;
    let data = fs::read(path).map_err(|e| format!("read {path}: {e}"))?;

    let ehdr = parse_ehdr(&data).map_err(|e| e.to_string())?;
    let phdrs = parse_phdrs(&data, &ehdr).map_err(|e| e.to_string())?;

    let load_bias =
        map_segments(phdrs, file.as_raw_fd(), ehdr.e_type).map_err(|e| e.to_string())?;

    apply_relocations(&data, phdrs, load_bias).map_err(|e| e.to_string())?;

    let entry = ehdr.e_entry + load_bias;
    transfer(
        entry,
        argv,
        phdr_runtime_addr(&ehdr, phdrs, load_bias),
        phdrs.len(),
        read_sysinfo_ehdr(),
    );
}

/// Translate e_phoff (a file offset) to its runtime address.  For ET_EXEC,
/// e_phoff is not a virtual address: the headers live inside the first
/// PT_LOAD, usually at 0x400000 + e_phoff, not e_phoff.
fn phdr_runtime_addr(ehdr: &Ehdr, phdrs: &[Phdr], load_bias: u64) -> u64 {
    for ph in phdrs {
        if ph.p_type == PT_PHDR {
            return load_bias + ph.p_vaddr;
        }
    }
    for ph in phdrs.iter().filter(|p| p.p_type == PT_LOAD) {
        if ehdr.e_phoff >= ph.p_offset && ehdr.e_phoff < ph.p_offset + ph.p_filesz {
            return load_bias + ph.p_vaddr + (ehdr.e_phoff - ph.p_offset);
        }
    }
    load_bias + ehdr.e_phoff
}

fn transfer(entry: u64, argv: &[String], phdr_addr: u64, phnum: usize, sysinfo_ehdr: u64) -> ! {
    let c_argv: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_str()).unwrap())
        .collect();

    let stack_size = 64 * 1024;
    let stack_base = unsafe {
        sys_mmap(
            core::ptr::null_mut(),
            stack_size,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if stack_base == MAP_FAILED {
        panic!("mmap stack failed");
    }

    let mut sp = stack_base as usize + stack_size;
    let mut str_ptrs = Vec::new();

    for c in &c_argv {
        let bytes = c.as_bytes_with_nul();
        sp -= bytes.len();
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), sp as *mut u8, bytes.len()) };
        str_ptrs.push(sp);
    }

    sp &= !7usize;

    let auxv_entries: [(u64, u64); 7] = [
        (AT_PHDR, phdr_addr),
        // Elf64_Phdr size; static libc uses it while walking PT_TLS.
        (AT_PHENT, 56),
        (AT_PHNUM, phnum as u64),
        (AT_PAGESZ, PAGE_SIZE as u64),
        (AT_ENTRY, entry),
        (AT_SYSINFO_EHDR, sysinfo_ehdr),
        (AT_NULL, 0),
    ];

    // Lay out the structured area so that the final stack pointer stays
    // 16-byte aligned.  argc is written last and must sit exactly at sp;
    // aligning after writing it (as this loader used to do) shifts argc.
    sp &= !15usize;
    let auxv_bytes = auxv_entries.len() * 16;
    let words = 1 /* argc */ + 1 /* argv NULL */ + str_ptrs.len() + 1 /* envp NULL */;
    if (auxv_bytes + words * 8) % 16 != 0 {
        // Padding above the auxv is ignored: the scan stops at AT_NULL.
        sp -= 8;
        unsafe { *(sp as *mut u64) = 0 };
    }
    sp -= auxv_bytes;
    let auxv_ptr = sp as *mut (u64, u64);
    for (i, &(tag, val)) in auxv_entries.iter().enumerate() {
        unsafe { *auxv_ptr.add(i) = (tag, val) };
    }

    sp -= 8;
    unsafe { *(sp as *mut u64) = 0 };
    sp -= 8;
    unsafe { *(sp as *mut u64) = 0 };

    for &ptr in str_ptrs.iter().rev() {
        sp -= 8;
        unsafe { *(sp as *mut u64) = ptr as u64 };
    }

    sp -= 8;
    unsafe { *(sp as *mut u64) = argv.len() as u64 };

    // The target is static: it must build its own TCB.  Do not let it inherit
    // the loader's Rust/glibc thread pointer.
    unsafe { arch::clear_thread_pointer() };

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "mov rsp, {sp}",
            "jmp {entry}",
            sp = in(reg) sp,
            entry = in(reg) entry,
            in("rdi") argv.len(),
            in("rsi") sp + 8,
            options(noreturn)
        );
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!(
            "mov sp, {sp}",
            "mov x0, {argc}",
            "mov x1, {argv}",
            "br {entry}",
            sp = in(reg) sp,
            argc = in(reg) argv.len(),
            argv = in(reg) sp + 8,
            entry = in(reg) entry,
            options(noreturn)
        );
    }

    #[cfg(target_arch = "riscv64")]
    unsafe {
        core::arch::asm!(
            "mv sp, {sp}",
            "mv a0, {argc}",
            "mv a1, {argv}",
            "jr {entry}",
            sp = in(reg) sp,
            argc = in(reg) argv.len(),
            argv = in(reg) sp + 8,
            entry = in(reg) entry,
            options(noreturn)
        );
    }
}
