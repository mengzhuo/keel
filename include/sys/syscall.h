#ifndef _CRABC_SYS_SYSCALL_H
#define _CRABC_SYS_SYSCALL_H
#include <sys/types.h>
#include <bits/syscall.h>
long syscall(long, ...);

/* musl 1.2.6 compatibility macros */
#define SYS_file_getattr 468
#define SYS_file_setattr 469
#define SYS_getxattrat 464
#define SYS_listns 470
#define SYS_listxattrat 465
#define SYS_open_tree_attr 467
#define SYS_removexattrat 466
#define SYS_setxattrat 463
#if defined(__x86_64__)
#define SYS_uprobe 336
#elif defined(__aarch64__)
#define SYS_fadvise64 223
#define SYS_fcntl 25
#define SYS_fstat 80
#define SYS_fstatfs 44
#define SYS_ftruncate 46
#define SYS_listmount 458
#define SYS_lseek 62
#define SYS_mmap 222
#define SYS_name_to_handle_at 264
#define SYS_newfstatat 79
#define SYS_open_by_handle_at 265
#define SYS_sendfile 71
#define SYS_statfs 43
#define SYS_statmount 457
#define SYS_truncate 45
#elif defined(__riscv)
#define SYS_fadvise64 223
#define SYS_fcntl 25
#define SYS_fstat 80
#define SYS_fstatfs 44
#define SYS_ftruncate 46
#define SYS_listmount 458
#define SYS_lseek 62
#define SYS_mmap 222
#define SYS_name_to_handle_at 264
#define SYS_newfstatat 79
#define SYS_open_by_handle_at 265
#define SYS_sendfile 71
#define SYS_statfs 43
#define SYS_statmount 457
#define SYS_truncate 45
#endif

#endif
