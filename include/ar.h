/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_AR_H
#define _CRABC_AR_H
#define ARMAG "!<arch>\n"
#define ARFMAG "`\n"
struct ar_hdr { char ar_name[16], ar_date[12], ar_uid[6], ar_gid[6], ar_mode[8], ar_size[10], ar_fmag[2]; };


/* musl 1.2.6 compatibility macros */
#define SARMAG 8
#endif
