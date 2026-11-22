#define _GNU_SOURCE
/* Deterministic parser fuzz smoke test for the security-sensitive entry
 * points.  It is not coverage-guided; it feeds a fixed pseudo-random corpus
 * through the parsers so a regression shows up as a crash/hang in CI. */
#include <arpa/inet.h>
#include <dlfcn.h>
#include <fnmatch.h>
#include <glob.h>
#include <grp.h>
#include <iconv.h>
#include <mntent.h>
#include <pwd.h>
#include <regex.h>
#include <shadow.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

extern int dn_expand(const unsigned char *, const unsigned char *,
                     const unsigned char *, char *, int);

static uint64_t rng_state = 0x9e3779b97f4a7c15ULL;
static uint32_t rnd(void) {
    rng_state ^= rng_state << 13;
    rng_state ^= rng_state >> 7;
    rng_state ^= rng_state << 17;
    return (uint32_t)(rng_state >> 16);
}
static void rand_str(char *dst, size_t cap) {
    static const char alphabet[] = "abcABC019.-_:/[]()|*+?^$ \t";
    size_t n = rnd() % (cap - 1);
    for (size_t i = 0; i < n; i++) {
        dst[i] = alphabet[rnd() % (sizeof alphabet - 1)];
    }
    dst[n] = 0;
}

int main(void) {
    char s[64], p[32], out[256], fmt[128];
    struct in_addr a4;
    struct in6_addr a6;

    for (int i = 0; i < 2000; i++) {
        rand_str(s, sizeof s);
        (void)inet_aton(s, &a4);
        (void)inet_pton(AF_INET, s, &a4);
        (void)inet_pton(AF_INET6, s, &a6);
        char *end = 0;
        (void)strtod(s, &end);
        (void)strtol(s, &end, 10);
        (void)strtoul(s, &end, 16);
        unsigned char raw[16];
        for (int k = 0; k < 16; k++) raw[k] = (unsigned char)rnd();
        (void)inet_ntop(AF_INET, raw, out, sizeof out);
        (void)inet_ntop(AF_INET6, raw, out, sizeof out);
    }

    for (int i = 0; i < 2000; i++) {
        rand_str(p, sizeof p);
        rand_str(s, sizeof s);
        int flags[] = {0, FNM_PATHNAME, FNM_NOESCAPE, FNM_PERIOD,
                       FNM_PATHNAME | FNM_LEADING_DIR};
        (void)fnmatch(p, s, flags[i % 5]);
    }

    for (int i = 0; i < 300; i++) {
        static const char atoms[] = "ab()|*+?[]^$.";
        size_t n = rnd() % 10;
        for (size_t k = 0; k < n; k++) p[k] = atoms[rnd() % (sizeof atoms - 1)];
        p[n] = 0;
        regex_t r;
        if (regcomp(&r, p, REG_EXTENDED) == 0) {
            regmatch_t m[2];
            rand_str(s, sizeof s);
            (void)regexec(&r, s, 2, m, 0);
            regfree(&r);
        }
    }

    /* DNS name decompression: random packets, bounded output. */
    for (int i = 0; i < 2000; i++) {
        unsigned char msg[64];
        size_t n = 1 + rnd() % (sizeof msg - 1);
        for (size_t k = 0; k < n; k++) msg[k] = (unsigned char)rnd();
        (void)dn_expand(msg, msg + n, msg, out, 255);
    }

    /* mntent: random lines through fmemopen. */
    for (int i = 0; i < 500; i++) {
        char line[128];
        rand_str(line, sizeof line);
        size_t l = strlen(line);
        if (l + 1 < sizeof line) { line[l] = '\n'; line[l + 1] = 0; }
        FILE *f = fmemopen(line, strlen(line), "r");
        if (f) {
            while (getmntent(f)) {}
            fclose(f);
        }
    }

    /* iconv: random bytes, both directions, with a state reset. */
    iconv_t cd = iconv_open("UTF-8", "ISO-8859-1");
    if (cd != (iconv_t)-1) {
        char in[64], conv[256];
        for (int i = 0; i < 500; i++) {
            size_t n = rnd() % sizeof in;
            for (size_t k = 0; k < n; k++) in[k] = (char)rnd();
            char *ip = in, *op = conv;
            size_t il = n, ol = sizeof conv;
            (void)iconv(cd, &ip, &il, &op, &ol);
            (void)iconv(cd, NULL, NULL, NULL, NULL);
        }
        iconv_close(cd);
    }

    /* strxfrm/strptime: length bookkeeping must stay bounded. */
    for (int i = 0; i < 500; i++) {
        rand_str(s, sizeof s);
        size_t n = rnd() % sizeof s;
        (void)strxfrm(out, s, n);
        struct tm tm;
        memset(&tm, 0, sizeof tm);
        (void)strptime(s, "%Y-%m-%d %H:%M:%S", &tm);
    }

    /* glob: random patterns, bounded to the current directory. */
    for (int i = 0; i < 100; i++) {
        rand_str(p, sizeof p);
        glob_t g;
        memset(&g, 0, sizeof g);
        int r = glob(p, GLOB_NOSORT, NULL, &g);
        if (r == 0) globfree(&g);
    }

    /* getopt: random argv/optstring; optind is reset each round. */
    for (int i = 0; i < 200; i++) {
        char *av[8];
        for (int k = 0; k < 7; k++) {
            static char slots[7][32];
            rand_str(slots[k], sizeof slots[k]);
            av[k] = slots[k];
        }
        av[7] = NULL;
        char opt[] = "ab:c::";
        optind = 1;
        opterr = 0;
        int c;
        while ((c = getopt(7, av, opt)) != -1) { (void)c; }
    }

    /* passwd/group/shadow line parsers via their f* entry points. */
    for (int i = 0; i < 300; i++) {
        char line[128];
        rand_str(line, sizeof line);
        size_t l = strlen(line);
        if (l + 1 < sizeof line) { line[l] = '\n'; line[l + 1] = 0; }
        FILE *f = fmemopen(line, strlen(line), "r");
        if (f) {
            while (fgetpwent(f)) {}
            while (fgetgrent(f)) {}
            while (fgetspent(f)) {}
            fclose(f);
        }
    }

    /* TZ: random POSIX/TZif names; restore UTC afterwards. */
    for (int i = 0; i < 100; i++) {
        rand_str(s, sizeof s);
        setenv("TZ", s, 1);
        tzset();
        time_t t = 1700000000;
        struct tm tm;
        (void)localtime_r(&t, &tm);
    }
    setenv("TZ", "UTC0", 1);
    tzset();

    /* printf: safe integer conversions with a random format skeleton. */
    for (int i = 0; i < 300; i++) {
        static const char *pieces[] = {
            "%d", "%u", "%x", "%c", "%%", "lit", "%5d", "%.3ld", "%08x"
        };
        fmt[0] = 0;
        size_t fl = 0;
        int n = 1 + rnd() % 6;
        for (int k = 0; k < n; k++) {
            const char *piece = pieces[rnd() % (sizeof pieces / sizeof *pieces)];
            size_t pl = strlen(piece);
            if (fl + pl + 1 >= sizeof fmt) break;
            memcpy(fmt + fl, piece, pl + 1);
            fl += pl;
        }
        (void)snprintf(out, sizeof out, fmt,
                       (int)rnd(), (unsigned)rnd(), (unsigned)rnd(),
                       (int)('a' + rnd() % 26), (long)rnd(),
                       (unsigned)rnd(), (long)rnd(), (unsigned)rnd());
    }

    puts("fuzz ok");
    return 0;
}
