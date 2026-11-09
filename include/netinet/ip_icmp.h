#ifndef _CRABC_NETINET_IP_ICMP_H
#define _CRABC_NETINET_IP_ICMP_H
#include <netinet/ip.h>
struct icmphdr { uint8_t type, code; uint16_t checksum; union { struct { uint16_t id, sequence; } echo; uint32_t gateway; } un; };

#endif
