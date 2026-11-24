/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <fcntl.h>
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

static void put32(unsigned char *p, unsigned v) {
    p[0] = (unsigned char)(v >> 24);
    p[1] = (unsigned char)(v >> 16);
    p[2] = (unsigned char)(v >> 8);
    p[3] = (unsigned char)v;
}

/* A minimal valid TZif v1 file: EST (UTC-5), no DST. */
static int write_tzif_v1(const char *path) {
    unsigned char buf[59];
    memset(buf, 0, sizeof buf);
    memcpy(buf, "TZif", 4);
    buf[4] = 0; /* version 1 */
    put32(buf + 20, 0); /* isutcnt */
    put32(buf + 24, 0); /* isstdcnt */
    put32(buf + 28, 0); /* leapcnt */
    put32(buf + 32, 1); /* timecnt */
    put32(buf + 36, 1); /* typecnt */
    put32(buf + 40, 4); /* charcnt */
    put32(buf + 44, 0); /* transition at epoch 0 */
    buf[48] = 0;        /* type index 0 */
    put32(buf + 49, (unsigned)-18000); /* utoff */
    buf[53] = 0;        /* isdst */
    buf[54] = 0;        /* abbrev index */
    memcpy(buf + 55, "EST", 4);

    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (fd < 0) return -1;
    ssize_t n = write(fd, buf, sizeof buf);
    close(fd);
    return n == (ssize_t)sizeof buf ? 0 : -1;
}


/* A malformed TZif with huge counts; the parser must fall back to UTC. */
static int write_tzif_bad(const char *path) {
    unsigned char buf[44];
    memset(buf, 0, sizeof buf);
    memcpy(buf, "TZif", 4);
    buf[4] = 0;
    put32(buf + 32, 0xffffffff); /* timecnt */
    put32(buf + 36, 0xffffffff); /* typecnt */
    put32(buf + 40, 0);          /* charcnt */

    int fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0600);
    if (fd < 0) return -1;
    ssize_t n = write(fd, buf, sizeof buf);
    close(fd);
    return n == (ssize_t)sizeof buf ? 0 : -1;
}

int main(void) {
    struct tm tm;

    /* %Z matches tzname[] and records tm_isdst. */
    setenv("TZ", "UTC", 1);
    tzset();
    memset(&tm, 0, sizeof tm);
    tm.tm_isdst = -1;
    char *p = strptime("UTC", "%Z", &tm);
    if (!p || *p != 0 || tm.tm_isdst != 0) { puts("strptime %Z fail"); return 1; }

    /* TZif v1 must be parsed as v1, not as a v2 file. */
    char path[] = "/tmp/keel-tzif-v1-XXXXXX";
    int fd = mkstemp(path);
    if (fd < 0) return 2;
    close(fd);
    if (write_tzif_v1(path) != 0) { unlink(path); return 3; }
    setenv("TZ", path, 1);
    tzset();
    time_t t = 0;
    struct tm *lt = localtime(&t);
    if (!lt || lt->tm_hour != 19 || !lt->tm_zone || strcmp(lt->tm_zone, "EST") != 0) {
        printf("tzif v1 fail: hour=%d zone=%s\n",
               lt ? lt->tm_hour : -1,
               lt && lt->tm_zone ? lt->tm_zone : "(none)");
        unlink(path);
        return 4;
    }
    unlink(path);

    /* Malformed TZif: bounded parsing, UTC fallback, no crash. */
    char bad[] = "/tmp/keel-tzif-bad-XXXXXX";
    fd = mkstemp(bad);
    if (fd < 0) return 6;
    close(fd);
    if (write_tzif_bad(bad) != 0) { unlink(bad); return 7; }
    setenv("TZ", bad, 1);
    tzset();
    lt = localtime(&t);
    if (!lt || !tzname[0] || strcmp(tzname[0], "UTC") != 0) {
        printf("bad tzif fail: zone=%s\n", tzname[0] ? tzname[0] : "(none)");
        unlink(bad);
        return 8;
    }
    unlink(bad);

    /* Invalid POSIX rule fields must be clamped, not used as indexes. */
    setenv("TZ", "EST5EDT,M99.1.1/0,M10.1.1/0", 1);
    tzset();
    lt = localtime(&t);
    if (!lt) { puts("invalid TZ rule fail"); return 9; }

    /* POSIX M-rules: the third field is the weekday (0 = Sunday).  US
       Eastern DST ends at 06:00 UTC and starts at 07:00 UTC in 2012. */
    setenv("TZ", "EST+05EDT,M3.2.0,M11.1.0", 1);
    tzset();
    t = 1352010600; /* 2012-11-04 06:30 UTC -> 01:30 EST */
    lt = localtime(&t);
    if (!lt || lt->tm_hour != 1 || lt->tm_isdst != 0) {
        puts("TZ DST end fail");
        return 11;
    }
    t = 1331451000; /* 2012-03-11 07:30 UTC -> 03:30 EDT */
    lt = localtime(&t);
    if (!lt || lt->tm_hour != 3 || lt->tm_isdst != 1) {
        puts("TZ DST start fail");
        return 12;
    }


    /* A long tm_zone must not overflow strftime's internal buffer. */
    {
        char zone[256];
        char out[128];
        memset(zone, 'A', sizeof zone - 1);
        zone[sizeof zone - 1] = 0;
        memset(&tm, 0, sizeof tm);
        tm.tm_zone = zone;
        tm.tm_isdst = 0;
        size_t zl = strftime(out, sizeof out, "%Z", &tm);
        if (zl >= sizeof out) { puts("strftime zone fail"); return 10; }
    }

    /* tm_mday-1 must be computed in a wide type. */
    memset(&tm, 0, sizeof tm);
    tm.tm_year = 120; /* 2020-01-01 */
    tm.tm_mday = INT_MIN;
    if (timegm(&tm) != -185541009436800LL) { puts("timegm tm_mday fail"); return 5; }

    puts("time regressions ok");
    return 0;
}
