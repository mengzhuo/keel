/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include "stdio.h"
#include <errno.h>
#include <fcntl.h>
#include <unistd.h>

int main(void) {
    int c = getchar();
    if (c != 'h') return 1;

    char buf[8];
    if (fgets(buf, sizeof(buf), stdin) == NULL) return 2;
    if (buf[0] != 'i' || buf[1] != '\n' || buf[2] != 0) return 3;

    c = fgetc(stdin);
    if (c != 'x') return 4;
    if (ungetc(c, stdin) != c) return 5;
    if (fgetc(stdin) != c) return 6;

    char big[4];
    size_t n = fread(big, 1, sizeof(big), stdin);
    if (n != 3) return 7;
    if (big[0] != 'y' || big[1] != '\n' || big[2] != 'z') return 8;

    /* EAGAIN from a nonblocking stream must stay visible as errno after a
       short fread; BusyBox wget polls and retries based on it. */
    int p[2];
    if (pipe(p) != 0) return 9;
    if (fcntl(p[0], F_SETFL, O_NONBLOCK) < 0) return 10;
    FILE *nb = fdopen(p[0], "r");
    if (!nb) return 11;
    errno = 0;
    if (fgetc(nb) != EOF) return 12;
    if (errno != EAGAIN) return 13;
    if (!ferror(nb)) return 14;
    clearerr(nb);
    if (write(p[1], "z", 1) != 1) return 15;
    if (fgetc(nb) != 'z') return 16;
    fclose(nb);
    close(p[1]);

    puts("stdio input ok");
    return 0;
}
