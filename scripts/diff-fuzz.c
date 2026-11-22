#define _GNU_SOURCE
#include <arpa/inet.h>
#include <fnmatch.h>
#include <regex.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static uint64_t rng_state = 0x243f6a8885a308d3ULL;
static uint32_t rnd(void) {
    rng_state ^= rng_state << 13;
    rng_state ^= rng_state >> 7;
    rng_state ^= rng_state << 17;
    return (uint32_t)(rng_state >> 16);
}
static void rand_str(char *dst, size_t cap) {
    static const char alphabet[] = "abcABC019.-_:/[]()|*+?^$ \\t";
    size_t n = rnd() % (cap - 1);
    for (size_t i = 0; i < n; i++) dst[i] = alphabet[rnd() % (sizeof alphabet - 1)];
    dst[n] = 0;
}

static const char *patterns[] = {
    "^[a-z]+([0-9]+)$", "^(a|b)*$", "^(ab|cd)*$", "a+b?",
    "[^a]*", "^[[:alpha:]]+$", "(a|ab)*c", "a|b|c",
};
#define NPAT (sizeof patterns / sizeof *patterns)

int main(int argc, char **argv) {
    int iters = argc > 1 ? atoi(argv[1]) : 2000;
    regex_t res[NPAT];
    int compiled[NPAT];
    for (size_t i = 0; i < NPAT; i++) {
        compiled[i] = regcomp(&res[i], patterns[i], REG_EXTENDED) == 0;
    }
    char s[64], p[32], out[256];
    for (int n = 0; n < iters; n++) {
        rand_str(s, sizeof s);
        char *end = 0;

        double d = strtod(s, &end);
        uint64_t bits;
        memcpy(&bits, &d, sizeof bits);
        printf("D %016llx %ld\n", (unsigned long long)bits, (long)(end - s));

        long l = strtol(s, &end, 10);
        printf("L %ld %ld\n", l, (long)(end - s));
        unsigned long u = strtoul(s, &end, 16);
        printf("U %lu %ld\n", u, (long)(end - s));

        struct in_addr a4;
        int r = inet_aton(s, &a4);
        printf("A %d %08x\n", r, r ? ntohl(a4.s_addr) : 0);
        unsigned char b[16];
        printf("P %d %d\n", inet_pton(AF_INET, s, b), inet_pton(AF_INET6, s, b));

        printf("F %d %d %d [%s]\n",
               fnmatch(s, s, 0),
               fnmatch(s, s, FNM_PATHNAME | FNM_LEADING_DIR),
               fnmatch("*", s, FNM_PATHNAME), s);

        memset(out, 0, sizeof out);
        printf("X %zu\n", strxfrm(out, s, sizeof out));
        struct tm tm;
        memset(&tm, 0, sizeof tm);
        char *tp = strptime(s, "%Y-%m-%d %H:%M:%S", &tm);
        if (tp) printf("T %ld %d %d %d %d %d %d\n", (long)(tp - s),
                       tm.tm_year, tm.tm_mon, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec);
        else printf("T none\n");

        printf("S %d|%u|%x|%s|%c|%o|%ld\n",
               (int)rnd(), (unsigned)rnd(), (unsigned)rnd(), s,
               (int)('a' + rnd() % 26), (unsigned)rnd() & 0777,
               (long)(int32_t)rnd());

        rand_str(p, sizeof p);
        for (size_t i = 0; i < NPAT; i++) {
            if (!compiled[i]) { printf("R %zu -1\n", i); continue; }
            regmatch_t m[2];
            int rc = regexec(&res[i], p, 2, m, 0);
            printf("R %zu %d %d %d [%s] [%s]\n", i, rc,
                   rc ? -1 : (int)m[0].rm_so, rc ? -1 : (int)m[0].rm_eo, patterns[i], p);
        }
    }
    for (size_t i = 0; i < NPAT; i++) if (compiled[i]) regfree(&res[i]);
    return 0;
}
