#ifndef _CRABC_SYS_SYSLOG_H
#define _CRABC_SYS_SYSLOG_H
#include <features.h>
#include <syslog.h>

/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
