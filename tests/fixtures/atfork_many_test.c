/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <pthread.h>
#include <stdio.h>
#include <sys/wait.h>
#include <unistd.h>

#define HANDLERS 128

static int prepare_count;
static int parent_count;
static int child_count;

static void on_prepare(void) { prepare_count++; }
static void on_parent(void) { parent_count++; }
static void on_child(void) { child_count++; }

int main(void) {
    for (int i = 0; i < HANDLERS; i++) {
        if (pthread_atfork(on_prepare, on_parent, on_child) != 0) {
            printf("register failed at %d\n", i);
            return 1;
        }
    }

    pid_t pid = fork();
    if (pid < 0) return 1;
    if (pid == 0) _exit(child_count == HANDLERS ? 0 : 1);

    int status = 0;
    if (waitpid(pid, &status, 0) != pid) return 1;
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        printf("child status=%d\n", status);
        return 1;
    }
    if (prepare_count != HANDLERS || parent_count != HANDLERS) {
        printf("prepare=%d parent=%d\n", prepare_count, parent_count);
        return 1;
    }
    puts("atfork many ok");
    return 0;
}
