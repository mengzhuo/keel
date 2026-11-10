/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <dlfcn.h>
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cond = PTHREAD_COND_INITIALIZER;
static int early_ready;
static int early_go;
static int failed;
static int (*bump)(void);
static int (*get)(void);

static void *early_thread(void *arg) {
    (void)arg;
    pthread_mutex_lock(&lock);
    early_ready = 1;
    pthread_cond_broadcast(&cond);
    while (!early_go) pthread_cond_wait(&cond, &lock);
    pthread_mutex_unlock(&lock);
    if (bump() != 8 || bump() != 9 || get() != 9) failed = 1;
    return NULL;
}

static void *late_thread(void *arg) {
    (void)arg;
    if (bump() != 8 || bump() != 9 || get() != 9) failed = 2;
    return NULL;
}

int main(void) {
    const char *path = getenv("TLSDESC_DSO");
    if (!path) {
        puts("TLSDESC_DSO is not set");
        return 1;
    }

    pthread_t early;
    if (pthread_create(&early, NULL, early_thread, NULL) != 0) return 1;
    pthread_mutex_lock(&lock);
    while (!early_ready) pthread_cond_wait(&cond, &lock);
    pthread_mutex_unlock(&lock);

    void *handle = dlopen(path, RTLD_NOW);
    if (!handle) {
        printf("dlopen: %s\n", dlerror());
        return 1;
    }
    bump = (int (*)(void))dlsym(handle, "tlsdesc_bump");
    get = (int (*)(void))dlsym(handle, "tlsdesc_get");
    if (!bump || !get) return 1;

    pthread_mutex_lock(&lock);
    early_go = 1;
    pthread_cond_broadcast(&cond);
    pthread_mutex_unlock(&lock);
    pthread_join(early, NULL);
    if (failed) {
        printf("early thread failed: %d\n", failed);
        return 1;
    }

    pthread_t late[4];
    for (int i = 0; i < 4; i++) {
        if (pthread_create(&late[i], NULL, late_thread, NULL) != 0) return 1;
    }
    for (int i = 0; i < 4; i++) pthread_join(late[i], NULL);
    if (failed) {
        printf("late thread failed: %d\n", failed);
        return 1;
    }

    if (bump() != 8 || bump() != 9) {
        printf("main thread failed: %d\n", get());
        return 1;
    }

    puts("tlsdesc ok");
    return 0;
}
