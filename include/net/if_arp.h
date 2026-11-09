#ifndef _CRABC_NET_IF_ARP_H
#define _CRABC_NET_IF_ARP_H
#include <inttypes.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
struct arphdr { uint16_t ar_hrd, ar_pro; uint8_t ar_hln, ar_pln; uint16_t ar_op; };
#define ARPHRD_ETHER 1
#define ARPOP_REQUEST 1
#define ARPOP_REPLY 2


/* musl 1.2.6 compatibility macros */
#define ARPD_FLUSH 0x03
#define ARPD_LOOKUP 0x02
#define ARPD_UPDATE 0x01
#define ARPHRD_6LOWPAN 825
#define ARPHRD_ADAPT 264
#define ARPHRD_APPLETLK 8
#define ARPHRD_ARCNET 7
#define ARPHRD_ASH 781
#define ARPHRD_ATM 19
#define ARPHRD_AX25 3
#define ARPHRD_BIF 775
#define ARPHRD_CAIF 822
#define ARPHRD_CAN 280
#define ARPHRD_CHAOS 5
#define ARPHRD_CISCO 513
#define ARPHRD_CSLIP 257
#define ARPHRD_CSLIP6 259
#define ARPHRD_DDCMP 517
#define ARPHRD_DLCI 15
#define ARPHRD_ECONET 782
#define ARPHRD_EETHER 2
#define ARPHRD_EUI64 27
#define ARPHRD_FCAL 785
#define ARPHRD_FCFABRIC 787
#define ARPHRD_FCPL 786
#define ARPHRD_FCPP 784
#define ARPHRD_FDDI 774
#define ARPHRD_FRAD 770
#define ARPHRD_HDLC ARPHRD_CISCO
#define ARPHRD_HIPPI 780
#define ARPHRD_HWX25 272
#define ARPHRD_IEEE1394 24
#define ARPHRD_IEEE802 6
#define ARPHRD_IEEE80211 801
#define ARPHRD_IEEE80211_PRISM 802
#define ARPHRD_IEEE80211_RADIOTAP 803
#define ARPHRD_IEEE802154 804
#define ARPHRD_IEEE802154_MONITOR 805
#define ARPHRD_IEEE802_TR 800
#define ARPHRD_INFINIBAND 32
#define ARPHRD_IP6GRE 823
#define ARPHRD_IPDDP 777
#define ARPHRD_IPGRE 778
#define ARPHRD_IRDA 783
#define ARPHRD_LAPB 516
#define ARPHRD_LOCALTLK 773
#define ARPHRD_LOOPBACK 772
#define ARPHRD_METRICOM 23
#define ARPHRD_NETLINK 824
#define ARPHRD_NETROM 0
#define ARPHRD_NONE 0xFFFE
#define ARPHRD_PHONET 820
#define ARPHRD_PHONET_PIPE 821
#define ARPHRD_PIMREG 779
#define ARPHRD_PPP 512
#define ARPHRD_PRONET 4
#define ARPHRD_RAWHDLC 518
#define ARPHRD_RAWIP 519
#define ARPHRD_ROSE 270
#define ARPHRD_RSRVD 260
#define ARPHRD_SIT 776
#define ARPHRD_SKIP 771
#define ARPHRD_SLIP 256
#define ARPHRD_SLIP6 258
#define ARPHRD_TUNNEL 768
#define ARPHRD_TUNNEL6 769
#define ARPHRD_VOID 0xFFFF
#define ARPHRD_VSOCKMON 826
#define ARPHRD_X25 271
#define ARPOP_InREPLY 9
#define ARPOP_InREQUEST 8
#define ARPOP_NAK 10
#define ARPOP_RREPLY 4
#define ARPOP_RREQUEST 3
#define ATF_COM 0x02
#define ATF_DONTPUB 0x40
#define ATF_MAGIC 0x80
#define ATF_NETMASK 0x20
#define ATF_PERM 0x04
#define ATF_PUBL 0x08
#define ATF_USETRAILERS 0x10
#define MAX_ADDR_LEN 7

/* musl 1.2.6 per-header compatibility macros */
#define FD_SET(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] |= (1UL<<((d)%(8*sizeof(long)))))
#define be64toh(x) __bswap64(x)
#define __PDP_ENDIAN 3412
#define le32toh(x) (uint32_t)(x)
#define FD_ISSET(d, s) !!((s)->fds_bits[(d)/(8*sizeof(long))] & (1UL<<((d)%(8*sizeof(long)))))
#define htole16(x) (uint16_t)(x)
#define le16toh(x) (uint16_t)(x)
#define FD_SETSIZE 1024
#define htole64(x) (uint64_t)(x)
#define htobe16(x) __bswap16(x)
#define BYTE_ORDER __BYTE_ORDER
#define LITTLE_ENDIAN __LITTLE_ENDIAN
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define betoh64(x) __bswap64(x)
#define letoh16(x) (uint16_t)(x)
#define letoh64(x) (uint64_t)(x)
#define be32toh(x) __bswap32(x)
#define htole32(x) (uint32_t)(x)
#define be16toh(x) __bswap16(x)
#define htobe32(x) __bswap32(x)
#define NFDBITS (8*(int)sizeof(long))
#define PDP_ENDIAN __PDP_ENDIAN
#define htobe64(x) __bswap64(x)
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define BIG_ENDIAN __BIG_ENDIAN
#define betoh32(x) __bswap32(x)
#define letoh32(x) (uint32_t)(x)
#define le64toh(x) (uint64_t)(x)
#define betoh16(x) __bswap16(x)
#endif
