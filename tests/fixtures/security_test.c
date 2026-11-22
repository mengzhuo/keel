/* Regression coverage for the 2026-11-22 security audit fixes. */
#include <arpa/inet.h>
#include <fnmatch.h>
#include <glob.h>
#include <mntent.h>
#include <regex.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <wchar.h>

#define CHECK(cond) do { if (!(cond)) { \
    fprintf(stderr, "security check failed at line %d: %s\n", __LINE__, #cond); \
    return 1; } } while (0)

static int ok = 1;

int main(void) {
    /* printf: a format ending in '%' must not read past the NUL. */
    {
        char b[8];
        memset(b, 'X', sizeof b);
        int n = snprintf(b, sizeof b, "%");
        CHECK(n == 1 && b[0] == '%' && b[1] == 0);
    }

    /* glob: an over-long component is rejected, not copied to the stack. */
    {
        size_t n = 5000;
        char *pat = malloc(n + 2);
        CHECK(pat != NULL);
        memset(pat, 'A', n);
        pat[n] = '*';
        pat[n + 1] = 0;
        glob_t g;
        memset(&g, 0, sizeof g);
        int r = glob(pat, 0, NULL, &g);
        CHECK(r == GLOB_NOSPACE);
        free(pat);
    }

    /* mntent: the read-only "defaults" fallback must not be unescaped. */
    {
        const char *line = "proc /proc proc\n";
        FILE *f = fmemopen((void *)line, strlen(line), "r");
        CHECK(f != NULL);
        struct mntent *m = getmntent(f);
        CHECK(m != NULL && m->mnt_opts != NULL && strcmp(m->mnt_opts, "defaults") == 0);
        fclose(f);
    }

    /* fnmatch: FNM_LEADING_DIR boundary handling. */
    CHECK(fnmatch("foo", "foo/bar", FNM_PATHNAME | FNM_LEADING_DIR) == 0);
    CHECK(fnmatch("foo/bar", "foo", FNM_PATHNAME | FNM_LEADING_DIR) == FNM_NOMATCH);
    CHECK(fnmatch("*", "a/b", FNM_PATHNAME | FNM_LEADING_DIR) == 0);
    CHECK(fnmatch("*", "a/b", FNM_PATHNAME) == FNM_NOMATCH);

    /* wcsrtombs: a character that does not fit must not be written. */
    {
        const wchar_t in[] = { 0x00e9, 0 };
        const wchar_t *src = in;
        char out[2] = { 0x7f, 0x7f };
        size_t r = wcsrtombs(out, &src, 1, NULL);
        CHECK(r == 0 && out[0] == 0x7f && out[1] == 0x7f);
    }

    /* asctime_r: canonical 25-character form, no extra month separator. */
    {
        struct tm tm;
        memset(&tm, 0, sizeof tm);
        tm.tm_year = 123; /* 2023 */
        tm.tm_mon = 10;   /* Nov */
        tm.tm_mday = 15;
        tm.tm_hour = 6;
        tm.tm_min = 13;
        tm.tm_sec = 20;
        tm.tm_wday = 3;   /* Wed */
        char b[64];
        CHECK(asctime_r(&tm, b) == b);
        CHECK(strcmp(b, "Wed Nov 15 06:13:20 2023\n") == 0);
    }

    /* inet_aton: overflowing components are rejected, not wrapped. */
    {
        struct in_addr a;
        CHECK(inet_aton("1.2.3.18446744073709551617", &a) == 0);
        CHECK(inet_aton("1.2.3.4", &a) == 1);
    }

    /* realpath(path, NULL) follows musl: it returns NULL. */
    CHECK(realpath("/tmp", NULL) == NULL);

    /* fmemopen: size arithmetic must not wrap. */
    CHECK(fmemopen(NULL, SIZE_MAX, "w") == NULL);

    /* fmemopen: reads larger than BUFSIZ return the correct bytes. */
    {
        static char src[3000], got[3000];
        for (int i = 0; i < 3000; i++) src[i] = (char)('A' + (i % 26));
        src[2999] = 'Z';
        FILE *f = fmemopen(src, sizeof src, "r");
        CHECK(f != NULL);
        size_t n = fread(got, 1, sizeof got, f);
        CHECK(n == sizeof src && memcmp(got, src, sizeof src) == 0);
        fclose(f);
    }

    /* scanf: a 4096-byte line must not write past the line buffer. */
    {
        static char line[4096];
        memset(line, '7', 4095);
        line[4095] = '\n';
        FILE *f = fmemopen(line, sizeof line, "r");
        CHECK(f != NULL);
        int x = 0;
        (void)fscanf(f, "%d", &x);
        fclose(f);
    }

    /* regex: quantified groups with alternation keep their internal jumps. */
    {
        regex_t r;
        regmatch_t m;
        CHECK(regcomp(&r, "^(a|b)*$", REG_EXTENDED) == 0);
        CHECK(regexec(&r, "ab", 1, &m, 0) == 0);
        CHECK(m.rm_so == 0 && m.rm_eo == 2);
        CHECK(regexec(&r, "c", 0, NULL, 0) == REG_NOMATCH);
        regfree(&r);
        CHECK(regcomp(&r, "^(ab|cd)*$", REG_EXTENDED) == 0);
        CHECK(regexec(&r, "abcd", 0, NULL, 0) == 0);
        regfree(&r);
    }

#ifdef __x86_64__
    /* The dynamic loader must install a nonzero stack canary. */
    {
        unsigned long canary;
        __asm__ volatile("movq %%fs:0x28, %0" : "=r"(canary));
        CHECK(canary != 0);
    }
#endif

    if (ok) puts("security ok");
    return 0;
}
