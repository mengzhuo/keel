/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

int who(void) { return 2; }

__thread int b_tls = 7;

int b_tls_get(void) { return b_tls; }
