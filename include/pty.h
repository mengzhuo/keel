/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_PTY_H
#define _KEEL_PTY_H
#include <features.h>
#include <termios.h>
#include <sys/ioctl.h>

#ifdef __cplusplus
extern "C" {
#endif

int openpty(int *, int *, char *, const struct termios *, const struct winsize *);
int forkpty(int *, char *, const struct termios *, const struct winsize *);

#ifdef __cplusplus
}
#endif

/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
