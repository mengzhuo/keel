/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>

#define THREADS 1500
#define STACK_SIZE (64 * 1024)

static pthread_mutex_t mutex = PTHREAD_MUTEX_INITIALIZER;
static pthread_cond_t cond = PTHREAD_COND_INITIALIZER;
static int started;
static int release;

static void *worker(void *arg) {
    (void)arg;
    if (pthread_self() == 0) return (void *)1;
    pthread_mutex_lock(&mutex);
    started++;
    pthread_cond_broadcast(&cond);
    while (!release) pthread_cond_wait(&cond, &mutex);
    pthread_mutex_unlock(&mutex);
    return NULL;
}

int main(void) {
    pthread_t self = pthread_self();
    if (self == 0 || !pthread_equal(self, pthread_self())) return 1;

    pthread_attr_t attr;
    pthread_t *threads = calloc(THREADS, sizeof(*threads));
    if (!threads) return 1;
    if (pthread_attr_init(&attr) != 0) return 1;
    if (pthread_attr_setstacksize(&attr, STACK_SIZE) != 0) return 1;

    /* Keep every thread alive at once: the old fixed table stopped at 1024. */
    for (int i = 0; i < THREADS; i++) {
        if (pthread_create(&threads[i], &attr, worker, NULL) != 0) {
            printf("create failed at %d\n", i);
            return 1;
        }
    }
    pthread_mutex_lock(&mutex);
    while (started != THREADS) pthread_cond_wait(&cond, &mutex);
    release = 1;
    pthread_cond_broadcast(&cond);
    pthread_mutex_unlock(&mutex);

    for (int i = 0; i < THREADS; i++) {
        void *ret = NULL;
        if (pthread_join(threads[i], &ret) != 0 || ret != NULL) {
            printf("join failed at %d\n", i);
            return 1;
        }
    }
    pthread_attr_destroy(&attr);
    free(threads);

    /* Slots must be reusable after the first batch exits. */
    for (int i = 0; i < 64; i++) {
        pthread_t t;
        if (pthread_create(&t, NULL, worker, NULL) != 0) return 1;
        pthread_mutex_lock(&mutex);
        release = 1;
        pthread_cond_broadcast(&cond);
        pthread_mutex_unlock(&mutex);
        if (pthread_join(t, NULL) != 0) return 1;
        pthread_mutex_lock(&mutex);
        release = 0;
        pthread_mutex_unlock(&mutex);
    }

    puts("thread many ok");
    return 0;
}
