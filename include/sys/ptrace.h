#ifndef _CRABC_SYS_PTRACE_H
#define _CRABC_SYS_PTRACE_H
#include <sys/types.h>
long ptrace(int, ...);
#define PTRACE_TRACEME 0
#define PTRACE_PEEKTEXT 1
#define PTRACE_PEEKDATA 2
#define PTRACE_POKETEXT 4
#define PTRACE_POKEDATA 5
#define PTRACE_CONT 7
#define PTRACE_KILL 8
#define PTRACE_SINGLESTEP 9
#define PTRACE_GETREGS 12
#define PTRACE_SETREGS 13
#define PTRACE_ATTACH 16
#define PTRACE_DETACH 17
#define PTRACE_SYSCALL 24

#endif
