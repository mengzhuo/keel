// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// SysV IPC: ftok, msg*, sem*, shm*
// x86_64 direct syscalls: msgget=68, msgsnd=69, msgrcv=70, msgctl=71
//   semget=64, semop=65, semctl=66, semtimedop=220
//   shmget=29, shmat=30, shmctl=31, shmdt=67
// ============================================================


#[inline]
unsafe fn sys_msgget(key: c_int, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_MSGGET, key as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgsnd(msqid: c_int, msgp: *const c_void, msgsz: usize, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_MSGSND, msqid as i64, msgp as i64, msgsz as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgrcv(msqid: c_int, msgp: *mut c_void, msgsz: usize, msgtyp: c_long, msgflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall5(SYS_MSGRCV, msqid as i64, msgp as i64, msgsz as i64, msgtyp as i64, msgflg as i64)
}

#[inline]
unsafe fn sys_msgctl(msqid: c_int, cmd: c_int, buf: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_MSGCTL, msqid as i64, cmd as i64, buf as i64)
}

#[inline]
unsafe fn sys_semget(key: c_int, nsems: c_int, semflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SEMGET, key as i64, nsems as i64, semflg as i64)
}

#[cfg(target_arch = "powerpc64")]
#[inline]
unsafe fn sys_semop(semid: c_int, sops: *const c_void, nsops: usize) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_SEMTIMEDOP, semid as i64, sops as i64, nsops as i64, 0)
}

#[cfg(not(target_arch = "powerpc64"))]
#[inline]
unsafe fn sys_semop(semid: c_int, sops: *const c_void, nsops: usize) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SEMOP, semid as i64, sops as i64, nsops as i64)
}

#[inline]
unsafe fn sys_semctl(semid: c_int, semnum: c_int, cmd: c_int, arg: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_SEMCTL, semid as i64, semnum as i64, cmd as i64, arg as i64)
}

#[inline]
unsafe fn sys_shmget(key: c_int, size: usize, shmflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMGET, key as i64, size as i64, shmflg as i64)
}

#[inline]
unsafe fn sys_shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMAT, shmid as i64, shmaddr as i64, shmflg as i64)
}

#[inline]
unsafe fn sys_shmdt(shmaddr: *const c_void) -> i64 {
    <Arch as Syscalls>::syscall1(SYS_SHMDT, shmaddr as i64)
}

#[inline]
unsafe fn sys_shmctl(shmid: c_int, cmd: c_int, buf: *mut c_void) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SHMCTL, shmid as i64, cmd as i64, buf as i64)
}

// ftok
#[no_mangle]
pub unsafe extern "C" fn ftok(path: *const c_char, id: c_int) -> c_int {
    let mut st: kernel_stat64 = core::mem::zeroed();
    let r = sys_stat(path as *const u8, &mut st);
    if r < 0 {
        ERRNO!() = (-r) as c_int;
        return -1;
    }
    ((st.st_ino & 0xffff) | ((st.st_dev & 0xff) << 16) | (((id as u32 & 0xff) << 24)) as u64) as c_int
}

// msgget
#[no_mangle]
pub unsafe extern "C" fn msgget(key: c_int, msgflg: c_int) -> c_int {
    let r = sys_msgget(key, msgflg);
    syscall_ret(r) as c_int
}

// msgsnd
#[no_mangle]
pub unsafe extern "C" fn msgsnd(msqid: c_int, msgp: *const c_void, msgsz: usize, msgflg: c_int) -> c_int {
    let r = sys_msgsnd(msqid, msgp, msgsz, msgflg);
    syscall_ret(r) as c_int
}

// msgrcv
#[no_mangle]
pub unsafe extern "C" fn msgrcv(msqid: c_int, msgp: *mut c_void, msgsz: usize, msgtyp: c_long, msgflg: c_int) -> isize {
    let r = sys_msgrcv(msqid, msgp, msgsz, msgtyp, msgflg);
    if r < 0 { ERRNO!() = (-r) as c_int; -1 } else { r as isize }
}

// msgctl
#[no_mangle]
pub unsafe extern "C" fn msgctl(msqid: c_int, cmd: c_int, buf: *mut c_void) -> c_int {
    let r = sys_msgctl(msqid, cmd, buf);
    syscall_ret(r) as c_int
}

// semget
const SEM_MAX_NSEMS: c_int = 65535;

#[no_mangle]
pub unsafe extern "C" fn semget(key: c_int, nsems: c_int, semflg: c_int) -> c_int {
    if nsems > SEM_MAX_NSEMS { ERRNO!() = EINVAL; return -1; }
    let r = sys_semget(key, nsems, semflg);
    syscall_ret(r) as c_int
}

// semop
#[no_mangle]
pub unsafe extern "C" fn semop(semid: c_int, sops: *const c_void, nsops: usize) -> c_int {
    let r = sys_semop(semid, sops, nsops);
    syscall_ret(r) as c_int
}

// semctl - C variadic
#[no_mangle]
pub unsafe extern "C" fn semctl(semid: c_int, semnum: c_int, cmd: c_int, mut args: ...) -> c_int {
    let mut arg: *mut c_void = core::ptr::null_mut();
    // Commands that take the union semun argument
    match cmd {
        // SETVAL/SETALL/GETALL, IPC_SET/IPC_STAT/IPC_INFO, SEM_STAT,
        // SEM_INFO, SEM_STAT_ANY and the 32-bit compat variants all carry
        // the `union semun` argument.
        16 | 13 | 17 | 1 | 2 | 3 | 18 | 19 | 20 | 0x102 | 0x104 | 0x106 => {
            arg = args.next_arg::<*mut c_void>();
        }
        _ => {}
    }
    let r = sys_semctl(semid, semnum, cmd, arg);
    syscall_ret(r) as c_int
}

// shmget
#[no_mangle]
pub unsafe extern "C" fn shmget(key: c_int, size: usize, shmflg: c_int) -> c_int {
    let r = sys_shmget(key, size, shmflg);
    syscall_ret(r) as c_int
}

// shmat
#[no_mangle]
pub unsafe extern "C" fn shmat(shmid: c_int, shmaddr: *const c_void, shmflg: c_int) -> *mut c_void {
    let r = sys_shmat(shmid, shmaddr, shmflg);
    if (r as usize) > (-(crate::SYSCALL_ERR_MAX as isize)) as usize {
        ERRNO!() = (-r) as c_int;
        return -1isize as *mut c_void;
    }
    r as *mut c_void
}

// shmdt
#[no_mangle]
pub unsafe extern "C" fn shmdt(shmaddr: *const c_void) -> c_int {
    let r = sys_shmdt(shmaddr);
    syscall_ret(r) as c_int
}

// shmctl
#[no_mangle]
pub unsafe extern "C" fn shmctl(shmid: c_int, cmd: c_int, buf: *mut c_void) -> c_int {
    let r = sys_shmctl(shmid, cmd, buf);
    syscall_ret(r) as c_int
}

include!("startup.rs");
include!("crypt_impl.rs");
include!("statvfs.rs");
include!("daemon.rs");
include!("safety.rs");
include!("dn_expand.rs");
include!("lrand48.rs");
include!("strverscmp.rs");
include!("syscall.rs");
include!("pthread_atfork.rs");
include!("fenv.rs");
include!("locale_ctype.rs");
include!("regression_stubs.rs");
include!("ctype_tables.rs");
include!("wctype_tables.rs");
include!("netdb_extra.rs");
include!("compat_extra.rs");
include!("stdio_extra.rs");
include!("ns_parse.rs");
include!("fts.rs");
include!("ifaddrs.rs");
include!("dl_extra.rs");
include!("des.rs");
include!("path_extra.rs");
include!("wordexp.rs");
include!("getopt_impl.rs");

