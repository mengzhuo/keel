/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_SYSMACROS_H
#define _KEEL_SYS_SYSMACROS_H

/* musl 1.2.6 compatibility macros */
#define major(x) ((unsigned)( (((x)>>31>>1) & 0xfffff000) | (((x)>>8) & 0x00000fff) ))
#define makedev(x, y) ( (((x)&0xfffff000ULL) << 32) | (((x)&0x00000fffULL) << 8) | (((y)&0xffffff00ULL) << 12) | (((y)&0x000000ffULL)) )
#define minor(x) ((unsigned)( (((x)>>12) & 0xffffff00) | ((x) & 0x000000ff) ))
#endif
