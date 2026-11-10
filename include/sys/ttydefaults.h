/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_SYS_TTYDEFAULTS_H
#define _CRABC_SYS_TTYDEFAULTS_H


/* musl 1.2.6 compatibility macros */
#define CBRK CEOL
#define CDISCARD CTRL('o')
#define CDSUSP CTRL('y')
#define CEOF CTRL('d')
#define CEOL '\0'
#define CEOT CEOF
#define CERASE 0177
#define CFLUSH CDISCARD
#define CINTR CTRL('c')
#define CKILL CTRL('u')
#define CLNEXT CTRL('v')
#define CMIN 1
#define CQUIT 034
#define CREPRINT CTRL('r')
#define CRPRNT CREPRINT
#define CSTART CTRL('q')
#define CSTATUS '\0'
#define CSTOP CTRL('s')
#define CSUSP CTRL('z')
#define CTIME 0
#define CTRL(x) ((x)&037)
#define CWERASE CTRL('w')
#define TTYDEF_CFLAG (CREAD | CS7 | PARENB | HUPCL)
#define TTYDEF_IFLAG (BRKINT | ISTRIP | ICRNL | IMAXBEL | IXON | IXANY)
#define TTYDEF_LFLAG (ECHO | ICANON | ISIG | IEXTEN | ECHOE|ECHOKE|ECHOCTL)
#define TTYDEF_OFLAG (OPOST | ONLCR | XTABS)
#define TTYDEF_SPEED (B9600)
#endif
