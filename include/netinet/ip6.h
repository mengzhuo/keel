/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_NETINET_IP6_H
#define _CRABC_NETINET_IP6_H
#include <netinet/in.h>


/* musl 1.2.6 compatibility macros */
#define IP6F_MORE_FRAG 0x0100
#define IP6F_OFF_MASK 0xf8ff
#define IP6F_RESERVED_MASK 0x0600
#define IP6OPT_JUMBO 0xc2
#define IP6OPT_JUMBO_LEN 6
#define IP6OPT_NSAP_ADDR 0xc3
#define IP6OPT_PAD1 0
#define IP6OPT_PADN 1
#define IP6OPT_ROUTER_ALERT 0x05
#define IP6OPT_TUNNEL_LIMIT 0x04
#define IP6OPT_TYPE(o) ((o) & 0xc0)
#define IP6OPT_TYPE_DISCARD 0x40
#define IP6OPT_TYPE_FORCEICMP 0x80
#define IP6OPT_TYPE_ICMP 0xc0
#define IP6OPT_TYPE_MUTABLE 0x20
#define IP6OPT_TYPE_SKIP 0x00
#define IP6_ALERT_AN 0x0200
#define IP6_ALERT_MLD 0x0000
#define IP6_ALERT_RSVP 0x0100
#define ip6_flow ip6_ctlun.ip6_un1.ip6_un1_flow
#define ip6_hlim ip6_ctlun.ip6_un1.ip6_un1_hlim
#define ip6_hops ip6_ctlun.ip6_un1.ip6_un1_hlim
#define ip6_nxt ip6_ctlun.ip6_un1.ip6_un1_nxt
#define ip6_plen ip6_ctlun.ip6_un1.ip6_un1_plen
#define ip6_vfc ip6_ctlun.ip6_un2_vfc
#endif
