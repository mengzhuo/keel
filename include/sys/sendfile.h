#ifndef _CRABC_SYS_SENDFILE_H
#define _CRABC_SYS_SENDFILE_H
#include <sys/types.h>
ssize_t sendfile(int, int, off_t *, size_t);

#endif
