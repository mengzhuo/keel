/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <unistd.h>

int main(void) {
    int d = open("/", O_RDONLY | O_DIRECTORY);
    if (d < 0) return 1;
    close(d);
    int f = open("/tmp", O_RDONLY | O_NOFOLLOW | O_DIRECTORY);
    if (f < 0) return 2;
    close(f);
    puts("oflags ok");
    return 0;
}
