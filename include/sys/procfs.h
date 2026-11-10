/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_SYS_PROCFS_H
#define _CRABC_SYS_PROCFS_H
#include <sys/time.h>
#include <sys/user.h>
#include <signal.h>
#include <sys/types.h>
typedef unsigned long elf_greg_t;


/* musl 1.2.6 compatibility macros */
#define ELF_PRARGSZ 80

/* musl 1.2.6 per-header compatibility macros */
#define be64toh(x) __bswap64(x)
#define L_SET 0
#define __PDP_ENDIAN 3412
#define le32toh(x) (uint32_t)(x)
#define htole16(x) (uint16_t)(x)
#define le16toh(x) (uint16_t)(x)
#define L_XTND 2
#define htole64(x) (uint64_t)(x)
#define __WORDSIZE 64
#define htobe16(x) __bswap16(x)
#define BYTE_ORDER __BYTE_ORDER
#define LITTLE_ENDIAN __LITTLE_ENDIAN
#define betoh64(x) __bswap64(x)
#define letoh16(x) (uint16_t)(x)
#define letoh64(x) (uint64_t)(x)
#define be32toh(x) __bswap32(x)
#define htole32(x) (uint32_t)(x)
#define be16toh(x) __bswap16(x)
#define htobe32(x) __bswap32(x)
#define PDP_ENDIAN __PDP_ENDIAN
#define htobe64(x) __bswap64(x)
#define BIG_ENDIAN __BIG_ENDIAN
#define L_INCR 1
#define betoh32(x) __bswap32(x)
#define letoh32(x) (uint32_t)(x)
#define le64toh(x) (uint64_t)(x)
#define betoh16(x) __bswap16(x)
#endif
