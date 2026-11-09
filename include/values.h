#ifndef _CRABC_VALUES_H
#define _CRABC_VALUES_H
#include <limits.h>
#include <float.h>
#define MAXSHORT SHRT_MAX
#define MAXINT INT_MAX
#define MAXLONG LONG_MAX
#define MAXCHAR CHAR_MAX
#define MAXDOUBLE DBL_MAX
#define MAXFLOAT FLT_MAX
#define HUGEDOUBLE DBL_MAX
#define HUGEFLOAT FLT_MAX


/* musl 1.2.6 compatibility macros */
#define BITSPERBYTE CHAR_BIT
#define CHARBITS (sizeof(char) * 8)
#define DMAXEXP DBL_MAX_EXP
#define DMINEXP DBL_MIN_EXP
#define DOUBLEBITS (sizeof(double) * 8)
#define FLOATBITS (sizeof(float) * 8)
#define FMAXEXP FLT_MAX_EXP
#define FMINEXP FLT_MIN_EXP
#define HIBITL MINLONG
#define HIBITS MINSHORT
#define INTBITS (sizeof(int) * 8)
#define LONGBITS (sizeof(long) * 8)
#define MINDOUBLE DBL_MIN
#define MINFLOAT FLT_MIN
#define MININT INT_MIN
#define MINLONG LONG_MIN
#define MINSHORT SHRT_MIN
#define PTRBITS (sizeof(char *) * 8)
#define SHORTBITS (sizeof(short) * 8)
#endif
