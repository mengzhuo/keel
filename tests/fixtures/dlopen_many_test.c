/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    if (argc < 3) return 1;
    const char *dir = argv[1];
    int count = atoi(argv[2]);
    char path[4096];
    int loaded = 0;
    for (int i = 1; i <= count; i++) {
        if (snprintf(path, sizeof path, "%s/libmany%d.so", dir, i) >= (int)sizeof path)
            return 1;
        void *handle = dlopen(path, RTLD_NOW);
        if (!handle) {
            printf("failed at %d: %s\n", i, dlerror());
            return 1;
        }
        int (*foo)(void) = (int (*)(void))dlsym(handle, "foo");
        if (!foo || foo() != 42) return 1;
        loaded++;
    }
    printf("loaded=%d\n", loaded);
    return loaded == count ? 0 : 1;
}
