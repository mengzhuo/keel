/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <stdio.h>
#include <sys/mman.h>
#include <unistd.h>
#include "string.h"

/* GNU extension, not declared by the project's strings.h. */
char *strcasestr(const char *, const char *);

int main(void) {
    char buf[64];

    strcpy(buf, "hello");
    if (strcmp(buf, "hello") != 0) return 1;

    strncpy(buf, "abc", 5);
    if (strncmp(buf, "abc", 5) != 0) return 2;

    strcat(buf, "de");
    if (strcmp(buf, "abcde") != 0) return 3;

    strncat(buf, "fgh", 1);
    if (strcmp(buf, "abcdef") != 0) return 4;

    char abc[16] = "abc";
    if (strlen(abc) != 3) return 5;
    if (strnlen(abc, 10) != 3) return 6;
    if (strnlen(abc, 2) != 2) return 7;

    if (strchr("abc", 'b') != "abc" + 1) return 8;
    if (strrchr("abca", 'a') != "abca" + 3) return 9;

    if (strspn("123abc", "123") != 3) return 10;
    if (strcspn("123abc", "abc") != 3) return 11;
    if (strpbrk("123abc", "ba") != "123abc" + 3) return 12;
    if (strstr("hello world", "world") != "hello world" + 6) return 13;
    if (strstr("hello", "") != "hello") return 14;

    /* Long haystack, match at the end: exercises the two-way path. */
    char hay[512];
    memset(hay, 'x', sizeof hay - 1);
    hay[sizeof hay - 1] = 0;
    memcpy(hay + 500, "needle", 6);
    if (strstr(hay, "needle") != hay + 500) return 21;
    if (strstr(hay, "needlf") != NULL) return 22;
    if (memmem(hay, sizeof hay - 1, "needle", 6) != hay + 500) return 23;
    if (memmem(hay, sizeof hay - 1, "needlf", 6) != NULL) return 24;
    /* Large reject set: exercises the bitmap path. */
    if (strcspn("abcXdef", "0123456789XYZ") != 3) return 25;
    if (strrchr(hay, 0) != hay + sizeof hay - 1) return 26;

    /* strstr grows its end pointer in bounded steps, so it must still stop
       at the NUL when the haystack ends flush against an unmapped page. */
    {
        long ps = sysconf(_SC_PAGESIZE);
        char *area = mmap(0, 2 * ps, PROT_READ | PROT_WRITE,
                          MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
        if (area == MAP_FAILED) return 28;
        if (mprotect(area + ps, ps, PROT_NONE) != 0) return 29;
        char *guard = area + ps;
        char needle[64];
        memset(needle, 'a', 34);
        needle[34] = 0;
        for (size_t hl = 129; hl <= 3000; hl += 41) {
            char *h = guard - hl;
            for (size_t i = 0; i + 1 < hl; i++) h[i] = (i & 1) ? 'a' : 'b';
            h[hl - 1] = 0;
            if (strstr(h, needle) != 0) return 30;
            memcpy(h + hl - 40, needle, 34);
            h[hl - 41] = 'b';
            if (strstr(h, needle) != h + hl - 40) return 31;
        }
        /* memchr/strnlen must not read across the guard page. */
        char *tail = guard - 10;
        memset(tail, 'q', 9);
        tail[9] = 0;
        if (strnlen(tail, 100) != 9) return 32;
        if (strnlen(tail, (size_t)-1) != 9) return 34;
        if (memchr(tail, 0, 100) != tail + 9) return 33;

        /* The rest of the memory primitives must also stop at the mapping
           edge, with the source or destination flush against it. */
        {
            char *edge = guard - 64;
            char *inner = guard - 128;
            memset(edge, 'm', 64);
            if (memcpy(inner, edge, 64) != inner || inner[63] != 'm') return 35;
            if (memcpy(edge, inner, 64) != edge || edge[0] != 'm') return 36;
            if (memmove(edge, edge + 8, 56) != edge) return 37;
            if (memset(edge, 'n', 64) != edge) return 38;
            if (memrchr(edge, 'n', 64) != edge + 63) return 39;
            if (memcmp(edge, inner, 64) == 0) return 40;

            char *es = guard - 16;
            memset(es, 's', 15);
            es[15] = 0;
            if (strlen(es) != 15) return 41;
            if (strchr(es, 'z') != 0 || strchr(es, 0) != es + 15) return 42;
            if (strrchr(es, 's') != es + 14 || strrchr(es, 0) != es + 15) return 43;
            if (strncpy(inner, es, 16) != inner) return 44;
            if (strcmp(es, inner) != 0 || strncmp(es, inner, 15) != 0) return 45;
            if (strcpy(guard - 6, "hello") != guard - 6 || guard[-6] != 'h') return 46;
            if (strncpy(guard - 16, inner, 16) != guard - 16) return 47;

            char *nd = guard - 3;
            memcpy(nd, "ss", 3);
            if (strstr(es, nd) != es) return 48;
            if (memmem(es, 15, nd, 2) != es) return 49;
            if (strcasestr(es, nd) != es) return 50;

            char *cd = guard - 64;
            strcpy(cd, "pre-");
            if (strcat(cd, es) != cd || strlen(cd) != 19) return 51;
            strcpy(cd, "pre-");
            if (strncat(cd, es, 15) != cd || strlen(cd) != 19) return 52;
        }
        munmap(area, 2 * ps);
    }

    char mem[] = {1, 2, 3, 2, 1};
    if (memchr(mem, 2, sizeof(mem)) != mem + 1) return 15;
    if (memrchr(mem, 2, sizeof(mem)) != mem + 3) return 16;

    char tok[] = "a,b,c";
    char *r;
    r = strtok(tok, ",");
    if (r == NULL || strcmp(r, "a") != 0) return 17;
    r = strtok(NULL, ",");
    if (r == NULL || strcmp(r, "b") != 0) return 18;
    r = strtok(NULL, ",");
    if (r == NULL || strcmp(r, "c") != 0) return 19;
    if (strtok(NULL, ",") != NULL) return 20;

    puts("string ok");
    return 0;
}
