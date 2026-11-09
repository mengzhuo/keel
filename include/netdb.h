#ifndef _NETDB_H
#define _NETDB_H

#include <features.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/socket.h>
#include <netinet/in.h>

#ifdef __cplusplus
extern "C" {
#endif

struct addrinfo {
    int ai_flags;
    int ai_family;
    int ai_socktype;
    int ai_protocol;
    socklen_t ai_addrlen;
    struct sockaddr *ai_addr;
    char *ai_canonname;
    struct addrinfo *ai_next;
};

#define AI_PASSIVE      0x01
#define AI_CANONNAME    0x02
#define AI_NUMERICHOST  0x04
#define AI_V4MAPPED     0x08
#define AI_ALL          0x10
#define AI_ADDRCONFIG   0x20
#define AI_NUMERICSERV  0x400

#define NI_NUMERICHOST  0x01
#define NI_NUMERICSERV  0x02
#define NI_NOFQDN       0x04
#define NI_NAMEREQD     0x08
#define NI_DGRAM        0x10
#define NI_NUMERICSCOPE 0x100

#define EAI_BADFLAGS   -1
#define EAI_NONAME     -2
#define EAI_AGAIN      -3
#define EAI_FAIL       -4
#define EAI_NODATA     -5
#define EAI_FAMILY     -6
#define EAI_SOCKTYPE   -7
#define EAI_SERVICE    -8
#define EAI_MEMORY     -10
#define EAI_SYSTEM     -11
#define EAI_OVERFLOW   -12

#define IPPORT_RESERVED 1024

#define HOST_NOT_FOUND 1
#define TRY_AGAIN      2
#define NO_RECOVERY    3
#define NO_DATA        4
#define NO_ADDRESS     NO_DATA

struct netent {
    char *n_name;
    char **n_aliases;
    int n_addrtype;
    uint32_t n_net;
};

struct hostent {
    char *h_name;
    char **h_aliases;
    int h_addrtype;
    int h_length;
    char **h_addr_list;
};
#define h_addr h_addr_list[0]

struct servent {
    char *s_name;
    char **s_aliases;
    int s_port;
    char *s_proto;
};

struct protoent {
    char *p_name;
    char **p_aliases;
    int p_proto;
};

int *__h_errno_location(void);
#define h_errno (*__h_errno_location())

int getaddrinfo(const char *, const char *, const struct addrinfo *, struct addrinfo **);
void freeaddrinfo(struct addrinfo *);
int getnameinfo(const struct sockaddr *, socklen_t, char *, socklen_t, char *, socklen_t, int);
const char *gai_strerror(int);

struct hostent *gethostbyname(const char *);
struct hostent *gethostbyname2(const char *, int);
struct hostent *gethostbyaddr(const void *, socklen_t, int);
void sethostent(int);
void endhostent(void);
struct hostent *gethostent(void);

struct servent *getservbyname(const char *, const char *);
struct servent *getservbyport(int, const char *);
void setservent(int);
void endservent(void);
struct servent *getservent(void);

struct protoent *getprotobyname(const char *);
struct protoent *getprotobynumber(int);
void setprotoent(int);
void endprotoent(void);
struct protoent *getprotoent(void);

struct netent *getnetbyname(const char *);
struct netent *getnetbyaddr(uint32_t, int);
void setnetent(int);
void endnetent(void);
struct netent *getnetent(void);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define EAI_ADDRFAMILY -9
#define EAI_ALLDONE -103
#define EAI_CANCELED -101
#define EAI_IDN_ENCODE -105
#define EAI_INPROGRESS -100
#define EAI_INTR -104
#define EAI_NOTCANCELED -102
#define NI_MAXHOST 255
#define NI_MAXSERV 32

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
