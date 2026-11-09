#ifndef _CRABC_SYS_IOCTL_H
#define _CRABC_SYS_IOCTL_H
#include <stdarg.h>
struct winsize { unsigned short ws_row, ws_col, ws_xpixel, ws_ypixel; };
int ioctl(int, unsigned long, ...);
#define TIOCGWINSZ 0x5413
#define TIOCSWINSZ 0x5414

#endif
