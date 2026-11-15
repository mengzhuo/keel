/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <pthread.h>
#include <stdio.h>
#include <errno.h>

static int counter = 0;
static pthread_mutex_t mutex;
static int self_join_result;

static void *self_joiner(void *arg) {
    (void)arg;
    self_join_result = pthread_join(pthread_self(), NULL);
    return NULL;
}

static void *worker(void *arg) {
    (void)arg;
    for (int i = 0; i < 10000; i++) {
        pthread_mutex_lock(&mutex);
        counter++;
        pthread_mutex_unlock(&mutex);
    }
    return NULL;
}

int main(void) {
    pthread_t t1, t2;

    pthread_mutex_init(&mutex, NULL);
    pthread_create(&t1, NULL, worker, NULL);
    pthread_create(&t2, NULL, worker, NULL);
    pthread_join(t1, NULL);
    pthread_join(t2, NULL);

    /* A thread joining itself must fail with EDEADLK, not hang. */
    pthread_t t3;
    if (pthread_create(&t3, NULL, self_joiner, NULL) != 0) return 1;
    pthread_join(t3, NULL);
    if (self_join_result != EDEADLK) return 1;

    if (counter == 20000) {
        printf("pthread ok\n");
        return 0;
    }
    printf("counter=%d\n", counter);
    return 1;
}
