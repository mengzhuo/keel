//! `iconv`: character-set conversion using crabc/musl's iconv(3).

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::raw::{c_char, c_int, c_void};
use std::process::ExitCode;

unsafe extern "C" {
    fn iconv_open(tocode: *const c_char, fromcode: *const c_char) -> *mut c_void;
    fn iconv(
        cd: *mut c_void,
        inbuf: *mut *mut c_char,
        inbytesleft: *mut usize,
        outbuf: *mut *mut c_char,
        outbytesleft: *mut usize,
    ) -> usize;
    fn iconv_close(cd: *mut c_void) -> c_int;
}

pub fn main(args: Vec<OsString>) -> ExitCode {
    let mut from = String::from("UTF-8");
    let mut to = String::from("UTF-8");
    let mut skip_invalid = false;
    let mut silent = false;
    let mut files = Vec::new();
    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-f" => {
                i += 1;
                if i >= args.len() {
                    return fail("iconv: -f requires an argument");
                }
                from = args[i].clone();
            }
            "-t" => {
                i += 1;
                if i >= args.len() {
                    return fail("iconv: -t requires an argument");
                }
                to = args[i].clone();
            }
            "-l" => {
                println!(
                    "UTF-8, UTF-16BE, UTF-16LE, UTF-32BE, UTF-32LE, UCS-2BE, UCS-2LE, \
                     WCHAR_T, US_ASCII, ISO8859-1, ISO8859-2, ISO8859-3, ISO8859-4, \
                     ISO8859-5, ISO8859-6, ISO8859-7, ISO8859-8, ISO8859-9, ISO8859-10, \
                     ISO8859-11, ISO8859-13, ISO8859-14, ISO8859-15, ISO8859-16"
                );
                return ExitCode::SUCCESS;
            }
            "-c" => skip_invalid = true,
            "-s" => silent = true,
            "--" => {
                i += 1;
                while i < args.len() {
                    files.push(args[i].clone());
                    i += 1;
                }
                break;
            }
            arg if arg.starts_with('-') && arg != "-" => {
                return fail(&format!("iconv: unknown option: {arg}"));
            }
            arg => files.push(arg.to_string()),
        }
        i += 1;
    }

    let from_c = match std::ffi::CString::new(from.clone()) {
        Ok(s) => s,
        Err(_) => return fail("iconv: source charset contains a NUL byte"),
    };
    let to_c = match std::ffi::CString::new(to.clone()) {
        Ok(s) => s,
        Err(_) => return fail("iconv: destination charset contains a NUL byte"),
    };
    let cd = unsafe { iconv_open(to_c.as_ptr(), from_c.as_ptr()) };
    if cd as isize == -1 {
        let err = io::Error::last_os_error();
        if !silent {
            eprintln!("iconv: {from} -> {to}: {err}");
        }
        return ExitCode::from(1);
    }

    let mut status = 0u8;
    if files.is_empty() {
        if let Err(err) = convert_stream(
            cd,
            io::stdin().lock(),
            &mut io::stdout().lock(),
            skip_invalid,
        ) {
            if !silent {
                eprintln!("iconv: (stdin): {err}");
            }
            status = 1;
        }
    } else {
        for name in files {
            let result = if name == "-" {
                convert_stream(
                    cd,
                    io::stdin().lock(),
                    &mut io::stdout().lock(),
                    skip_invalid,
                )
            } else {
                match File::open(&name) {
                    Ok(mut file) => {
                        convert_stream(cd, &mut file, &mut io::stdout().lock(), skip_invalid)
                    }
                    Err(err) => Err(err),
                }
            };
            if let Err(err) = result {
                if !silent {
                    eprintln!("iconv: {name}: {err}");
                }
                status = 1;
            }
        }
    }

    unsafe {
        iconv_close(cd);
    }
    ExitCode::from(status)
}

fn convert_stream<R: Read, W: Write>(
    cd: *mut c_void,
    mut input: R,
    output: &mut W,
    skip_invalid: bool,
) -> io::Result<()> {
    let mut pending: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = input.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        pending.extend_from_slice(&chunk[..n]);
        convert_buffer(cd, &mut pending, output, false, skip_invalid)?;
    }
    convert_buffer(cd, &mut pending, output, true, skip_invalid)?;
    Ok(())
}

fn convert_buffer<W: Write>(
    cd: *mut c_void,
    pending: &mut Vec<u8>,
    output: &mut W,
    at_eof: bool,
    skip_invalid: bool,
) -> io::Result<()> {
    let mut consumed = 0usize;
    while consumed < pending.len() {
        let mut in_ptr = pending[consumed..].as_ptr() as *mut c_char;
        let mut in_left = pending.len() - consumed;
        let mut out = vec![0u8; 32768];
        let mut out_ptr = out.as_mut_ptr() as *mut c_char;
        let mut out_left = out.len();

        let result = unsafe { iconv(cd, &mut in_ptr, &mut in_left, &mut out_ptr, &mut out_left) };
        let produced = out.len() - out_left;
        if produced > 0 {
            output.write_all(&out[..produced])?;
        }
        let used = (pending.len() - consumed) - in_left;
        consumed += used;

        if result != usize::MAX {
            continue;
        }

        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::E2BIG) => continue,
            Some(libc::EILSEQ) if skip_invalid => {
                // Skip one input byte and retry, matching iconv -c.
                if consumed < pending.len() {
                    consumed += 1;
                } else {
                    break;
                }
            }
            Some(libc::EINVAL) if at_eof => break,
            _ => return Err(err),
        }
    }
    pending.drain(..consumed);
    Ok(())
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(1)
}
