#ifndef _CRABC_LINK_H
#define _CRABC_LINK_H
#include <elf.h>
struct link_map { void *l_addr; char *l_name; void *l_ld; struct link_map *l_next, *l_prev; };

#endif
