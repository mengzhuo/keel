/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_SYS_IO_H
#define _CRABC_SYS_IO_H
#include <features.h>
#include <sys/types.h>
int ioperm(unsigned long, unsigned long, int);
int iopl(int);


/* musl 1.2.6 per-header compatibility macros */
#endif
