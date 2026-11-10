/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _GLOB_H
#define _GLOB_H

#include <features.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    size_t gl_pathc;
    char **gl_pathv;
    size_t gl_offs;
    int __dummy1;
    void *__dummy2[5];
} glob_t;

int glob(const char *__restrict, int, int (*)(const char *, int), glob_t *__restrict);
void globfree(glob_t *);

#define GLOB_ERR       0x01
#define GLOB_MARK      0x02
#define GLOB_NOSORT    0x04
#define GLOB_DOOFFS    0x08
#define GLOB_NOCHECK   0x10
#define GLOB_APPEND    0x20
#define GLOB_NOESCAPE  0x40
#define GLOB_PERIOD    0x80
#define GLOB_ONLYDIR   0x200
#define GLOB_BRACE     0x800
#define GLOB_TILDE     0x1000
#define GLOB_TILDE_CHECK 0x4000

#define GLOB_NOSPACE 1
#define GLOB_ABORTED 2
#define GLOB_NOMATCH 3
#define GLOB_NOSYS   4

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
