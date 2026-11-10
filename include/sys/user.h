/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_SYS_USER_H
#define _CRABC_SYS_USER_H
#include <unistd.h>
#include <limits.h>
#include <stdint.h>
struct user_regs_struct { unsigned long r15,r14,r13,r12,rbp,rbx,r11,r10,r9,r8,rax,rcx,rdx,rsi,rdi,orig_rax,rip,cs,eflags,rsp,ss,fs_base,gs_base,ds,es,fs,gs; };


/* musl 1.2.6 compatibility macros */
#if defined(__x86_64__)
#define ELF_NGREG 27
#define HOST_STACK_END_ADDR (u.start_stack + u.u_ssize * NBPG)
#define HOST_TEXT_START_ADDR (u.start_code)
#define NBPG PAGESIZE
#define PAGE_MASK (~(PAGESIZE-1))
#define UPAGES 1
#elif defined(__riscv)
#define ELF_NGREG 32
#endif

/* musl 1.2.6 architecture macros */
#ifdef __aarch64__
#define ELF_NREG 34
#endif

/* musl 1.2.6 architecture macros */
#ifdef __riscv
#define ELF_NFPREG 33
#endif

/* musl 1.2.6 per-header compatibility macros */
#define L_XTND 2
#define L_INCR 1
#define __WORDSIZE 64
#define L_SET 0
#if defined(__riscv)
#define si_pkey __si_fields.__sigfault.__first.si_pkey
#define si_pid __si_fields.__si_common.__first.__piduid.si_pid
#define SIG_SETMASK 2
#define SI_ASYNCIO (-4)
#define POLL_PRI 5
#define SA_NOCLDWAIT 2
#define si_syscall __si_fields.__sigsys.si_syscall
#define si_addr_lsb __si_fields.__sigfault.si_addr_lsb
#define SIGEV_THREAD_ID 4
#define SA_NOMASK SA_NODEFER
#define SIGUNUSED SIGSYS
#define SIGTRAP 5
#define BUS_MCEERR_AR 4
#define ILL_ILLTRP 4
#define SIGSTKFLT 16
#define SIGSEGV 11
#define CLD_STOPPED 5
#define SI_KERNEL 128
#define SIGUSR2 12
#define FPE_FLTINV 7
#define SEGV_ACCERR 2
#define ILL_ILLOPN 2
#define SIGQUIT 3
#define SIG_BLOCK 0
#define CLD_EXITED 1
#define __ucontext ucontext
#define si_stime __si_fields.__si_common.__second.__sigchld.si_stime
#define SI_TIMER (-2)
#define SIGILL 4
#define SA_ONSTACK 0x08000000
#define SIGBUS 7
#define CLD_CONTINUED 6
#define BUS_OBJERR 3
#define BUS_ADRERR 2
#define SEGV_PKUERR 4
#define SA_UNSUPPORTED 0x00000400
#define FPE_FLTRES 6
#define REG_S1 9
#define FPE_FLTDIV 3
#define SIGINT 2
#define FPE_FLTOVF 4
#define SIGPIPE 13
#define REG_RA 1
#define FPE_FLTUND 5
#define si_band __si_fields.__sigpoll.si_band
#define CLD_DUMPED 3
#define si_ptr si_value.sival_ptr
#define SI_QUEUE (-1)
#define SIGUSR1 10
#define BUS_ADRALN 1
#define SIG_DFL ((void (*)(int)) 0)
#define SEGV_MTESERR 9
#define SIGALRM 14
#define si_lower __si_fields.__sigfault.__first.__addr_bnd.si_lower
#define SA_ONESHOT SA_RESETHAND
#define TRAP_UNK 5
#define si_value __si_fields.__si_common.__second.si_value
#define SIGWINCH 28
#define si_int si_value.sival_int
#define SYS_SECCOMP 1
#define sa_handler __sa_handler.sa_handler
#define SIGSTOP 19
#define si_addr __si_fields.__sigfault.si_addr
#define SIGSYS 31
#define ILL_PRVREG 6
#define SIG_HOLD ((void (*)(int)) 2)
#define SI_MESGQ (-3)
#define ILL_COPROC 7
#define REG_A0 10
#define TRAP_HWBKPT 4
#define ILL_ILLADR 3
#define SA_EXPOSE_TAGBITS 0x00000800
#define POLL_ERR 4
#define SIGVTALRM 26
#define SIGPWR 30
#define SIGEV_THREAD 2
#define SIGPOLL SIGIO
#define TRAP_TRACE 2
#define SIGIOT SIGABRT
#define SEGV_MTEAERR 8
#define SIGFPE 8
#define sa_sigaction __sa_handler.sa_sigaction
#define SI_ASYNCNL (-60)
#define si_uid __si_fields.__si_common.__first.__piduid.si_uid
#define BUS_MCEERR_AO 5
#define TRAP_BRANCH 3
#define REG_PC 0
#define SIGEV_SIGNAL 0
#define CLD_TRAPPED 4
#define SS_ONSTACK 1
#define SIG_IGN ((void (*)(int)) 1)
#define SA_SIGINFO 4
#define SI_SIGIO (-5)
#define SIGCONT 18
#define SIGKILL 9
#define SI_TKILL (-6)
#define si_status __si_fields.__si_common.__second.__sigchld.si_status
#define SIGTTOU 22
#define _NSIG 65
#define SIGTSTP 20
#define si_upper __si_fields.__sigfault.__first.__addr_bnd.si_upper
#define si_utime __si_fields.__si_common.__second.__sigchld.si_utime
#define SIGXFSZ 25
#define SIGCHLD 17
#define REG_S0 8
#define SIGXCPU 24
#define POLL_OUT 2
#define SS_FLAG_BITS SS_AUTODISARM
#define POLL_HUP 6
#define REG_S2 18
#define REG_TP 4
#define SIGEV_NONE 1
#define NSIG _NSIG
#define SS_DISABLE 2
#define FPE_INTDIV 1
#define SIGURG 23
#define SEGV_MAPERR 1
#define sigev_notify_thread_id __sev_fields.sigev_notify_thread_id
#define SI_USER 0
#define SA_RESETHAND 0x80000000
#define SA_NOCLDSTOP 1
#define SIGPROF 27
#define ILL_PRVOPC 5
#define ILL_ILLOPC 1
#define MINSIGSTKSZ 2048
#define SIG_ERR ((void (*)(int))-1)
#define ILL_BADSTK 8
#define sigev_notify_function __sev_fields.__sev_thread.sigev_notify_function
#define SIGRTMIN (__libc_current_sigrtmin())
#define CLD_KILLED 2
#define si_call_addr __si_fields.__sigsys.si_call_addr
#define SIGTERM 15
#define SS_AUTODISARM (1U << 31)
#define SIGHUP 1
#define SIGIO 29
#define SIGSTKSZ 8192
#define FPE_FLTSUB 8
#define si_fd __si_fields.__sigpoll.si_fd
#define SA_NODEFER 0x40000000
#define SYS_USER_DISPATCH 2
#define SA_RESTART 0x10000000
#define SIGTTIN 21
#define SEGV_BNDERR 3
#define FPE_INTOVF 2
#define SIGRTMAX (__libc_current_sigrtmax())
#define REG_SP 2
#define si_timerid __si_fields.__si_common.__first.__timer.si_timerid
#define POLL_IN 1
#define TRAP_BRKPT 1
#define sigev_notify_attributes __sev_fields.__sev_thread.sigev_notify_attributes
#define si_arch __si_fields.__sigsys.si_arch
#define SIG_UNBLOCK 1
#define SIGABRT 6
#define si_overrun __si_fields.__si_common.__first.__timer.si_overrun
#define POLL_MSG 3
#endif
#endif
