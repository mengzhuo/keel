/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SHADOW_H
#define _SHADOW_H

#include <sys/types.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

struct spwd {
    char *sp_namp;
    char *sp_pwdp;
    long sp_lstchg;
    long sp_min;
    long sp_max;
    long sp_warn;
    long sp_inact;
    long sp_expire;
    unsigned long sp_flag;
};

void setspent(void);
void endspent(void);
struct spwd *getspent(void);
struct spwd *getspnam(const char *);
struct spwd *fgetspent(FILE *);
int putspent(const struct spwd *, FILE *);
int getspnam_r(const char *, struct spwd *, char *, size_t, struct spwd **);
int lckpwdf(void);
int ulckpwdf(void);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define SHADOW "/etc/shadow"

/* musl 1.2.6 per-header compatibility macros */
#define __LONG_MAX 0x7fffffffffffffffL
#define __LITTLE_ENDIAN 1234
#define __BYTE_ORDER 1234
#define __BIG_ENDIAN 4321
#endif
