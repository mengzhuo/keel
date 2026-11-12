/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include "signal.h"
#include "stdio.h"
#include <errno.h>
#include <pthread.h>
#include <sys/mman.h>
#include <sys/select.h>
#include <sys/wait.h>
#include <unistd.h>

static void *mmap_anon(size_t size) {
    return mmap(NULL, size, PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
}

static volatile int got_signal = 0;
static volatile int on_altstack = 0;
static volatile int got_chld = 0;

static void chld_handler(int sig) {
    got_chld = sig;
}

static void usr1_handler(int sig) {
    got_signal = sig;
}

static void usr1_handler_alt(int sig) {
    got_signal = sig;
    stack_t ss;
    if (sigaltstack(NULL, &ss) == 0 && (ss.ss_flags & SS_ONSTACK))
        on_altstack = 1;
}

int main(void) {
    sigset_t set, oldmask, pending, fullset;
    struct sigaction sa, old_sa;
    int sig;

    sigfillset(&fullset);
    if (!sigismember(&fullset, SIGHUP)) return 1;
    if (!sigismember(&fullset, SIGTERM)) return 2;
    /* musl: signals 32/33 excluded from fillset */
    if (sigismember(&fullset, 32)) return 3;

    sigemptyset(&set);
    if (sigaddset(&set, 0) != -1) return 4;
    if (sigaddset(&set, 65) != -1) return 5;
    if (sigdelset(&set, 0) != -1) return 6;
    if (sigdelset(&set, 65) != -1) return 7;
    if (sigismember(&set, 0) != 0) return 8;
    if (sigismember(&set, 65) != 0) return 9;

    sa.sa_handler = usr1_handler;
    sa.sa_flags = 0;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGUSR1, &sa, &old_sa);

    sigemptyset(&set);
    errno = 0;
    if (sigprocmask(1700, &set, NULL) != -1 || errno != EINVAL) return 30;
    if (pthread_sigmask(SIG_BLOCK, &set, NULL) != 0) return 31;
    if (pthread_sigmask(1700, &set, NULL) != EINVAL) return 32;
    sigaddset(&set, SIGUSR1);
    sigprocmask(SIG_BLOCK, &set, &oldmask);

    raise(SIGUSR1);
    sigpending(&pending);
    if (!sigismember(&pending, SIGUSR1)) return 10;

    {
        sigset_t empty;
        sigemptyset(&empty);
        got_signal = 0;
        sigsuspend(&empty);
    }
    if (got_signal != SIGUSR1) return 11;

    raise(SIGUSR1);
    {
        siginfo_t info;
        struct timespec ts = {1, 0};
        if (sigtimedwait(&set, &info, &ts) != SIGUSR1) return 12;
        if (info.si_signo != SIGUSR1) return 13;
    }

    raise(SIGUSR1);
    {
        siginfo_t info;
        if (sigwaitinfo(&set, &info) != SIGUSR1) return 14;
        if (info.si_signo != SIGUSR1) return 15;
    }

    raise(SIGUSR1);
    sig = 0;
    if (sigwait(&set, &sig) != 0) return 16;
    if (sig != SIGUSR1) return 17;

    sigprocmask(SIG_SETMASK, &oldmask, NULL);

    {
        stack_t altstack, oldstack;
        altstack.ss_sp = mmap_anon(SIGSTKSZ);
        altstack.ss_size = SIGSTKSZ;
        altstack.ss_flags = 0;
        if (sigaltstack(&altstack, &oldstack) != 0) return 18;

        sa.sa_handler = usr1_handler_alt;
        sa.sa_flags = SA_ONSTACK;
        sigemptyset(&sa.sa_mask);
        sigaction(SIGUSR1, &sa, NULL);

        got_signal = 0;
        on_altstack = 0;
        raise(SIGUSR1);
        if (got_signal != SIGUSR1) return 19;
        if (!on_altstack) return 20;

        sigaltstack(&oldstack, NULL);
    }

    sa.sa_handler = usr1_handler;
    sa.sa_flags = 0;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGUSR1, &sa, NULL);
    got_signal = 0;
    tgkill(getpid(), getpid(), SIGUSR1);
    if (got_signal != SIGUSR1) return 21;

    sigaction(SIGUSR1, &old_sa, NULL);

    {
        struct sigaction ca, oldca;
        sigset_t block, empty, saved;
        struct timespec ts = {5, 0};
        ca.sa_handler = chld_handler;
        ca.sa_flags = 0;
        sigemptyset(&ca.sa_mask);
        sigaction(SIGCHLD, &ca, &oldca);
        sigemptyset(&block);
        sigaddset(&block, SIGCHLD);
        sigprocmask(SIG_BLOCK, &block, &saved);
        pid_t pid = fork();
        if (pid < 0) return 33;
        if (pid == 0) _exit(0);
        sigemptyset(&empty);
        got_chld = 0;
        errno = 0;
        int pr = pselect(0, NULL, NULL, NULL, &ts, &empty);
        if (pr != -1 || errno != EINTR || got_chld != SIGCHLD) return 34;
        int status;
        if (waitpid(pid, &status, 0) != pid) return 35;
        sigprocmask(SIG_SETMASK, &saved, NULL);
        sigaction(SIGCHLD, &oldca, NULL);
    }

    puts("signal ok");
    return 0;
}
