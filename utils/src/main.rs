// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! Rust implementations of the commands provided by Alpine's `musl-utils`.
//!
//! The package installs a single `crabc-utils` multicall binary and symlinks
//! named after each command.  Dispatching on `argv[0]` keeps the installed
//! interface identical to `musl-utils` without duplicating the runtime.

mod elf;
mod getconf;
mod getent;
mod iconv_cmd;
mod ldconfig;
mod ldd;

use std::env;
use std::path::Path;
use std::process::ExitCode;

const COMMANDS: &[&str] = &["getconf", "getent", "iconv", "ldd", "ldconfig"];

fn usage(program: &str) -> ExitCode {
    eprintln!(
        "usage: {program} <command> [arguments...]\n\
         commands: {}",
        COMMANDS.join(", ")
    );
    ExitCode::from(1)
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let argv0 = args.next().unwrap_or_default();
    let name = Path::new(&argv0)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("crabc-utils");

    let (command, rest): (String, Vec<std::ffi::OsString>) = if COMMANDS.contains(&name) {
        (name.to_string(), args.collect())
    } else {
        match args.next() {
            Some(cmd) => (cmd.to_string_lossy().into_owned(), args.collect::<Vec<_>>()),
            None => return usage(name),
        }
    };

    match command.as_str() {
        "getconf" => getconf::main(rest),
        "getent" => getent::main(rest),
        "iconv" => iconv_cmd::main(rest),
        "ldd" => ldd::main(rest),
        "ldconfig" => ldconfig::main(rest),
        "-h" | "--help" | "help" => {
            println!(
                "crabc-utils: Rust implementations of musl-utils commands\n\
                 commands: {}",
                COMMANDS.join(", ")
            );
            ExitCode::SUCCESS
        }
        "-V" | "--version" | "version" => {
            println!("crabc-utils {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("crabc-utils: unknown command: {command}");
            usage(name)
        }
    }
}
