#ifndef _CRABC_SYS_TIMERFD_H
#define _CRABC_SYS_TIMERFD_H
#include <time.h>
int timerfd_create(int, int);
int timerfd_settime(int, int, const struct itimerspec *, struct itimerspec *);
int timerfd_gettime(int, struct itimerspec *);
#define TFD_CLOEXEC 1
#define TFD_NONBLOCK 2

#endif
