/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_XATTR_H
#define _KEEL_SYS_XATTR_H
#include <stddef.h>
ssize_t getxattr(const char *, const char *, void *, size_t);
ssize_t lgetxattr(const char *, const char *, void *, size_t);
ssize_t setxattr(const char *, const char *, const void *, size_t, int);
ssize_t lsetxattr(const char *, const char *, const void *, size_t, int);
int removexattr(const char *, const char *);
int lremovexattr(const char *, const char *);
ssize_t listxattr(const char *, char *, size_t);
ssize_t llistxattr(const char *, char *, size_t);


/* musl 1.2.6 compatibility macros */
#define XATTR_CREATE 1
#define XATTR_REPLACE 2
#endif
