/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <wordexp.h>

#define CHECK(x) do { if (!(x)) { printf("fail %d: %s\n", __LINE__, #x); return 1; } } while (0)

static int long_word(int n)
{
    char *s = malloc((size_t)n + 1);
    if (!s) return 1;
    memset(s, 'a', (size_t)n);
    s[n] = 0;

    wordexp_t p;
    memset(&p, 0, sizeof p);
    if (wordexp(s, &p, 0) != 0) { free(s); return 1; }
    if (p.we_wordc != 1 || !p.we_wordv[0] || strlen(p.we_wordv[0]) != (size_t)n) {
        wordfree(&p);
        free(s);
        return 1;
    }
    wordfree(&p);
    free(s);
    return 0;
}

int main(void)
{
    wordexp_t p;

    memset(&p, 0, sizeof p);
    CHECK(wordexp("a b", &p, 0) == 0);
    CHECK(p.we_wordc == 2);
    CHECK(strcmp(p.we_wordv[0], "a") == 0);
    CHECK(strcmp(p.we_wordv[1], "b") == 0);
    CHECK(p.we_wordv[2] == NULL);
    wordfree(&p);

    memset(&p, 0, sizeof p);
    CHECK(wordexp("'a b'", &p, 0) == 0);
    CHECK(p.we_wordc == 1);
    CHECK(strcmp(p.we_wordv[0], "a b") == 0);
    wordfree(&p);

    CHECK(setenv("KEEL_WORDEXP_VAR", "hello", 1) == 0);
    memset(&p, 0, sizeof p);
    CHECK(wordexp("$KEEL_WORDEXP_VAR", &p, 0) == 0);
    CHECK(p.we_wordc == 1);
    CHECK(strcmp(p.we_wordv[0], "hello") == 0);
    wordfree(&p);

    memset(&p, 0, sizeof p);
    p.we_offs = 2;
    CHECK(wordexp("a", &p, WRDE_DOOFFS) == 0);
    CHECK(p.we_wordc == 1);
    CHECK(p.we_wordv[0] == NULL);
    CHECK(p.we_wordv[1] == NULL);
    CHECK(strcmp(p.we_wordv[2], "a") == 0);
    wordfree(&p);

    memset(&p, 0, sizeof p);
    CHECK(wordexp("$(echo hi)", &p, WRDE_NOCMD) == WRDE_CMDSUB);

    /* Exact-capacity words exercise the terminator reservation: 4096 is
       the inline buffer size, 8192 is a power-of-two heap size. */
    CHECK(long_word(4095) == 0);
    CHECK(long_word(4096) == 0);
    CHECK(long_word(4097) == 0);
    CHECK(long_word(8191) == 0);
    CHECK(long_word(8192) == 0);
    CHECK(long_word(8193) == 0);

    printf("wordexp ok\n");
    return 0;
}
