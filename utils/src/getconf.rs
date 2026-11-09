//! `getconf`: print POSIX configuration values.
//!
//! The table below covers the values exposed by musl's getconf.  Limits
//! come from `sysconf`/`confstr`/`pathconf`; fixed POSIX minimums are
//! compiled in as Rust constants.

use std::ffi::{CString, OsString};
use std::process::ExitCode;
use std::ptr;

#[derive(Clone, Copy)]
enum Kind {
    Sys(i32),
    Path(i32),
    Str(i32),
    Num(i64),
}

struct Var {
    name: &'static str,
    kind: Kind,
}

macro_rules! vars {
    ($( $name:literal => $kind:expr ),* $(,)?) => {
        &[ $( Var { name: $name, kind: $kind } ),* ]
    };
}

static VARS: &[Var] = vars![
    "PATH" => Kind::Str(libc::_CS_PATH),
    "PAGESIZE" => Kind::Sys(libc::_SC_PAGESIZE),
    "PAGE_SIZE" => Kind::Sys(libc::_SC_PAGESIZE),
    "ARG_MAX" => Kind::Sys(libc::_SC_ARG_MAX),
    "CHILD_MAX" => Kind::Sys(libc::_SC_CHILD_MAX),
    "CLK_TCK" => Kind::Sys(libc::_SC_CLK_TCK),
    "NGROUPS_MAX" => Kind::Sys(libc::_SC_NGROUPS_MAX),
    "OPEN_MAX" => Kind::Sys(libc::_SC_OPEN_MAX),
    "STREAM_MAX" => Kind::Sys(libc::_SC_STREAM_MAX),
    "TZNAME_MAX" => Kind::Sys(libc::_SC_TZNAME_MAX),
    "JOB_CONTROL" => Kind::Sys(libc::_SC_JOB_CONTROL),
    "SAVED_IDS" => Kind::Sys(libc::_SC_SAVED_IDS),
    "REALTIME_SIGNALS" => Kind::Sys(libc::_SC_REALTIME_SIGNALS),
    "PRIORITY_SCHEDULING" => Kind::Sys(libc::_SC_PRIORITY_SCHEDULING),
    "TIMERS" => Kind::Sys(libc::_SC_TIMERS),
    "ASYNCHRONOUS_IO" => Kind::Sys(libc::_SC_ASYNCHRONOUS_IO),
    "PRIORITIZED_IO" => Kind::Sys(libc::_SC_PRIORITIZED_IO),
    "SYNCHRONIZED_IO" => Kind::Sys(libc::_SC_SYNCHRONIZED_IO),
    "FSYNC" => Kind::Sys(libc::_SC_FSYNC),
    "MAPPED_FILES" => Kind::Sys(libc::_SC_MAPPED_FILES),
    "MEMLOCK" => Kind::Sys(libc::_SC_MEMLOCK),
    "MEMLOCK_RANGE" => Kind::Sys(libc::_SC_MEMLOCK_RANGE),
    "MEMORY_PROTECTION" => Kind::Sys(libc::_SC_MEMORY_PROTECTION),
    "MESSAGE_PASSING" => Kind::Sys(libc::_SC_MESSAGE_PASSING),
    "SEMAPHORES" => Kind::Sys(libc::_SC_SEMAPHORES),
    "SHARED_MEMORY_OBJECTS" => Kind::Sys(libc::_SC_SHARED_MEMORY_OBJECTS),
    "AIO_LISTIO_MAX" => Kind::Sys(libc::_SC_AIO_LISTIO_MAX),
    "AIO_MAX" => Kind::Sys(libc::_SC_AIO_MAX),
    "AIO_PRIO_DELTA_MAX" => Kind::Sys(libc::_SC_AIO_PRIO_DELTA_MAX),
    "DELAYTIMER_MAX" => Kind::Sys(libc::_SC_DELAYTIMER_MAX),
    "MQ_OPEN_MAX" => Kind::Sys(libc::_SC_MQ_OPEN_MAX),
    "MQ_PRIO_MAX" => Kind::Sys(libc::_SC_MQ_PRIO_MAX),
    "VERSION" => Kind::Sys(libc::_SC_VERSION),
    "RTSIG_MAX" => Kind::Sys(libc::_SC_RTSIG_MAX),
    "SEM_NSEMS_MAX" => Kind::Sys(libc::_SC_SEM_NSEMS_MAX),
    "SEM_VALUE_MAX" => Kind::Sys(libc::_SC_SEM_VALUE_MAX),
    "SIGQUEUE_MAX" => Kind::Sys(libc::_SC_SIGQUEUE_MAX),
    "TIMER_MAX" => Kind::Sys(libc::_SC_TIMER_MAX),
    "IOV_MAX" => Kind::Sys(libc::_SC_IOV_MAX),
    "LOGIN_NAME_MAX" => Kind::Sys(libc::_SC_LOGIN_NAME_MAX),
    "TTY_NAME_MAX" => Kind::Sys(libc::_SC_TTY_NAME_MAX),
    "HOST_NAME_MAX" => Kind::Sys(libc::_SC_HOST_NAME_MAX),
    "SYMLOOP_MAX" => Kind::Sys(libc::_SC_SYMLOOP_MAX),
    "NPROCESSORS_CONF" => Kind::Sys(libc::_SC_NPROCESSORS_CONF),
    "NPROCESSORS_ONLN" => Kind::Sys(libc::_SC_NPROCESSORS_ONLN),
    "PHYS_PAGES" => Kind::Sys(libc::_SC_PHYS_PAGES),
    "AVPHYS_PAGES" => Kind::Sys(libc::_SC_AVPHYS_PAGES),
    "ATEXIT_MAX" => Kind::Sys(libc::_SC_ATEXIT_MAX),
    "THREAD_DESTRUCTOR_ITERATIONS" => Kind::Sys(libc::_SC_THREAD_DESTRUCTOR_ITERATIONS),
    "THREAD_KEYS_MAX" => Kind::Sys(libc::_SC_THREAD_KEYS_MAX),
    "THREAD_STACK_MIN" => Kind::Sys(libc::_SC_THREAD_STACK_MIN),
    "THREAD_THREADS_MAX" => Kind::Sys(libc::_SC_THREAD_THREADS_MAX),
    "XOPEN_VERSION" => Kind::Sys(libc::_SC_XOPEN_VERSION),
    "POSIX_VERSION" => Kind::Sys(libc::_SC_VERSION),
    "POSIX2_VERSION" => Kind::Num(200809),
    "POSIX2_BC_BASE_MAX" => Kind::Num(99),
    "POSIX2_BC_DIM_MAX" => Kind::Num(2048),
    "POSIX2_BC_SCALE_MAX" => Kind::Num(99),
    "POSIX2_BC_STRING_MAX" => Kind::Num(1000),
    "POSIX2_COLL_WEIGHTS_MAX" => Kind::Num(2),
    "POSIX2_EXPR_NEST_MAX" => Kind::Num(32),
    "POSIX2_LINE_MAX" => Kind::Num(2048),
    "POSIX2_RE_DUP_MAX" => Kind::Num(255),
    "POSIX2_CHAR_TERM" => Kind::Num(200809),
    "POSIX2_C_BIND" => Kind::Num(200809),
    "POSIX2_C_DEV" => Kind::Num(200809),
    "POSIX2_C_VERSION" => Kind::Num(200809),
    "POSIX2_FORT_DEV" => Kind::Num(-1),
    "POSIX2_FORT_RUN" => Kind::Num(-1),
    "POSIX2_LOCALEDEF" => Kind::Num(200809),
    "POSIX2_SW_DEV" => Kind::Num(200809),
    "POSIX2_UPE" => Kind::Num(200809),
    "PATH_MAX" => Kind::Path(libc::_PC_PATH_MAX),
    "LINK_MAX" => Kind::Path(libc::_PC_LINK_MAX),
    "MAX_CANON" => Kind::Path(libc::_PC_MAX_CANON),
    "MAX_INPUT" => Kind::Path(libc::_PC_MAX_INPUT),
    "NAME_MAX" => Kind::Path(libc::_PC_NAME_MAX),
    "PIPE_BUF" => Kind::Path(libc::_PC_PIPE_BUF),
    "CHOWN_RESTRICTED" => Kind::Path(libc::_PC_CHOWN_RESTRICTED),
    "NO_TRUNC" => Kind::Path(libc::_PC_NO_TRUNC),
    "VDISABLE" => Kind::Path(libc::_PC_VDISABLE),
    "_POSIX_ARG_MAX" => Kind::Num(4096),
    "_POSIX_CHILD_MAX" => Kind::Num(25),
    "_POSIX_LINK_MAX" => Kind::Num(8),
    "_POSIX_MAX_CANON" => Kind::Num(255),
    "_POSIX_MAX_INPUT" => Kind::Num(255),
    "_POSIX_NAME_MAX" => Kind::Num(14),
    "_POSIX_NGROUPS_MAX" => Kind::Num(8),
    "_POSIX_OPEN_MAX" => Kind::Num(20),
    "_POSIX_PATH_MAX" => Kind::Num(256),
    "_POSIX_PIPE_BUF" => Kind::Num(512),
    "_POSIX_RE_DUP_MAX" => Kind::Num(255),
    "_POSIX_SSIZE_MAX" => Kind::Num(32767),
    "_POSIX_STREAM_MAX" => Kind::Num(8),
    "_POSIX_TZNAME_MAX" => Kind::Num(6),
    "_POSIX_VERSION" => Kind::Num(200809),
    "_POSIX2_BC_BASE_MAX" => Kind::Num(99),
    "_POSIX2_BC_DIM_MAX" => Kind::Num(2048),
    "_POSIX2_BC_SCALE_MAX" => Kind::Num(99),
    "_POSIX2_BC_STRING_MAX" => Kind::Num(1000),
    "_POSIX2_COLL_WEIGHTS_MAX" => Kind::Num(2),
    "_POSIX2_EXPR_NEST_MAX" => Kind::Num(32),
    "_POSIX2_LINE_MAX" => Kind::Num(2048),
    "_POSIX2_RE_DUP_MAX" => Kind::Num(255),
];

pub fn main(args: Vec<OsString>) -> ExitCode {
    let mut all = false;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;
    let mut i = 0usize;
    let args: Vec<String> = args
        .into_iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    while i < args.len() {
        match args[i].as_str() {
            "-a" => all = true,
            "-v" => {
                i += 1; // specification name; the supported set is the default
                if i >= args.len() {
                    return fail("getconf: -v requires a specification");
                }
            }
            "--" => {
                i += 1;
                while i < args.len() {
                    if name.is_none() {
                        name = Some(args[i].clone());
                    } else if path.is_none() {
                        path = Some(args[i].clone());
                    }
                    i += 1;
                }
                break;
            }
            arg if arg.starts_with('-') && arg != "-" => {
                return fail(&format!("getconf: unknown option: {arg}"));
            }
            arg => {
                if name.is_none() {
                    name = Some(arg.to_string());
                } else if path.is_none() {
                    path = Some(arg.to_string());
                }
            }
        }
        i += 1;
    }

    if all {
        for var in VARS {
            if let Kind::Path(_) = var.kind {
                continue;
            }
            match query(var, None) {
                Ok(Some(value)) => println!("{} = {}", var.name, value),
                Ok(None) => {}
                Err(err) => {
                    eprintln!("getconf: {}: {err}", var.name);
                }
            }
        }
        return ExitCode::SUCCESS;
    }

    let Some(name) = name else {
        return fail("usage: getconf [-a] name [path]");
    };
    let Some(var) = VARS.iter().find(|v| v.name == name) else {
        return fail(&format!("getconf: {name}: unknown variable"));
    };

    match query(var, path.as_deref()) {
        Ok(Some(value)) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Ok(None) => {
            println!("undefined");
            ExitCode::SUCCESS
        }
        Err(err) => fail(&format!("getconf: {name}: {err}")),
    }
}

fn query(var: &Var, path: Option<&str>) -> Result<Option<String>, String> {
    match var.kind {
        Kind::Sys(id) => {
            let value = unsafe { libc::sysconf(id) };
            Ok(format_number(value))
        }
        Kind::Num(value) => Ok(format_number(value)),
        Kind::Path(id) => {
            let Some(path) = path else {
                return Err("invalid variable type".into());
            };
            let cpath = CString::new(path).map_err(|_| "path contains a NUL byte".to_string())?;
            let value = unsafe { libc::pathconf(cpath.as_ptr(), id) };
            Ok(format_number(value))
        }
        Kind::Str(id) => {
            let needed = unsafe { libc::confstr(id, ptr::null_mut(), 0) };
            if needed == 0 {
                return Ok(None);
            }
            let mut buf = vec![0u8; needed];
            let written =
                unsafe { libc::confstr(id, buf.as_mut_ptr() as *mut libc::c_char, buf.len()) };
            if written == 0 {
                return Ok(None);
            }
            let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
            Ok(Some(String::from_utf8_lossy(&buf[..end]).into_owned()))
        }
    }
}

fn format_number(value: i64) -> Option<String> {
    if value < 0 {
        None
    } else {
        Some(value.to_string())
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::from(1)
}
