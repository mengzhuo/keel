/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

/* x86_64 TEXTREL-style PC-relative dynamic relocation. */
extern int ext;
extern int pc32_value;

int pc32_check(void) {
    return (int)((long)pc32_value + (long)&pc32_value - (long)&ext);
}

__asm__(".data\n.globl pc32_value\n.align 4\npc32_value: .long ext - .\n.text\n");
