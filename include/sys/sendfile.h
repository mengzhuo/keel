#ifndef _CRABC_SYS_SENDFILE_H
#define _CRABC_SYS_SENDFILE_H
#include <features.h>
#include <unistd.h>
#include <sys/types.h>
ssize_t sendfile(int, int, off_t *, size_t);


/* musl 1.2.6 per-header compatibility macros */
#define L_XTND 2
#define L_INCR 1
#define L_SET 0
#endif
