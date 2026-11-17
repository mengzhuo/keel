/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>

int ext = 123;

int main(void) {
    const char *path = getenv("PC32_DSO");
    if (!path) {
        puts("PC32_DSO is not set");
        return 1;
    }
    void *handle = dlopen(path, RTLD_NOW);
    if (!handle) {
        printf("dlopen: %s\n", dlerror());
        return 1;
    }
    int (*check)(void) = (int (*)(void))dlsym(handle, "pc32_check");
    if (!check || check() != 0) {
        puts("pc32 relocation failed");
        return 1;
    }
    puts("pc32 ok");
    return 0;
}
