#ifndef _CRABC_SYS_EVENTFD_H
#define _CRABC_SYS_EVENTFD_H
#include <stdint.h>
int eventfd(unsigned int, int);
#define EFD_CLOEXEC 1
#define EFD_NONBLOCK 2
#define EFD_SEMAPHORE 1

#endif
