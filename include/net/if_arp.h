#ifndef _CRABC_NET_IF_ARP_H
#define _CRABC_NET_IF_ARP_H
#include <netinet/in.h>
struct arphdr { uint16_t ar_hrd, ar_pro; uint8_t ar_hln, ar_pln; uint16_t ar_op; };
#define ARPHRD_ETHER 1
#define ARPOP_REQUEST 1
#define ARPOP_REPLY 2

#endif
