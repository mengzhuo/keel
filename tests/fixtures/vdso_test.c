/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/auxv.h>
#include <sys/random.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

static void *thread_random(void *arg) {
    uintptr_t seed = (uintptr_t)arg;
    unsigned char buf[64];
    for (int i = 0; i < 100; i++) {
        if (getrandom(buf, sizeof buf, 0) != (ssize_t)sizeof buf) return (void *)1;
        int nonzero = 0;
        for (size_t j = 0; j < sizeof buf; j++) nonzero |= buf[j];
        if (!nonzero) return (void *)1;
    }
    /* Every thread must have its own getrandom state. */
    if (getrandom(buf, sizeof buf, GRND_NONBLOCK) != (ssize_t)sizeof buf)
        return (void *)1;
    (void)seed;
    return NULL;
}

static void *thread_exit_random(void *arg) {
    unsigned char buf[32];
    (void)arg;
    if (getrandom(buf, sizeof buf, 0) != (ssize_t)sizeof buf) return (void *)1;
    pthread_exit(NULL);
}

static int same_bytes(const unsigned char *a, const unsigned char *b, size_t n) {
    return memcmp(a, b, n) == 0;
}

int main(void) {
    unsigned char buf[64];
    int nonzero;

    if (getauxval(AT_SYSINFO_EHDR) == 0) {
        puts("vDSO missing");
        return 1;
    }

    /* Basic getrandom/getentropy semantics. */
    if (getrandom(buf, sizeof buf, 0) != (ssize_t)sizeof buf) {
        perror("getrandom");
        return 1;
    }
    nonzero = 0;
    for (size_t i = 0; i < sizeof buf; i++) nonzero |= buf[i];
    if (!nonzero) {
        puts("getrandom returned zeros");
        return 1;
    }
    if (getrandom(buf, 0, 0) != 0) {
        puts("zero-length getrandom");
        return 1;
    }
    errno = 0;
    if (getrandom(buf, sizeof buf, 0x100) != -1 || errno != EINVAL) {
        printf("bad flags: errno=%d\n", errno);
        return 1;
    }
    if (getrandom(buf, sizeof buf, GRND_RANDOM) != (ssize_t)sizeof buf) {
        perror("getrandom GRND_RANDOM");
        return 1;
    }
    if (getrandom(buf, sizeof buf, GRND_INSECURE) != (ssize_t)sizeof buf) {
        perror("getrandom GRND_INSECURE");
        return 1;
    }
    unsigned char entropy[16];
    if (getentropy(entropy, sizeof entropy) != 0) {
        perror("getentropy");
        return 1;
    }
    {
        unsigned char big[257];
        errno = 0;
        if (getentropy(big, sizeof big) != -1 || errno != EIO) {
            printf("getentropy limit: errno=%d\n", errno);
            return 1;
        }
    }

    /* Loading a DSO with TLS can grow this thread's TCB, which may leave the
     * thread pointer only 4-byte aligned.  The getrandom state slot must
     * survive that move and must not be touched with aligned atomics. */
    const char *dso_path = getenv("VDSO_TLS_DSO");
    if (dso_path) {
        void *handle = dlopen(dso_path, RTLD_NOW);
        if (!handle) {
            printf("dlopen: %s\n", dlerror());
            return 1;
        }
        int (*touch)(void) = (int (*)(void))dlsym(handle, "vdso_tls_touch");
        if (!touch || touch() != 1) {
            puts("TLS DSO touch failed");
            return 1;
        }
        if (getrandom(buf, sizeof buf, 0) != (ssize_t)sizeof buf) {
            puts("getrandom after TLS expansion failed");
            return 1;
        }
        if (pthread_self() == 0) {
            puts("pthread_self after TLS expansion failed");
            return 1;
        }
        dlclose(handle);
    }

    /* Time fast paths must agree with each other. */
    struct timespec ts;
    struct timeval tv;
    time_t t = 0;
    if (clock_gettime(CLOCK_REALTIME, &ts) != 0) return 1;
    if (gettimeofday(&tv, NULL) != 0) return 1;
    if (gettimeofday(NULL, NULL) != 0) return 1;
    if (time(&t) != t) return 1;
    if (t < ts.tv_sec - 2 || t > ts.tv_sec + 2) return 1;
    if (tv.tv_sec < ts.tv_sec - 2 || tv.tv_sec > ts.tv_sec + 2) return 1;
    if (tv.tv_usec < 0 || tv.tv_usec >= 1000000) return 1;
    errno = 0;
    if (clock_gettime((clockid_t)0x7fffffff, &ts) != -1 || errno != EINVAL) {
        printf("bad clockid: errno=%d\n", errno);
        return 1;
    }
    if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) return 1;
    if (clock_gettime(CLOCK_BOOTTIME, &ts) != 0) return 1;

    if (sched_getcpu() < 0) {
        perror("sched_getcpu");
        return 1;
    }

    /* Threads each get their own state, and exiting threads release it. */
    for (int round = 0; round < 4; round++) {
        pthread_t th[8];
        for (int i = 0; i < 8; i++) {
            if (pthread_create(&th[i], NULL, thread_random, (void *)(uintptr_t)i))
                return 1;
        }
        for (int i = 0; i < 8; i++) {
            void *ret = NULL;
            if (pthread_join(th[i], &ret) || ret != NULL) return 1;
        }
        pthread_t exiter;
        if (pthread_create(&exiter, NULL, thread_exit_random, NULL)) return 1;
        void *ret = NULL;
        if (pthread_join(exiter, &ret) || ret != NULL) return 1;
    }

    /* After fork the child must not reuse the parent's buffered bytes. */
    unsigned char parent_pre[32];
    if (getrandom(parent_pre, sizeof parent_pre, 0) != (ssize_t)sizeof parent_pre) return 1;
    pid_t pid = fork();
    if (pid < 0) return 1;
    if (pid == 0) {
        unsigned char child[32];
        if (getrandom(child, sizeof child, 0) != (ssize_t)sizeof child) _exit(1);
        if (same_bytes(child, parent_pre, sizeof child)) _exit(2);
        _exit(0);
    }
    int status = 0;
    if (waitpid(pid, &status, 0) != pid) return 1;
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        printf("fork getrandom status=%d\n", status);
        return 1;
    }

    puts("vdso ok");
    return 0;
}
