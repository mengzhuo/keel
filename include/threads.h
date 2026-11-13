/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_THREADS_H
#define _KEEL_THREADS_H
#include <stddef.h>
#include <features.h>

/* musl 1.2.6 compatibility macros */
#define ONCE_FLAG_INIT 0
#define TSS_DTOR_ITERATIONS 4
#define thrd_equal(A, B) ((A) == (B))
#define thread_local _Thread_local

/* musl 1.2.6 per-header compatibility macros */
#define CLOCK_REALTIME_ALARM 8
#define CLOCK_SGI_CYCLE 10
#define TIMER_ABSTIME 1
#define CLOCK_MONOTONIC_COARSE 6
#define CLOCK_MONOTONIC 1
#define CLOCKS_PER_SEC 1000000L
#define CLOCK_REALTIME_COARSE 5
#define CLOCK_MONOTONIC_RAW 4
#define CLOCK_PROCESS_CPUTIME_ID 2
#define CLOCK_TAI 11
#define TIME_UTC 1
#define __BIG_ENDIAN 4321
#define __tm_zone tm_zone
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define CLOCK_THREAD_CPUTIME_ID 3
#define CLOCK_BOOTTIME 7
#define __BYTE_ORDER 1234
#define CLOCK_BOOTTIME_ALARM 9
#define __tm_gmtoff tm_gmtoff
#define CLOCK_REALTIME 0
#endif
