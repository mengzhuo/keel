#ifndef _CRABC_SYS_SWAP_H
#define _CRABC_SYS_SWAP_H
#include <sys/types.h>
int swapon(const char *, int);
int swapoff(const char *);
#define SWAP_FLAG_PREFER 0x8000

#endif
