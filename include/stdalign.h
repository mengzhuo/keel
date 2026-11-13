/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_STDALIGN_H
#define _KEEL_STDALIGN_H

/* musl 1.2.6 compatibility macros */
#define alignas _Alignas
#define alignof _Alignof

/* musl 1.2.6 compatibility macros */
#define __alignas_is_defined 1
#define __alignof_is_defined 1
#endif
