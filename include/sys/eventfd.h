/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_EVENTFD_H
#define _KEEL_SYS_EVENTFD_H
#include <stdint.h>
#include <fcntl.h>
int eventfd(unsigned int, int);
#define EFD_CLOEXEC O_CLOEXEC
#define EFD_NONBLOCK O_NONBLOCK
#define EFD_SEMAPHORE 1

#endif
