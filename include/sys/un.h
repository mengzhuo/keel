#ifndef _SYS_UN_H
#define _SYS_UN_H
#include <sys/socket.h>
struct sockaddr_un { sa_family_t sun_family; char sun_path[108]; };

/* musl 1.2.6 compatibility macros */
#define SUN_LEN(s) (2+strlen((s)->sun_path))
#endif
