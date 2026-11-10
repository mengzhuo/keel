/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SYS_RESOURCE_H
#define _SYS_RESOURCE_H

#ifdef __cplusplus
extern "C" {
#endif

#include <sys/types.h>
#include <time.h>

typedef unsigned long rlim_t;

struct rlimit {
    rlim_t rlim_cur;
    rlim_t rlim_max;
};

struct rusage {
    struct timeval ru_utime;
    struct timeval ru_stime;
    long ru_maxrss;
    long ru_ixrss;
    long ru_idrss;
    long ru_isrss;
    long ru_minflt;
    long ru_majflt;
    long ru_nswap;
    long ru_inblock;
    long ru_oublock;
    long ru_msgsnd;
    long ru_msgrcv;
    long ru_nsignals;
    long ru_nvcsw;
    long ru_nivcsw;
    long __reserved[16];
};

#define PRIO_MIN (-20)
#define PRIO_MAX 20

#define PRIO_PROCESS 0
#define PRIO_PGRP    1
#define PRIO_USER    2

#define RUSAGE_SELF     0
#define RUSAGE_CHILDREN (-1)
#define RUSAGE_THREAD   1

#define RLIM_INFINITY   (~0ULL)
#define RLIM_SAVED_CUR  RLIM_INFINITY
#define RLIM_SAVED_MAX  RLIM_INFINITY

#define RLIMIT_CPU      0
#define RLIMIT_FSIZE    1
#define RLIMIT_DATA     2
#define RLIMIT_STACK    3
#define RLIMIT_CORE     4
#define RLIMIT_RSS      5
#define RLIMIT_NPROC    6
#define RLIMIT_NOFILE   7
#define RLIMIT_MEMLOCK  8
#define RLIMIT_AS       9
#define RLIMIT_LOCKS    10
#define RLIMIT_SIGPENDING 11
#define RLIMIT_MSGQUEUE 12
#define RLIMIT_NICE     13
#define RLIMIT_RTPRIO   14
#define RLIMIT_RTTIME   15
#define RLIMIT_NLIMITS  16
#define RLIM_NLIMITS    RLIMIT_NLIMITS

int getrlimit(int, struct rlimit *);
int setrlimit(int, const struct rlimit *);
int getrusage(int, struct rusage *);

int getpriority(int, id_t);
int setpriority(int, id_t, int);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define prlimit64 prlimit

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
