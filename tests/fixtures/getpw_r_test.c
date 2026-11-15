/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <errno.h>
#include <grp.h>
#include <netdb.h>
#include <pwd.h>
#include <shadow.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define CHECK(x) do { if (!(x)) { printf("fail %d: %s\n", __LINE__, #x); return 1; } } while (0)

static int in_buf(const void *p, const char *buf, size_t n)
{
    return p && (uintptr_t)p >= (uintptr_t)buf && (uintptr_t)p < (uintptr_t)buf + n;
}

int main(void)
{
    struct passwd pw, pw2, *pres;
    struct group gr, g2, *gres;
    char pbuf[1024], pbuf2[1024], gbuf[2048];
    int r;

    memset(&pw, 0, sizeof pw);
    memset(pbuf, 0xcc, sizeof pbuf);
    r = getpwnam_r("root", &pw, pbuf, sizeof pbuf, &pres);
    CHECK(r == 0 && pres == &pw);
    CHECK(strcmp(pw.pw_name, "root") == 0);
    CHECK(in_buf(pw.pw_name, pbuf, sizeof pbuf));

    memset(&pw2, 0, sizeof pw2);
    memset(pbuf2, 0xcc, sizeof pbuf2);
    r = getpwuid_r(pw.pw_uid, &pw2, pbuf2, sizeof pbuf2, &pres);
    CHECK(r == 0 && pres == &pw2);
    CHECK(strcmp(pw2.pw_name, "root") == 0);
    CHECK(in_buf(pw2.pw_name, pbuf2, sizeof pbuf2));

    char tiny[1];
    r = getpwnam_r("root", &pw, tiny, sizeof tiny, &pres);
    CHECK(r == ERANGE);
    r = getpwnam_r("root", NULL, pbuf, sizeof pbuf, &pres);
    CHECK(r == EINVAL);
    r = getpwnam_r("root", &pw, NULL, 0, &pres);
    CHECK(r == EINVAL);

    memset(&gr, 0, sizeof gr);
    memset(gbuf, 0xcc, sizeof gbuf);
    r = getgrnam_r("root", &gr, gbuf, sizeof gbuf, &gres);
    CHECK(r == 0 && gres == &gr);
    CHECK(strcmp(gr.gr_name, "root") == 0);
    CHECK(in_buf(gr.gr_name, gbuf, sizeof gbuf));
    CHECK(gr.gr_mem != NULL);
    CHECK(in_buf(gr.gr_mem, gbuf, sizeof gbuf));
    for (int i = 0; gr.gr_mem[i]; i++) {
        CHECK(in_buf(gr.gr_mem[i], gbuf, sizeof gbuf));
        CHECK(strlen(gr.gr_mem[i]) < (size_t)(gbuf + sizeof gbuf - gr.gr_mem[i]));
    }

    /* If the host has a group with members, verify the member array. */
    setgrent();
    char member_group[256] = "";
    struct group *g0;
    while ((g0 = getgrent()) != NULL) {
        if (g0->gr_mem && g0->gr_mem[0]) {
            snprintf(member_group, sizeof member_group, "%s", g0->gr_name);
            break;
        }
    }
    endgrent();
    if (member_group[0]) {
        memset(&g2, 0, sizeof g2);
        memset(gbuf, 0xcc, sizeof gbuf);
        r = getgrnam_r(member_group, &g2, gbuf, sizeof gbuf, &gres);
        CHECK(r == 0 && gres == &g2);
        CHECK(strcmp(g2.gr_name, member_group) == 0);
        CHECK(in_buf(g2.gr_mem, gbuf, sizeof gbuf));
        for (int i = 0; g2.gr_mem[i]; i++) {
            CHECK(in_buf(g2.gr_mem[i], gbuf, sizeof gbuf));
            CHECK(strlen(g2.gr_mem[i]) < (size_t)(gbuf + sizeof gbuf - g2.gr_mem[i]));
        }
    }

    char tinyg[1];
    r = getgrnam_r("root", &gr, tinyg, sizeof tinyg, &gres);
    CHECK(r == ERANGE);

    int neg = -1;
    unsigned int groups1[1];
    CHECK(getgrouplist("root", 0, groups1, &neg) == -1);

    /* getserv*_r must return the caller's struct, not a dead stack copy. */
    {
        struct servent se, *sres;
        char sbuf[512];
        memset(&se, 0, sizeof se);
        memset(sbuf, 0xcc, sizeof sbuf);
        r = getservbyname_r("http", "tcp", &se, sbuf, sizeof sbuf, &sres);
        if (r == 0) {
            CHECK(sres == &se);
            CHECK(in_buf(se.s_name, sbuf, sizeof sbuf));
            CHECK(strcmp(se.s_name, "http") == 0);
        } else {
            CHECK(r == 2); /* NDB_ENOENT: service database may be minimal */
        }
    }

    /* Shadow may be unreadable in unprivileged CI; only check when the
       non-reentrant lookup succeeds. */
    struct spwd sp, *spres;
    char sbuf[1024];
    if (getspnam("root") != NULL) {
        memset(&sp, 0, sizeof sp);
        memset(sbuf, 0xcc, sizeof sbuf);
        r = getspnam_r("root", &sp, sbuf, sizeof sbuf, &spres);
        CHECK(r == 0 && spres == &sp);
        CHECK(strcmp(sp.sp_namp, "root") == 0);
        CHECK(in_buf(sp.sp_namp, sbuf, sizeof sbuf));

        char stiny[1];
        r = getspnam_r("root", &sp, stiny, sizeof stiny, &spres);
        CHECK(r == ERANGE);
    }

    printf("getpw_r ok\n");
    return 0;
}
