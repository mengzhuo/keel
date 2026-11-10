/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _IFADDRS_H
#define _IFADDRS_H

#include <features.h>
#include <netinet/in.h>
#include <sys/types.h>
#include <sys/socket.h>

#ifdef __cplusplus
extern "C" {
#endif

struct ifaddrs {
    struct ifaddrs *ifa_next;
    char *ifa_name;
    unsigned int ifa_flags;
    struct sockaddr *ifa_addr;
    struct sockaddr *ifa_netmask;
    union {
        struct sockaddr *ifu_broadaddr;
        struct sockaddr *ifu_dstaddr;
    } ifa_ifu;
    void *ifa_data;
};

#define ifa_broadaddr ifa_ifu.ifu_broadaddr
#define ifa_dstaddr   ifa_ifu.ifu_dstaddr

int getifaddrs(struct ifaddrs **);
void freeifaddrs(struct ifaddrs *);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 per-header compatibility macros */
#define SCNuFAST16 "u"
#define SCNuPTR __PRIPTR "u"
#define SCNu8 "hhu"
#define SCNoFAST16 "o"
#define SCNoFAST64 __PRI64 "o"
#define SCNo32 "o"
#define SCNiLEAST8 "hhi"
#define PRIdLEAST64 __PRI64 "d"
#define PRIoFAST16 "o"
#define SCNuMAX __PRI64 "u"
#define PRIxMAX __PRI64 "x"
#define PRId32 "d"
#define PRIi64 __PRI64 "i"
#define __BIG_ENDIAN 4321
#define SCNiFAST64 __PRI64 "i"
#define SCNd64 __PRI64 "d"
#define SCNiLEAST64 __PRI64 "i"
#define SCNi8 "hhi"
#define SCNdPTR __PRIPTR "d"
#define PRIX16 "X"
#define PRIxLEAST32 "x"
#define SCNxPTR __PRIPTR "x"
#define PRIiLEAST16 "i"
#define SCNuLEAST64 __PRI64 "u"
#define SCNdFAST64 __PRI64 "d"
#define PRIuLEAST64 __PRI64 "u"
#define __PRI64 "l"
#define __LONG_MAX 0x7fffffffffffffffL
#define PRIX8 "X"
#define SCNiFAST32 "i"
#define SCNuLEAST16 "hu"
#define SCNx8 "hhx"
#define __BYTE_ORDER 1234
#define PRIoLEAST64 __PRI64 "o"
#define PRIdPTR __PRIPTR "d"
#define SCNdMAX __PRI64 "d"
#define SCNo16 "ho"
#define SCNoLEAST16 "ho"
#define PRIXFAST64 __PRI64 "X"
#define PRIiMAX __PRI64 "i"
#define __PRIPTR "l"
#define SCNdFAST16 "d"
#define SCNu64 __PRI64 "u"
#define SCNxLEAST64 __PRI64 "x"
#define PRIXLEAST64 __PRI64 "X"
#define PRIxFAST16 "x"
#define PRIu64 __PRI64 "u"
#define PRIxLEAST64 __PRI64 "x"
#define SCNdLEAST64 __PRI64 "d"
#define PRIoFAST32 "o"
#define PRIdLEAST16 "d"
#define PRIi16 "i"
#define PRIXFAST8 "X"
#define PRIxFAST8 "x"
#define SCNx16 "hx"
#define PRIuFAST16 "u"
#define PRIo32 "o"
#define PRIXFAST32 "X"
#define PRId16 "d"
#define PRIuLEAST8 "u"
#define SCNoLEAST32 "o"
#define SCNoFAST8 "hho"
#define PRIiFAST8 "i"
#define PRIXLEAST32 "X"
#define SCNd8 "hhd"
#define SCNoLEAST64 __PRI64 "o"
#define PRIu16 "u"
#define SCNxMAX __PRI64 "x"
#define SCNxFAST64 __PRI64 "x"
#define PRIx64 __PRI64 "x"
#define PRIX32 "X"
#define PRIXMAX __PRI64 "X"
#define SCNxLEAST8 "hhx"
#define PRIuPTR __PRIPTR "u"
#define PRId8 "d"
#define PRIdMAX __PRI64 "d"
#define SCNi16 "hi"
#define SCNd16 "hd"
#define PRIx32 "x"
#define PRIiPTR __PRIPTR "i"
#define SCNuLEAST8 "hhu"
#define PRIiLEAST8 "i"
#define SCNiLEAST32 "i"
#define PRIoFAST64 __PRI64 "o"
#define PRIiFAST16 "i"
#define SCNuLEAST32 "u"
#define SCNxFAST32 "x"
#define SCNdFAST32 "d"
#define SCNdLEAST8 "hhd"
#define PRIoMAX __PRI64 "o"
#define SCNuFAST64 __PRI64 "u"
#define PRIdFAST32 "d"
#define PRIXFAST16 "X"
#define SCNi32 "i"
#define PRIxPTR __PRIPTR "x"
#define PRIo64 __PRI64 "o"
#define SCNu16 "hu"
#define SCNiFAST8 "hhi"
#define PRIiFAST32 "i"
#define PRIXLEAST8 "X"
#define PRIiFAST64 __PRI64 "i"
#define PRIx16 "x"
#define SCNoMAX __PRI64 "o"
#define PRIoFAST8 "o"
#define PRIuFAST8 "u"
#define PRIi8 "i"
#define SCNi64 __PRI64 "i"
#define SCNx64 __PRI64 "x"
#define SCNiMAX __PRI64 "i"
#define SCNuFAST32 "u"
#define PRIuLEAST32 "u"
#define SCNdFAST8 "hhd"
#define SCNoPTR __PRIPTR "o"
#define PRIxLEAST16 "x"
#define SCNoLEAST8 "hho"
#define PRIXLEAST16 "X"
#define SCNuFAST8 "hhu"
#define SCNxFAST16 "x"
#define IPPORT_RESERVED 1024
#define SCNiFAST16 "i"
#define SCNoFAST32 "o"
#define PRIdFAST16 "d"
#define PRIuFAST64 __PRI64 "u"
#define PRIoLEAST8 "o"
#define SCNu32 "u"
#define SCNxFAST8 "hhx"
#define SCNiPTR __PRIPTR "i"
#define PRIdLEAST32 "d"
#define SCNx32 "x"
#define PRIuLEAST16 "u"
#define PRIuFAST32 "u"
#define SCNxLEAST32 "x"
#define PRIx8 "x"
#define PRIdFAST64 __PRI64 "d"
#define PRIuMAX __PRI64 "u"
#define SCNiLEAST16 "hi"
#define PRIoPTR __PRIPTR "o"
#define PRIiLEAST64 __PRI64 "i"
#define PRIoLEAST16 "o"
#define PRIxLEAST8 "x"
#define PRIu8 "u"
#define PRIdLEAST8 "d"
#define PRIdFAST8 "d"
#define PRIXPTR __PRIPTR "X"
#define PRIo8 "o"
#define PRIoLEAST32 "o"
#define PRIu32 "u"
#define SCNdLEAST32 "d"
#define SCNdLEAST16 "hd"
#define PRIo16 "o"
#define PRIxFAST32 "x"
#define SCNo8 "hho"
#define PRIiLEAST32 "i"
#define PRIxFAST64 __PRI64 "x"
#define __LITTLE_ENDIAN 1234
#define PRIX64 __PRI64 "X"
#define SCNxLEAST16 "hx"
#define SCNd32 "d"
#define PRIi32 "i"
#define PRId64 __PRI64 "d"
#define SCNo64 __PRI64 "o"
#endif
