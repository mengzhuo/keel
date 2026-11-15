/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <arpa/inet.h>
#include <netdb.h>
#include <netinet/in.h>
#include <pthread.h>
#include <stdio.h>
#include <string.h>

#define THREADS 4
#define LOOPS 200

static int *thread_h_errno[THREADS];

static void *worker(void *arg)
{
    int idx = (int)(long)arg;
    thread_h_errno[idx] = __h_errno_location();
    for (int i = 0; i < LOOPS; i++) {
        struct addrinfo *res = NULL;
        int r = getaddrinfo("localhost", "80", NULL, &res);
        if (r != 0 || res == NULL) return (void *)1;
        freeaddrinfo(res);
    }
    return NULL;
}

int main(void)
{
    pthread_t th[THREADS];
    int *main_h = __h_errno_location();
    h_errno = 77;

    for (int i = 0; i < THREADS; i++) {
        if (pthread_create(&th[i], NULL, worker, (void *)(long)i) != 0) return 1;
    }
    for (int i = 0; i < THREADS; i++) {
        void *ret = NULL;
        if (pthread_join(th[i], &ret) != 0 || ret != NULL) return 2;
    }

    if (h_errno != 77) return 3;
    for (int i = 0; i < THREADS; i++) {
        if (thread_h_errno[i] == NULL) return 4;
        if (thread_h_errno[i] == main_h) return 5;
        for (int j = 0; j < i; j++) {
            if (thread_h_errno[i] == thread_h_errno[j]) return 6;
        }
    }

    /* Concurrent getnameinfo calls must not corrupt shared buffers. */
    struct sockaddr_in sa;
    memset(&sa, 0, sizeof sa);
    sa.sin_family = AF_INET;
    sa.sin_port = htons(80);
    sa.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    char host[64];
    if (getnameinfo((struct sockaddr *)&sa, sizeof sa, host, sizeof host,
                    NULL, 0, NI_NUMERICHOST) != 0) return 7;
    if (strcmp(host, "127.0.0.1") != 0) return 8;

    puts("netdb threads ok");
    return 0;
}
