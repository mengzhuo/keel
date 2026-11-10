/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SYS_UTSNAME_H
#define _SYS_UTSNAME_H
#include <features.h>

#ifdef __cplusplus
extern "C" {
#endif

struct utsname {
    char sysname[65];
    char nodename[65];
    char release[65];
    char version[65];
    char machine[65];
    char domainname[65];
};

int uname(struct utsname *);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 per-header compatibility macros */
#endif
