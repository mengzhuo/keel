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

    /* tm_mday-1 must be computed in a wide type. */
    memset(&tm, 0, sizeof tm);
    tm.tm_year = 120; /* 2020-01-01 */
    tm.tm_mday = INT_MIN;
    if (timegm(&tm) != -185541009436800LL) { puts("timegm tm_mday fail"); return 5; }

    puts("time regressions ok");
    return 0;
}
