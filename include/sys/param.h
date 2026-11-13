/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_PARAM_H
#define _KEEL_SYS_PARAM_H
#include <sys/resource.h>
#include <endian.h>
#include <limits.h>

/* musl 1.2.6 compatibility macros */
#define CANBSIZ 255
#define DEV_BSIZE 512
#define MAX(a, b) (((a)>(b))?(a):(b))
#define MAXHOSTNAMELEN 64
#define MAXNAMLEN 255
#define MAXPATHLEN 4096
#define MAXSYMLINKS 20
#define MIN(a, b) (((a)<(b))?(a):(b))
#define NBBY 8
#define NCARGS 131072
#define NGROUPS 32
#define NOFILE 256
#define NOGROUP (-1)
#define clrbit(x, i) __bitop(x,i,&=~)
#define howmany(n, d) (((n)+((d)-1))/(d))
#define isclr(x, i) !isset(x,i)
#define isset(x, i) __bitop(x,i,&)
#define powerof2(n) !(((n)-1) & (n))
#define roundup(n, d) (howmany(n,d)*(d))
#define setbit(x, i) __bitop(x,i,|=)

/* musl 1.2.6 compatibility helpers */
#define __bitop(x,i,o) ((x)[(i)/8] o (1<<(i)%8))

/* musl 1.2.6 per-header compatibility macros */
#define FD_SET(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] |= (1UL<<((d)%(8*sizeof(long)))))
#define NFDBITS (8*(int)sizeof(long))
#define TIMESPEC_TO_TIMEVAL(tv, ts) ( (tv)->tv_sec = (ts)->tv_sec, (tv)->tv_usec = (ts)->tv_nsec / 1000, (void)0 )
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define timercmp(s, t, op) ((s)->tv_sec == (t)->tv_sec ? (s)->tv_usec op (t)->tv_usec : (s)->tv_sec op (t)->tv_sec)
#define ITIMER_REAL 0
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define ITIMER_PROF 2
#define TIMEVAL_TO_TIMESPEC(tv, ts) ( (ts)->tv_sec = (tv)->tv_sec, (ts)->tv_nsec = (tv)->tv_usec * 1000, (void)0 )
#define FD_ISSET(d, s) !!((s)->fds_bits[(d)/(8*sizeof(long))] & (1UL<<((d)%(8*sizeof(long)))))
#define FD_SETSIZE 1024
#define timerisset(t) ((t)->tv_sec || (t)->tv_usec)
#define timersub(s, t, a) (void) ( (a)->tv_sec = (s)->tv_sec - (t)->tv_sec, ((a)->tv_usec = (s)->tv_usec - (t)->tv_usec) < 0 && ((a)->tv_usec += 1000000, (a)->tv_sec--) )
#define timerclear(t) ((t)->tv_sec = (t)->tv_usec = 0)
#define ITIMER_VIRTUAL 1
#define timeradd(s, t, a) (void) ( (a)->tv_sec = (s)->tv_sec + (t)->tv_sec, ((a)->tv_usec = (s)->tv_usec + (t)->tv_usec) >= 1000000 && ((a)->tv_usec -= 1000000, (a)->tv_sec++) )
#endif
