/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include "unistd.h"
#include "stdio.h"
#include "stdlib.h"

static int qc_fd = -1;
static void qc_a(void) { write(qc_fd, "A", 1); }
static void qc_b(void) { write(qc_fd, "B", 1); }

int main(void) {
    int status = -1;
    pid_t pid = fork();
    if (pid < 0) return 1;
    if (pid == 0) {
        _exit(42);
    }
    pid_t w = waitpid(pid, &status, 0);
    if (w != pid) return 2;
    if (!WIFEXITED(status)) return 3;
    if (WEXITSTATUS(status) != 42) return 4;

    pid = fork();
    if (pid < 0) return 5;
    if (pid == 0) {
        char *const argv[] = {(char*)"/bin/true", NULL};
        char *const envp[] = {NULL};
        execve("/bin/true", argv, envp);
        _exit(99);
    }
    w = wait(&status);
    if (w != pid) return 6;
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) return 7;

    if (getpid() <= 0) return 8;
    if (getppid() <= 0) return 9;
    if (getuid() == (uid_t)-1) return 10;
    if (getgid() == (gid_t)-1) return 11;

    /* quick_exit runs at_quick_exit handlers in reverse order. */
    {
        int p[2];
        if (pipe(p) != 0) return 12;
        pid = fork();
        if (pid < 0) return 13;
        if (pid == 0) {
            close(p[0]);
            qc_fd = p[1];
            if (at_quick_exit(qc_a) != 0 || at_quick_exit(qc_b) != 0) _exit(90);
            quick_exit(7);
        }
        close(p[1]);
        char buf[3] = {0, 0, 0};
        ssize_t n = 0;
        while (n < 2) {
            ssize_t r = read(p[0], buf + n, 2 - n);
            if (r <= 0) break;
            n += r;
        }
        close(p[0]);
        w = waitpid(pid, &status, 0);
        if (n != 2 || buf[0] != 'B' || buf[1] != 'A') return 14;
        if (!WIFEXITED(status) || WEXITSTATUS(status) != 7) return 15;
    }

    puts("process ok");
    return 0;
}
