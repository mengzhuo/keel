#ifndef _CRABC_NETINET_UDP_H
#define _CRABC_NETINET_UDP_H
#include <netinet/in.h>
struct udphdr { uint16_t source, dest, len, check; };

#endif
