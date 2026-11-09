#ifndef _CRABC_SYS_REG_H
#define _CRABC_SYS_REG_H
#include <limits.h>
#include <unistd.h>
#include <bits/reg.h>

/* musl 1.2.6 compatibility macros */
#if defined(__x86_64__)
#define CS 17
#define DS 23
#define EFLAGS 18
#define ES 24
#define FS 25
#define FS_BASE 21
#define GS 26
#define GS_BASE 22
#define ORIG_RAX 15
#define R10 7
#define R11 6
#define R12 3
#define R13 2
#define R14 1
#define R15 0
#define R8 9
#define R9 8
#define RAX 10
#define RBP 4
#define RBX 5
#define RCX 11
#define RDI 14
#define RDX 12
#define RIP 16
#define RSI 13
#define RSP 19
#define SS 20
#endif

/* musl 1.2.6 per-header compatibility macros */
#define L_XTND 2
#define L_INCR 1
#define __WORDSIZE 64
#define L_SET 0
#endif
