/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <sys/mman.h>

int main(void)
{
    /* A zero-capacity stream whose buffer ends exactly at a guard page.
       fmemopen(..., 0, "w+") must not write a NUL terminator: there is no
       byte available in the stream's storage. */
    unsigned char *pages = mmap(0, 8192, PROT_READ | PROT_WRITE,
                                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (pages == MAP_FAILED) { puts("mmap fail"); exit(1); }
    if (mprotect(pages + 4096, 4096, PROT_NONE) != 0) {
        puts("mprotect fail");
        exit(1);
    }

    FILE *f = fmemopen(pages + 4096, 0, "w+");
    if (!f) { puts("fmemopen zero nonnull fail"); exit(1); }
    fclose(f);

    f = fmemopen(NULL, 0, "w+");
    if (!f) { puts("fmemopen zero null fail"); exit(1); }
    fclose(f);

    f = fmemopen(NULL, 0, "a+");
    if (!f) { puts("fmemopen zero append fail"); exit(1); }
    fclose(f);

    puts("fmemopen zero ok");
    return 0;
}
