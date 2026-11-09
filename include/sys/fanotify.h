#ifndef _CRABC_SYS_FANOTIFY_H
#define _CRABC_SYS_FANOTIFY_H
#include <stdint.h>
#define FAN_CLOEXEC 1
#define FAN_NONBLOCK 2
#define FAN_ACCESS 1
#define FAN_MODIFY 2
#define FAN_CLOSE_WRITE 8
#define FAN_CLOSE_NOWRITE 16
#define FAN_OPEN 32
#define FAN_EVENT_ON_CHILD 0x08000000

#endif
