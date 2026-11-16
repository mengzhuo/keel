/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <signal.h>
#include <stdio.h>

static volatile sig_atomic_t seen;

static void handler(int sig, siginfo_t *info, void *ctx) {
    ucontext_t *uc = (ucontext_t *)ctx;
    (void)sig;
    (void)info;
    void *m = uc ? (void *)&uc->uc_mcontext : 0;
    void *m2 = uc ? (void *)&uc->uc_sigmask : 0;
    seen = (uc != NULL && m != NULL && m2 != NULL);
}

int main(void) {
    struct sigaction sa = {0};
    sa.sa_sigaction = handler;
    sa.sa_flags = SA_SIGINFO;
    if (sigaction(SIGUSR1, &sa, 0) != 0) return 1;
    if (raise(SIGUSR1) != 0) return 2;
    if (!seen) return 3;
    puts("ucontext ok");
    return 0;
}
