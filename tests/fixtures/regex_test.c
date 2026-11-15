/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <regex.h>
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { printf("fail %d: %s\n", __LINE__, #x); return 1; } } while (0)

static int matches(const char *pat, const char *s)
{
    regex_t re;
    if (regcomp(&re, pat, REG_EXTENDED) != 0) return -1;
    int r = regexec(&re, s, 0, NULL, 0);
    regfree(&re);
    return r;
}

int main(void)
{
    CHECK(matches("a|b", "a") == 0);
    CHECK(matches("a|b", "b") == 0);
    CHECK(matches("a|b", "c") == REG_NOMATCH);
    CHECK(matches("(a|b)", "b") == 0);
    CHECK(matches("(a|b)c", "ac") == 0);
    CHECK(matches("(a|b)c", "bc") == 0);
    CHECK(matches("(a|b)c", "cc") == REG_NOMATCH);
    CHECK(matches("(ab)+", "abab") == 0);
    CHECK(matches("(ab)*", "abab") == 0);
    CHECK(matches("(ab)*", "aba") == 0); /* empty match is valid */
    CHECK(matches("a|", "") == 0);
    CHECK(matches("a+", "aaa") == 0);
    CHECK(matches("a+", "") == REG_NOMATCH);
    CHECK(matches("[a-c]+", "abc") == 0);
    CHECK(matches("^a$", "a") == 0);
    CHECK(matches("^a$", "aa") == REG_NOMATCH);

    printf("regex ok\n");
    return 0;
}
