#ifndef _CRABC_STROPTS_H
#define _CRABC_STROPTS_H
#include <sys/ioctl.h>
#define __SID ('S' << 8)
#define I_PUSH (__SID | 2)
#define I_POP  (__SID | 3)

#endif
