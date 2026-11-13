/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SYS_UIO_H
#define _SYS_UIO_H
#include <stddef.h>
#include <sys/types.h>
#ifndef __keel_struct_iovec_defined
#define __keel_struct_iovec_defined
struct iovec { void *iov_base; size_t iov_len; };
#endif
ssize_t readv(int, const struct iovec *, int);
ssize_t writev(int, const struct iovec *, int);
ssize_t preadv(int, const struct iovec *, int, off_t);
ssize_t pwritev(int, const struct iovec *, int, off_t);
ssize_t preadv2(int, const struct iovec *, int, off_t, int);
ssize_t pwritev2(int, const struct iovec *, int, off_t, int);

/* musl 1.2.6 compatibility macros */
#define RWF_APPEND 0x00000010
#define RWF_DSYNC 0x00000002
#define RWF_HIPRI 0x00000001
#define RWF_NOAPPEND 0x00000020
#define RWF_NOWAIT 0x00000008
#define RWF_SYNC 0x00000004
#define UIO_MAXIOV 1024
#endif
