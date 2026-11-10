/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

/* Loaded by tests/vdso.rs to force the loader's TLS block expansion. */
__thread int vdso_tls_var;

int vdso_tls_touch(void) {
    return ++vdso_tls_var;
}
