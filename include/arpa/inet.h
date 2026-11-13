/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _ARPA_INET_H
#define _ARPA_INET_H

#include <features.h>
#include <netinet/in.h>
#include <stdint.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef __keel_in_addr_t_defined
#define __keel_in_addr_t_defined
typedef uint32_t in_addr_t;
#endif

#ifndef __keel_in_port_t_defined
#define __keel_in_port_t_defined
typedef uint16_t in_port_t;
#endif

#ifndef __keel_in_addr_struct
#define __keel_in_addr_struct
struct in_addr {
    in_addr_t s_addr;
};
#endif

#define INET_ADDRSTRLEN  16
#define INET6_ADDRSTRLEN 46

int inet_pton(int, const char *, void *);
const char *inet_ntop(int, const void *, char *, socklen_t);
in_addr_t inet_addr(const char *);
char *inet_ntoa(struct in_addr);
int inet_aton(const char *, struct in_addr *);
in_addr_t inet_network(const char *);
struct in_addr inet_makeaddr(in_addr_t, in_addr_t);
in_addr_t inet_lnaof(struct in_addr);
in_addr_t inet_netof(struct in_addr);

/* keel historically exposes the raw 32-bit address in these two entry
 * points; keep both calling conventions source-compatible. */
#define inet_aton(s, p) inet_aton((s), (void *)(p))
#define inet_ntoa(addr) inet_ntoa((struct in_addr){ .s_addr = (in_addr_t)(addr) })

uint32_t htonl(uint32_t);
uint16_t htons(uint16_t);
uint32_t ntohl(uint32_t);
uint16_t ntohs(uint16_t);

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
