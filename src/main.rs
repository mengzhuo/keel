// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Standalone mapper for static ELF and static-PIE binaries.
//
// ELF parsing, segment mapping and startup-stack construction are shared
// with the in-libc dynamic linker; the architecture/syscall layer lives in
// arch.rs.

mod arch;
mod loader_core;

use arch::sys_mmap;

use loader_core::*;
use std::env;
use std::ffi::CString;
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: keel <elf-binary> [args...]");
        process::exit(1);
    }
    if let Err(e) = load_and_run(&args[1], &args[1..]) {
        eprintln!("keel: {e}");
        process::exit(1);
    }
}

/// Read the kernel's auxv for this process.  The target inherits the same
/// address space, so pointers such as AT_RANDOM remain valid for the copy
/// performed while building the new stack.
fn read_host_auxv() -> Vec<(u64, u64)> {
    let Ok(data) = fs::read("/proc/self/auxv") else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(data.len() / 16);
    let mut i = 0;
    while i + 16 <= data.len() {
        let tag = u64::from_ne_bytes(data[i..i + 8].try_into().unwrap());
        let val = u64::from_ne_bytes(data[i + 8..i + 16].try_into().unwrap());
        i += 16;
        if tag == AT_NULL {
            break;
        }
        out.push((tag, val));
    }
    out
}

fn auxv_get(auxv: &[(u64, u64)], tag: u64) -> u64 {
    auxv.iter()
        .find_map(|&(t, v)| if t == tag { Some(v) } else { None })
        .unwrap_or(0)
}

fn load_and_run(path: &str, argv: &[String]) -> Result<(), String> {
    // Read through the same descriptor used for mmap: reopening by path can
    // observe a different file after a rename.
    let mut file = fs::File::open(path).map_err(|e| format!("open {path}: {e}"))?;
    let mut data = Vec::new();
    file.read_to_end(&mut data)
        .map_err(|e| format!("read {path}: {e}"))?;

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
        &read_host_auxv(),
    )
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

fn transfer(
    entry: u64,
    argv: &[String],
    phdr_addr: u64,
    phnum: usize,
    host_auxv: &[(u64, u64)],
) -> Result<(), String> {
    let c_argv: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_str()).map_err(|_| "argument contains a NUL byte".to_string()))
        .collect::<Result<_, _>>()?;
    let argv_ptrs: Vec<*const u8> = c_argv.iter().map(|c| c.as_ptr() as *const u8).collect();
    let envp_ptrs: Vec<*const u8> = Vec::new();

    // The standalone mapper does not forward the host environment, so the
    // target sees an empty envp (documented in docs/limitations.md).
    let auxv = [
        (AT_PHDR, phdr_addr),
        (AT_PHENT, PHDR_SIZE as u64),
        (AT_PHNUM, phnum as u64),
        (AT_PAGESZ, PAGE_SIZE as u64),
        (AT_ENTRY, entry),
        (AT_BASE, 0),
        (AT_SECURE, 0),
        (AT_UID, auxv_get(host_auxv, AT_UID)),
        (AT_EUID, auxv_get(host_auxv, AT_EUID)),
        (AT_GID, auxv_get(host_auxv, AT_GID)),
        (AT_EGID, auxv_get(host_auxv, AT_EGID)),
        (AT_SYSINFO_EHDR, auxv_get(host_auxv, AT_SYSINFO_EHDR)),
    ];
    let random = auxv_get(host_auxv, AT_RANDOM) as *const u8;

    let stack_size = 8 * 1024 * 1024;
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
        return Err("mmap stack failed".to_string());
    }

    let mut scratch = vec![0usize; argv_ptrs.len()];
    let sp = unsafe {
        build_initial_stack(
            stack_base as usize,
            stack_size,
            &argv_ptrs,
            &envp_ptrs,
            &auxv,
            random,
            scratch.as_mut_ptr(),
        )
    }
    .ok_or_else(|| "initial stack overflow".to_string())?;

    // The target is static: it must build its own TCB.  Do not let it inherit
    // the loader's Rust/glibc thread pointer.
    unsafe {
        arch::clear_thread_pointer();
        jump_to_entry(entry, sp);
    }

    #[allow(unreachable_code)]
    Ok(())
}
