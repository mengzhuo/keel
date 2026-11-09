#ifndef _NET_IF_H
#define _NET_IF_H

#include <features.h>
#include <sys/socket.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

#define IF_NAMESIZE 16
#define IFNAMSIZ    16

#define IFF_UP          0x1
#define IFF_BROADCAST   0x2
#define IFF_DEBUG       0x4
#define IFF_LOOPBACK    0x8
#define IFF_POINTOPOINT 0x10
#define IFF_NOTRAILERS  0x20
#define IFF_RUNNING     0x40
#define IFF_NOARP       0x80
#define IFF_PROMISC     0x100
#define IFF_ALLMULTI    0x200
#define IFF_MASTER      0x400
#define IFF_SLAVE       0x800
#define IFF_MULTICAST   0x1000
#define IFF_PORTSEL     0x2000
#define IFF_AUTOMEDIA   0x4000
#define IFF_DYNAMIC     0x8000

struct if_nameindex {
    unsigned if_index;
    char *if_name;
};

unsigned if_nametoindex(const char *);
char *if_indextoname(unsigned, char *);
struct if_nameindex *if_nameindex(void);
void if_freenameindex(struct if_nameindex *);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define IFF_DORMANT 0x20000
#define IFF_ECHO 0x40000
#define IFF_LOWER_UP 0x10000
#define IFF_VOLATILE (IFF_LOOPBACK|IFF_POINTOPOINT|IFF_BROADCAST| IFF_ECHO|IFF_MASTER|IFF_SLAVE|IFF_RUNNING|IFF_LOWER_UP|IFF_DORMANT)
#define IFHWADDRLEN 6
#define ifc_buf ifc_ifcu.ifcu_buf
#define ifc_req ifc_ifcu.ifcu_req
#define ifr_addr ifr_ifru.ifru_addr
#define ifr_bandwidth ifr_ifru.ifru_ivalue
#define ifr_broadaddr ifr_ifru.ifru_broadaddr
#define ifr_data ifr_ifru.ifru_data
#define ifr_dstaddr ifr_ifru.ifru_dstaddr
#define ifr_flags ifr_ifru.ifru_flags
#define ifr_hwaddr ifr_ifru.ifru_hwaddr
#define ifr_ifindex ifr_ifru.ifru_ivalue
#define ifr_map ifr_ifru.ifru_map
#define ifr_metric ifr_ifru.ifru_ivalue
#define ifr_mtu ifr_ifru.ifru_mtu
#define ifr_name ifr_ifrn.ifrn_name
#define ifr_netmask ifr_ifru.ifru_netmask
#define ifr_newname ifr_ifru.ifru_newname
#define ifr_qlen ifr_ifru.ifru_ivalue
#define ifr_slave ifr_ifru.ifru_slave

/* musl 1.2.6 compatibility macros */
#define _IOT_ifconf _IOT(_IOTS(struct ifconf),1,0,0,0,0)
#define _IOT_ifreq _IOT(_IOTS(char),IFNAMSIZ,_IOTS(char),16,0,0)
#define _IOT_ifreq_int _IOT(_IOTS(char),IFNAMSIZ,_IOTS(int),1,0,0)
#define _IOT_ifreq_short _IOT(_IOTS(char),IFNAMSIZ,_IOTS(short),1,0,0)

/* musl 1.2.6 per-header compatibility macros */
#define __BYTE_ORDER 1234
#define ifa_broadaddr ifa_ifu.ifu_broadaddr
#define __LITTLE_ENDIAN 1234
#define __LONG_MAX 0x7fffffffffffffffL
#define ifa_dstaddr ifa_ifu.ifu_dstaddr
#define __BIG_ENDIAN 4321
#endif
