/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <grp.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

#define MEMBERS 100

int main(void) {
    char path[64];
    snprintf(path, sizeof path, "/tmp/crabc-group-%d", (int)getpid());
    FILE *out = fopen(path, "w");
    if (!out) return 1;
    fputs("biggroup:x:1234:", out);
    for (int i = 0; i < MEMBERS; i++) {
        if (i) fputc(',', out);
        fprintf(out, "user%03d", i);
    }
    fputc('\n', out);
    fclose(out);

    FILE *in = fopen(path, "r");
    unlink(path);
    if (!in) return 1;
    struct group *g = fgetgrent(in);
    fclose(in);
    if (!g) return 1;

    int n = 0;
    while (g->gr_mem[n]) n++;
    printf("members=%d last=%s\n", n, g->gr_mem[n - 1]);
    return n == MEMBERS && strcmp(g->gr_mem[n - 1], "user099") == 0 ? 0 : 1;
}
