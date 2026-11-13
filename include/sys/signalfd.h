/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_SYS_SIGNALFD_H
#define _KEEL_SYS_SIGNALFD_H
#include <signal.h>
#include <stdint.h>
#include <fcntl.h>
struct signalfd_siginfo { uint32_t ssi_signo, ssi_errno, ssi_code, ssi_pid, ssi_uid, ssi_fd, ssi_tid, ssi_band, ssi_overrun, ssi_trapno, ssi_status, ssi_int, ssi_ptr, ssi_utime, ssi_stime, ssi_addr, ssi_addr_lsb; };
int signalfd(int, const sigset_t *, int);
#define SFD_CLOEXEC O_CLOEXEC
#define SFD_NONBLOCK O_NONBLOCK
#endif
