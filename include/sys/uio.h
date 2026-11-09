#ifndef _SYS_UIO_H
#define _SYS_UIO_H
#include <stddef.h>
#include <sys/types.h>
#ifndef __crabc_struct_iovec_defined
#define __crabc_struct_iovec_defined
struct iovec { void *iov_base; size_t iov_len; };
#endif
ssize_t readv(int, const struct iovec *, int);
ssize_t writev(int, const struct iovec *, int);
ssize_t preadv(int, const struct iovec *, int, off_t);
ssize_t pwritev(int, const struct iovec *, int, off_t);
ssize_t preadv2(int, const struct iovec *, int, off_t, int);
ssize_t pwritev2(int, const struct iovec *, int, off_t, int);
#endif
