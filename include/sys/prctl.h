#ifndef _CRABC_SYS_PRCTL_H
#define _CRABC_SYS_PRCTL_H
int prctl(int, ...);
#define PR_SET_NAME 15
#define PR_GET_NAME 16
#define PR_SET_PDEATHSIG 1
#define PR_GET_PDEATHSIG 2

#endif
