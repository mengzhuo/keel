/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SYS_TYPES_H
#define _SYS_TYPES_H

#include <features.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef __keel_ssize_t_defined
#define __keel_ssize_t_defined
#ifndef __DEFINED_ssize_t
#define __DEFINED_ssize_t
typedef long ssize_t;
#endif
#endif

#ifndef __keel_off_t_defined
#define __keel_off_t_defined
#ifndef __DEFINED_off_t
#define __DEFINED_off_t
typedef long off_t;
#endif
#endif

#ifndef __keel_pid_t_defined
#define __keel_pid_t_defined
#ifndef __DEFINED_pid_t
#define __DEFINED_pid_t
typedef int pid_t;
#endif
#endif

#ifndef __keel_id_t_defined
#define __keel_id_t_defined
#ifndef __DEFINED_id_t
#define __DEFINED_id_t
typedef unsigned int id_t;
#endif
#endif

#ifndef __keel_uid_t_defined
#define __keel_uid_t_defined
#ifndef __DEFINED_uid_t
#define __DEFINED_uid_t
typedef unsigned int uid_t;
#endif
#endif

#ifndef __keel_gid_t_defined
#define __keel_gid_t_defined
#ifndef __DEFINED_gid_t
#define __DEFINED_gid_t
typedef unsigned int gid_t;
#endif
#endif

#ifndef __keel_mode_t_defined
#define __keel_mode_t_defined
#ifndef __DEFINED_mode_t
#define __DEFINED_mode_t
typedef unsigned int mode_t;
#endif
#endif

#ifndef __keel_dev_t_defined
#define __keel_dev_t_defined
#ifndef __DEFINED_dev_t
#define __DEFINED_dev_t
typedef unsigned long long dev_t;
#endif
#endif

#ifndef __keel_ino_t_defined
#define __keel_ino_t_defined
#ifndef __DEFINED_ino_t
#define __DEFINED_ino_t
typedef unsigned long long ino_t;
#endif
#endif

#ifndef __keel_nlink_t_defined
#define __keel_nlink_t_defined
#ifndef __DEFINED_nlink_t
#define __DEFINED_nlink_t
#if defined(__aarch64__) || defined(__riscv)
typedef unsigned int nlink_t;
#else
typedef unsigned long nlink_t;
#endif
#endif
#endif

#ifndef __keel_blksize_t_defined
#define __keel_blksize_t_defined
#ifndef __DEFINED_blksize_t
#define __DEFINED_blksize_t
#if defined(__aarch64__) || defined(__riscv)
typedef int blksize_t;
#else
typedef long blksize_t;
#endif
#endif
#endif

#ifndef __keel_blkcnt_t_defined
#define __keel_blkcnt_t_defined
#ifndef __DEFINED_blkcnt_t
#define __DEFINED_blkcnt_t
typedef long blkcnt_t;
#endif
#endif

#ifndef __keel_fsblkcnt_t_defined
#define __keel_fsblkcnt_t_defined
#ifndef __DEFINED_fsblkcnt_t
#define __DEFINED_fsblkcnt_t
typedef unsigned long long fsblkcnt_t;
#endif
#endif

#ifndef __keel_fsfilcnt_t_defined
#define __keel_fsfilcnt_t_defined
#ifndef __DEFINED_fsfilcnt_t
#define __DEFINED_fsfilcnt_t
typedef unsigned long long fsfilcnt_t;
#endif
#endif

#ifndef __keel_key_t_defined
#define __keel_key_t_defined
#ifndef __DEFINED_key_t
#define __DEFINED_key_t
typedef int key_t;
#endif
#endif

#ifndef __keel_suseconds_t_defined
#define __keel_suseconds_t_defined
#ifndef __DEFINED_suseconds_t
#define __DEFINED_suseconds_t
typedef long suseconds_t;
#endif
#endif

#ifndef __keel_useconds_t_defined
#define __keel_useconds_t_defined
#ifndef __DEFINED_useconds_t
#define __DEFINED_useconds_t
typedef unsigned int useconds_t;
#endif
#endif

#ifndef __keel_time_t_defined
#define __keel_time_t_defined
#ifndef __DEFINED_time_t
#define __DEFINED_time_t
typedef long time_t;
#endif
#endif

#ifndef __keel_clock_t_defined
#define __keel_clock_t_defined
#ifndef __DEFINED_clock_t
#define __DEFINED_clock_t
typedef long clock_t;
#endif
#endif

#ifndef __keel_clockid_t_defined
#define __keel_clockid_t_defined
#ifndef __DEFINED_clockid_t
#define __DEFINED_clockid_t
typedef int clockid_t;
#endif
#endif

#ifndef __keel_timer_t_defined
#define __keel_timer_t_defined
#ifndef __DEFINED_timer_t
#define __DEFINED_timer_t
typedef void *timer_t;
#endif
#endif

#ifndef __keel_in_addr_t_defined
#define __keel_in_addr_t_defined
typedef unsigned int in_addr_t;
#endif

#ifndef __keel_in_port_t_defined
#define __keel_in_port_t_defined
typedef unsigned short in_port_t;
#endif

#ifndef __keel_sa_family_t_defined
#define __keel_sa_family_t_defined
#ifndef __DEFINED_sa_family_t
#define __DEFINED_sa_family_t
typedef unsigned short sa_family_t;
#endif
#endif

#ifndef __keel_socklen_t_defined
#define __keel_socklen_t_defined
#ifndef __DEFINED_socklen_t
#define __DEFINED_socklen_t
typedef unsigned int socklen_t;
#endif
#endif

/* pthread types */
#ifndef __keel_pthread_t_defined
#define __keel_pthread_t_defined
#ifndef __DEFINED_pthread_t
#define __DEFINED_pthread_t
typedef unsigned long pthread_t;
#endif
#endif

#ifndef __keel_pthread_once_t_defined
#define __keel_pthread_once_t_defined
#ifndef __DEFINED_pthread_once_t
#define __DEFINED_pthread_once_t
typedef int pthread_once_t;
#endif
#endif

#ifndef __keel_pthread_key_t_defined
#define __keel_pthread_key_t_defined
#ifndef __DEFINED_pthread_key_t
#define __DEFINED_pthread_key_t
typedef unsigned int pthread_key_t;
#endif
#endif

#ifndef __keel_pthread_spinlock_t_defined
#define __keel_pthread_spinlock_t_defined
#ifndef __DEFINED_pthread_spinlock_t
#define __DEFINED_pthread_spinlock_t
typedef int pthread_spinlock_t;
#endif
#endif

#ifndef __keel_pthread_mutexattr_t_defined
#define __keel_pthread_mutexattr_t_defined
#ifndef __DEFINED_pthread_mutexattr_t
#define __DEFINED_pthread_mutexattr_t
typedef struct { unsigned __attr; } pthread_mutexattr_t;
#endif
#endif

#ifndef __keel_pthread_condattr_t_defined
#define __keel_pthread_condattr_t_defined
#ifndef __DEFINED_pthread_condattr_t
#define __DEFINED_pthread_condattr_t
typedef struct { unsigned __attr; } pthread_condattr_t;
#endif
#endif

#ifndef __keel_pthread_barrierattr_t_defined
#define __keel_pthread_barrierattr_t_defined
#ifndef __DEFINED_pthread_barrierattr_t
#define __DEFINED_pthread_barrierattr_t
typedef struct { unsigned __attr; } pthread_barrierattr_t;
#endif
#endif

#ifndef __keel_pthread_rwlockattr_t_defined
#define __keel_pthread_rwlockattr_t_defined
#ifndef __DEFINED_pthread_rwlockattr_t
#define __DEFINED_pthread_rwlockattr_t
typedef struct { unsigned __attr[2]; } pthread_rwlockattr_t;
#endif
#endif

#ifndef __keel_pthread_attr_t_defined
#define __keel_pthread_attr_t_defined
#ifndef __DEFINED_pthread_attr_t
#define __DEFINED_pthread_attr_t
typedef struct { int __i[14]; } pthread_attr_t;
#endif
#endif

#ifndef __keel_pthread_mutex_t_defined
#define __keel_pthread_mutex_t_defined
#ifndef __DEFINED_pthread_mutex_t
#define __DEFINED_pthread_mutex_t
typedef struct { union { int __i[sizeof(long)==8?10:6]; volatile int __vi[sizeof(long)==8?10:6]; volatile void *volatile __p[sizeof(long)==8?5:6]; } __u; } pthread_mutex_t;
#endif
#endif

#ifndef __keel_pthread_cond_t_defined
#define __keel_pthread_cond_t_defined
#ifndef __DEFINED_pthread_cond_t
#define __DEFINED_pthread_cond_t
typedef struct { union { int __i[12]; volatile int __vi[12]; void *__p[12*sizeof(int)/sizeof(void*)]; } __u; } pthread_cond_t;
#endif
#endif

#ifndef __keel_pthread_rwlock_t_defined
#define __keel_pthread_rwlock_t_defined
#ifndef __DEFINED_pthread_rwlock_t
#define __DEFINED_pthread_rwlock_t
typedef struct { union { int __i[sizeof(long)==8?14:8]; volatile int __vi[sizeof(long)==8?14:8]; void *__p[sizeof(long)==8?7:8]; } __u; } pthread_rwlock_t;
#endif
#endif

#ifndef __keel_pthread_barrier_t_defined
#define __keel_pthread_barrier_t_defined
#ifndef __DEFINED_pthread_barrier_t
#define __DEFINED_pthread_barrier_t
typedef struct { int __i[8]; } pthread_barrier_t;
#endif
#endif

#ifndef NULL
#endif

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 per-header compatibility macros */
#define FD_SET(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] |= (1UL<<((d)%(8*sizeof(long)))))
#define be64toh(x) __bswap64(x)
#define __PDP_ENDIAN 3412
#define le32toh(x) (uint32_t)(x)
#define FD_ISSET(d, s) !!((s)->fds_bits[(d)/(8*sizeof(long))] & (1UL<<((d)%(8*sizeof(long)))))
#define htole16(x) (uint16_t)(x)
#define le16toh(x) (uint16_t)(x)
#define FD_SETSIZE 1024
#define htole64(x) (uint64_t)(x)
#define htobe16(x) __bswap16(x)
#define BYTE_ORDER __BYTE_ORDER
#define LITTLE_ENDIAN __LITTLE_ENDIAN
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define betoh64(x) __bswap64(x)
#define letoh16(x) (uint16_t)(x)
#define letoh64(x) (uint64_t)(x)
#define be32toh(x) __bswap32(x)
#define htole32(x) (uint32_t)(x)
#define be16toh(x) __bswap16(x)
#define htobe32(x) __bswap32(x)
#define NFDBITS (8*(int)sizeof(long))
#define PDP_ENDIAN __PDP_ENDIAN
#define htobe64(x) __bswap64(x)
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define BIG_ENDIAN __BIG_ENDIAN
#define betoh32(x) __bswap32(x)
#define letoh32(x) (uint32_t)(x)
#define le64toh(x) (uint64_t)(x)
#define betoh16(x) __bswap16(x)
#endif
