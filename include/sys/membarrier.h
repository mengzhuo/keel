#ifndef _CRABC_SYS_MEMBARRIER_H
#define _CRABC_SYS_MEMBARRIER_H
#include <unistd.h>
int membarrier(int, int, int);
#define MEMBARRIER_CMD_QUERY 0
#define MEMBARRIER_CMD_GLOBAL 1
#define MEMBARRIER_CMD_PRIVATE_EXPEDITED 8

#endif
