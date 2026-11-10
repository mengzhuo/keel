/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <stdio.h>

int main(int argc, char **argv, char **envp) {
    int envc = 0;
    while (envp[envc] != NULL) envc++;
    printf("%d %d %s %s\n", argc, envc, argv[1], argv[argc - 1]);
    return 0;
}
