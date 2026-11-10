/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#ifndef _SYS_CACHECTL_H
#define _SYS_CACHECTL_H

#ifdef __cplusplus
extern "C" {
#endif

int riscv_flush_icache(void *, void *, unsigned long);
int __riscv_flush_icache(void *, void *, unsigned long);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define BCACHE (ICACHE|DCACHE)
#define CACHEABLE 0
#define DCACHE (1<<1)
#define ICACHE (1<<0)
#define UNCACHEABLE 1
#endif
