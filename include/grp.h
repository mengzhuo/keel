/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _GRP_H
#define _GRP_H

#include <features.h>
#include <sys/types.h>

#ifdef _GNU_SOURCE
#include <stdio.h>
#endif

struct group {
    char *gr_name;
    char *gr_passwd;
    gid_t gr_gid;
    char **gr_mem;
};

struct group *getgrgid(gid_t);
struct group *getgrnam(const char *);
int getgrgid_r(gid_t, struct group *, char *, size_t, struct group **);
int getgrnam_r(const char *, struct group *, char *, size_t, struct group **);
struct group *getgrent(void);
void setgrent(void);
void endgrent(void);
#ifdef _GNU_SOURCE
struct group *fgetgrent(FILE *);
int putgrent(const struct group *, FILE *);
#endif
int getgrouplist(const char *, gid_t, gid_t *, int *);
int setgroups(size_t, const gid_t *);
int initgroups(const char *, gid_t);


/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif