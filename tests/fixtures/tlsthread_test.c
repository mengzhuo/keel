/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <pthread.h>
#include <stdio.h>

static __thread int v = 5;

static void *worker(void *a) {
    long id = (long)a;
    if (v != 5) return (void *)-1;
    v = (int)(id * 10);
    if (v != (int)(id * 10)) return (void *)-2;
    return (void *)(long)v;
}

int main(void) {
    pthread_t t[4];
    for (long i = 0; i < 4; i++) pthread_create(&t[i], 0, worker, (void *)(i + 1));
    for (long i = 0; i < 4; i++) {
        void *r;
        pthread_join(t[i], &r);
        if ((long)r != (i + 1) * 10) return 1;
    }
    if (v != 5) return 2;
    puts("tls threads ok");
    return 0;
}
