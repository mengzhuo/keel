// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! Check that every numeric macro in keel's headers agrees with the Rust
//! constant the implementation actually uses.
//!
//! The headers and the Rust sources (`libc/src` and the standalone loader in
//! `src`) are independent sources of truth: the headers are hand-maintained
//! musl-compatible declarations, while the library keeps its own numeric
//! constants.  This tool expands the headers with the target preprocessor
//! (`cc -dM -E`, one compiler per architecture) and compares each integer
//! macro with the matching Rust constant, resolving macro references and
//! simple expressions on both sides.
//!
//! It replaces `tests/header_rust_consistency.py` and runs as the first gate
//! of CI.

mod eval;
mod headers;
mod rustsrc;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use headers::{eval_c, header_source, parse_defines, preprocess, MacroKind};
use rustsrc::eval_rust;

/// Architectures keel supports, with the compiler that predefines each one's
/// macros.  `gcc` is the host compiler on the x86_64 runners.
const ARCHS: [(&str, &str); 3] = [
    ("x86_64", "gcc"),
    ("aarch64", "aarch64-linux-gnu-gcc"),
    ("riscv64", "riscv64-linux-gnu-gcc"),
];

struct Mismatch {
    arch: &'static str,
    name: String,
    rust: String,
    c: String,
    rust_value: i128,
    c_value: i128,
}

fn main() -> ExitCode {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("tools/header-check lives inside the repository");
    let (include, min_compared) = match parse_args(root) {
        Ok(args) => args,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let constants = rustsrc::scan(root);
    let source = match header_source(&include) {
        Ok(source) => source,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let mut compared = 0u64;
    let mut matched = 0u64;
    let mut skipped: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut mismatches: Vec<Mismatch> = Vec::new();

    for (arch, cc) in ARCHS {
        let stdout = match preprocess(cc, &include, &source) {
            Ok(stdout) => stdout,
            Err(message) => {
                eprintln!("error: {message}");
                return ExitCode::FAILURE;
            }
        };
        let table = parse_defines(&stdout);

        for (name, by_arch) in &constants {
            let mut values: BTreeSet<&String> = BTreeSet::new();
            if let Some(expressions) = by_arch.get(&Some(arch.to_string())) {
                values.extend(expressions);
            }
            if let Some(expressions) = by_arch.get(&None) {
                values.extend(expressions);
            }
            if values.is_empty() {
                continue;
            }
            if values.len() > 1 {
                *skipped.entry("rust-conflict").or_default() += 1;
                continue;
            }
            // SB is an internal character-range constant unrelated to ELF's SB.
            if name == "SB" {
                continue;
            }
            let Some(entry) = table.get(name) else {
                continue;
            };
            if entry.kind != MacroKind::Object {
                *skipped.entry("function-macro").or_default() += 1;
                continue;
            }
            let rust_expr = values.iter().next().expect("non-empty").to_string();
            let Ok(rust_value) = eval_rust(&rust_expr) else {
                *skipped.entry("rust-expr").or_default() += 1;
                continue;
            };
            let Ok(c_value) = eval_c(&entry.body, &table, &[]) else {
                *skipped.entry("c-expr").or_default() += 1;
                continue;
            };
            compared += 1;
            if rust_value == c_value {
                matched += 1;
            } else {
                mismatches.push(Mismatch {
                    arch,
                    name: name.clone(),
                    rust: rust_expr,
                    c: entry.body.clone(),
                    rust_value,
                    c_value,
                });
            }
        }

        // musl exposes syscall numbers as __NR_name while the Rust code uses
        // SYS_NAME.  Check that the two spellings agree too.
        for (name, entry) in &table {
            let Some(number) = name.strip_prefix("__NR_") else {
                continue;
            };
            if entry.kind != MacroKind::Object {
                continue;
            }
            let rust_name = format!("SYS_{}", number.to_ascii_uppercase());
            let Some(by_arch) = constants.get(&rust_name) else {
                continue;
            };
            let mut values: BTreeSet<&String> = BTreeSet::new();
            if let Some(expressions) = by_arch.get(&Some(arch.to_string())) {
                values.extend(expressions);
            }
            if let Some(expressions) = by_arch.get(&None) {
                values.extend(expressions);
            }
            if values.len() != 1 {
                continue;
            }
            let rust_expr = values.iter().next().expect("non-empty").to_string();
            let (Ok(rust_value), Ok(c_value)) =
                (eval_rust(&rust_expr), eval_c(&entry.body, &table, &[]))
            else {
                *skipped.entry("syscall-expr").or_default() += 1;
                continue;
            };
            compared += 1;
            if rust_value == c_value {
                matched += 1;
            } else {
                mismatches.push(Mismatch {
                    arch,
                    name: name.clone(),
                    rust: rust_expr,
                    c: entry.body.clone(),
                    rust_value,
                    c_value,
                });
            }
        }
    }

    let skipped = format_skipped(&skipped);
    println!(
        "compared={compared} matches={matched} mismatches={} skipped={skipped}",
        mismatches.len()
    );
    for mismatch in &mismatches {
        println!(
            "  mismatch: ('{}', '{}', '{}', '{}', {}, {})",
            mismatch.arch,
            mismatch.name,
            mismatch.rust,
            mismatch.c,
            mismatch.rust_value,
            mismatch.c_value
        );
    }
    if compared < min_compared {
        println!("error: only {compared} constants compared (expected at least {min_compared})");
        return ExitCode::FAILURE;
    }
    if mismatches.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn format_skipped(skipped: &BTreeMap<&'static str, u64>) -> String {
    let entries: Vec<String> = skipped
        .iter()
        .map(|(reason, count)| format!("'{reason}': {count}"))
        .collect();
    format!("{{{}}}", entries.join(", "))
}

fn parse_args(root: &Path) -> Result<(PathBuf, u64), String> {
    let mut include = root.join("include");
    let mut min_compared = 1300;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--include" => {
                include = PathBuf::from(args.next().ok_or("--include needs a path")?);
            }
            "--min-compared" => {
                let value = args.next().ok_or("--min-compared needs a number")?;
                min_compared = value
                    .parse()
                    .map_err(|_| format!("invalid --min-compared: {value}"))?;
            }
            "--help" | "-h" => {
                println!(
                    "usage: keel-header-check [--include DIR] [--min-compared N]\n\
                     \n\
                     Expands keel's headers for x86_64, aarch64 and riscv64 and checks\n\
                     every integer macro against the matching Rust constant."
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok((include, min_compared))
}
