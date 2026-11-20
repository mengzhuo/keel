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

static int captures(const char *pat, const char *s, size_t nmatch, regmatch_t *m)
{
    regex_t re;
    if (regcomp(&re, pat, REG_EXTENDED) != 0) return -1;
    int r = regexec(&re, s, nmatch, m, 0);
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

    /* Subexpression captures (Bash's BASH_REMATCH uses these). */
    {
        regex_t re;
        regmatch_t m[4];

        CHECK(regcomp(&re, "^(a+)(b+)$", REG_EXTENDED) == 0);
        CHECK(re.re_nsub == 2);
        CHECK(regexec(&re, "aaabbb", 3, m, 0) == 0);
        CHECK(m[0].rm_so == 0 && m[0].rm_eo == 6);
        CHECK(m[1].rm_so == 0 && m[1].rm_eo == 3);
        CHECK(m[2].rm_so == 3 && m[2].rm_eo == 6);
        regfree(&re);

        CHECK(captures("(x)?(foo)", "foo", 3, m) == 0);
        CHECK(m[1].rm_so == -1 && m[1].rm_eo == -1);
        CHECK(m[2].rm_so == 0 && m[2].rm_eo == 3);

        /* A repeated group reports its last iteration. */
        CHECK(captures("(ab)+", "abab", 2, m) == 0);
        CHECK(m[1].rm_so == 2 && m[1].rm_eo == 4);

        CHECK(captures("(a|b)c", "bc", 2, m) == 0);
        CHECK(m[1].rm_so == 0 && m[1].rm_eo == 1);
    }

    printf("regex ok\n");
    return 0;
}
