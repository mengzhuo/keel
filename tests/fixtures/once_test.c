/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */
#include <pthread.h>
#include <stdio.h>
#include <threads.h>

static once_flag flag = ONCE_FLAG_INIT;
static int calls;

static void init_once(void) { calls++; }

static void *worker(void *arg)
{
    (void)arg;
    call_once(&flag, init_once);
    return 0;
}

int main(void)
{
    pthread_t t[8];
    for (int i = 0; i < 8; i++) if (pthread_create(&t[i], 0, worker, 0)) return 1;
    for (int i = 0; i < 8; i++) pthread_join(t[i], 0);
    if (calls != 1) { printf("calls=%d\n", calls); return 2; }
    puts("once ok");
    return 0;
}
