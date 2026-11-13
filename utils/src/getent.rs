// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

//! `getent`: query the system databases.
//!
//! The passwd/group/hosts/services/protocols/networks/shells databases are
//! read directly from `/etc`, so the command works identically on a keel
//! system without relying on NSS modules.  Host name resolution falls back
//! to the platform resolver when `/etc/hosts` does not contain the name.

use std::ffi::OsString;
use std::fs;
use std::net::{IpAddr, ToSocketAddrs};
use std::process::ExitCode;

pub fn main(args: Vec<OsString>) -> ExitCode {
    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    if args.is_empty() {
        eprintln!("usage: getent database [key ...]");
        return ExitCode::from(1);
    }

    let mut index = 0usize;
    while index < args.len() && args[index].starts_with('-') {
        match args[index].as_str() {
            "-s" => index += 1, // separator; accepted for compatibility
            "--" => {
                index += 1;
                break;
            }
            other => {
                eprintln!("getent: unknown option: {other}");
                return ExitCode::from(1);
            }
        }
    }
    if index >= args.len() {
        eprintln!("usage: getent database [key ...]");
        return ExitCode::from(1);
    }

    let database = args[index].as_str();
    let keys = &args[index + 1..];
    match lookup(database, keys) {
        Ok(found) => {
            if found {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
        Err(err) => {
            eprintln!("getent: {err}");
            ExitCode::from(1)
        }
    }
}

fn lookup(database: &str, keys: &[String]) -> Result<bool, String> {
    match database {
        "passwd" => parse_colon_file("/etc/passwd", keys, |fields, key| {
            fields.first() == Some(&key) || fields.get(2) == Some(&key)
        }),
        "group" => parse_colon_file("/etc/group", keys, |fields, key| {
            fields.first() == Some(&key) || fields.get(2) == Some(&key)
        }),
        "services" => parse_whitespace_file("/etc/services", keys, |fields, key| {
            let name = fields.first().copied().unwrap_or("");
            let port = fields.get(1).copied().unwrap_or("");
            name == key || port == key || port.split('/').next() == Some(key)
        }),
        "protocols" => parse_whitespace_file("/etc/protocols", keys, |fields, key| {
            fields.first().copied().unwrap_or("") == key
                || fields.get(1).copied().unwrap_or("") == key
        }),
        "networks" => parse_whitespace_file("/etc/networks", keys, |fields, key| {
            fields.first().copied().unwrap_or("") == key
        }),
        "hosts" => hosts(keys),
        "ahosts" => ahosts(keys, true, true),
        "ahostsv4" => ahosts(keys, true, false),
        "ahostsv6" => ahosts(keys, false, true),
        "shadow" => parse_colon_file("/etc/shadow", keys, |fields, key| {
            fields.first() == Some(&key)
        }),
        "ethers" => {
            if keys.is_empty() {
                return Err("Enumeration not supported on ethers".into());
            }
            parse_whitespace_file("/etc/ethers", keys, |fields, key| {
                fields.first().copied().unwrap_or("") == key
                    || fields.get(1).copied().unwrap_or("") == key
            })
        }
        "shells" => shells(keys),
        "aliases" => {
            if keys.is_empty() {
                return Err("Enumeration not supported on aliases".into());
            }
            parse_colon_file("/etc/aliases", keys, |fields, key| {
                fields.first().copied().unwrap_or("") == key
            })
        }
        other => Err(format!("unknown database: {other}")),
    }
}

fn parse_colon_file<F>(path: &str, keys: &[String], matches: F) -> Result<bool, String>
where
    F: Fn(&[&str], &str) -> bool,
{
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("{path}: {err}")),
    };
    let mut found = false;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split(':').collect();
        if fields.len() < 2 {
            continue;
        }
        if keys.is_empty() || keys.iter().any(|key| matches(&fields, key)) {
            println!("{}", line);
            found = true;
        }
    }
    Ok(found)
}

fn parse_whitespace_file<F>(path: &str, keys: &[String], matches: F) -> Result<bool, String>
where
    F: Fn(&[&str], &str) -> bool,
{
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("{path}: {err}")),
    };
    let mut found = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 2 {
            continue;
        }
        if keys.is_empty() || keys.iter().any(|key| matches(&fields, key)) {
            println!("{}", line);
            found = true;
        }
    }
    Ok(found)
}

fn hosts(keys: &[String]) -> Result<bool, String> {
    let mut found = false;
    if let Ok(text) = fs::read_to_string("/etc/hosts") {
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let mut fields = line.split_whitespace();
            let Some(addr) = fields.next() else { continue };
            let names: Vec<&str> = fields.collect();
            let matched = keys.is_empty()
                || keys
                    .iter()
                    .any(|key| key == addr || names.iter().any(|name| name == key));
            if matched {
                println!("{}\t{}", addr, names.join(" "));
                found = true;
            }
        }
    }
    if found || keys.is_empty() {
        return Ok(found);
    }

    for key in keys {
        if let Ok(ip) = key.parse::<IpAddr>() {
            println!("{}\t{}", ip, key);
            found = true;
            continue;
        }
        if let Ok(addrs) = (key.as_str(), 0u16).to_socket_addrs() {
            for addr in addrs {
                println!("{}\t{}", addr.ip(), key);
                found = true;
            }
        }
    }
    Ok(found)
}

fn ahosts(keys: &[String], v4: bool, v6: bool) -> Result<bool, String> {
    let mut found = false;
    for key in keys {
        let addrs = match (key.as_str(), 0u16).to_socket_addrs() {
            Ok(addrs) => addrs,
            Err(_) => continue,
        };
        for addr in addrs {
            let is_v4 = addr.is_ipv4();
            if (is_v4 && !v4) || (!is_v4 && !v6) {
                continue;
            }
            println!("{:<15} STREAM {}", addr.ip().to_string(), key);
            println!("{:<15} DGRAM  {}", addr.ip().to_string(), key);
            found = true;
        }
    }
    Ok(found)
}

fn shells(keys: &[String]) -> Result<bool, String> {
    let text = match fs::read_to_string("/etc/shells") {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("/etc/shells: {err}")),
    };
    let mut found = false;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if keys.is_empty() || keys.iter().any(|key| key == line) {
            println!("{line}");
            found = true;
        }
    }
    Ok(found)
}
