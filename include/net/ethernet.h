#ifndef _CRABC_NET_ETHERNET_H
#define _CRABC_NET_ETHERNET_H
#include <netinet/if_ether.h>

/* musl 1.2.6 compatibility macros */
#define ETHERMIN (ETHER_MIN_LEN - ETHER_HDR_LEN - ETHER_CRC_LEN)
#define ETHERMTU ETH_DATA_LEN
#define ETHERTYPE_AARP 0x80F3
#define ETHERTYPE_ARP 0x0806
#define ETHERTYPE_AT 0x809B
#define ETHERTYPE_IP 0x0800
#define ETHERTYPE_IPV6 0x86dd
#define ETHERTYPE_IPX 0x8137
#define ETHERTYPE_LOOPBACK 0x9000
#define ETHERTYPE_NTRAILER 16
#define ETHERTYPE_PUP 0x0200
#define ETHERTYPE_REVARP 0x8035
#define ETHERTYPE_SPRITE 0x0500
#define ETHERTYPE_TRAIL 0x1000
#define ETHERTYPE_VLAN 0x8100
#define ETHER_ADDR_LEN ETH_ALEN
#define ETHER_CRC_LEN 4
#define ETHER_HDR_LEN ETH_HLEN
#define ETHER_IS_VALID_LEN(foo) ((foo) >= ETHER_MIN_LEN && (foo) <= ETHER_MAX_LEN)
#define ETHER_MAX_LEN (ETH_FRAME_LEN + ETHER_CRC_LEN)
#define ETHER_MIN_LEN (ETH_ZLEN + ETHER_CRC_LEN)
#define ETHER_TYPE_LEN 2

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
