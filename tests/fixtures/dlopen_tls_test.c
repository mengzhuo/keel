/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <dlfcn.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc < 2) { puts("usage: dlopen_tls_test lib.so"); return 1; }
    void *h = dlopen(argv[1], RTLD_NOW);
    if (!h) { printf("dlopen fail: %s\n", dlerror()); return 2; }
    int (*who)(void) = (int (*)(void))dlsym(h, "who");
    int (*btls)(void) = (int (*)(void))dlsym(h, "b_tls_get");
    if (!who || !btls) { printf("dlsym fail\n"); return 3; }
    printf("who=%d tls=%d\n", who(), btls());
    return (who() == 2 && btls() == 7) ? 0 : 4;
}
