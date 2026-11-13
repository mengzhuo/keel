/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_ARPA_FTP_H
#define _KEEL_ARPA_FTP_H

/* musl 1.2.6 compatibility macros */
#define BLK_BYTECOUNT 2
#define BLK_EOF 0x40
#define BLK_EOR 0x80
#define BLK_ERRORS 0x20
#define BLK_RESTART 0x10
#define COMPLETE 2
#define CONTINUE 3
#define ERROR 05
#define FORM_C 3
#define FORM_N 1
#define FORM_T 2
#define MODE_B 2
#define MODE_C 3
#define MODE_S 1
#define PRELIM 1
#define REC_EOF '\002'
#define REC_EOR '\001'
#define REC_ESC '\377'
#define STRU_F 1
#define STRU_P 3
#define STRU_R 2
#define TRANSIENT 4
#define TYPE_A 1
#define TYPE_E 2
#define TYPE_I 3
#define TYPE_L 4
#endif
