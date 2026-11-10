/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _CRABC_LINK_H
#define _CRABC_LINK_H
#include <elf.h>
struct link_map { void *l_addr; char *l_name; void *l_ld; struct link_map *l_next, *l_prev; };


/* musl 1.2.6 compatibility macros */
#define ElfW(type) Elf64_ ## type
#endif
