// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// ============================================================
// sys/socket.h
// ============================================================

pub const AF_UNIX: c_int = 1;
pub const AF_INET: c_int = 2;
pub const AF_INET6: c_int = 10;

pub const SOCK_STREAM: c_int = 1;
pub const SOCK_DGRAM: c_int = 2;

pub const SHUT_RD: c_int = 0;
pub const SHUT_WR: c_int = 1;
pub const SHUT_RDWR: c_int = 2;

pub const SOL_SOCKET: c_int = 1;
pub const SO_REUSEADDR: c_int = 2;

#[repr(C)]
pub struct sockaddr {
    pub sa_family: u16,
    pub sa_data: [u8; 14],
}

#[inline]
unsafe fn sys_socket(domain: c_int, ty: c_int, protocol: c_int) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_SOCKET, domain as i64, ty as i64, protocol as i64)
}

#[inline]
unsafe fn sys_socketpair(domain: c_int, ty: c_int, protocol: c_int, sv: *mut c_int) -> i64 {
    <Arch as Syscalls>::syscall4(SYS_SOCKETPAIR, domain as i64, ty as i64, protocol as i64, sv as i64)
}

#[inline]
unsafe fn sys_bind(fd: c_int, addr: *const sockaddr, len: c_uint) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_BIND, fd as i64, addr as i64, len as i64)
}

#[inline]
unsafe fn sys_listen(fd: c_int, backlog: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_LISTEN, fd as i64, backlog as i64)
}

#[inline]
unsafe fn sys_accept(fd: c_int, addr: *mut sockaddr, len: *mut c_uint) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_ACCEPT, fd as i64, addr as i64, len as i64)
}

#[inline]
unsafe fn sys_connect(fd: c_int, addr: *const sockaddr, len: c_uint) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_CONNECT, fd as i64, addr as i64, len as i64)
}

#[inline]
unsafe fn sys_sendto(fd: c_int,
    buf: *const c_void,
    len: usize,
    flags: c_int,
    addr: *const sockaddr,
    addrlen: c_uint,) -> i64 {
    <Arch as Syscalls>::syscall6(SYS_SENDTO, fd as i64, buf as i64, len as i64, flags as i64, addr as i64, addrlen as i64)
}

#[inline]
unsafe fn sys_recvfrom(fd: c_int,
    buf: *mut c_void,
    len: usize,
    flags: c_int,
    addr: *mut sockaddr,
    addrlen: *mut c_uint,) -> i64 {
    <Arch as Syscalls>::syscall6(SYS_RECVFROM, fd as i64, buf as i64, len as i64, flags as i64, addr as i64, addrlen as i64)
}

#[inline]
unsafe fn sys_shutdown(fd: c_int, how: c_int) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SHUTDOWN, fd as i64, how as i64)
}

#[inline]
unsafe fn sys_setsockopt(fd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: c_uint,) -> i64 {
    <Arch as Syscalls>::syscall5(SYS_SETSOCKOPT, fd as i64, level as i64, optname as i64, optval as i64, optlen as i64)
}

#[inline]
unsafe fn sys_getsockname(fd: c_int, addr: *mut sockaddr, len: *mut c_uint) -> i64 {
    <Arch as Syscalls>::syscall3(SYS_GETSOCKNAME, fd as i64, addr as i64, len as i64)
}

#[no_mangle]
pub unsafe extern "C" fn socket(domain: c_int, ty: c_int, protocol: c_int) -> c_int {
    let r = sys_socket(domain, ty, protocol);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn socketpair(domain: c_int, ty: c_int, protocol: c_int, sv: *mut c_int) -> c_int {
    let r = sys_socketpair(domain, ty, protocol, sv);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn bind(fd: c_int, addr: *const sockaddr, len: c_uint) -> c_int {
    let r = sys_bind(fd, addr, len);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn listen(fd: c_int, backlog: c_int) -> c_int {
    let r = sys_listen(fd, backlog);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn accept(fd: c_int, addr: *mut sockaddr, len: *mut c_uint) -> c_int {
    let r = sys_accept(fd, addr, len);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn connect(fd: c_int, addr: *const sockaddr, len: c_uint) -> c_int {
    let r = sys_connect(fd, addr, len);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn send(fd: c_int, buf: *const c_void, len: usize, flags: c_int) -> isize {
    let r = sys_sendto(fd, buf, len, flags, core::ptr::null(), 0);
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn recv(fd: c_int, buf: *mut c_void, len: usize, flags: c_int) -> isize {
    let r = sys_recvfrom(fd, buf, len, flags, core::ptr::null_mut(), core::ptr::null_mut());
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn sendto(
    fd: c_int,
    buf: *const c_void,
    len: usize,
    flags: c_int,
    addr: *const sockaddr,
    addrlen: c_uint,
) -> isize {
    let r = sys_sendto(fd, buf, len, flags, addr, addrlen);
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn recvfrom(
    fd: c_int,
    buf: *mut c_void,
    len: usize,
    flags: c_int,
    addr: *mut sockaddr,
    addrlen: *mut c_uint,
) -> isize {
    let r = sys_recvfrom(fd, buf, len, flags, addr, addrlen);
    if r < 0 { ERRNO!() = (-r) as c_int; return -1; }
    r as isize
}

#[no_mangle]
pub unsafe extern "C" fn shutdown(fd: c_int, how: c_int) -> c_int {
    let r = sys_shutdown(fd, how);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn setsockopt(
    fd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: c_uint,
) -> c_int {
    let r = sys_setsockopt(fd, level, optname, optval, optlen);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub unsafe extern "C" fn getsockname(fd: c_int, addr: *mut sockaddr, len: *mut c_uint) -> c_int {
    let r = sys_getsockname(fd, addr, len);
    syscall_ret(r) as c_int
}

#[no_mangle]
pub extern "C" fn htonl(hostlong: c_uint) -> c_uint {
    hostlong.to_be()
}

#[no_mangle]
pub extern "C" fn ntohl(netlong: c_uint) -> c_uint {
    c_uint::from_be(netlong)
}

#[no_mangle]
pub extern "C" fn htons(hostshort: u16) -> u16 {
    hostshort.to_be()
}

#[no_mangle]
pub extern "C" fn ntohs(netshort: u16) -> u16 {
    u16::from_be(netshort)
}

pub type PthreadT = c_ulong;

// pthread_attr_t: musl 64-bit layout (56 bytes = int[14])
#[repr(C)]
pub struct pthread_attr_t {
    __i: [c_int; 14],
}

// pthread_mutex_t: musl 64-bit layout (40 bytes = int[10])
// _m_type=__i[0], _m_lock=__i[1], _m_waiters=__i[2], _m_count=__i[5]
#[repr(C)]
pub struct pthread_mutex_t {
    __i: [c_int; 10],
}

#[repr(C)]
pub struct pthread_mutexattr_t {
    __attr: c_uint,
}

// pthread_cond_t: musl 64-bit layout (48 bytes = int[12])
// _c_seq=__i[2], _c_waiters=__i[3], _c_clock=__i[4]
#[repr(C)]
pub struct pthread_cond_t {
    __i: [c_int; 12],
}

#[repr(C)]
pub struct pthread_condattr_t {
    __attr: c_uint,
}

// pthread_rwlock_t: musl 64-bit layout (56 bytes = int[14])
// _rw_lock=__i[0], _rw_waiters=__i[1], _rw_shared=__i[2]
#[repr(C)]
pub struct pthread_rwlock_t {
    __i: [c_int; 14],
}

#[repr(C)]
pub struct pthread_rwlockattr_t {
    __attr: [c_uint; 2],
}

// pthread_barrier_t: musl 64-bit layout (32 bytes = int[8])
#[repr(C)]
pub struct pthread_barrier_t {
    __i: [c_int; 8],
}

#[repr(C)]
pub struct pthread_barrierattr_t {
    __attr: c_uint,
}

pub type pthread_spinlock_t = c_int;
pub type pthread_once_t = c_int;
pub type pthread_key_t = c_uint;

// sem_t: musl 64-bit layout (32 bytes = int[8])
#[repr(C)]
pub struct sem_t {
    __val: [c_int; 8],
}

#[repr(C)]
pub struct sched_param {
    pub sched_priority: c_int,
}

const CLONE_VM: c_ulong = 0x00000100;
const CLONE_FS: c_ulong = 0x00000200;
const CLONE_FILES: c_ulong = 0x00000400;
const CLONE_SIGHAND: c_ulong = 0x00000800;
const CLONE_THREAD: c_ulong = 0x00010000;
const CLONE_SYSVSEM: c_ulong = 0x00040000;
const CLONE_SETTLS: c_ulong = 0x00080000;
const CLONE_PARENT_SETTID: c_ulong = 0x00100000;
const CLONE_CHILD_CLEARTID: c_ulong = 0x00200000;

const FUTEX_WAIT: c_int = 0;
const FUTEX_WAKE: c_int = 1;
const FUTEX_LOCK_PI: c_int = 6;
const FUTEX_UNLOCK_PI: c_int = 7;
const FUTEX_TRYLOCK_PI: c_int = 8;

// SIGCANCEL (32) is musl's internal cancel signal
const SIGCANCEL: c_int = 32;

const PTHREAD_MUTEX_NORMAL: c_int = 0;
const PTHREAD_MUTEX_DEFAULT: c_int = 0;
const PTHREAD_MUTEX_RECURSIVE: c_int = 1;
const PTHREAD_MUTEX_ERRORCHECK: c_int = 2;

const PTHREAD_CREATE_JOINABLE: c_int = 0;
const PTHREAD_CREATE_DETACHED: c_int = 1;

const PTHREAD_CANCEL_ENABLE: c_int = 0;
const PTHREAD_CANCEL_DISABLE: c_int = 1;
const PTHREAD_CANCEL_DEFERRED: c_int = 0;
const PTHREAD_CANCEL_ASYNCHRONOUS: c_int = 1;

const PTHREAD_SCOPE_SYSTEM: c_int = 0;
const PTHREAD_PROCESS_PRIVATE: c_int = 0;
const PTHREAD_PROCESS_SHARED: c_int = 1;
const PTHREAD_INHERIT_SCHED: c_int = 0;
const PTHREAD_EXPLICIT_SCHED: c_int = 1;

const PTHREAD_BARRIER_SERIAL_THREAD: c_int = -1;

const PTHREAD_KEYS_MAX: usize = 128;
const PTHREAD_DESTRUCTOR_ITERATIONS: usize = 4;
const SEM_VALUE_MAX: c_int = 0x7fffffff;

const DT_EXITED: c_int = 0;
const DT_EXITING: c_int = 1;
const DT_JOINABLE: c_int = 2;
const DT_DETACHED: c_int = 3;

// musl __ptcb cleanup handler node
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __ptcb {
    __f: Option<unsafe extern "C" fn(*mut c_void)>,
    __x: *mut c_void,
    __next: *mut __ptcb,
}

// robust_list_head for kernel robust mutex support
#[repr(C)]
#[derive(Copy, Clone)]
struct robust_list {
    next: *mut robust_list,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct robust_list_head {
    list: robust_list,
    futex_offset: c_long,
    pending: *mut robust_list,
}

// Adapted from musl's x86_64 clone.s.
#[cfg(all(target_arch = "x86_64", not(test)))]
core::arch::global_asm!(
    ".global __keel_clone",
    ".type __keel_clone, @function",
    "__keel_clone:",
    "mov rax, rdi",
    "mov rdi, rdx",
    "mov rdx, r8",
    "mov r10, [rsp + 8]",
    "mov r8, r9",
    "mov r9, rax",
    "and rsi, -16",
    "sub rsi, 8",
    "mov [rsi], rcx",
    "mov eax, 56",
    "syscall",
    "test rax, rax",
    "jnz 1f",
    "pop rdi",
    "call r9",
    // A clone() child that returns from its function must exit with that
    // value (musl does the same); falling through to hlt raises SIGSEGV.
    "mov edi, eax",
    "mov eax, 60",
    "syscall",
    "hlt",
    "1:",
    "ret",
);

// Adapted from musl's aarch64 clone.s.
#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "aarch64")]
pub unsafe extern "C" fn __keel_clone(
    fn_: usize,
    stack: *mut u8,
    flags: c_ulong,
    arg: *mut c_void,
    ptid: *mut c_int,
    tls: c_ulong,
    ctid: *mut c_int,
) -> i64 {
    core::arch::naked_asm!(
        "and x1, x1, #-16",
        "stp x0, x3, [x1, #-16]!",
        "uxtw x0, w2",
        "mov x2, x4",
        "mov x3, x5",
        "mov x4, x6",
        "mov x8, #220",
        "svc #0",
        "cbz x0, 1f",
        "ret",
        "1:",
        "mov x29, #0",
        "ldp x1, x0, [sp], #16",
        "blr x1",
        "mov x8, #93",
        "svc #0",
    );
}

#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "riscv64")]
pub unsafe extern "C" fn __keel_clone(
    fn_: usize,
    stack: *mut u8,
    flags: c_ulong,
    arg: *mut c_void,
    ptid: *mut c_int,
    tls: c_ulong,
    ctid: *mut c_int,
) -> i64 {
    core::arch::naked_asm!(
        "andi a1, a1, -16",
        "addi a1, a1, -16",
        "sd a0, 0(a1)",
        "sd a3, 8(a1)",
        "mv a0, a2",
        "mv a2, a4",
        "mv a3, a5",
        "mv a4, a6",
        "li a7, 220",
        "ecall",
        "beqz a0, 1f",
        "ret",
        "1:",
        "ld a1, 0(sp)",
        "ld a0, 8(sp)",
        "jalr a1",
        "li a7, 93",
        "ecall",
    );
}

// Adapted from musl's powerpc64 clone.s.
#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "powerpc64")]
pub unsafe extern "C" fn __keel_clone(
    fn_: usize,
    stack: *mut u8,
    flags: c_ulong,
    arg: *mut c_void,
    ptid: *mut c_int,
    tls: c_ulong,
    ctid: *mut c_int,
) -> i64 {
    core::arch::naked_asm!(
        "clrrdi 4, 4, 4",
        "li 0, 0",
        "stdu 0, -32(4)",
        "std 3, 8(4)",
        "std 6, 16(4)",
        "mr 3, 5",
        "mr 5, 7",
        "mr 6, 8",
        "mr 7, 9",
        "li 0, 120",
        "sc",
        "bns+ 1f",
        "neg 3, 3",
        "1:",
        "cmpwi cr7, 3, 0",
        "bnelr cr7",
        "ld 3, 16(1)",
        "ld 12, 8(1)",
        "mtctr 12",
        "bctrl",
        "li 0, 1",
        "sc",
    );
}

// Adapted from musl's loongarch64 clone.s.
#[no_mangle]
#[unsafe(naked)]
#[cfg(target_arch = "loongarch64")]
pub unsafe extern "C" fn __keel_clone(
    fn_: usize,
    stack: *mut u8,
    flags: c_ulong,
    arg: *mut c_void,
    ptid: *mut c_int,
    tls: c_ulong,
    ctid: *mut c_int,
) -> i64 {
    core::arch::naked_asm!(
        "bstrins.d $a1, $zero, 3, 0",
        "addi.d $a1, $a1, -16",
        "st.d $a0, $a1, 0",
        "st.d $a3, $a1, 8",
        "or $a0, $a2, $zero",
        "or $a2, $a4, $zero",
        "or $a3, $a6, $zero",
        "or $a4, $a5, $zero",
        "ori $a7, $zero, 220",
        "syscall 0",
        "beqz $a0, 1f",
        "jirl $zero, $ra, 0",
        "1:",
        "move $fp, $zero",
        "ld.d $t8, $sp, 0",
        "ld.d $a0, $sp, 8",
        "jirl $ra, $t8, 0",
        "ori $a7, $zero, 93",
        "syscall 0",
    );
}

#[cfg(all(target_arch = "x86_64", test))]
#[inline(never)]
unsafe fn __keel_clone(
    _fn_: usize,
    _stack: *mut u8,
    _flags: c_ulong,
    _arg: *mut c_void,
    _ptid: *mut c_int,
    _tls: c_ulong,
    _ctid: *mut c_int,
) -> i64 {
    -1
}

#[cfg(all(target_arch = "x86_64", not(test)))]
extern "C" {
    fn __keel_clone(
        fn_: usize,
        stack: *mut u8,
        flags: c_ulong,
        arg: *mut c_void,
        ptid: *mut c_int,
        tls: c_ulong,
        ctid: *mut c_int,
    ) -> i64;
}

#[derive(Copy, Clone)]
struct TlsHooks {
    create: unsafe extern "C" fn() -> *mut u8,
    init: unsafe extern "C" fn(*mut u8) -> *mut u8,
    block_size: extern "C" fn() -> usize,
    base_offset: extern "C" fn() -> usize,
}

// Installed by the dynamic loader before any thread is created. A static
// executable leaves this as None and uses the weak fallbacks.
static mut TLS_HOOKS: Option<TlsHooks> = None;

/// Called by the dynamic loader once it is self-relocated.  A static
/// executable does not call this and keeps the fallback behaviour.
#[no_mangle]
pub unsafe extern "C" fn __keel_set_thread_tls_hooks(
    create: usize,
    init: usize,
    block_size: usize,
    base_offset: usize,
) {
    TLS_HOOKS = Some(TlsHooks {
        create: core::mem::transmute(create),
        init: core::mem::transmute(init),
        block_size: core::mem::transmute(block_size),
        base_offset: core::mem::transmute(base_offset),
    });
}

#[inline(never)]
#[no_mangle]
pub extern "C" fn __keel_create_thread_tls() -> *mut u8 {
    match unsafe { TLS_HOOKS } {
        Some(hooks) => unsafe { (hooks.create)() },
        None => core::ptr::null_mut(),
    }
}

#[inline(never)]
#[no_mangle]
pub unsafe extern "C" fn __keel_init_thread_tls_at(block: *mut u8) -> *mut u8 {
    match TLS_HOOKS {
        Some(hooks) => (hooks.init)(block),
        None => core::ptr::null_mut(),
    }
}

#[inline(never)]
#[no_mangle]
pub extern "C" fn __keel_tls_block_size() -> usize {
    match unsafe { TLS_HOOKS } {
        Some(hooks) => (hooks.block_size)(),
        None => 0,
    }
}

#[inline(never)]
#[no_mangle]
pub extern "C" fn __keel_tls_base_offset() -> usize {
    match unsafe { TLS_HOOKS } {
        Some(hooks) => (hooks.base_offset)(),
        None => 0,
    }
}

#[inline]
unsafe fn sys_gettid() -> i64 {
    <Arch as Syscalls>::syscall0(SYS_GETTID)
}

#[inline]
unsafe fn sys_futex(uaddr: *mut c_int,
    futex_op: c_int,
    val: c_int,
    timeout: *mut c_void,
    uaddr2: *mut c_int,
    val3: c_int,) -> i64 {
    <Arch as Syscalls>::syscall6(SYS_FUTEX, uaddr as i64, futex_op as i64, val as i64, timeout as i64, uaddr2 as i64, val3 as i64)
}

const THREAD_CHUNK: usize = 1024;
const STACK_SIZE: usize = 1024 * 1024;
// Per-thread pointer to its Thread record.  The TCB has room after errno
// (0x30), the vDSO getrandom state (0x38), and this pointer (0x40).
const THREAD_TCB_OFFSET: usize = 0x40;

#[repr(C)]
#[derive(Copy, Clone)]
struct Thread {
    tid: c_int,
    detach_state: c_int,
    result: *mut c_void,
    cancel: c_int,
    cancel_state: c_int,
    cancel_type: c_int,
    user_fn: usize,
    user_arg: *mut c_void,
    stack: *mut u8,
    stack_size: usize,
    stack_owned: bool,
    fs_base: *mut u8,
    // TLS block as allocated at creation. The global TLS size can grow when
    // a DSO is dlopen'ed, so it must not be used to free an older thread's
    // block: the address and length would be wrong and could unmap a
    // neighbouring thread stack.
    tls_base: *mut u8,
    tls_size: usize,
    tsd: [*mut c_void; PTHREAD_KEYS_MAX],
    cancelbuf: *mut __ptcb,
    robust_list: robust_list_head,
    free_next: *mut Thread,
}

static mut THREADS: [Thread; THREAD_CHUNK] = [Thread {
    tid: -1,
    detach_state: DT_JOINABLE,
    result: core::ptr::null_mut(),
    cancel: 0,
    cancel_state: PTHREAD_CANCEL_ENABLE,
    cancel_type: PTHREAD_CANCEL_DEFERRED,
    user_fn: 0,
    user_arg: core::ptr::null_mut(),
    stack: core::ptr::null_mut(),
    stack_size: 0,
    stack_owned: false,
    fs_base: core::ptr::null_mut(),
    tls_base: core::ptr::null_mut(),
    tls_size: 0,
    tsd: [core::ptr::null_mut(); PTHREAD_KEYS_MAX],
    cancelbuf: core::ptr::null_mut(),
    robust_list: robust_list_head {
        list: robust_list { next: core::ptr::null_mut() },
        futex_offset: 0,
        pending: core::ptr::null_mut(),
    },
    free_next: core::ptr::null_mut(),
}; THREAD_CHUNK];
#[repr(C)]
struct ThreadChunk {
    next: *mut ThreadChunk,
    base: *mut Thread,
}
static THREAD_CHUNK_LIST: AtomicPtr<ThreadChunk> =
    AtomicPtr::new(core::ptr::null_mut());
static EXTRA_CHUNK_COUNT: AtomicUsize = AtomicUsize::new(0);
// Free list of thread slots, protected by THREAD_LOCK.  Slots are never
// moved, so pthread_t values stay valid for the life of a thread.
static mut FREE_THREADS: *mut Thread = core::ptr::null_mut();
static mut THREAD_TABLE_READY: bool = false;
static mut THREAD_LOCK: c_int = 0;
static mut THREAD_WAITERS: c_int = 0;
static ACTIVE_THREADS: AtomicUsize = AtomicUsize::new(0);
static mut KEY_DTORS: [Option<unsafe extern "C" fn(*mut c_void)>; PTHREAD_KEYS_MAX] = [None; PTHREAD_KEYS_MAX];
static NEXT_KEY: AtomicUsize = AtomicUsize::new(0);

// futex helpers
unsafe fn futex_wait(addr: *mut c_int, expected: c_int) -> c_int {
    let r = sys_futex(addr, FUTEX_WAIT, expected, null_mut(), null_mut(), 0);
    if r < 0 { let e = (-r) as c_int; if e == EAGAIN { 0 } else { e } } else { 0 }
}

unsafe fn futex_wake(addr: *mut c_int, count: c_int) {
    let c = if count < 0 { c_int::MAX } else { count };
    sys_futex(addr, FUTEX_WAKE, c, null_mut(), null_mut(), 0);
}

unsafe fn futex_lock_pi(addr: *mut c_int, abs_timeout: *const timespec) -> c_int {
    let timeout_ptr = if abs_timeout.is_null() { null_mut() } else { abs_timeout as *mut c_void };
    let r = sys_futex(addr, FUTEX_LOCK_PI, 0, timeout_ptr, null_mut(), 0);
    if r < 0 { (-r) as c_int } else { 0 }
}

unsafe fn futex_unlock_pi(addr: *mut c_int) -> c_int {
    let r = sys_futex(addr, FUTEX_UNLOCK_PI, 0, null_mut(), null_mut(), 0);
    if r < 0 { (-r) as c_int } else { 0 }
}

#[inline]
unsafe fn sys_set_robust_list(head: *mut robust_list_head, len: usize) -> i64 {
    <Arch as Syscalls>::syscall2(SYS_SET_ROBUST_LIST, head as i64, len as i64)
}

unsafe fn futex_timedwait(addr: *mut c_int, expected: c_int, abs_timeout: *const timespec) -> c_int {
    if abs_timeout.is_null() { return futex_wait(addr, expected); }
    let mut now: timespec = core::mem::zeroed();
    if sys_clock_gettime(CLOCK_REALTIME, &mut now) < 0 { return EINVAL; }
    let mut rel: timespec = core::mem::zeroed();
    rel.tv_sec = (*abs_timeout).tv_sec - now.tv_sec;
    rel.tv_nsec = (*abs_timeout).tv_nsec - now.tv_nsec;
    if rel.tv_nsec < 0 { rel.tv_sec -= 1; rel.tv_nsec += 1_000_000_000; }
    if rel.tv_sec < 0 { return ETIMEDOUT; }
    let r = sys_futex(addr, FUTEX_WAIT, expected, &mut rel as *mut timespec as *mut c_void, null_mut(), 0);
    if r < 0 { let e = (-r) as c_int; if e == EAGAIN { 0 } else { e } } else { 0 }
}

// atomic helpers wrapping AtomicI32
unsafe fn a_cas(addr: *mut c_int, expected: c_int, desired: c_int) -> c_int {
    let a = &*(addr as *const AtomicI32);
    match a.compare_exchange(expected, desired, Ordering::AcqRel, Ordering::Acquire) {
        Ok(v) | Err(v) => v,
    }
}

unsafe fn a_store(addr: *mut c_int, val: c_int) {
    (*(addr as *const AtomicI32)).store(val, Ordering::Release);
}

unsafe fn a_load(addr: *const c_int) -> c_int {
    (*(addr as *const AtomicI32)).load(Ordering::Acquire)
}

unsafe fn a_swap(addr: *mut c_int, val: c_int) -> c_int {
    (*(addr as *const AtomicI32)).swap(val, Ordering::AcqRel)
}

unsafe fn a_fetch_add(addr: *mut c_int, val: c_int) -> c_int {
    (*(addr as *const AtomicI32)).fetch_add(val, Ordering::AcqRel)
}

unsafe fn a_fetch_sub(addr: *mut c_int, val: c_int) -> c_int {
    (*(addr as *const AtomicI32)).fetch_sub(val, Ordering::AcqRel)
}

unsafe fn spinlock_lock(lock: *mut c_int, waiters: *mut c_int) {
    while a_swap(lock, 1) != 0 {
        let mut spins = 100;
        while spins > 0 && a_load(lock) != 0 { core::hint::spin_loop(); spins -= 1; }
        if a_load(lock) == 0 { continue; }
        a_fetch_add(waiters, 1);
        futex_wait(lock, 1);
        a_fetch_sub(waiters, 1);
    }
}

unsafe fn spinlock_unlock(lock: *mut c_int, waiters: *mut c_int) {
    a_store(lock, 0);
    if a_load(waiters) > 0 { futex_wake(lock, 1); }
}

unsafe fn thread_slot_count() -> usize {
    THREAD_CHUNK + EXTRA_CHUNK_COUNT.load(Ordering::Acquire) * THREAD_CHUNK
}

unsafe fn thread_slot_at(index: usize) -> *mut Thread {
    if index < THREAD_CHUNK {
        core::ptr::addr_of_mut!(THREADS[index])
    } else {
        let extra = index - THREAD_CHUNK;
        let chunk_index = extra / THREAD_CHUNK;
        let offset = extra % THREAD_CHUNK;
        let mut chunk = THREAD_CHUNK_LIST.load(Ordering::Acquire);
        let mut i = 0usize;
        while i < chunk_index && !chunk.is_null() {
            chunk = (*chunk).next;
            i += 1;
        }
        if chunk.is_null() {
            return core::ptr::null_mut();
        }
        (*chunk).base.add(offset)
    }
}

unsafe fn release_thread_slot(slot: *const Thread) {
    let slot = slot as *mut Thread;
    spinlock_lock(&raw mut THREAD_LOCK, &raw mut THREAD_WAITERS);
    (*slot).tid = -1;
    (*slot).free_next = FREE_THREADS;
    FREE_THREADS = slot;
    spinlock_unlock(&raw mut THREAD_LOCK, &raw mut THREAD_WAITERS);
}

/// Link slots `start..THREAD_CHUNK` of `base` into the free list.  Caller
/// holds THREAD_LOCK.
unsafe fn push_free_slots(base: *mut Thread, start: usize) {
    let mut i = THREAD_CHUNK;
    while i > start {
        i -= 1;
        let slot = base.add(i);
        (*slot).free_next = FREE_THREADS;
        FREE_THREADS = slot;
    }
}

unsafe fn alloc_thread_slot() -> Option<&'static mut Thread> {
    spinlock_lock(&raw mut THREAD_LOCK, &raw mut THREAD_WAITERS);
    if !THREAD_TABLE_READY {
        // Slot 0 is reserved for the process's initial thread.  Registering
        // it must not require touching the rest of the table.
        push_free_slots(core::ptr::addr_of_mut!(THREADS[0]), 1);
        THREAD_TABLE_READY = true;
    }
    let mut slot = FREE_THREADS;
    if slot.is_null() {
        let bytes = core::mem::size_of::<ThreadChunk>()
            + THREAD_CHUNK * core::mem::size_of::<Thread>();
        let raw = sys_mmap(
            core::ptr::null_mut(),
            bytes,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        ) as *mut u8;
        if raw.is_null() || raw == MMAP_FAILED {
            spinlock_unlock(&raw mut THREAD_LOCK, &raw mut THREAD_WAITERS);
            return None;
        }
        let header = raw as *mut ThreadChunk;
        let base = raw.add(core::mem::size_of::<ThreadChunk>()) as *mut Thread;
        (*header).base = base;
        push_free_slots(base, 1);
        (*header).next = THREAD_CHUNK_LIST.load(Ordering::Relaxed);
        THREAD_CHUNK_LIST.store(header, Ordering::Release);
        EXTRA_CHUNK_COUNT.fetch_add(1, Ordering::AcqRel);
        slot = base;
    } else {
        FREE_THREADS = (*slot).free_next;
        (*slot).free_next = core::ptr::null_mut();
    }
    spinlock_unlock(&raw mut THREAD_LOCK, &raw mut THREAD_WAITERS);
    Some(&mut *slot)
}

/// The child of fork() keeps only the calling thread.  Rebuild the free list
/// so slots belonging to vanished threads are reusable, and update the
/// current thread's tid to the child's.
unsafe fn thread_allocator_after_fork() {
    a_store(&raw mut THREAD_LOCK, 0);
    a_store(&raw mut THREAD_WAITERS, 0);
    let tp = tcb_slot(THREAD_TCB_OFFSET);
    let current = if tp.is_null() {
        core::ptr::null_mut()
    } else {
        core::ptr::read_unaligned(tp) as *mut Thread
    };
    let child_tid = sys_gettid() as c_int;
    if !THREAD_TABLE_READY {
        // The initial thread is registered directly in slot 0, so the free
        // list may never have been built.  Only update that one slot; the
        // first pthread_create() will build the free list.
        if !current.is_null() {
            (*current).tid = child_tid;
            ACTIVE_THREADS.store(1, Ordering::Release);
        }
        return;
    }
    let count = thread_slot_count();
    let mut free = core::ptr::null_mut();
    let mut active = 0usize;
    let mut i = 0usize;
    while i < count {
        let slot = thread_slot_at(i);
        if slot == current {
            (*slot).tid = child_tid;
            (*slot).free_next = core::ptr::null_mut();
            active += 1;
        } else {
            (*slot).tid = -1;
            (*slot).free_next = free;
            free = slot;
        }
        i += 1;
    }
    FREE_THREADS = free;
    ACTIVE_THREADS.store(active, Ordering::Release);
}

unsafe fn run_key_dtors(slot: &mut Thread) {
    for _ in 0..PTHREAD_DESTRUCTOR_ITERATIONS {
        let mut any = false;
        for i in 0..PTHREAD_KEYS_MAX {
            let val = slot.tsd[i];
            if !val.is_null() {
                if let Some(dtor) = KEY_DTORS[i] {
                    slot.tsd[i] = core::ptr::null_mut();
                    any = true;
                    dtor(val);
                }
            }
        }
        if !any { break; }
    }
}

unsafe extern "C" fn thread_entry(slot: *mut c_void) -> *mut c_void {
    let slot = &mut *(slot as *mut Thread);
    let tp = tcb_slot(THREAD_TCB_OFFSET);
    if !tp.is_null() {
        core::ptr::write_unaligned(tp, slot as *mut Thread as usize);
    }
    // A reused slot may still carry the previous thread's robust-list state;
    // register a fresh list for this thread.
    slot.robust_list.list.next = core::ptr::null_mut();
    slot.robust_list.futex_offset = 0;
    slot.robust_list.pending = core::ptr::null_mut();
    ensure_robust_list(slot);
    let mut set: SigSetT = 0;
    sigemptyset(&mut set);
    sigaddset(&mut set, SIGCANCEL);
    pthread_sigmask(SIG_UNBLOCK, &set, core::ptr::null_mut());
    let user_fn: unsafe extern "C" fn(*mut c_void) -> *mut c_void =
        core::mem::transmute::<usize, _>(slot.user_fn);
    let ret = user_fn(slot.user_arg);
    thread_finish(slot, ret)
}

/// Common exit path for a thread that returned from its start routine and
/// for pthread_exit(): run cleanup handlers and TSD destructors, publish the
/// result, and give up the CPU.
unsafe fn thread_finish(slot: &mut Thread, retval: *mut c_void) -> ! {
    run_cleanup_handlers(slot);
    run_key_dtors(slot);
    vdso_getrandom_thread_exit();
    slot.result = retval;
    a_store(&raw mut slot.detach_state, DT_EXITED);
    futex_wake(&raw mut slot.detach_state, 1);
    if ACTIVE_THREADS.fetch_sub(1, Ordering::AcqRel) <= 1 {
        exit(0);
    }
    sys_exit_thread(0)
}

unsafe fn init_thread_slot(slot: *mut Thread, me: c_int) {
    (*slot).tid = me;
    (*slot).detach_state = DT_JOINABLE;
    (*slot).result = core::ptr::null_mut();
    (*slot).cancel = 0;
    (*slot).cancel_state = PTHREAD_CANCEL_ENABLE;
    (*slot).cancel_type = PTHREAD_CANCEL_DEFERRED;
    (*slot).cancelbuf = core::ptr::null_mut();
    core::ptr::write_bytes((*slot).tsd.as_mut_ptr(), 0, PTHREAD_KEYS_MAX);
    (*slot).robust_list.list.next = core::ptr::null_mut();
    (*slot).robust_list.futex_offset = 0;
    (*slot).robust_list.pending = core::ptr::null_mut();
    (*slot).free_next = core::ptr::null_mut();
}

unsafe fn register_main_thread() -> Option<&'static mut Thread> {
    let me = sys_gettid() as c_int;
    let tp = tcb_slot(THREAD_TCB_OFFSET);
    if !tp.is_null() {
        let p = core::ptr::read_unaligned(tp) as *mut Thread;
        if !p.is_null() && (*p).tid == me {
            return Some(&mut *p);
        }
    }
    let slot = core::ptr::addr_of_mut!(THREADS[0]);
    init_thread_slot(slot, me);
    if !tp.is_null() {
        core::ptr::write_unaligned(tp, slot as usize);
    }
    ACTIVE_THREADS.fetch_add(1, Ordering::AcqRel);
    Some(&mut *slot)
}

unsafe fn register_current_thread() -> Option<&'static mut Thread> {
    let me = sys_gettid() as c_int;
    let slot_ref = alloc_thread_slot()?;
    let slot = slot_ref as *mut Thread;
    init_thread_slot(slot, me);
    let tp = tcb_slot(THREAD_TCB_OFFSET);
    if !tp.is_null() {
        core::ptr::write_unaligned(tp, slot as usize);
    }
    ACTIVE_THREADS.fetch_add(1, Ordering::AcqRel);
    Some(&mut *slot)
}

unsafe fn find_thread() -> Option<&'static mut Thread> {
    let me = sys_gettid() as c_int;
    let tp = tcb_slot(THREAD_TCB_OFFSET);
    if !tp.is_null() {
        let p = core::ptr::read_unaligned(tp) as *mut Thread;
        if !p.is_null() && (*p).tid == me {
            return Some(&mut *p);
        }
    }
    let main = core::ptr::addr_of_mut!(THREADS[0]);
    if (*main).tid == me {
        if !tp.is_null() {
            core::ptr::write_unaligned(tp, main as usize);
        }
        return Some(&mut *main);
    }
    let count = thread_slot_count();
    let mut i = 0usize;
    while i < count {
        let slot = thread_slot_at(i);
        if core::ptr::read_volatile(&raw const (*slot).tid) == me {
            if !tp.is_null() {
                core::ptr::write_unaligned(tp, slot as usize);
            }
            return Some(&mut *slot);
        }
        i += 1;
    }
    register_current_thread()
}

#[no_mangle]
pub unsafe extern "C" fn pthread_self() -> PthreadT {
    match find_thread() {
        Some(slot) => slot as *mut Thread as PthreadT,
        None => 0,
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_equal(t1: PthreadT, t2: PthreadT) -> c_int {
    (t1 == t2) as c_int
}

unsafe fn dynamic_tls_init(block: *mut u8) -> *mut u8 {
    __keel_init_thread_tls_at(block)
}

unsafe fn dynamic_tls_create() -> *mut u8 {
    __keel_create_thread_tls()
}

/// Prefer carving TLS out of the top of an application-provided stack; a
/// separate mapping can land below a MAP_GROWSDOWN stack and block growth.
unsafe fn carve_thread_tls(
    init_at: unsafe fn(*mut u8) -> *mut u8,
    create: unsafe fn() -> *mut u8,
    user_top: *mut c_void,
    stack_size: usize,
    tls_total: usize,
    stack_owned: bool,
    stack_top: &mut *mut u8,
    tls_owned: &mut bool,
) -> *mut u8 {
    if !stack_owned && tls_total > 0 && stack_size > tls_total + PAGE {
        let block = ((user_top as usize - tls_total) & !63usize) as *mut u8;
        let fs = init_at(block);
        if !fs.is_null() {
            *stack_top = block;
            *tls_owned = false;
            return fs;
        }
    }
    create()
}

#[no_mangle]
pub unsafe extern "C" fn pthread_create(
    thread: *mut PthreadT,
    attr: *const pthread_attr_t,
    start_routine: usize,
    arg: *mut c_void,
) -> c_int {
    if start_routine == 0 || thread.is_null() { return EINVAL; }
    let Some(slot_ref) = alloc_thread_slot() else { return EAGAIN; };
    let slot = slot_ref as *mut Thread;
    let stack_size = if !attr.is_null() {
        let s = attr_stack_size(attr);
        if s > 0 { s } else { STACK_SIZE }
    } else { STACK_SIZE };
    // musl stores the stack *top* in pthread_attr_t; zero means "allocate".
    let user_top = if !attr.is_null() {
        attr_stack_top(attr)
    } else {
        core::ptr::null_mut()
    };
    let stack_owned = user_top.is_null();
    let stack = if stack_owned {
        sys_mmap(null_mut(), stack_size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0)
    } else {
        (user_top as *mut u8).sub(stack_size)
    };
    if stack == MMAP_FAILED { release_thread_slot(slot); return EAGAIN; }
    let static_mode = RC_STATIC_EXEC;
    let tls_total = if static_mode {
        STATIC_TLS_BLOCK_SIZE
    } else {
        __keel_tls_block_size()
    };
    let mut stack_top = stack.add(stack_size);
    let mut tls_owned = true;
    let fs_base = if static_mode {
        carve_thread_tls(
            static_tls_init_block,
            static_tls_create_block,
            user_top,
            stack_size,
            tls_total,
            stack_owned,
            &mut stack_top,
            &mut tls_owned,
        )
    } else {
        carve_thread_tls(
            dynamic_tls_init,
            dynamic_tls_create,
            user_top,
            stack_size,
            tls_total,
            stack_owned,
            &mut stack_top,
            &mut tls_owned,
        )
    };
    if fs_base.is_null() {
        if stack_owned { sys_munmap(stack, stack_size); }
        release_thread_slot(slot);
        return EAGAIN;
    }
    let tls_base = if static_mode {
        fs_base.sub(STATIC_TLS_TP_OFFSET)
    } else {
        fs_base.sub(__keel_tls_base_offset())
    };
    let tls_size = if tls_owned { tls_total } else { 0 };
    init_thread_slot(slot, 1);
    (*slot).user_fn = start_routine;
    (*slot).user_arg = arg;
    (*slot).stack = stack;
    (*slot).stack_size = stack_size;
    (*slot).stack_owned = stack_owned;
    (*slot).fs_base = fs_base;
    (*slot).tls_base = tls_base;
    (*slot).tls_size = tls_size;
    let tid_ptr = &raw mut (*slot).tid;
    let flags = CLONE_VM | CLONE_FS | CLONE_FILES | CLONE_SIGHAND | CLONE_THREAD
        | CLONE_SYSVSEM | CLONE_PARENT_SETTID | CLONE_CHILD_CLEARTID | CLONE_SETTLS;
    ACTIVE_THREADS.fetch_add(1, Ordering::AcqRel);
    let tid = __keel_clone(thread_entry as *const () as usize, stack_top, flags, slot as *mut c_void, tid_ptr, fs_base as c_ulong, tid_ptr);
    if tid < 0 {
        ACTIVE_THREADS.fetch_sub(1, Ordering::AcqRel);
        (*slot).tid = -1;
        if stack_owned { sys_munmap(stack, stack_size); }
        if tls_size > 0 { sys_munmap(tls_base, tls_size); }
        release_thread_slot(slot);
        return EAGAIN;
    }
    if !attr.is_null() && (*attr).__i[6] == PTHREAD_CREATE_DETACHED {
        (*slot).detach_state = DT_DETACHED;
    }
    *thread = slot as PthreadT;
    0
}

/// Tear down a joined or detached thread slot. The TLS block is kept mapped:
/// some runtimes (notably V8) retain pointers into an exited thread's TLS
/// and dereference them during process teardown.
unsafe fn release_finished_slot(slot: *mut Thread) {
    let stack = (*slot).stack;
    let stack_size = (*slot).stack_size;
    let stack_owned = (*slot).stack_owned;
    (*slot).tid = -1;
    (*slot).detach_state = DT_JOINABLE;
    (*slot).result = core::ptr::null_mut();
    (*slot).stack = core::ptr::null_mut();
    (*slot).stack_size = 0;
    (*slot).stack_owned = false;
    (*slot).fs_base = core::ptr::null_mut();
    (*slot).tls_base = core::ptr::null_mut();
    (*slot).tls_size = 0;
    if stack_owned && !stack.is_null() && stack_size > 0 {
        sys_munmap(stack, stack_size);
    }
    release_thread_slot(slot);
}

#[no_mangle]
pub unsafe extern "C" fn pthread_join(thread: PthreadT, retval: *mut *mut c_void) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    if thread == pthread_self() { return EDEADLK; }
    if (*slot).detach_state == DT_DETACHED { return EINVAL; }
    let tid_ptr = &raw mut (*slot).tid;
    loop {
        pthread_testcancel();
        let tid = core::ptr::read_volatile(tid_ptr);
        if tid == 0 { break; }
        if tid < 0 { return EINVAL; }
        sys_futex(tid_ptr, FUTEX_WAIT, tid, null_mut(), null_mut(), 0);
        pthread_testcancel();
    }
    if !retval.is_null() { *retval = (*slot).result; }
    release_finished_slot(slot);
    0
}

#[no_mangle]
pub unsafe extern "C" fn pthread_detach(thread: PthreadT) -> c_int {
    let slot = thread as *mut Thread;
    if slot.is_null() { return EINVAL; }
    let old = a_cas(&raw mut (*slot).detach_state, DT_JOINABLE, DT_DETACHED);
    if old == DT_EXITED {
        release_finished_slot(slot);
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn pthread_exit(retval: *mut c_void) -> ! {
    if let Some(slot) = find_thread() {
        thread_finish(slot, retval)
    }
    sys_exit_thread(0)
}

// pthread_attr_t stores a usize stack size at __i[0..2], a usize guard at
// __i[2..4], the stack top pointer at __i[4..6], and scalar flags after it.
unsafe fn attr_stack_size(attr: *const pthread_attr_t) -> usize {
    *((*attr).__i.as_ptr() as *const usize)
}
unsafe fn attr_set_stack_size(attr: *mut pthread_attr_t, sz: usize) {
    *((*attr).__i.as_mut_ptr() as *mut usize) = sz;
}
unsafe fn attr_stack_top(attr: *const pthread_attr_t) -> *mut c_void {
    *((*attr).__i.as_ptr().add(4) as *const *mut c_void)
}
unsafe fn attr_set_stack_top(attr: *mut pthread_attr_t, top: *mut c_void) {
    *((*attr).__i.as_mut_ptr().add(4) as *mut *mut c_void) = top;
}
unsafe fn attr_guard_size(attr: *const pthread_attr_t) -> usize {
    *((*attr).__i.as_ptr().add(2) as *const usize)
}
unsafe fn attr_set_guard_size(attr: *mut pthread_attr_t, sz: usize) {
    *((*attr).__i.as_mut_ptr().add(2) as *mut usize) = sz;
}

// --- pthread_attr_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_init(attr: *mut pthread_attr_t) -> c_int {
    core::ptr::write_bytes(attr, 0, 1);
    attr_set_stack_size(attr, STACK_SIZE);
    attr_set_guard_size(attr, PAGE);
    (*attr).__i[6] = PTHREAD_CREATE_JOINABLE;
    (*attr).__i[7] = PTHREAD_INHERIT_SCHED;
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_destroy(_attr: *mut pthread_attr_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setdetachstate(attr: *mut pthread_attr_t, s: c_int) -> c_int {
    if s != PTHREAD_CREATE_JOINABLE && s != PTHREAD_CREATE_DETACHED { return EINVAL; }
    (*attr).__i[6] = s; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getdetachstate(attr: *const pthread_attr_t, s: *mut c_int) -> c_int {
    *s = (*attr).__i[6]; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setstacksize(attr: *mut pthread_attr_t, sz: usize) -> c_int {
    if sz < 2048 { return EINVAL; }
    attr_set_stack_size(attr, sz);
    attr_set_stack_top(attr, core::ptr::null_mut());
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getstacksize(attr: *const pthread_attr_t, sz: *mut usize) -> c_int {
    *sz = attr_stack_size(attr); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setstack(attr: *mut pthread_attr_t, addr: *mut c_void, sz: usize) -> c_int {
    if sz < 2048 { return EINVAL; }
    attr_set_stack_size(attr, sz);
    // musl stores the top of the stack, not its base.
    attr_set_stack_top(attr, (addr as *mut u8).add(sz) as *mut c_void);
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getstack(attr: *const pthread_attr_t, addr: *mut *mut c_void, sz: *mut usize) -> c_int {
    let size = attr_stack_size(attr);
    *sz = size;
    let top = attr_stack_top(attr);
    *addr = if top.is_null() {
        core::ptr::null_mut()
    } else {
        (top as *mut u8).sub(size) as *mut c_void
    };
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setguardsize(attr: *mut pthread_attr_t, sz: usize) -> c_int {
    attr_set_guard_size(attr, sz); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getguardsize(attr: *const pthread_attr_t, sz: *mut usize) -> c_int {
    *sz = attr_guard_size(attr); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setscope(_attr: *mut pthread_attr_t, scope: c_int) -> c_int {
    if scope != PTHREAD_SCOPE_SYSTEM { return EINVAL; }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getscope(_attr: *const pthread_attr_t, scope: *mut c_int) -> c_int {
    *scope = PTHREAD_SCOPE_SYSTEM; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setinheritsched(attr: *mut pthread_attr_t, inh: c_int) -> c_int {
    if inh != PTHREAD_INHERIT_SCHED && inh != PTHREAD_EXPLICIT_SCHED { return EINVAL; }
    (*attr).__i[7] = inh; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getinheritsched(attr: *const pthread_attr_t, inh: *mut c_int) -> c_int {
    *inh = (*attr).__i[7]; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setschedpolicy(attr: *mut pthread_attr_t, p: c_int) -> c_int {
    (*attr).__i[8] = p; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getschedpolicy(attr: *const pthread_attr_t, p: *mut c_int) -> c_int {
    *p = (*attr).__i[8]; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_setschedparam(attr: *mut pthread_attr_t, param: *const sched_param) -> c_int {
    (*attr).__i[9] = (*param).sched_priority; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_attr_getschedparam(attr: *const pthread_attr_t, param: *mut sched_param) -> c_int {
    (*param).sched_priority = (*attr).__i[9]; 0
}

// --- pthread_mutexattr_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_init(attr: *mut pthread_mutexattr_t) -> c_int {
    (*attr).__attr = 0; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_destroy(_attr: *mut pthread_mutexattr_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_settype(attr: *mut pthread_mutexattr_t, t: c_int) -> c_int {
    if (t as c_uint) > 2 { return EINVAL; }
    (*attr).__attr = ((*attr).__attr & !3) | t as c_uint; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_gettype(attr: *const pthread_mutexattr_t, t: *mut c_int) -> c_int {
    *t = ((*attr).__attr & 3) as c_int; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_setpshared(attr: *mut pthread_mutexattr_t, p: c_int) -> c_int {
    if (p as c_uint) > 1 { return EINVAL; }
    (*attr).__attr = ((*attr).__attr & !(MUTEX_PSHARED as c_uint)) | ((p as c_uint) << 7); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_getpshared(attr: *const pthread_mutexattr_t, p: *mut c_int) -> c_int {
    *p = (((*attr).__attr >> 7) & 1) as c_int; 0
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_setprotocol(attr: *mut pthread_mutexattr_t, protocol: c_int) -> c_int {
    match protocol {
        0 => { (*attr).__attr &= !(MUTEX_PI as c_uint); 0 }
        1 => {
            let mut lk: c_int = 0;
            let r = sys_futex(&raw mut lk, FUTEX_LOCK_PI, 0, null_mut(), null_mut(), 0);
            if r < 0 { return (-r) as c_int; }
            sys_futex(&raw mut lk, FUTEX_UNLOCK_PI, 0, null_mut(), null_mut(), 0);
            (*attr).__attr |= MUTEX_PI as c_uint;
            0
        }
        2 => ENOTSUP,
        _ => EINVAL,
    }
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_getprotocol(attr: *const pthread_mutexattr_t, protocol: *mut c_int) -> c_int {
    *protocol = (((*attr).__attr >> 3) & 3) as c_int; 0
}


#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_setrobust(attr: *mut pthread_mutexattr_t, robust: c_int) -> c_int {
    if (robust as c_uint) > 1 { return EINVAL; }
    if robust != 0 {
        (*attr).__attr |= MUTEX_ROBUST as c_uint;
    } else {
        (*attr).__attr &= !(MUTEX_ROBUST as c_uint);
    }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutexattr_getrobust(attr: *const pthread_mutexattr_t, robust: *mut c_int) -> c_int {
    *robust = (((*attr).__attr >> 2) & 1) as c_int; 0
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_consistent(m: *mut pthread_mutex_t) -> c_int {
    let old = a_load(&raw const (*m).__i[1]);
    let own = old & MUTEX_OWNER_MASK;
    if ((*m).__i[0] & MUTEX_ROBUST) == 0 || own == 0 || (old & MUTEX_ROBUST_PENDING) == 0 {
        return EINVAL;
    }
    let self_tid = sys_gettid() as c_int;
    if own != self_tid { return EPERM; }
    a_and(&raw mut (*m).__i[1], !(MUTEX_ROBUST_PENDING));
    0
}

unsafe fn a_and(addr: *mut c_int, val: c_int) {
    let a = &*(addr as *const AtomicI32);
    a.fetch_and(val, Ordering::AcqRel);
}

// --- pthread_mutex_* ---
const MUTEX_TYPE_MASK: c_int = 3;
const MUTEX_ROBUST: c_int = 4;
const MUTEX_PI: c_int = 8;
const MUTEX_PSHARED: c_int = 128;
const MUTEX_OWNER_MASK: c_int = 0x3fff_ffff;
const MUTEX_ROBUST_PENDING: c_int = 0x4000_0000;
const FUTEX_WAITER_BIT: c_int = 1i32 << 31;

unsafe fn ensure_robust_list(slot: &mut Thread) {
    if slot.robust_list.futex_offset == 0 {
        slot.robust_list.futex_offset = -28;
        slot.robust_list.pending = core::ptr::null_mut();
        sys_set_robust_list(&mut slot.robust_list, 24);
    }
}

unsafe fn mutex_trylock(m: *mut pthread_mutex_t) -> c_int {
    let type_ = (*m).__i[0];
    // Fast path: plain NORMAL only (no PI, no robust, no pshared).
    // A failed CAS must report EBUSY unconditionally: the old value may be
    // the waiter bit (0x80000000), whose low bits are zero, and masking it
    // with EBUSY would falsely report that the lock was acquired.
    if (type_ & 0x8f) == PTHREAD_MUTEX_NORMAL {
        let old = a_cas(&raw mut (*m).__i[1], 0, EBUSY);
        return if old == 0 { 0 } else { EBUSY };
    }
    let base_type = type_ & MUTEX_TYPE_MASK;
    let self_tid = sys_gettid() as c_int;
    let old = a_load(&raw const (*m).__i[1]);
    let own = old & MUTEX_OWNER_MASK;
    if own == self_tid {
        if (type_ & MUTEX_PI) != 0 && (*m).__i[5] < 0 {
            (*m).__i[5] = 0;
            return 0;
        }
        if base_type == PTHREAD_MUTEX_RECURSIVE {
            if (*m).__i[5] >= c_int::MAX { return EAGAIN; }
            (*m).__i[5] += 1;
            return 0;
        }
        if base_type == PTHREAD_MUTEX_ERRORCHECK { return EDEADLK; }
        return EBUSY;
    }
    if own == MUTEX_OWNER_MASK { return ENOTRECOVERABLE; }
    if own != 0 || (old != 0 && (type_ & MUTEX_ROBUST) == 0) { return EBUSY; }
    let mut newtid = self_tid;
    if (type_ & MUTEX_ROBUST) != 0 {
        if (*m).__i[2] != 0 { newtid |= FUTEX_WAITER_BIT; }
        newtid |= old & MUTEX_ROBUST_PENDING;
    }
    if a_cas(&raw mut (*m).__i[1], old, newtid) == old {
        (*m).__i[5] = 0;
        if (type_ & MUTEX_ROBUST) != 0 {
            if let Some(slot) = find_thread() {
                ensure_robust_list(slot);
                let m_next = &raw mut (*m).__i[8] as *mut robust_list;
                let head = &mut slot.robust_list.list;
                (*m_next).next = head.next;
                head.next = m_next;
            }
        }
        if old != 0 { return EOWNERDEAD; }
        0
    } else { EBUSY }
}

unsafe fn mutex_lock_internal(m: *mut pthread_mutex_t, abs_timeout: *const timespec) -> c_int {
    let type_ = (*m).__i[0];
    if (type_ & 0x8f) == PTHREAD_MUTEX_NORMAL && a_cas(&raw mut (*m).__i[1], 0, EBUSY) == 0 {
        return 0;
    }
    let r = mutex_trylock(m);
    if r != EBUSY { return r; }
    if (type_ & MUTEX_PI) != 0 {
        let timeout_ptr = if abs_timeout.is_null() { core::ptr::null() } else { abs_timeout };
        loop {
            let e = futex_lock_pi(&raw mut (*m).__i[1], timeout_ptr);
            match e {
                0 => {
                    if (type_ & MUTEX_ROBUST) != 0 && (((*m).__i[1] & MUTEX_ROBUST_PENDING) != 0 || (*m).__i[2] != 0) {
                        (*m).__i[5] = -1;
                        let _ = mutex_trylock(m);
                        return EOWNERDEAD;
                    }
                    if (type_ & MUTEX_ROBUST) == 0 && (((*m).__i[1] & MUTEX_ROBUST_PENDING) != 0 || (*m).__i[2] != 0) {
                        (*m).__i[2] = -1;
                        futex_unlock_pi(&raw mut (*m).__i[1]);
                        return EBUSY;
                    }
                    (*m).__i[5] = -1;
                    let r2 = mutex_trylock(m);
                    if r2 == 0 { return 0; }
                    return r2;
                }
                EINTR => continue,
                EDEADLK => {
                    if (type_ & MUTEX_TYPE_MASK) == PTHREAD_MUTEX_ERRORCHECK { return e; }
                    loop { pause(); }
                }
                EOWNERDEAD => {
                    (*m).__i[5] = -1;
                    let _ = mutex_trylock(m);
                    return EOWNERDEAD;
                }
                _ => return e,
            }
        }
    }
    let mut spins = 100;
    while spins > 0 {
        let r = mutex_trylock(m);
        if r != EBUSY { return r; }
        core::hint::spin_loop();
        spins -= 1;
    }
    loop {
        let r = mutex_trylock(m);
        if r != EBUSY { return r; }
        if (type_ & MUTEX_TYPE_MASK) == PTHREAD_MUTEX_ERRORCHECK {
            let self_tid = sys_gettid() as c_int;
            if (a_load(&raw const (*m).__i[1]) & MUTEX_OWNER_MASK) == self_tid { return EDEADLK; }
        }
        a_fetch_add(&raw mut (*m).__i[2], 1);
        let val = a_load(&raw const (*m).__i[1]);
        // If the previous owner released the lock between trylock() and the
        // load above, retry instead of publishing a waiter-only lock word
        // (0x80000000) that no thread owns.
        if val == 0 {
            a_fetch_sub(&raw mut (*m).__i[2], 1);
            continue;
        }
        a_cas(&raw mut (*m).__i[1], val, val | FUTEX_WAITER_BIT);
        let e = futex_timedwait(&raw mut (*m).__i[1], val | FUTEX_WAITER_BIT, abs_timeout);
        a_fetch_sub(&raw mut (*m).__i[2], 1);
        if e != 0 && e != EINTR { return e; }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_init(mutex: *mut pthread_mutex_t, attr: *const pthread_mutexattr_t) -> c_int {
    core::ptr::write_bytes(mutex, 0, 1);
    if !attr.is_null() { (*mutex).__i[0] = (*attr).__attr as c_int; }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_destroy(_mutex: *mut pthread_mutex_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_lock(mutex: *mut pthread_mutex_t) -> c_int {
    mutex_lock_internal(mutex, core::ptr::null())
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_trylock(mutex: *mut pthread_mutex_t) -> c_int {
    mutex_trylock(mutex)
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_timedlock(mutex: *mut pthread_mutex_t, abs_timeout: *const timespec) -> c_int {
    mutex_lock_internal(mutex, abs_timeout)
}
#[no_mangle]
pub unsafe extern "C" fn pthread_mutex_unlock(mutex: *mut pthread_mutex_t) -> c_int {
    let type_ = (*mutex).__i[0];
    let base_type = type_ & MUTEX_TYPE_MASK;
    let old = a_load(&raw const (*mutex).__i[1]);
    if base_type != PTHREAD_MUTEX_NORMAL {
        let self_tid = sys_gettid() as c_int;
        if (old & MUTEX_OWNER_MASK) != self_tid { return EPERM; }
        if base_type == PTHREAD_MUTEX_RECURSIVE && (*mutex).__i[5] > 0 {
            (*mutex).__i[5] -= 1;
            return 0;
        }
    }
    let mut new = 0;
    if (type_ & MUTEX_ROBUST) != 0 && (old & MUTEX_ROBUST_PENDING) != 0 {
        new = 0x7fffffff;
    }
    if (type_ & MUTEX_PI) != 0 {
        if old < 0 || a_cas(&raw mut (*mutex).__i[1], old, new) != old {
            if new != 0 { a_store(&raw mut (*mutex).__i[2], -1); }
            return futex_unlock_pi(&raw mut (*mutex).__i[1]);
        }
        return 0;
    }
    a_store(&raw mut (*mutex).__i[1], new);
    if new == 0 && (a_load(&raw const (*mutex).__i[2]) > 0 || old < 0) {
        futex_wake(&raw mut (*mutex).__i[1], 1);
    }
    0
}

// --- pthread_condattr_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_init(attr: *mut pthread_condattr_t) -> c_int {
    (*attr).__attr = 0; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_destroy(_attr: *mut pthread_condattr_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_setclock(attr: *mut pthread_condattr_t, clk: c_int) -> c_int {
    if clk != CLOCK_REALTIME && clk != CLOCK_MONOTONIC { return EINVAL; }
    (*attr).__attr = ((*attr).__attr & 0x80000000) | clk as c_uint; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_getclock(attr: *const pthread_condattr_t, clk: *mut c_int) -> c_int {
    *clk = ((*attr).__attr & 0x7fffffff) as c_int; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_setpshared(attr: *mut pthread_condattr_t, p: c_int) -> c_int {
    if (p as c_uint) > 1 { return EINVAL; }
    (*attr).__attr = ((*attr).__attr & 0x7fffffff) | ((p as c_uint) << 31); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_condattr_getpshared(attr: *const pthread_condattr_t, p: *mut c_int) -> c_int {
    *p = ((*attr).__attr >> 31) as c_int; 0
}

// --- pthread_cond_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_init(cond: *mut pthread_cond_t, attr: *const pthread_condattr_t) -> c_int {
    core::ptr::write_bytes(cond, 0, 1);
    if !attr.is_null() {
        (*cond).__i[4] = ((*attr).__attr & 0x7fffffff) as c_int;
    }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_destroy(_cond: *mut pthread_cond_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_wait(cond: *mut pthread_cond_t, mutex: *mut pthread_mutex_t) -> c_int {
    pthread_cond_timedwait(cond, mutex, core::ptr::null())
}


#[no_mangle]
pub unsafe extern "C" fn pthread_cond_timedwait(cond: *mut pthread_cond_t, mutex: *mut pthread_mutex_t, abs_timeout: *const timespec) -> c_int {
    pthread_testcancel();
    let seq_ptr = &raw mut (*cond).__i[2];
    let waiters_ptr = &raw mut (*cond).__i[3];
    let seq = a_load(seq_ptr);
    a_fetch_add(waiters_ptr, 1);
    pthread_mutex_unlock(mutex);
    let e = futex_timedwait(seq_ptr, seq, abs_timeout);
    a_fetch_sub(waiters_ptr, 1);
    let r = pthread_mutex_lock(mutex);
    if r != 0 { return r; }
    if e == EINTR {
        pthread_testcancel();
    }
    if e != 0 && e != EINTR { return e; }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_signal(cond: *mut pthread_cond_t) -> c_int {
    if a_load(&raw const (*cond).__i[3]) == 0 { return 0; }
    a_fetch_add(&raw mut (*cond).__i[2], 1);
    futex_wake(&raw mut (*cond).__i[2], 1);
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_cond_broadcast(cond: *mut pthread_cond_t) -> c_int {
    if a_load(&raw const (*cond).__i[3]) == 0 { return 0; }
    a_fetch_add(&raw mut (*cond).__i[2], 1);
    futex_wake(&raw mut (*cond).__i[2], -1);
    0
}

// --- pthread_rwlockattr_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlockattr_init(attr: *mut pthread_rwlockattr_t) -> c_int {
    (*attr).__attr = [0; 2]; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlockattr_destroy(_attr: *mut pthread_rwlockattr_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlockattr_setpshared(attr: *mut pthread_rwlockattr_t, p: c_int) -> c_int {
    if (p as c_uint) > 1 { return EINVAL; }
    (*attr).__attr[0] = ((*attr).__attr[0] & !128) | ((p as c_uint) << 7); 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlockattr_getpshared(attr: *const pthread_rwlockattr_t, p: *mut c_int) -> c_int {
    *p = (((*attr).__attr[0] >> 7) & 1) as c_int; 0
}

// --- pthread_rwlock_* ---
unsafe fn rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int {
    loop {
        let val = a_load(&raw const (*rw).__i[0]);
        let cnt = val & 0x7fffffff;
        if cnt == 0x7fffffff { return EBUSY; }
        if cnt == 0x7ffffffe { return EAGAIN; }
        if a_cas(&raw mut (*rw).__i[0], val, val + 1) == val { return 0; }
    }
}
unsafe fn rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int {
    if a_cas(&raw mut (*rw).__i[0], 0, 0x7fffffff) == 0 { 0 } else { EBUSY }
}
unsafe fn rwlock_timedrdlock(rw: *mut pthread_rwlock_t, abs_timeout: *const timespec) -> c_int {
    let r = rwlock_tryrdlock(rw);
    if r != EBUSY { return r; }
    let mut spins = 100;
    while spins > 0 { let r = rwlock_tryrdlock(rw); if r != EBUSY { return r; } core::hint::spin_loop(); spins -= 1; }
    loop {
        let r = rwlock_tryrdlock(rw);
        if r != EBUSY { return r; }
        let val = a_load(&raw const (*rw).__i[0]);
        if val & 0x7fffffff != 0x7fffffff { continue; }
        a_fetch_add(&raw mut (*rw).__i[1], 1);
        a_cas(&raw mut (*rw).__i[0], val, val | FUTEX_WAITER_BIT);
        let e = futex_timedwait(&raw mut (*rw).__i[0], val | FUTEX_WAITER_BIT, abs_timeout);
        a_fetch_sub(&raw mut (*rw).__i[1], 1);
        if e != 0 && e != EINTR { return e; }
    }
}
unsafe fn rwlock_timedwrlock(rw: *mut pthread_rwlock_t, abs_timeout: *const timespec) -> c_int {
    let r = rwlock_trywrlock(rw);
    if r != EBUSY { return r; }
    let mut spins = 100;
    while spins > 0 { let r = rwlock_trywrlock(rw); if r != EBUSY { return r; } core::hint::spin_loop(); spins -= 1; }
    loop {
        let r = rwlock_trywrlock(rw);
        if r != EBUSY { return r; }
        let val = a_load(&raw const (*rw).__i[0]);
        if val == 0 { continue; }
        a_fetch_add(&raw mut (*rw).__i[1], 1);
        a_cas(&raw mut (*rw).__i[0], val, val | FUTEX_WAITER_BIT);
        let e = futex_timedwait(&raw mut (*rw).__i[0], val | FUTEX_WAITER_BIT, abs_timeout);
        a_fetch_sub(&raw mut (*rw).__i[1], 1);
        if e != 0 && e != EINTR { return e; }
    }
}

#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_init(rw: *mut pthread_rwlock_t, attr: *const pthread_rwlockattr_t) -> c_int {
    core::ptr::write_bytes(rw, 0, 1);
    if !attr.is_null() { (*rw).__i[2] = (*attr).__attr[0] as c_int & 128; }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_destroy(_rw: *mut pthread_rwlock_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_rdlock(rw: *mut pthread_rwlock_t) -> c_int { rwlock_timedrdlock(rw, core::ptr::null()) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_tryrdlock(rw: *mut pthread_rwlock_t) -> c_int { rwlock_tryrdlock(rw) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_timedrdlock(rw: *mut pthread_rwlock_t, t: *const timespec) -> c_int { rwlock_timedrdlock(rw, t) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_wrlock(rw: *mut pthread_rwlock_t) -> c_int { rwlock_timedwrlock(rw, core::ptr::null()) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_trywrlock(rw: *mut pthread_rwlock_t) -> c_int { rwlock_trywrlock(rw) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_timedwrlock(rw: *mut pthread_rwlock_t, t: *const timespec) -> c_int { rwlock_timedwrlock(rw, t) }
#[no_mangle]
pub unsafe extern "C" fn pthread_rwlock_unlock(rw: *mut pthread_rwlock_t) -> c_int {
    loop {
        let val = a_load(&raw const (*rw).__i[0]);
        let cnt = val & 0x7fffffff;
        let new_val = if cnt == 0x7fffffff || cnt == 1 { 0 } else { val - 1 };
        if a_cas(&raw mut (*rw).__i[0], val, new_val) == val {
            if new_val == 0 && (a_load(&raw const (*rw).__i[1]) > 0 || val < 0) {
                futex_wake(&raw mut (*rw).__i[0], cnt);
            }
            return 0;
        }
    }
}

// --- pthread_barrierattr_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_barrierattr_init(attr: *mut pthread_barrierattr_t) -> c_int { (*attr).__attr = 0; 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_barrierattr_destroy(_attr: *mut pthread_barrierattr_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_barrierattr_setpshared(attr: *mut pthread_barrierattr_t, p: c_int) -> c_int {
    if (p as c_uint) > 1 { return EINVAL; }
    (*attr).__attr = p as c_uint; 0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_barrierattr_getpshared(attr: *const pthread_barrierattr_t, p: *mut c_int) -> c_int {
    *p = (*attr).__attr as c_int; 0
}

// --- pthread_barrier_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_barrier_init(b: *mut pthread_barrier_t, _attr: *const pthread_barrierattr_t, count: c_uint) -> c_int {
    if count == 0 { return EINVAL; }
    core::ptr::write_bytes(b, 0, 1);
    (*b).__i[2] = count as c_int - 1;
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_barrier_destroy(_b: *mut pthread_barrier_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_barrier_wait(b: *mut pthread_barrier_t) -> c_int {
    let limit = (*b).__i[2];
    if limit == 0 { return PTHREAD_BARRIER_SERIAL_THREAD; }
    spinlock_lock(&raw mut (*b).__i[0], &raw mut (*b).__i[1]);
    let arrived = a_load(&raw const (*b).__i[3]) + 1;
    a_store(&raw mut (*b).__i[3], arrived);
    if arrived > limit {
        a_store(&raw mut (*b).__i[3], 0);
        let gen = a_load(&raw const (*b).__i[4]);
        a_store(&raw mut (*b).__i[4], gen + 1);
        spinlock_unlock(&raw mut (*b).__i[0], &raw mut (*b).__i[1]);
        futex_wake(&raw mut (*b).__i[4], -1);
        return PTHREAD_BARRIER_SERIAL_THREAD;
    }
    let gen = a_load(&raw const (*b).__i[4]);
    spinlock_unlock(&raw mut (*b).__i[0], &raw mut (*b).__i[1]);
    while a_load(&raw const (*b).__i[4]) == gen {
        futex_wait(&raw mut (*b).__i[4], gen);
    }
    0
}

// --- pthread_spin_* ---
#[no_mangle]
pub unsafe extern "C" fn pthread_spin_init(s: *mut pthread_spinlock_t, _pshared: c_int) -> c_int { a_store(s, 0); 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_spin_destroy(_s: *mut pthread_spinlock_t) -> c_int { 0 }
#[no_mangle]
pub unsafe extern "C" fn pthread_spin_lock(s: *mut pthread_spinlock_t) -> c_int {
    while a_load(s) != 0 || a_cas(s, 0, EBUSY) != 0 { core::hint::spin_loop(); }
    0
}
#[no_mangle]
pub unsafe extern "C" fn pthread_spin_trylock(s: *mut pthread_spinlock_t) -> c_int { a_cas(s, 0, EBUSY) }
#[no_mangle]
pub unsafe extern "C" fn pthread_spin_unlock(s: *mut pthread_spinlock_t) -> c_int { a_store(s, 0); 0 }

// --- sem_* ---
#[no_mangle]
pub unsafe extern "C" fn sem_init(sem: *mut sem_t, pshared: c_int, value: c_uint) -> c_int {
    if value > SEM_VALUE_MAX as c_uint { return EINVAL; }
    (*sem).__val[0] = value as c_int;
    (*sem).__val[1] = 0;
    (*sem).__val[2] = if pshared != 0 { 0 } else { 128 };
    0
}
#[no_mangle]
pub unsafe extern "C" fn sem_destroy(_sem: *mut sem_t) -> c_int { 0 }

unsafe fn sem_trywait_internal(sem: *mut sem_t) -> c_int {
    loop {
        let val = a_load(&raw const (*sem).__val[0]);
        if val & SEM_VALUE_MAX == 0 { return EAGAIN; }
        if a_cas(&raw mut (*sem).__val[0], val, val - 1) == val { return 0; }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sem_trywait(sem: *mut sem_t) -> c_int {
    if sem_trywait_internal(sem) == 0 { 0 } else { ERRNO!() = EAGAIN; -1 }
}
unsafe fn sem_wait_common(sem: *mut sem_t, abs_timeout: *const timespec) -> c_int {
    pthread_testcancel();
    if sem_trywait_internal(sem) == 0 { return 0; }
    let mut spins = 100;
    while spins > 0 {
        if sem_trywait_internal(sem) == 0 { return 0; }
        core::hint::spin_loop();
        spins -= 1;
    }
    loop {
        pthread_testcancel();
        if sem_trywait_internal(sem) == 0 { return 0; }
        let val = a_load(&raw const (*sem).__val[0]);
        if val & SEM_VALUE_MAX != 0 { continue; }
        a_cas(&raw mut (*sem).__val[0], val, val | FUTEX_WAITER_BIT);
        a_fetch_add(&raw mut (*sem).__val[1], 1);
        let e = if abs_timeout.is_null() {
            futex_wait(&raw mut (*sem).__val[0], FUTEX_WAITER_BIT);
            0
        } else {
            futex_timedwait(&raw mut (*sem).__val[0], FUTEX_WAITER_BIT, abs_timeout)
        };
        a_fetch_sub(&raw mut (*sem).__val[1], 1);
        if e != 0 && e != EINTR { ERRNO!() = e; return -1; }
    }
}

#[no_mangle]
pub unsafe extern "C" fn sem_wait(sem: *mut sem_t) -> c_int {
    sem_wait_common(sem, core::ptr::null())
}

#[no_mangle]
pub unsafe extern "C" fn sem_timedwait(sem: *mut sem_t, abs_timeout: *const timespec) -> c_int {
    sem_wait_common(sem, abs_timeout)
}

#[no_mangle]
pub unsafe extern "C" fn sem_post(sem: *mut sem_t) -> c_int {
    loop {
        let val = a_load(&raw const (*sem).__val[0]);
        if (val & SEM_VALUE_MAX) == SEM_VALUE_MAX { ERRNO!() = EOVERFLOW; return -1; }
        let mut new = val + 1;
        let waiters = a_load(&raw const (*sem).__val[1]);
        if waiters <= 1 { new &= !FUTEX_WAITER_BIT; }
        if a_cas(&raw mut (*sem).__val[0], val, new) == val {
            if val < 0 || waiters > 0 {
                futex_wake(&raw mut (*sem).__val[0], if waiters > 1 { 1 } else { -1 });
            }
            return 0;
        }
    }
}
#[no_mangle]
pub unsafe extern "C" fn sem_getvalue(sem: *mut sem_t, sval: *mut c_int) -> c_int {
    *sval = a_load(&raw const (*sem).__val[0]) & SEM_VALUE_MAX;
    0
}

