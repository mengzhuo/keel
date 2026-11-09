#ifndef _CRABC_SYS_ACCT_H
#define _CRABC_SYS_ACCT_H
#include <sys/types.h>
struct acct { char ac_flag; char ac_version; uint16_t ac_uid, ac_gid; uint16_t ac_tty; uint32_t ac_btime; uint32_t ac_utime, ac_stime; uint32_t ac_etime; uint32_t ac_mem; uint32_t ac_io; uint32_t ac_rw; uint32_t ac_minflt, ac_majflt; uint32_t ac_swaps; uint32_t ac_exitcode; char ac_comm[17]; };
#define AFORK 1
#define ASU 2
#define ACORE 8

#endif
