#ifndef _CRABC_NETINET_IN_SYSTM_H
#define _CRABC_NETINET_IN_SYSTM_H
#include <stdint.h>
typedef uint16_t n_short;
typedef uint32_t n_long;


/* musl 1.2.6 per-header compatibility macros */
#define __LONG_MAX 0x7fffffffffffffffL
#define __LITTLE_ENDIAN 1234
#define __BYTE_ORDER 1234
#define __BIG_ENDIAN 4321
#endif
