/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _REGEX_H
#define _REGEX_H

#include <features.h>
#include <stddef.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef long regoff_t;
typedef struct {
    size_t re_nsub;
    void *__opaque;
    void *__padding[4];
    size_t __nsub2;
    char __padding2;
} regex_t;
typedef struct {
    regoff_t rm_so;
    regoff_t rm_eo;
} regmatch_t;

#define REG_EXTENDED 1
#define REG_ICASE 2
#define REG_NEWLINE 4
#define REG_NOSUB 8
#define REG_NOTBOL 1
#define REG_NOTEOL 2
#define REG_NOMATCH 1
#define REG_BADPAT 2
#define REG_ECOLLATE 3
#define REG_ECTYPE 4
#define REG_EESCAPE 5
#define REG_ESUBREG 6
#define REG_EBRACK 7
#define REG_EPAREN 8
#define REG_EBRACE 9
#define REG_BADBR 10
#define REG_ERANGE 11
#define REG_ESPACE 12
#define REG_BADRPT 13

int regcomp(regex_t *, const char *, int);
int regexec(const regex_t *, const char *, size_t, regmatch_t[], int);
size_t regerror(int, const regex_t *, char *, size_t);
void regfree(regex_t *);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define REG_ENOSYS -1
#define REG_OK 0

/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
