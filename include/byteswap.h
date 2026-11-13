/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_BYTESWAP_H
#define _KEEL_BYTESWAP_H
#include <features.h>
#include <stdint.h>

static __inline uint16_t __bswap_16(uint16_t __x)
{
    return __x << 8 | __x >> 8;
}

static __inline uint32_t __bswap_32(uint32_t __x)
{
    return __x >> 24 | __x >> 8 & 0xff00 | __x << 8 & 0xff0000 | __x << 24;
}

static __inline uint64_t __bswap_64(uint64_t __x)
{
    return __bswap_32(__x) + 0ULL << 32 | __bswap_32(__x >> 32);
}

#define bswap_16(x) __bswap_16(x)
#define bswap_32(x) __bswap_32(x)
#define bswap_64(x) __bswap_64(x)


/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define __BIG_ENDIAN 4321
#endif
