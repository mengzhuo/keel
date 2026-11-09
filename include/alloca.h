#ifndef _CRABC_ALLOCA_H
#define _CRABC_ALLOCA_H
#include <stdlib.h>

/* musl 1.2.6 compatibility macros */
#define alloca __builtin_alloca

/* musl 1.2.6 per-header compatibility macros */
#define __LONG_MAX 0x7fffffffffffffffL
#define __LITTLE_ENDIAN 1234
#define __BYTE_ORDER 1234
#define __BIG_ENDIAN 4321
#endif
