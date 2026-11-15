/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <errno.h>
#include <stdio.h>
#include <string.h>
#include "arpa/nameser.h"

#define CHECK(x) do { if (!(x)) { printf("fail %d: %s\n", __LINE__, #x); return 1; } } while (0)

int main(void)
{
    /* id=0x1234, flags=0x8180, qd=1, an=1; question example.com A IN;
       answer: pointer to offset 12, A IN ttl=300, rdata=192.0.2.1. */
    unsigned char msg[] = {
        0x12, 0x34, 0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
        7, 'e', 'x', 'a', 'm', 'p', 'l', 'e', 3, 'c', 'o', 'm', 0,
        0x00, 0x01, 0x00, 0x01,
        0xc0, 0x0c,
        0x00, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x01, 0x2c,
        0x00, 0x04,
        192, 0, 2, 1
    };
    ns_msg handle;
    ns_rr rr;
    char name[NS_MAXDNAME];
    int r;

    CHECK(ns_initparse(msg, (int)sizeof msg, &handle) == 0);
    CHECK(ns_msg_id(handle) == 0x1234);
    CHECK(ns_msg_count(handle, ns_s_qd) == 1);
    CHECK(ns_msg_count(handle, ns_s_an) == 1);
    CHECK(ns_msg_count(handle, ns_s_ns) == 0);
    CHECK(ns_msg_count(handle, ns_s_ar) == 0);
    CHECK(ns_msg_getflag(handle, ns_f_qr) == 1);
    CHECK(ns_msg_getflag(handle, ns_f_rd) == 1);
    CHECK(ns_msg_getflag(handle, ns_f_ra) == 1);

    CHECK(ns_skiprr(msg + 12, msg + sizeof msg, ns_s_qd, 1) == 17);
    CHECK(ns_skiprr(msg + 29, msg + sizeof msg, ns_s_an, 1) == 16);

    r = ns_name_uncompress(msg, msg + sizeof msg, msg + 12, name, sizeof name);
    CHECK(r == 13);
    CHECK(strcmp(name, "example.com") == 0);

    memset(&rr, 0, sizeof rr);
    CHECK(ns_parserr(&handle, ns_s_qd, 0, &rr) == 0);
    CHECK(strcmp(ns_rr_name(rr), "example.com") == 0);
    CHECK(ns_rr_type(rr) == ns_t_a);
    CHECK(ns_rr_class(rr) == ns_c_in);
    CHECK(ns_rr_ttl(rr) == 0);
    CHECK(ns_rr_rdlen(rr) == 0);
    CHECK(rr.rdata == NULL);

    memset(&rr, 0, sizeof rr);
    CHECK(ns_parserr(&handle, ns_s_an, 0, &rr) == 0);
    CHECK(strcmp(ns_rr_name(rr), "example.com") == 0);
    CHECK(ns_rr_type(rr) == ns_t_a);
    CHECK(ns_rr_class(rr) == ns_c_in);
    CHECK(ns_rr_ttl(rr) == 300);
    CHECK(ns_rr_rdlen(rr) == 4);
    CHECK(rr.rdata != NULL);
    CHECK(memcmp(rr.rdata, "\xc0\x00\x02\x01", 4) == 0);

    errno = 0;
    CHECK(ns_parserr(&handle, ns_s_max, 0, &rr) == -1);
    CHECK(errno == ENODEV);
    errno = 0;
    CHECK(ns_parserr(&handle, ns_s_qd, 1, &rr) == -1);
    CHECK(errno == ENODEV);

    /* rrnum == -1 iterates the current section and advances _rrnum. */
    CHECK(ns_initparse(msg, (int)sizeof msg, &handle) == 0);
    memset(&rr, 0, sizeof rr);
    CHECK(ns_parserr(&handle, ns_s_qd, -1, &rr) == 0);
    CHECK(strcmp(ns_rr_name(rr), "example.com") == 0);
    errno = 0;
    CHECK(ns_parserr(&handle, ns_s_qd, -1, &rr) == -1);
    CHECK(errno == ENODEV);
    memset(&rr, 0, sizeof rr);
    CHECK(ns_parserr(&handle, ns_s_an, -1, &rr) == 0);
    CHECK(ns_rr_ttl(rr) == 300);
    errno = 0;
    CHECK(ns_parserr(&handle, ns_s_an, -1, &rr) == -1);
    CHECK(errno == ENODEV);

    errno = 0;
    CHECK(ns_initparse(msg, 11, &handle) == -1);
    CHECK(errno == EMSGSIZE);

    unsigned char empty_hdr[12] = {0, 0, 0x81, 0x80, 0, 1, 0, 0, 0, 0, 0, 0};
    errno = 0;
    CHECK(ns_initparse(empty_hdr, (int)sizeof empty_hdr, &handle) == -1);
    CHECK(errno == EMSGSIZE);

    unsigned char trailing[sizeof msg + 1];
    memcpy(trailing, msg, sizeof msg);
    trailing[sizeof msg] = 0;
    errno = 0;
    CHECK(ns_initparse(trailing, (int)sizeof trailing, &handle) == -1);
    CHECK(errno == EMSGSIZE);

    errno = 0;
    CHECK(ns_skiprr(msg + 12, msg + 12, ns_s_qd, 1) == -1);
    CHECK(errno == EMSGSIZE);

    /* Deterministic malformed-input loop: every successful parse must keep
       ns_parserr inside the message. */
    {
        unsigned seed = 0x12345678u;
        unsigned char fuzz[80];
        for (int iter = 0; iter < 2000; iter++) {
            seed = seed * 1103515245u + 12345u;
            int flen = (int)(seed % sizeof fuzz);
            for (int i = 0; i < flen; i++) {
                seed = seed * 1103515245u + 12345u;
                fuzz[i] = (unsigned char)(seed >> 16);
            }
            ns_msg fh;
            memset(&fh, 0, sizeof fh);
            if (ns_initparse(fuzz, flen, &fh) != 0) continue;
            for (int sect = 0; sect < ns_s_max; sect++) {
                int count = fh._counts[sect];
                if (count > 100) count = 100;
                for (int k = 0; k < count; k++) {
                    ns_rr frr;
                    memset(&frr, 0, sizeof frr);
                    if (ns_parserr(&fh, (ns_sect)sect, k, &frr) != 0) break;
                    if (frr.rdlength && frr.rdata) (void)frr.rdata[0];
                    (void)frr.name[0];
                }
            }
        }
    }

    printf("ns_parse ok\n");
    return 0;
}
