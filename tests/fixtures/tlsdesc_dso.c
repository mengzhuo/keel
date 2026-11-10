/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

/* Loaded by tests/tlsdesc.rs after threads already exist. */
__thread int tlsdesc_var = 7;

int tlsdesc_bump(void) {
    return ++tlsdesc_var;
}

int tlsdesc_get(void) {
    return tlsdesc_var;
}
