/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_UCONTEXT_H
#define _KEEL_UCONTEXT_H
#include <features.h>
#include <signal.h>

/* musl 1.2.6 compatibility macros */
#define NGREG (sizeof(gregset_t)/sizeof(greg_t))
#endif
