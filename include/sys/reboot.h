#ifndef _CRABC_SYS_REBOOT_H
#define _CRABC_SYS_REBOOT_H
#include <unistd.h>
#define RB_AUTOBOOT 0x01234567
#define RB_HALT_SYSTEM 0xcdef0123
#define RB_POWER_OFF 0x4321fedc


/* musl 1.2.6 compatibility macros */
#define RB_DISABLE_CAD 0
#define RB_ENABLE_CAD 0x89abcdef
#define RB_KEXEC 0x45584543
#define RB_SW_SUSPEND 0xd000fce2
#endif
