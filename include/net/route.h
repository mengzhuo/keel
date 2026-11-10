/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_NET_ROUTE_H
#define _CRABC_NET_ROUTE_H
#include <features.h>
#include <stdint.h>
#include <sys/socket.h>
#include <sys/types.h>
#include <netinet/in.h>


/* musl 1.2.6 compatibility macros */
#define RTCF_DIRECTSRC 0x04000000
#define RTCF_DOREDIRECT 0x01000000
#define RTCF_LOG 0x02000000
#define RTCF_MASQ 0x00400000
#define RTCF_NAT 0x00800000
#define RTCF_VALVE 0x00200000
#define RTF_ADDRCLASSMASK 0xF8000000
#define RTF_ADDRCONF 0x00040000
#define RTF_ALLONLINK 0x00020000
#define RTF_BROADCAST 0x10000000
#define RTF_CACHE 0x01000000
#define RTF_DEFAULT 0x00010000
#define RTF_DYNAMIC 0x0010
#define RTF_FLOW 0x02000000
#define RTF_GATEWAY 0x0002
#define RTF_HOST 0x0004
#define RTF_INTERFACE 0x40000000
#define RTF_IRTT 0x0100
#define RTF_LINKRT 0x00100000
#define RTF_LOCAL 0x80000000
#define RTF_MODIFIED 0x0020
#define RTF_MSS RTF_MTU
#define RTF_MTU 0x0040
#define RTF_MULTICAST 0x20000000
#define RTF_NAT 0x08000000
#define RTF_NOFORWARD 0x1000
#define RTF_NONEXTHOP 0x00200000
#define RTF_NOPMTUDISC 0x4000
#define RTF_POLICY 0x04000000
#define RTF_REINSTATE 0x0008
#define RTF_REJECT 0x0200
#define RTF_STATIC 0x0400
#define RTF_THROW 0x2000
#define RTF_UP 0x0001
#define RTF_WINDOW 0x0080
#define RTF_XRESOLVE 0x0800
#define RTMSG_ACK NLMSG_ACK
#define RTMSG_AR_FAILED 0x51
#define RTMSG_CONTROL 0x40
#define RTMSG_DELDEVICE 0x12
#define RTMSG_DELROUTE 0x22
#define RTMSG_DELRULE 0x32
#define RTMSG_NEWDEVICE 0x11
#define RTMSG_NEWROUTE 0x21
#define RTMSG_NEWRULE 0x31
#define RTMSG_OVERRUN NLMSG_OVERRUN
#define RT_ADDRCLASS(flags) ((uint32_t) flags >> 23)
#define RT_CLASS_DEFAULT 253
#define RT_CLASS_LOCAL 255
#define RT_CLASS_MAIN 254
#define RT_CLASS_MAX 255
#define RT_CLASS_UNSPEC 0
#define RT_LOCALADDR(flags) ((flags & RTF_ADDRCLASSMASK) == (RTF_LOCAL|RTF_INTERFACE))
#define RT_TOS(tos) ((tos) & IPTOS_TOS_MASK)
#define rt_mss rt_mtu

/* musl 1.2.6 per-header compatibility macros */
#define SCNuFAST16 "u"
#define be64toh(x) __bswap64(x)
#define SCNuPTR __PRIPTR "u"
#define le32toh(x) (uint32_t)(x)
#define FD_ISSET(d, s) !!((s)->fds_bits[(d)/(8*sizeof(long))] & (1UL<<((d)%(8*sizeof(long)))))
#define SCNu8 "hhu"
#define SCNoFAST16 "o"
#define SCNoFAST64 __PRI64 "o"
#define SCNo32 "o"
#define SCNiLEAST8 "hhi"
#define PRIdLEAST64 __PRI64 "d"
#define PRIoFAST16 "o"
#define BYTE_ORDER __BYTE_ORDER
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
#define letoh64(x) (uint64_t)(x)
#define __PRI64 "l"
#define be32toh(x) __bswap32(x)
#define __LONG_MAX 0x7fffffffffffffffL
#define PRIX8 "X"
#define htobe32(x) __bswap32(x)
#define SCNiFAST32 "i"
#define SCNuLEAST16 "hu"
#define SCNx8 "hhx"
#define __BYTE_ORDER 1234
#define PRIoLEAST64 __PRI64 "o"
#define PRIdPTR __PRIPTR "d"
#define htobe64(x) __bswap64(x)
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
#define FD_SET(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] |= (1UL<<((d)%(8*sizeof(long)))))
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
#define htobe16(x) __bswap16(x)
#define SCNoLEAST64 __PRI64 "o"
#define PRIu16 "u"
#define SCNxMAX __PRI64 "x"
#define SCNxFAST64 __PRI64 "x"
#define PRIx64 __PRI64 "x"
#define LITTLE_ENDIAN __LITTLE_ENDIAN
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
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define BIG_ENDIAN __BIG_ENDIAN
#define SCNuLEAST32 "u"
#define betoh32(x) __bswap32(x)
#define SCNxFAST32 "x"
#define letoh32(x) (uint32_t)(x)
#define SCNdFAST32 "d"
#define SCNdLEAST8 "hhd"
#define PRIoMAX __PRI64 "o"
#define SCNuFAST64 __PRI64 "u"
#define betoh16(x) __bswap16(x)
#define PRIdFAST32 "d"
#define PRIXFAST16 "X"
#define SCNi32 "i"
#define htole16(x) (uint16_t)(x)
#define PRIxPTR __PRIPTR "x"
#define PRIo64 __PRI64 "o"
#define SCNu16 "hu"
#define SCNiFAST8 "hhi"
#define PRIiFAST32 "i"
#define FD_SETSIZE 1024
#define PRIXLEAST8 "X"
#define PRIiFAST64 __PRI64 "i"
#define PRIx16 "x"
#define htole64(x) (uint64_t)(x)
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
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define SCNoLEAST8 "hho"
#define PRIXLEAST16 "X"
#define letoh16(x) (uint16_t)(x)
#define SCNuFAST8 "hhu"
#define SCNxFAST16 "x"
#define IPPORT_RESERVED 1024
#define SCNiFAST16 "i"
#define htole32(x) (uint32_t)(x)
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
#define __PDP_ENDIAN 3412
#define PRIuMAX __PRI64 "u"
#define SCNiLEAST16 "hi"
#define le16toh(x) (uint16_t)(x)
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
#define betoh64(x) __bswap64(x)
#define __LITTLE_ENDIAN 1234
#define PRIX64 __PRI64 "X"
#define SCNxLEAST16 "hx"
#define SCNd32 "d"
#define be16toh(x) __bswap16(x)
#define NFDBITS (8*(int)sizeof(long))
#define PDP_ENDIAN __PDP_ENDIAN
#define PRIi32 "i"
#define PRId64 __PRI64 "d"
#define le64toh(x) (uint64_t)(x)
#define SCNo64 __PRI64 "o"
#endif
