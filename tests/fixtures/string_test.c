/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <stdio.h>
#include <sys/mman.h>
#include <unistd.h>
#include "string.h"

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
