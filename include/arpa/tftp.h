/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_ARPA_TFTP_H
#define _CRABC_ARPA_TFTP_H

/* musl 1.2.6 compatibility macros */
#define ACK 04
#define DATA 03
#define EACCESS 2
#define EBADID 5
#define EBADOP 4
#define EEXISTS 6
#define ENOSPACE 3
#define ENOTFOUND 1
#define ENOUSER 7
#define EUNDEF 0
#define RRQ 01
#define SEGSIZE 512
#define WRQ 02
#define th_block th_u.tu_block
#define th_code th_u.tu_code
#define th_msg th_data
#define th_stuff th_u.tu_stuff

/* musl 1.2.6 per-header compatibility macros */
#define ERROR 05
#endif
