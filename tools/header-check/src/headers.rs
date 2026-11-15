// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! Reading keel's C headers: expand them with the target compiler's
//! preprocessor and evaluate the resulting object-like macros.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::eval::{tokenize, Bad, Eval, Parser};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroKind {
    Object,
    Function,
}

#[derive(Debug, Clone)]
pub struct Macro {
    pub kind: MacroKind,
    pub body: String,
}

pub type MacroTable = BTreeMap<String, Macro>;

/// Build the translation unit that pulls in every public header.  `bits/` is
/// excluded, as it is in musl: those headers are implementation details.
pub fn header_source(include: &Path) -> Result<String, String> {
    let mut headers = Vec::new();
    collect_headers(include, include, &mut headers)?;
    headers.sort();
    let mut source = String::from("#define _GNU_SOURCE 1\n");
    for header in headers {
        source.push_str("#include <");
        source.push_str(&header);
        source.push_str(">\n");
    }
    Ok(source)
}

fn collect_headers(root: &Path, dir: &Path, out: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut paths = Vec::new();
    for entry in entries {
        paths.push(entry.map_err(|e| e.to_string())?.path());
    }
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect_headers(root, &path, out)?;
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("h") {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.starts_with("bits/") {
            continue;
        }
        out.push(rel);
    }
    Ok(())
}

/// Run `cc -dM -E` on the header source and return its stdout.
pub fn preprocess(cc: &str, include: &Path, source: &str) -> Result<String, String> {
    let mut child = Command::new(cc)
        .args([
            "-std=gnu11",
            "-D_GNU_SOURCE",
            "-nostdinc",
            "-I",
            include.to_str().unwrap(),
            "-dM",
            "-E",
            "-x",
            "c",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to run {cc}: {e}"))?;
    child
        .stdin
        .as_mut()
        .expect("piped stdin")
        .write_all(source.as_bytes())
        .map_err(|e| format!("failed to write to {cc}: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("failed to wait for {cc}: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{cc} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("{cc} emitted invalid UTF-8: {e}"))
}

/// Parse the `#define` lines of `cc -dM -E` output.
pub fn parse_defines(stdout: &str) -> MacroTable {
    let mut table = MacroTable::new();
    for line in stdout.lines() {
        let Some(rest) = line.strip_prefix("#define ") else {
            continue;
        };
        let bytes = rest.as_bytes();
        let mut end = 0;
        while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
            end += 1;
        }
        if end == 0 {
            continue;
        }
        let name = rest[..end].to_string();
        let (kind, body) = if bytes.get(end) == Some(&b'(') {
            let Some(close) = rest[end..].find(')') else {
                continue;
            };
            let body = rest[end + close + 1..].trim_start();
            (MacroKind::Function, body.to_string())
        } else {
            (MacroKind::Object, rest[end..].trim_start().to_string())
        };
        table.insert(name, Macro { kind, body });
    }
    table
}

/// Evaluate `expr`, resolving object-like macros through `table`.
pub fn eval_c(expr: &str, table: &MacroTable, seen: &[String]) -> Eval {
    let tokens = tokenize(expr);
    let mut parser = Parser::new(&tokens);
    parser.parse(&mut |name: &str| {
        if seen.iter().any(|previous| previous == name) {
            return Err(Bad("recursive"));
        }
        let entry = table.get(name).ok_or(Bad("missing"))?;
        if entry.kind != MacroKind::Object {
            return Err(Bad("function"));
        }
        let mut seen = seen.to_vec();
        seen.push(name.to_string());
        eval_c(&entry.body, table, &seen)
    })
}
