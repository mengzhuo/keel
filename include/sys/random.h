/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_RANDOM_H
#define _KEEL_SYS_RANDOM_H
#include <stddef.h>
ssize_t getrandom(void *, size_t, unsigned);
#define GRND_NONBLOCK 1
#define GRND_RANDOM 2


/* musl 1.2.6 compatibility macros */
#define GRND_INSECURE 0x0004
#endif
