/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _KEEL_NETINET_ICMP6_H
#define _KEEL_NETINET_ICMP6_H
#include <stdint.h>
#include <string.h>
#include <sys/types.h>
#include <netinet/in.h>


/* musl 1.2.6 compatibility macros */
#define ICMP6_DST_UNREACH 1
#define ICMP6_DST_UNREACH_ADDR 3
#define ICMP6_DST_UNREACH_ADMIN 1
#define ICMP6_DST_UNREACH_BEYONDSCOPE 2
#define ICMP6_DST_UNREACH_NOPORT 4
#define ICMP6_DST_UNREACH_NOROUTE 0
#define ICMP6_ECHO_REPLY 129
#define ICMP6_ECHO_REQUEST 128
#define ICMP6_FILTER 1
#define ICMP6_FILTER_BLOCK 1
#define ICMP6_FILTER_BLOCKOTHERS 3
#define ICMP6_FILTER_PASS 2
#define ICMP6_FILTER_PASSONLY 4
#define ICMP6_FILTER_SETBLOCK(type, filterp) ((((filterp)->icmp6_filt[(type) >> 5]) |= (1 << ((type) & 31))))
#define ICMP6_FILTER_SETBLOCKALL(filterp) memset (filterp, 0xFF, sizeof (struct icmp6_filter));
#define ICMP6_FILTER_SETPASS(type, filterp) ((((filterp)->icmp6_filt[(type) >> 5]) &= ~(1 << ((type) & 31))))
#define ICMP6_FILTER_SETPASSALL(filterp) memset (filterp, 0, sizeof (struct icmp6_filter));
#define ICMP6_FILTER_WILLBLOCK(type, filterp) ((((filterp)->icmp6_filt[(type) >> 5]) & (1 << ((type) & 31))) != 0)
#define ICMP6_FILTER_WILLPASS(type, filterp) ((((filterp)->icmp6_filt[(type) >> 5]) & (1 << ((type) & 31))) == 0)
#define ICMP6_INFOMSG_MASK 0x80
#define ICMP6_PACKET_TOO_BIG 2
#define ICMP6_PARAMPROB_HEADER 0
#define ICMP6_PARAMPROB_NEXTHEADER 1
#define ICMP6_PARAMPROB_OPTION 2
#define ICMP6_PARAM_PROB 4
#define ICMP6_ROUTER_RENUMBERING 138
#define ICMP6_RR_FLAGS_FORCEAPPLY 0x20
#define ICMP6_RR_FLAGS_PREVDONE 0x08
#define ICMP6_RR_FLAGS_REQRESULT 0x40
#define ICMP6_RR_FLAGS_SPECSITE 0x10
#define ICMP6_RR_FLAGS_TEST 0x80
#define ICMP6_RR_PCOUSE_FLAGS_DECRPLTIME 0x40
#define ICMP6_RR_PCOUSE_FLAGS_DECRVLTIME 0x80
#define ICMP6_RR_PCOUSE_RAFLAGS_AUTO 0x10
#define ICMP6_RR_PCOUSE_RAFLAGS_ONLINK 0x20
#define ICMP6_RR_RESULT_FLAGS_FORBIDDEN 0x0100
#define ICMP6_RR_RESULT_FLAGS_OOB 0x0200
#define ICMP6_TIME_EXCEEDED 3
#define ICMP6_TIME_EXCEED_REASSEMBLY 1
#define ICMP6_TIME_EXCEED_TRANSIT 0
#define MLD_LISTENER_QUERY 130
#define MLD_LISTENER_REDUCTION 132
#define MLD_LISTENER_REPORT 131
#define ND_NA_FLAG_OVERRIDE 0x00000020
#define ND_NA_FLAG_ROUTER 0x00000080
#define ND_NA_FLAG_SOLICITED 0x00000040
#define ND_NEIGHBOR_ADVERT 136
#define ND_NEIGHBOR_SOLICIT 135
#define ND_OPT_HOME_AGENT_INFO 8
#define ND_OPT_MTU 5
#define ND_OPT_PI_FLAG_AUTO 0x40
#define ND_OPT_PI_FLAG_ONLINK 0x80
#define ND_OPT_PI_FLAG_RADDR 0x20
#define ND_OPT_PREFIX_INFORMATION 3
#define ND_OPT_REDIRECTED_HEADER 4
#define ND_OPT_RTR_ADV_INTERVAL 7
#define ND_OPT_SOURCE_LINKADDR 1
#define ND_OPT_TARGET_LINKADDR 2
#define ND_RA_FLAG_HOME_AGENT 0x20
#define ND_RA_FLAG_MANAGED 0x80
#define ND_RA_FLAG_OTHER 0x40
#define ND_REDIRECT 137
#define ND_ROUTER_ADVERT 134
#define ND_ROUTER_SOLICIT 133
#define RPM_PCO_ADD 1
#define RPM_PCO_CHANGE 2
#define RPM_PCO_SETGLOBAL 3
#define icmp6_data16 icmp6_dataun.icmp6_un_data16
#define icmp6_data32 icmp6_dataun.icmp6_un_data32
#define icmp6_data8 icmp6_dataun.icmp6_un_data8
#define icmp6_id icmp6_data16[0]
#define icmp6_maxdelay icmp6_data16[0]
#define icmp6_mtu icmp6_data32[0]
#define icmp6_pptr icmp6_data32[0]
#define icmp6_seq icmp6_data16[1]
#define mld_cksum mld_icmp6_hdr.icmp6_cksum
#define mld_code mld_icmp6_hdr.icmp6_code
#define mld_maxdelay mld_icmp6_hdr.icmp6_data16[0]
#define mld_reserved mld_icmp6_hdr.icmp6_data16[1]
#define mld_type mld_icmp6_hdr.icmp6_type
#define nd_na_cksum nd_na_hdr.icmp6_cksum
#define nd_na_code nd_na_hdr.icmp6_code
#define nd_na_flags_reserved nd_na_hdr.icmp6_data32[0]
#define nd_na_type nd_na_hdr.icmp6_type
#define nd_ns_cksum nd_ns_hdr.icmp6_cksum
#define nd_ns_code nd_ns_hdr.icmp6_code
#define nd_ns_reserved nd_ns_hdr.icmp6_data32[0]
#define nd_ns_type nd_ns_hdr.icmp6_type
#define nd_ra_cksum nd_ra_hdr.icmp6_cksum
#define nd_ra_code nd_ra_hdr.icmp6_code
#define nd_ra_curhoplimit nd_ra_hdr.icmp6_data8[0]
#define nd_ra_flags_reserved nd_ra_hdr.icmp6_data8[1]
#define nd_ra_router_lifetime nd_ra_hdr.icmp6_data16[1]
#define nd_ra_type nd_ra_hdr.icmp6_type
#define nd_rd_cksum nd_rd_hdr.icmp6_cksum
#define nd_rd_code nd_rd_hdr.icmp6_code
#define nd_rd_reserved nd_rd_hdr.icmp6_data32[0]
#define nd_rd_type nd_rd_hdr.icmp6_type
#define nd_rs_cksum nd_rs_hdr.icmp6_cksum
#define nd_rs_code nd_rs_hdr.icmp6_code
#define nd_rs_reserved nd_rs_hdr.icmp6_data32[0]
#define nd_rs_type nd_rs_hdr.icmp6_type
#define rr_cksum rr_hdr.icmp6_cksum
#define rr_code rr_hdr.icmp6_code
#define rr_seqnum rr_hdr.icmp6_data32[0]
#define rr_type rr_hdr.icmp6_type

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
#define PRIX8 "X"
#define htobe32(x) __bswap32(x)
#define SCNiFAST32 "i"
#define SCNuLEAST16 "hu"
#define SCNx8 "hhx"
#define PRIoLEAST64 __PRI64 "o"
#define htobe64(x) __bswap64(x)
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
