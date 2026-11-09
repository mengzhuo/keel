#ifndef _CRABC_SYS_TIMERFD_H
#define _CRABC_SYS_TIMERFD_H
#include <time.h>
#include <fcntl.h>
int timerfd_create(int, int);
int timerfd_settime(int, int, const struct itimerspec *, struct itimerspec *);
int timerfd_gettime(int, struct itimerspec *);
#define TFD_CLOEXEC O_CLOEXEC
#define TFD_NONBLOCK O_NONBLOCK


/* musl 1.2.6 compatibility macros */
#define TFD_TIMER_ABSTIME 1
#define TFD_TIMER_CANCEL_ON_SET (1 << 1)
#endif
