/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SETJMP_H
#define _SETJMP_H
#include <features.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * musl's jmp_buf is a struct, not just the callee-saved register area:
 *   __jb[__JMP_BUF_SIZE] + __fl + __ss[128/sizeof(long)]
 * sigjmp_buf is the same type.  Sizing this exactly matters: GCC allocates
 * the caller's stack object from this definition, while sigsetjmp writes the
 * saved mask and rbx/x19 at the end of it.
 */
#if defined(__aarch64__)
#define __JMP_BUF_SIZE 22
typedef unsigned long __jmp_buf_element;
#elif defined(__riscv)
#define __JMP_BUF_SIZE 26
typedef unsigned long __jmp_buf_element;
#elif defined(__powerpc64__)
/* IBM double-double long double; musl uses unsigned __int128[32]. */
#define __JMP_BUF_SIZE 32
typedef unsigned __int128 __jmp_buf_element;
#elif defined(__loongarch__)
#define __JMP_BUF_SIZE 23
typedef unsigned long __jmp_buf_element;
#else
#define __JMP_BUF_SIZE 8
typedef unsigned long __jmp_buf_element;
#endif

typedef struct __jmp_buf_tag {
    __jmp_buf_element __jb[__JMP_BUF_SIZE];
    unsigned long __fl;
    unsigned long __ss[128 / sizeof(long)];
} jmp_buf[1];

typedef jmp_buf sigjmp_buf;

#if defined(__GNUC__) && (__GNUC__ > 4 || (__GNUC__ == 4 && __GNUC_MINOR__ >= 1))
#define __setjmp_attr __attribute__((__returns_twice__))
#else
#define __setjmp_attr
#endif

int setjmp(jmp_buf) __setjmp_attr;
void longjmp(jmp_buf, int) __attribute__((__noreturn__));
int sigsetjmp(sigjmp_buf, int) __setjmp_attr;
void siglongjmp(sigjmp_buf, int) __attribute__((__noreturn__));
int _setjmp(jmp_buf) __setjmp_attr;
void _longjmp(jmp_buf, int) __attribute__((__noreturn__));

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define setjmp setjmp

/* musl 1.2.6 per-header compatibility macros */
#endif
