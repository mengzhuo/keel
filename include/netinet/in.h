#ifndef _NETINET_IN_H
#define _NETINET_IN_H

#include <features.h>
#include <stdint.h>
#include <sys/socket.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef uint16_t in_port_t;
typedef uint32_t in_addr_t;

#ifndef __crabc_in_addr_struct
#define __crabc_in_addr_struct
struct in_addr { in_addr_t s_addr; };
#endif

struct sockaddr_in {
    sa_family_t sin_family;
    in_port_t sin_port;
    struct in_addr sin_addr;
    unsigned char sin_zero[8];
};

struct in6_addr {
    union {
        uint8_t __s6_addr[16];
        uint16_t __s6_addr16[8];
        uint32_t __s6_addr32[4];
    } __in6_union;
};
#define s6_addr __in6_union.__s6_addr
#define s6_addr16 __in6_union.__s6_addr16
#define s6_addr32 __in6_union.__s6_addr32

struct sockaddr_in6 {
    sa_family_t sin6_family;
    in_port_t sin6_port;
    uint32_t sin6_flowinfo;
    struct in6_addr sin6_addr;
    uint32_t sin6_scope_id;
};

struct ipv6_mreq {
    struct in6_addr ipv6mr_multiaddr;
    unsigned ipv6mr_interface;
};

extern const struct in6_addr in6addr_any;
extern const struct in6_addr in6addr_loopback;

#define IN6ADDR_ANY_INIT      { { { 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0 } } }
#define IN6ADDR_LOOPBACK_INIT { { { 0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,1 } } }

#define IPPROTO_IP   0
#define IPPROTO_ICMP 1
#define IPPROTO_TCP  6
#define IPPROTO_UDP  17
#define IPPROTO_IPV6 41
#define IPPROTO_RAW  255

#define INET_ADDRSTRLEN  16
#define INET6_ADDRSTRLEN 46

#define IPV6_JOIN_GROUP       20
#define IPV6_LEAVE_GROUP      21
#define IPV6_MULTICAST_HOPS   18
#define IPV6_MULTICAST_IF     17
#define IPV6_MULTICAST_LOOP   19
#define IPV6_UNICAST_HOPS     16
#define IPV6_V6ONLY           26

#define IN6_IS_ADDR_UNSPECIFIED(a) \
        (((uint32_t *) (a))[0] == 0 && ((uint32_t *) (a))[1] == 0 && \
         ((uint32_t *) (a))[2] == 0 && ((uint32_t *) (a))[3] == 0)

#define IN6_IS_ADDR_LOOPBACK(a) \
        (((uint32_t *) (a))[0] == 0 && ((uint32_t *) (a))[1] == 0 && \
         ((uint32_t *) (a))[2] == 0 && \
         ((uint8_t *) (a))[12] == 0 && ((uint8_t *) (a))[13] == 0 && \
         ((uint8_t *) (a))[14] == 0 && ((uint8_t *) (a))[15] == 1 )

#define IN6_IS_ADDR_MULTICAST(a) (((uint8_t *) (a))[0] == 0xff)

#define IN6_IS_ADDR_LINKLOCAL(a) \
        ((((uint8_t *) (a))[0]) == 0xfe && (((uint8_t *) (a))[1] & 0xc0) == 0x80)

#define IN6_IS_ADDR_SITELOCAL(a) \
        ((((uint8_t *) (a))[0]) == 0xfe && (((uint8_t *) (a))[1] & 0xc0) == 0xc0)

#define IN6_IS_ADDR_V4MAPPED(a) \
        (((uint32_t *) (a))[0] == 0 && ((uint32_t *) (a))[1] == 0 && \
         ((uint8_t *) (a))[8] == 0 && ((uint8_t *) (a))[9] == 0 && \
         ((uint8_t *) (a))[10] == 0xff && ((uint8_t *) (a))[11] == 0xff)

#define IN6_IS_ADDR_V4COMPAT(a) \
        (((uint32_t *) (a))[0] == 0 && ((uint32_t *) (a))[1] == 0 && \
         ((uint32_t *) (a))[2] == 0 && \
         !IN6_IS_ADDR_UNSPECIFIED(a) && !IN6_IS_ADDR_LOOPBACK(a))

#define IN6_IS_ADDR_MC_NODELOCAL(a) \
        (IN6_IS_ADDR_MULTICAST(a) && ((((uint8_t *) (a))[1] & 0xf) == 0x1))

#define IN6_IS_ADDR_MC_LINKLOCAL(a) \
        (IN6_IS_ADDR_MULTICAST(a) && ((((uint8_t *) (a))[1] & 0xf) == 0x2))

#define IN6_IS_ADDR_MC_SITELOCAL(a) \
        (IN6_IS_ADDR_MULTICAST(a) && ((((uint8_t *) (a))[1] & 0xf) == 0x5))

#define IN6_IS_ADDR_MC_ORGLOCAL(a) \
        (IN6_IS_ADDR_MULTICAST(a) && ((((uint8_t *) (a))[1] & 0xf) == 0x8))

#define IN6_IS_ADDR_MC_GLOBAL(a) \
        (IN6_IS_ADDR_MULTICAST(a) && ((((uint8_t *) (a))[1] & 0xf) == 0xe))

#define __ARE_4_EQUAL(a,b) \
        (!( (0[a]-0[b]) | (1[a]-1[b]) | (2[a]-2[b]) | (3[a]-3[b]) ))
#define IN6_ARE_ADDR_EQUAL(a,b) \
        __ARE_4_EQUAL((const uint32_t *)(a), (const uint32_t *)(b))

#define INADDR_ANY       ((in_addr_t)0x00000000)
#define INADDR_LOOPBACK  ((in_addr_t)0x7f000001)
#define INADDR_BROADCAST ((in_addr_t)0xffffffff)
#define INADDR_NONE      ((in_addr_t)0xffffffff)

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define GROUP_FILTER_SIZE(numsrc) (sizeof(struct group_filter) - sizeof(struct sockaddr_storage) + (numsrc) * sizeof(struct sockaddr_storage))
#define INADDR_ALLHOSTS_GROUP ((in_addr_t) 0xe0000001)
#define INADDR_ALLRTRS_GROUP ((in_addr_t) 0xe0000002)
#define INADDR_ALLSNOOPERS_GROUP ((in_addr_t) 0xe000006a)
#define INADDR_DUMMY ((in_addr_t) 0xc0000008)
#define INADDR_MAX_LOCAL_GROUP ((in_addr_t) 0xe00000ff)
#define INADDR_UNSPEC_GROUP ((in_addr_t) 0xe0000000)
#define IN_BADCLASS(a) ((((in_addr_t)(a)) & 0xf0000000) == 0xf0000000)
#define IN_CLASSA(a) ((((in_addr_t)(a)) & 0x80000000) == 0)
#define IN_CLASSA_HOST (0xffffffff & ~IN_CLASSA_NET)
#define IN_CLASSA_MAX 128
#define IN_CLASSA_NET 0xff000000
#define IN_CLASSA_NSHIFT 24
#define IN_CLASSB(a) ((((in_addr_t)(a)) & 0xc0000000) == 0x80000000)
#define IN_CLASSB_HOST (0xffffffff & ~IN_CLASSB_NET)
#define IN_CLASSB_MAX 65536
#define IN_CLASSB_NET 0xffff0000
#define IN_CLASSB_NSHIFT 16
#define IN_CLASSC(a) ((((in_addr_t)(a)) & 0xe0000000) == 0xc0000000)
#define IN_CLASSC_HOST (0xffffffff & ~IN_CLASSC_NET)
#define IN_CLASSC_NET 0xffffff00
#define IN_CLASSC_NSHIFT 8
#define IN_CLASSD(a) ((((in_addr_t)(a)) & 0xf0000000) == 0xe0000000)
#define IN_EXPERIMENTAL(a) ((((in_addr_t)(a)) & 0xe0000000) == 0xe0000000)
#define IN_LOOPBACKNET 127
#define IN_MULTICAST(a) IN_CLASSD(a)
#define IPPROTO_AH 51
#define IPPROTO_BEETPH 94
#define IPPROTO_COMP 108
#define IPPROTO_DCCP 33
#define IPPROTO_DSTOPTS 60
#define IPPROTO_EGP 8
#define IPPROTO_ENCAP 98
#define IPPROTO_ESP 50
#define IPPROTO_ETHERNET 143
#define IPPROTO_FRAGMENT 44
#define IPPROTO_GRE 47
#define IPPROTO_HOPOPTS 0
#define IPPROTO_ICMPV6 58
#define IPPROTO_IDP 22
#define IPPROTO_IGMP 2
#define IPPROTO_IPIP 4
#define IPPROTO_MAX 263
#define IPPROTO_MH 135
#define IPPROTO_MPLS 137
#define IPPROTO_MPTCP 262
#define IPPROTO_MTP 92
#define IPPROTO_NONE 59
#define IPPROTO_PIM 103
#define IPPROTO_PUP 12
#define IPPROTO_ROUTING 43
#define IPPROTO_RSVP 46
#define IPPROTO_SCTP 132
#define IPPROTO_TP 29
#define IPPROTO_UDPLITE 136
#define IPV6_2292DSTOPTS 4
#define IPV6_2292HOPLIMIT 8
#define IPV6_2292HOPOPTS 3
#define IPV6_2292PKTINFO 2
#define IPV6_2292PKTOPTIONS 6
#define IPV6_2292RTHDR 5
#define IPV6_ADDRFORM 1
#define IPV6_ADDR_PREFERENCES 72
#define IPV6_ADD_MEMBERSHIP IPV6_JOIN_GROUP
#define IPV6_AUTHHDR 10
#define IPV6_AUTOFLOWLABEL 70
#define IPV6_CHECKSUM 7
#define IPV6_DONTFRAG 62
#define IPV6_DROP_MEMBERSHIP IPV6_LEAVE_GROUP
#define IPV6_DSTOPTS 59
#define IPV6_FREEBIND 78
#define IPV6_HDRINCL 36
#define IPV6_HOPLIMIT 52
#define IPV6_HOPOPTS 54
#define IPV6_IPSEC_POLICY 34
#define IPV6_JOIN_ANYCAST 27
#define IPV6_LEAVE_ANYCAST 28
#define IPV6_MINHOPCOUNT 73
#define IPV6_MTU 24
#define IPV6_MTU_DISCOVER 23
#define IPV6_MULTICAST_ALL 29
#define IPV6_NEXTHOP 9
#define IPV6_ORIGDSTADDR 74
#define IPV6_PATHMTU 61
#define IPV6_PKTINFO 50
#define IPV6_PMTUDISC_DO 2
#define IPV6_PMTUDISC_DONT 0
#define IPV6_PMTUDISC_INTERFACE 4
#define IPV6_PMTUDISC_OMIT 5
#define IPV6_PMTUDISC_PROBE 3
#define IPV6_PMTUDISC_WANT 1
#define IPV6_PREFER_SRC_CGA 0x0008
#define IPV6_PREFER_SRC_COA 0x0004
#define IPV6_PREFER_SRC_HOME 0x0400
#define IPV6_PREFER_SRC_NONCGA 0x0800
#define IPV6_PREFER_SRC_PUBLIC 0x0002
#define IPV6_PREFER_SRC_PUBTMP_DEFAULT 0x0100
#define IPV6_PREFER_SRC_TMP 0x0001
#define IPV6_RECVDSTOPTS 58
#define IPV6_RECVERR 25
#define IPV6_RECVFRAGSIZE 77
#define IPV6_RECVHOPLIMIT 51
#define IPV6_RECVHOPOPTS 53
#define IPV6_RECVORIGDSTADDR IPV6_ORIGDSTADDR
#define IPV6_RECVPATHMTU 60
#define IPV6_RECVPKTINFO 49
#define IPV6_RECVRTHDR 56
#define IPV6_RECVTCLASS 66
#define IPV6_ROUTER_ALERT 22
#define IPV6_ROUTER_ALERT_ISOLATE 30
#define IPV6_RTHDR 57
#define IPV6_RTHDRDSTOPTS 55
#define IPV6_RTHDR_LOOSE 0
#define IPV6_RTHDR_STRICT 1
#define IPV6_RTHDR_TYPE_0 0
#define IPV6_RXDSTOPTS IPV6_DSTOPTS
#define IPV6_RXHOPOPTS IPV6_HOPOPTS
#define IPV6_TCLASS 67
#define IPV6_TRANSPARENT 75
#define IPV6_UNICAST_IF 76
#define IPV6_XFRM_POLICY 35
#define IP_ADD_MEMBERSHIP 35
#define IP_ADD_SOURCE_MEMBERSHIP 39
#define IP_BIND_ADDRESS_NO_PORT 24
#define IP_BLOCK_SOURCE 38
#define IP_CHECKSUM 23
#define IP_DEFAULT_MULTICAST_LOOP 1
#define IP_DEFAULT_MULTICAST_TTL 1
#define IP_DROP_MEMBERSHIP 36
#define IP_DROP_SOURCE_MEMBERSHIP 40
#define IP_FREEBIND 15
#define IP_HDRINCL 3
#define IP_IPSEC_POLICY 16
#define IP_MAX_MEMBERSHIPS 20
#define IP_MINTTL 21
#define IP_MSFILTER 41
#define IP_MSFILTER_SIZE(numsrc) (sizeof(struct ip_msfilter) - sizeof(struct in_addr) + (numsrc) * sizeof(struct in_addr))
#define IP_MTU 14
#define IP_MTU_DISCOVER 10
#define IP_MULTICAST_ALL 49
#define IP_MULTICAST_IF 32
#define IP_MULTICAST_LOOP 34
#define IP_MULTICAST_TTL 33
#define IP_NODEFRAG 22
#define IP_OPTIONS 4
#define IP_ORIGDSTADDR 20
#define IP_PASSSEC 18
#define IP_PKTINFO 8
#define IP_PKTOPTIONS 9
#define IP_PMTUDISC 10
#define IP_PMTUDISC_DO 2
#define IP_PMTUDISC_DONT 0
#define IP_PMTUDISC_INTERFACE 4
#define IP_PMTUDISC_OMIT 5
#define IP_PMTUDISC_PROBE 3
#define IP_PMTUDISC_WANT 1
#define IP_RECVERR 11
#define IP_RECVERR_RFC4884 26
#define IP_RECVFRAGSIZE 25
#define IP_RECVOPTS 6
#define IP_RECVORIGDSTADDR IP_ORIGDSTADDR
#define IP_RECVRETOPTS IP_RETOPTS
#define IP_RECVTOS 13
#define IP_RECVTTL 12
#define IP_RETOPTS 7
#define IP_ROUTER_ALERT 5
#define IP_TOS 1
#define IP_TRANSPARENT 19
#define IP_TTL 2
#define IP_UNBLOCK_SOURCE 37
#define IP_UNICAST_IF 50
#define IP_XFRM_POLICY 17
#define MCAST_BLOCK_SOURCE 43
#define MCAST_EXCLUDE 0
#define MCAST_INCLUDE 1
#define MCAST_JOIN_GROUP 42
#define MCAST_JOIN_SOURCE_GROUP 46
#define MCAST_LEAVE_GROUP 45
#define MCAST_LEAVE_SOURCE_GROUP 47
#define MCAST_MSFILTER 48
#define MCAST_UNBLOCK_SOURCE 44

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
