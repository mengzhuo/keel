/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <string.h>
#include "arpa/nameser.h"

/* Not present in keel's public headers yet, but exported by libc.so. */
int dn_skipname(const unsigned char *, const unsigned char *);

struct guarded_name {
    char name[3];
    unsigned char guard[16];
};

static int guard_intact(const struct guarded_name *b)
{
    size_t i;
    for (i = 0; i < sizeof b->guard; i++)
        if (b->guard[i] != 0xcc) return 0;
    return 1;
}

int main(void) {
    unsigned char pkt_simple[] = {
        3, 'w', 'w', 'w',
        7, 'e', 'x', 'a', 'm', 'p', 'l', 'e',
        3, 'c', 'o', 'm',
        0
    };
    char name[256];
    int r;

    r = dn_expand(pkt_simple, pkt_simple + sizeof(pkt_simple),
                  pkt_simple, name, sizeof(name));
    if (r != 17) return 1;
    if (strcmp(name, "www.example.com") != 0) return 2;

    unsigned char pkt_compress[] = {
        3, 'w', 'w', 'w',
        7, 'e', 'x', 'a', 'm', 'p', 'l', 'e',
        3, 'c', 'o', 'm',
        0,
        4, 'm', 'a', 'i', 'l',
        0xc0, 4
    };
    r = dn_expand(pkt_compress, pkt_compress + sizeof(pkt_compress),
                  pkt_compress + 17, name, sizeof(name));
    if (r != 7) return 3;
    if (strcmp(name, "mail.example.com") != 0) return 4;

    unsigned char pkt_empty[] = { 0 };
    r = dn_expand(pkt_empty, pkt_empty + 1, pkt_empty, name, 256);
    if (r != 1) return 5;
    if (name[0] != 0) return 6;

    /* A name that expands to exactly `space` bytes does not fit: space
       includes the terminating NUL.  It must be rejected without writing
       past the caller's buffer. */
    unsigned char pkt_fit[] = { 1, 'a', 1, 'b', 0 };
    struct guarded_name box;
    memset(&box, 0xcc, sizeof box);
    r = dn_expand(pkt_fit, pkt_fit + sizeof(pkt_fit), pkt_fit, box.name, 3);
    if (r != -1) return 7;
    if (!guard_intact(&box)) return 8;

    /* One byte more is enough for "a.b\0". */
    memset(&box, 0xcc, sizeof box);
    r = dn_expand(pkt_fit, pkt_fit + sizeof(pkt_fit), pkt_fit, box.name, 4);
    if (r != 5) return 9;
    if (strcmp(box.name, "a.b") != 0) return 10;

    /* Once a label exactly fills the buffer, a following label must not
       make the remaining-space subtraction wrap around. */
    unsigned char pkt_fit2[] = { 3, 'a', 'b', 'c', 1, 'd', 1, 'e', 0 };
    memset(&box, 0xcc, sizeof box);
    r = dn_expand(pkt_fit2, pkt_fit2 + sizeof(pkt_fit2), pkt_fit2, box.name, 3);
    if (r != -1) return 11;
    if (!guard_intact(&box)) return 12;

    /* Zero and negative sizes are always invalid. */
    memset(&box, 0xcc, sizeof box);
    r = dn_expand(pkt_fit, pkt_fit + sizeof(pkt_fit), pkt_fit, box.name, 0);
    if (r != -1) return 13;
    r = dn_expand(pkt_fit, pkt_fit + sizeof(pkt_fit), pkt_fit, box.name, -1);
    if (r != -1) return 14;
    if (!guard_intact(&box)) return 15;

    /* src == end has no byte to parse. */
    r = dn_expand(pkt_fit, pkt_fit, pkt_fit, box.name, sizeof box.name);
    if (r != -1) return 16;

    /* Upstream regression: a compression pointer to an empty name must
       still work when space is exactly large enough for "pq". */
    unsigned char pkt_ptr[] = { 2, 'p', 'q', 0xc0, 5, 0 };
    memset(&box, 0xcc, sizeof box);
    r = dn_expand(pkt_ptr, pkt_ptr + sizeof(pkt_ptr), pkt_ptr, box.name, 3);
    if (r != 5) return 17;
    if (strcmp(box.name, "pq") != 0) return 18;

    /* A compression pointer outside the message must be rejected. */
    unsigned char pkt_badptr[] = { 0xc0, 0x40, 0 };
    r = dn_expand(pkt_badptr, pkt_badptr + sizeof(pkt_badptr),
                  pkt_badptr, box.name, sizeof box.name);
    if (r != -1) return 19;

    /* dn_skipname keeps musl's return convention: encoded length for a
       valid name, -1 for truncation. */
    if (dn_skipname(pkt_simple, pkt_simple + sizeof(pkt_simple)) != 17) return 20;
    if (dn_skipname(pkt_compress + 17, pkt_compress + sizeof(pkt_compress)) != 7) return 21;
    unsigned char pkt_trunc[] = { 3, 'a', 'b' };
    if (dn_skipname(pkt_trunc, pkt_trunc + sizeof(pkt_trunc)) != -1) return 22;
    unsigned char pkt_truncptr[] = { 0xc0 };
    if (dn_skipname(pkt_truncptr, pkt_truncptr + sizeof(pkt_truncptr)) != -1) return 23;

    return 0;
}
