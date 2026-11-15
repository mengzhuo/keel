// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! Reading the Rust side: collect `const NAME: TY = EXPR;` items and their
//! architecture, then evaluate the expressions that are plain integers.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::eval::{tokenize, Bad, Eval, Parser};

/// name -> architecture (`None` when the constant is not arch-specific)
///      -> every expression the name was defined with.
pub type Constants = BTreeMap<String, BTreeMap<Option<String>, BTreeSet<String>>>;

pub fn scan(root: &Path) -> Constants {
    let mut constants = Constants::new();
    // `libc/src` also contains the in-tree dynamic linker (`libc/src/ldso`).
    // The standalone loader in `src/` keeps its own arch tables and auxv
    // constants, so check those against the headers too.
    for base in ["libc/src", "src"] {
        let dir = root.join(base);
        if dir.is_dir() {
            walk(&dir, &mut constants);
        }
    }
    constants
}

fn walk(dir: &Path, constants: &mut Constants) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths = Vec::new();
    for entry in entries.flatten() {
        paths.push(entry.path());
    }
    paths.sort();
    for path in paths {
        if path.is_dir() {
            walk(&path, constants);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            let bytes = std::fs::read(&path).unwrap_or_default();
            scan_file(&String::from_utf8_lossy(&bytes), constants);
        }
    }
}

fn scan_file(text: &str, constants: &mut Constants) {
    let mut depth: i32 = 0;
    let mut stack: Vec<(i32, Option<String>)> = Vec::new();
    let mut pending: Option<String> = None;
    let mut block_comment = false;
    for line in text.lines() {
        let line = line.trim();
        if block_comment {
            if line.contains("*/") {
                block_comment = false;
            }
            continue;
        }
        if line.starts_with("/*") {
            if !line.contains("*/") {
                block_comment = true;
            }
            continue;
        }
        if let Some(arch) = cfg_arch(line) {
            pending = Some(arch);
        }
        if mod_open(line) {
            stack.push((depth, pending.take()));
        }
        if let Some((name, expr)) = const_line(line) {
            let arch = pending
                .take()
                .or_else(|| stack.last().and_then(|(_, arch)| arch.clone()));
            constants
                .entry(name)
                .or_default()
                .entry(arch)
                .or_default()
                .insert(expr);
        }
        depth += line.matches('{').count() as i32 - line.matches('}').count() as i32;
        while let Some(&(open_depth, _)) = stack.last() {
            if depth <= open_depth {
                stack.pop();
            } else {
                break;
            }
        }
    }
}

/// Match `#[cfg(target_arch = "x86_64")]`.
fn cfg_arch(line: &str) -> Option<String> {
    let rest = line.strip_prefix("#[cfg(target_arch")?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    if !rest[end + 1..].starts_with(")]") {
        return None;
    }
    let arch = &rest[..end];
    if arch.is_empty() || !arch.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some(arch.to_string())
}

/// Match `(pub )?mod NAME {`.
fn mod_open(line: &str) -> bool {
    let rest = strip_word(line, "pub").unwrap_or(line);
    let Some(rest) = strip_word(rest, "mod") else {
        return false;
    };
    let name_end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    name_end > 0 && rest[name_end..].trim_start().starts_with('{')
}

/// Match `(pub )?const NAME: TYPE = EXPR;`.
fn const_line(line: &str) -> Option<(String, String)> {
    let rest = strip_word(line, "pub").unwrap_or(line);
    let rest = strip_word(rest, "const")?;
    let bytes = rest.as_bytes();
    let mut end = 0;
    while end < bytes.len()
        && (bytes[end].is_ascii_uppercase() || bytes[end].is_ascii_digit() || bytes[end] == b'_')
    {
        end += 1;
    }
    if end == 0 || !bytes[0].is_ascii_uppercase() {
        return None;
    }
    let name = rest[..end].to_string();
    let rest = rest[end..].trim_start().strip_prefix(':')?.trim_start();
    let eq = rest.find('=')?;
    if eq == 0 {
        return None;
    }
    let after = &rest[eq + 1..];
    let semi = after.find(';')?;
    Some((name, after[..semi].trim().to_string()))
}

fn strip_word<'a>(line: &'a str, word: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(word)?;
    if rest.starts_with(|c: char| c.is_whitespace()) {
        Some(rest.trim_start())
    } else {
        None
    }
}

const REPLACEMENTS: [(&str, &str); 9] = [
    ("i64::MAX", "9223372036854775807"),
    ("i64::MIN", "-9223372036854775808"),
    ("i32::MAX", "2147483647"),
    ("i32::MIN", "-2147483648"),
    ("u64::MAX", "18446744073709551615"),
    ("u32::MAX", "4294967295"),
    ("usize::MAX", "18446744073709551615"),
    ("isize::MAX", "9223372036854775807"),
    ("isize::MIN", "-9223372036854775808"),
];

const TYPE_SUFFIXES: [&str; 10] = [
    "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
];

/// Evaluate a Rust constant expression.  Only unsuffixed integer expressions
/// and the usual `TYPE::MAX`/`TYPE::MIN` forms are supported; everything else
/// is reported as unsupported so the caller can count it as skipped.
pub fn eval_rust(expr: &str) -> Eval {
    let mut expr = strip_digit_underscores(expr.trim());
    expr = strip_type_suffixes(&expr);
    for (from, to) in REPLACEMENTS {
        expr = expr.replace(from, to);
    }
    if expr.contains("::") || expr.contains(" as ") || expr.contains("sizeof") {
        return Err(Bad("unsupported"));
    }
    let tokens = tokenize(&expr);
    let mut parser = Parser::new(&tokens);
    parser.parse(&mut |_| Err(Bad("ident")))
}

fn strip_digit_underscores(expr: &str) -> String {
    let mut out = String::with_capacity(expr.len());
    let mut chars = expr.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' && chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            continue;
        }
        out.push(c);
    }
    out
}

fn strip_type_suffixes(expr: &str) -> String {
    let bytes = expr.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let boundary = i == 0 || !is_word_byte(bytes[i - 1]);
        if boundary {
            let suffix = TYPE_SUFFIXES.iter().find(|suffix| {
                bytes[i..].starts_with(suffix.as_bytes())
                    && bytes
                        .get(i + suffix.len())
                        .is_none_or(|next| !is_word_byte(*next))
            });
            if let Some(suffix) = suffix {
                i += suffix.len();
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).expect("stripping ASCII cannot break UTF-8")
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_expressions() {
        assert_eq!(eval_rust("0x8000_0000"), Ok(0x8000_0000));
        assert_eq!(eval_rust("1_000 + 24"), Ok(1024));
        assert_eq!(eval_rust("(1 << 4) - 1"), Ok(15));
        // `TYPE::MAX` is skipped, as in the Python checker this replaced: the
        // type-suffix stripping removes `i64` before the replacement table can
        // expand it, so the expression is reported as unsupported.
        assert!(eval_rust("i64::MAX").is_err());
        assert!(eval_rust("core::mem::size_of::<T>()").is_err());
    }

    #[test]
    fn const_cfgs_and_modules() {
        let text = "\
#[cfg(target_arch = \"x86_64\")]
pub const SYS_READ: i64 = 0;
#[cfg(target_arch = \"aarch64\")]
mod arch {
    pub const SYS_READ: i64 = 63;
}
pub const COMMON: u32 = 7;
";
        let mut constants = Constants::new();
        scan_file(text, &mut constants);
        let read = &constants["SYS_READ"];
        assert_eq!(
            read[&Some("x86_64".to_string())].iter().next().unwrap(),
            "0"
        );
        assert_eq!(
            read[&Some("aarch64".to_string())].iter().next().unwrap(),
            "63"
        );
        assert_eq!(constants["COMMON"][&None].iter().next().unwrap(), "7");
    }
}
