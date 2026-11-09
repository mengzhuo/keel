#ifndef _NETINET_TCP_H
#define _NETINET_TCP_H
#include <features.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <stdint.h>
#define TCP_NODELAY 1
#define TCP_MAXSEG 2
#define TCP_CORK 3
#define TCP_KEEPIDLE 4
#define TCP_KEEPINTVL 5
#define TCP_KEEPCNT 6
#define TCP_SYNCNT 7
#define TCP_LINGER2 8
#define TCP_DEFER_ACCEPT 9
#define TCP_WINDOW_CLAMP 10
#define TCP_INFO 11
#define TCP_QUICKACK 12
#define TCP_CONGESTION 13
#define TCP_MD5SIG 14
#define TCP_THIN_LINEAR_TIMEOUTS 16
#define TCP_THIN_DUPACK 17
#define TCP_USER_TIMEOUT 18
#define TCP_REPAIR 19
#define TCP_REPAIR_QUEUE 20
#define TCP_QUEUE_SEQ 21
#define TCP_REPAIR_OPTIONS 22
#define TCP_ULP 31
#define TCP_MD5SIG_MAXKEYLEN 80
struct tcphdr {
    uint16_t source, dest;
    uint32_t seq, ack_seq;
    uint16_t res1:4, doff:4, fin:1, syn:1, rst:1, psh:1, ack:1, urg:1, ece:1, cwr:1;
    uint16_t window, check, urg_ptr;
};

/* musl 1.2.6 compatibility macros */
#define SOL_TCP 6
#define TCPI_OPT_ECN 8
#define TCPI_OPT_SACK 2
#define TCPI_OPT_TIMESTAMPS 1
#define TCPI_OPT_WSCALE 4
#define TCPOLEN_MAXSEG 4
#define TCPOLEN_SACK_PERMITTED 2
#define TCPOLEN_TIMESTAMP 10
#define TCPOLEN_WINDOW 3
#define TCPOPT_EOL 0
#define TCPOPT_MAXSEG 2
#define TCPOPT_NOP 1
#define TCPOPT_SACK 5
#define TCPOPT_SACK_PERMITTED 4
#define TCPOPT_TIMESTAMP 8
#define TCPOPT_WINDOW 3
#define TCP_CA_CWR 2
#define TCP_CA_Disorder 1
#define TCP_CA_Loss 4
#define TCP_CA_Open 0
#define TCP_CA_Recovery 3
#define TCP_CC_INFO 26
#define TCP_CLOSE 7
#define TCP_CLOSE_WAIT 8
#define TCP_CLOSING 11
#define TCP_CM_INQ TCP_INQ
#define TCP_ESTABLISHED 1
#define TCP_FASTOPEN 23
#define TCP_FASTOPEN_CONNECT 30
#define TCP_FASTOPEN_KEY 33
#define TCP_FASTOPEN_NO_COOKIE 34
#define TCP_FIN_WAIT1 4
#define TCP_FIN_WAIT2 5
#define TCP_INQ 36
#define TCP_LAST_ACK 9
#define TCP_LISTEN 10
#define TCP_MD5SIG_EXT 32
#define TCP_MD5SIG_FLAG_IFINDEX 0x2
#define TCP_MD5SIG_FLAG_PREFIX 0x1
#define TCP_NOTSENT_LOWAT 25
#define TCP_RECEIVE_ZEROCOPY_FLAG_TLB_CLEAN_HINT 0x1
#define TCP_REPAIR_OFF 0
#define TCP_REPAIR_OFF_NO_WP -1
#define TCP_REPAIR_ON 1
#define TCP_REPAIR_WINDOW 29
#define TCP_SAVED_SYN 28
#define TCP_SAVE_SYN 27
#define TCP_SYN_RECV 3
#define TCP_SYN_SENT 2
#define TCP_TIMESTAMP 24
#define TCP_TIME_WAIT 6
#define TCP_TX_DELAY 37
#define TCP_ZEROCOPY_RECEIVE 35
#define TH_ACK 0x10
#define TH_FIN 0x01
#define TH_PUSH 0x08
#define TH_RST 0x04
#define TH_SYN 0x02
#define TH_URG 0x20

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
#define __BIG_ENDIAN 4321
#define FD_ZERO(s) do { int __i; unsigned long *__b=(s)->fds_bits; for(__i=sizeof (fd_set)/sizeof (long); __i; __i--) *__b++=0; } while(0)
#define betoh64(x) __bswap64(x)
#define letoh16(x) (uint16_t)(x)
#define __LITTLE_ENDIAN 1234
#define letoh64(x) (uint64_t)(x)
#define be32toh(x) __bswap32(x)
#define __LONG_MAX 0x7fffffffffffffffL
#define htole32(x) (uint32_t)(x)
#define be16toh(x) __bswap16(x)
#define htobe32(x) __bswap32(x)
#define NFDBITS (8*(int)sizeof(long))
#define PDP_ENDIAN __PDP_ENDIAN
#define __BYTE_ORDER 1234
#define FD_CLR(d, s) ((s)->fds_bits[(d)/(8*sizeof(long))] &= ~(1UL<<((d)%(8*sizeof(long)))))
#define BIG_ENDIAN __BIG_ENDIAN
#define htobe64(x) __bswap64(x)
#define betoh32(x) __bswap32(x)
#define letoh32(x) (uint32_t)(x)
#define le64toh(x) (uint64_t)(x)
#define betoh16(x) __bswap16(x)
#endif
