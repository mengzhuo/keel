/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_SYS_TIMEX_H
#define _CRABC_SYS_TIMEX_H
#include <features.h>

/* musl 1.2.6 compatibility macros */
#define ADJ_ESTERROR 0x0008
#define ADJ_FREQUENCY 0x0002
#define ADJ_MAXERROR 0x0004
#define ADJ_MICRO 0x1000
#define ADJ_NANO 0x2000
#define ADJ_OFFSET 0x0001
#define ADJ_OFFSET_SINGLESHOT 0x8001
#define ADJ_OFFSET_SS_READ 0xa001
#define ADJ_SETOFFSET 0x0100
#define ADJ_STATUS 0x0010
#define ADJ_TAI 0x0080
#define ADJ_TICK 0x4000
#define ADJ_TIMECONST 0x0020
#define MAXTC 6
#define MOD_CLKA ADJ_OFFSET_SINGLESHOT
#define MOD_CLKB ADJ_TICK
#define MOD_ESTERROR ADJ_ESTERROR
#define MOD_FREQUENCY ADJ_FREQUENCY
#define MOD_MAXERROR ADJ_MAXERROR
#define MOD_MICRO ADJ_MICRO
#define MOD_NANO ADJ_NANO
#define MOD_OFFSET ADJ_OFFSET
#define MOD_STATUS ADJ_STATUS
#define MOD_TAI ADJ_TAI
#define MOD_TIMECONST ADJ_TIMECONST
#define STA_CLK 0x8000
#define STA_CLOCKERR 0x1000
#define STA_DEL 0x0020
#define STA_FLL 0x0008
#define STA_FREQHOLD 0x0080
#define STA_INS 0x0010
#define STA_MODE 0x4000
#define STA_NANO 0x2000
#define STA_PLL 0x0001
#define STA_PPSERROR 0x0800
#define STA_PPSFREQ 0x0002
#define STA_PPSJITTER 0x0200
#define STA_PPSSIGNAL 0x0100
#define STA_PPSTIME 0x0004
#define STA_PPSWANDER 0x0400
#define STA_RONLY (STA_PPSSIGNAL | STA_PPSJITTER | STA_PPSWANDER | STA_PPSERROR | STA_CLOCKERR | STA_NANO | STA_MODE | STA_CLK)
#define STA_UNSYNC 0x0040
#define TIME_BAD TIME_ERROR
#define TIME_DEL 2
#define TIME_ERROR 5
#define TIME_INS 1
#define TIME_OK 0
#define TIME_OOP 3
#define TIME_WAIT 4

/* musl 1.2.6 per-header compatibility macros */
#define FD_SET(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] |= (1UL<<((d)%(8*sizeof(long)))))
#define FD_ISSET(d, s) !!((s)->fds_bits[(d)/(8*sizeof(long))] & (1UL<<((d)%(8*sizeof(long)))))
#define timerclear(t) ((t)->tv_sec = (t)->tv_usec = 0)
#define FD_SETSIZE 1024
#define TIMESPEC_TO_TIMEVAL(tv, ts) ( (tv)->tv_sec = (ts)->tv_sec, (tv)->tv_usec = (ts)->tv_nsec / 1000, (void)0 )
#define ITIMER_PROF 2
#define timersub(s, t, a) (void) ( (a)->tv_sec = (s)->tv_sec - (t)->tv_sec, ((a)->tv_usec = (s)->tv_usec - (t)->tv_usec) < 0 && ((a)->tv_usec += 1000000, (a)->tv_sec--) )
#define timeradd(s, t, a) (void) ( (a)->tv_sec = (s)->tv_sec + (t)->tv_sec, ((a)->tv_usec = (s)->tv_usec + (t)->tv_usec) >= 1000000 && ((a)->tv_usec -= 1000000, (a)->tv_sec++) )
#define __BIG_ENDIAN 4321
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define timercmp(s, t, op) ((s)->tv_sec == (t)->tv_sec ? (s)->tv_usec op (t)->tv_usec : (s)->tv_sec op (t)->tv_sec)
#define ITIMER_REAL 0
#define __LITTLE_ENDIAN 1234
#define ITIMER_VIRTUAL 1
#define __LONG_MAX 0x7fffffffffffffffL
#define NFDBITS (8*(int)sizeof(long))
#define __BYTE_ORDER 1234
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define TIMEVAL_TO_TIMESPEC(tv, ts) ( (ts)->tv_sec = (tv)->tv_sec, (ts)->tv_nsec = (tv)->tv_usec * 1000, (void)0 )
#define timerisset(t) ((t)->tv_sec || (t)->tv_usec)
#endif
