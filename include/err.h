#ifndef _CRABC_ERR_H
#define _CRABC_ERR_H
#include <features.h>

/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define va_start(v, l) __builtin_va_start(v,l)
#define va_arg(v, l) __builtin_va_arg(v,l)
#define va_end(v) __builtin_va_end(v)
#define __LITTLE_ENDIAN 1234
#define va_copy(d, s) __builtin_va_copy(d,s)
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
