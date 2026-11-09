//! `ldconfig`: create soname symlinks for shared libraries.
//!
//! musl does not use an `ld.so.cache`; Alpine's ldconfig is a small shell
//! script that delegates the ELF parsing to `scanelf`.  This Rust version
//! reads the SONAME directly and creates the same links.  With no directory
//! arguments it is a no-op, matching musl-utils.

use crate::elf::ElfFile;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn main(args: Vec<OsString>) -> ExitCode {
    let mut verbose = false;
    let mut root: Option<PathBuf> = None;
    let mut dirs = Vec::new();
    let mut i = 0usize;
    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-v" | "--verbose" => verbose = true,
            "-n" | "-N" | "-X" => {}
            "-f" | "-C" => {
                i += 1; // consume the cache-file argument
            }
            "-r" => {
                i += 1;
                if i < args.len() {
                    root = Some(PathBuf::from(&args[i]));
                }
            }
            "--" => {
                i += 1;
                while i < args.len() {
                    dirs.push(PathBuf::from(&args[i]));
                    i += 1;
                }
                break;
            }
            _ if arg.starts_with("-f") || arg.starts_with("-C") || arg.starts_with("-r") => {
                // Attached forms: -fFILE, -CDIR, -rROOT.
                if let Some(value) = arg.get(2..) {
                    if arg.starts_with("-r") {
                        root = Some(PathBuf::from(value));
                    }
                }
            }
            _ if arg.starts_with('-') => {
                eprintln!("ldconfig: unknown option: {arg}");
                return ExitCode::from(1);
            }
            _ => dirs.push(PathBuf::from(arg)),
        }
        i += 1;
    }

    if dirs.is_empty() {
        return ExitCode::SUCCESS;
    }

    let mut status = 0u8;
    for dir in dirs {
        let dir = match &root {
            Some(root) => {
                let stripped = dir.strip_prefix("/").unwrap_or(&dir);
                root.join(stripped)
            }
            None => dir,
        };
        if let Err(err) = scan_dir(&dir, verbose) {
            eprintln!("ldconfig: {}: {err}", dir.display());
            status = 1;
        }
    }
    ExitCode::from(status)
}

fn scan_dir(dir: &Path, verbose: bool) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if !file_type.is_file() {
            continue;
        }
        let elf = match ElfFile::open(&path) {
            Ok(elf) => elf,
            Err(_) => continue,
        };
        let Some(soname) = elf.soname else { continue };
        if soname.is_empty() || soname.contains('/') {
            continue;
        }

        // Package managers own the canonical system directories.  This
        // mirrors Alpine's ldconfig script, which only creates links in
        // directories passed explicitly outside those trees.
        if is_system_dir(dir) {
            continue;
        }

        let link = dir.join(&soname);
        let target = path.file_name().unwrap_or_default();
        let replace = match fs::symlink_metadata(&link) {
            Ok(meta) => meta.file_type().is_symlink(),
            Err(_) => true,
        };
        if !replace {
            continue;
        }
        if verbose {
            println!(
                "Adding link {} -> {}",
                link.display(),
                target.to_string_lossy()
            );
        }
        let _ = fs::remove_file(&link);
        symlink(target, &link)?;
    }
    Ok(())
}

fn is_system_dir(dir: &Path) -> bool {
    let s = dir.to_string_lossy();
    s == "/lib"
        || s.starts_with("/lib/")
        || s == "/usr/lib"
        || s.starts_with("/usr/lib/")
        || s == "/usr/local/lib"
        || s.starts_with("/usr/local/lib/")
}
