#ifndef _SYS_IPC_H
#define _SYS_IPC_H

#ifdef __cplusplus
extern "C" {
#endif

#include <features.h>
#include <sys/types.h>

struct ipc_perm {
    key_t key;
    uid_t uid;
    gid_t gid;
    uid_t cuid;
    gid_t cgid;
    mode_t mode;
    int seq;
    long __pad1;
    long __pad2;
};

#define IPC_CREAT  01000
#define IPC_EXCL   02000
#define IPC_NOWAIT 04000

#define IPC_RMID 0
#define IPC_SET  1
#define IPC_STAT 2
#define IPC_INFO 3

#define IPC_PRIVATE ((key_t) 0)

key_t ftok(const char *, int);

#ifdef __cplusplus
}
#endif

/* musl 1.2.6 compatibility macros */
#define __ipc_perm_key __key
#define __ipc_perm_seq __seq
#define __key key
#define __seq seq

/* musl 1.2.6 per-header compatibility macros */
#endif
