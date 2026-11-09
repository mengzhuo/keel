#ifndef _CRABC_SYS_ACCT_H
#define _CRABC_SYS_ACCT_H
#include <endian.h>
#include <sys/types.h>
struct acct { char ac_flag; char ac_version; uint16_t ac_uid, ac_gid; uint16_t ac_tty; uint32_t ac_btime; uint32_t ac_utime, ac_stime; uint32_t ac_etime; uint32_t ac_mem; uint32_t ac_io; uint32_t ac_rw; uint32_t ac_minflt, ac_majflt; uint32_t ac_swaps; uint32_t ac_exitcode; char ac_comm[17]; };
#define AFORK 1
#define ASU 2
#define ACORE 8


/* musl 1.2.6 compatibility macros */
#define ACCT_BYTEORDER (128*(__BYTE_ORDER==__BIG_ENDIAN))
#define ACCT_COMM 16
#define AHZ 100
#define AXSIG 16

/* musl 1.2.6 per-header compatibility macros */
#define TIMER_ABSTIME 1
#define CLOCK_REALTIME_ALARM 8
#define CLOCK_BOOTTIME 7
#define CLOCK_MONOTONIC 1
#define CLOCKS_PER_SEC 1000000L
#define CLOCK_SGI_CYCLE 10
#define __tm_gmtoff tm_gmtoff
#define CLOCK_BOOTTIME_ALARM 9
#define CLOCK_REALTIME_COARSE 5
#define __tm_zone tm_zone
#define CLOCK_MONOTONIC_RAW 4
#define CLOCK_PROCESS_CPUTIME_ID 2
#define CLOCK_TAI 11
#define CLOCK_THREAD_CPUTIME_ID 3
#define TIME_UTC 1
#define CLOCK_REALTIME 0
#define CLOCK_MONOTONIC_COARSE 6
#endif
