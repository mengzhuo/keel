// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! `ldd`: print the shared-library dependencies of a dynamic executable.
//!
//! musl's `ldd` is a two-line shell wrapper around the loader's `--list`
//! mode.  keel's loader is deliberately not a command-line interpreter, so
//! this implementation parses ELF files directly.  The output is the same
//! shape (`name => path`) but omits load addresses.

use crate::elf::ElfFile;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn main(args: Vec<OsString>) -> ExitCode {
    let mut files = Vec::new();
    for arg in args {
        let s = arg.to_string_lossy();
        match s.as_ref() {
            "--list" | "--verify" | "-v" | "--verbose" => {}
            "-r" | "--function-relocs" | "-u" | "--unused" | "-d" | "--data-relocs" => {}
            _ if s.starts_with('-') && s != "-" => {
                eprintln!("ldd: unknown option: {s}");
                return ExitCode::from(1);
            }
            _ => files.push(PathBuf::from(arg)),
        }
    }
    if files.is_empty() {
        eprintln!("usage: ldd [options] file...");
        return ExitCode::from(1);
    }

    let mut status = 0u8;
    for file in files {
        if let Err(err) = one(&file) {
            eprintln!("ldd: {}: {err}", file.display());
            status = 1;
        }
    }
    ExitCode::from(status)
}

fn one(file: &Path) -> Result<(), String> {
    let elf = ElfFile::open(file).map_err(|e| e.to_string())?;
    if !elf.is_dynamic() {
        return Err("not a dynamic executable".into());
    }

    let mut seen: Vec<(u64, u64)> = Vec::new();
    if let Some(id) = file_identity(file) {
        seen.push(id);
    }
    let root = file.to_path_buf();

    if let Some(interp) = &elf.interp {
        println!("\t{interp} (0x0)");
    }

    let mut missing = false;
    let mut queue: Vec<(PathBuf, ElfFile, Vec<PathBuf>)> = vec![(root, elf, Vec::new())];
    let mut index = 0usize;
    while index < queue.len() {
        let (parent, current, inherited) = queue[index].clone();
        index += 1;

        let origin = parent
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        let child_rpaths = rpath_dirs(&current, &origin);

        for name in &current.needed {
            match resolve(&parent, &current, name, &inherited) {
                Some(path) => {
                    let seen_before = match file_identity(&path) {
                        Some(id) if seen.contains(&id) => true,
                        Some(id) => {
                            seen.push(id);
                            false
                        }
                        None => false,
                    };
                    if !seen_before {
                        match ElfFile::open(&path) {
                            Ok(dep) => {
                                println!("\t{name} => {} (0x0)", path.display());
                                let mut child_inherited = child_rpaths.clone();
                                for dir in &inherited {
                                    push_unique(&mut child_inherited, dir.clone());
                                }
                                queue.push((path, dep, child_inherited));
                            }
                            Err(err) => {
                                println!("\t{name} => {} (invalid: {err})", path.display());
                                missing = true;
                            }
                        }
                    } else {
                        println!("\t{name} => {} (0x0)", path.display());
                    }
                }
                None => {
                    println!("\t{name} => not found");
                    missing = true;
                }
            }
        }
    }

    if missing {
        Err("not all dependencies were found".into())
    } else {
        Ok(())
    }
}

fn resolve(parent: &Path, elf: &ElfFile, name: &str, inherited: &[PathBuf]) -> Option<PathBuf> {
    let name_path = Path::new(name);
    if name_path.is_absolute() {
        return name_path.is_file().then(|| name_path.to_path_buf());
    }
    if name.contains('/') {
        let p = parent.parent().unwrap_or_else(|| Path::new(".")).join(name);
        return p.is_file().then_some(p);
    }

    let origin = parent
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let mut dirs = Vec::new();
    if let Some(path) = env::var_os("LD_LIBRARY_PATH") {
        for part in env::split_paths(&path) {
            push_unique(&mut dirs, part);
        }
    }
    for dir in rpath_dirs(elf, &origin) {
        push_unique(&mut dirs, dir);
    }
    for dir in inherited {
        push_unique(&mut dirs, dir.clone());
    }
    // Use the loader's normal system order last.  /usr/lib/keel is a
    // fallback for a host that has keel installed but not as the system
    // libc; keel-linked objects normally reach it through RPATH/RUNPATH.
    for fallback in ["/lib", "/usr/lib", "/usr/local/lib", "/usr/lib/keel"] {
        push_unique(&mut dirs, PathBuf::from(fallback));
    }

    for dir in dirs {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn rpath_dirs(elf: &ElfFile, origin: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let rpath = elf.runpath.as_deref().or(elf.rpath.as_deref());
    if let Some(rpath) = rpath {
        for part in rpath.split(':') {
            if part.is_empty() {
                continue;
            }
            let expanded = part
                .replace("${ORIGIN}", &origin.to_string_lossy())
                .replace("$ORIGIN", &origin.to_string_lossy());
            push_unique(&mut dirs, PathBuf::from(expanded));
        }
    }
    dirs
}

fn push_unique(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dirs.iter().any(|d| d == &dir) {
        dirs.push(dir);
    }
}

fn file_identity(path: &Path) -> Option<(u64, u64)> {
    fs::metadata(path).ok().map(|meta| (meta.dev(), meta.ino()))
}
