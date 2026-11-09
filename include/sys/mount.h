#ifndef _CRABC_SYS_MOUNT_H
#define _CRABC_SYS_MOUNT_H
#include <sys/types.h>
int mount(const char *, const char *, const char *, unsigned long, const void *);
int umount(const char *);
int umount2(const char *, int);
#define MS_RDONLY 1
#define MS_NOSUID 2
#define MS_NODEV 4
#define MS_NOEXEC 8
#define MS_REMOUNT 32
#define MNT_FORCE 1
#define MNT_DETACH 2

#endif
