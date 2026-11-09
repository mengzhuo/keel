#ifndef _STDARG_H
#define _STDARG_H
typedef __builtin_va_list va_list;
#define va_start(v,l) __builtin_va_start(v,l)
#define va_end(v) __builtin_va_end(v)
#define va_arg(v,l) __builtin_va_arg(v,l)
#define va_copy(d,s) __builtin_va_copy(d,s)

/* musl 1.2.6 per-header compatibility macros */
#define __LONG_MAX 0x7fffffffffffffffL
#define __LITTLE_ENDIAN 1234
#define __BYTE_ORDER 1234
#define __BIG_ENDIAN 4321
#endif
