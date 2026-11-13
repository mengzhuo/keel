/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_SWAP_H
#define _KEEL_SYS_SWAP_H
#include <sys/types.h>
int swapon(const char *, int);
int swapoff(const char *);
#define SWAP_FLAG_PREFER 0x8000


/* musl 1.2.6 compatibility macros */
#define SWAP_FLAG_DISCARD 0x10000
#define SWAP_FLAG_PRIO_MASK 0x7fff
#define SWAP_FLAG_PRIO_SHIFT 0
#endif
