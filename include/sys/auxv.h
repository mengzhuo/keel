#ifndef _CRABC_SYS_AUXV_H
#define _CRABC_SYS_AUXV_H
#include <stddef.h>
unsigned long getauxval(unsigned long);
#define AT_NULL 0
#define AT_PHDR 3
#define AT_PHNUM 5
#define AT_PAGESZ 6
#define AT_BASE 7
#define AT_ENTRY 9
#define AT_RANDOM 25

#endif
