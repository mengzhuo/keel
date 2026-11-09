#ifndef _CRABC_NETINET_IP_H
#define _CRABC_NETINET_IP_H
#include <netinet/in.h>
struct iphdr { unsigned int ihl:4, version:4; uint8_t tos; uint16_t tot_len, id, frag_off; uint8_t ttl, protocol; uint16_t check; uint32_t saddr, daddr; };

#endif
