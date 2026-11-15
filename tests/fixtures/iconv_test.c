/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <iconv.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

int main(void) {
    iconv_t cd = iconv_open("UTF-8", "UTF-8");
    size_t r;
    if (cd == (iconv_t)-1) return 1;

    char in[] = "hello";
    char out[16];
    char *pin = in;
    char *pout = out;
    size_t inleft = strlen(in);
    size_t outleft = sizeof(out);

    r = iconv(cd, &pin, &inleft, &pout, &outleft);
    if (r != 0) return 2;
    if (inleft != 0) return 3;
    *pout = '\0';
    if (strcmp(out, "hello") != 0) return 4;
    if (iconv_close(cd) != 0) return 5;

    cd = iconv_open("UTF-8", "UTF-16LE");
    if (cd == (iconv_t)-1) return 6;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "ISO-8859-1");
    if (cd == (iconv_t)-1) return 7;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "KOI8R");
    if (cd == (iconv_t)-1) return 8;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "CP1252");
    if (cd == (iconv_t)-1) return 9;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "GBK");
    if (cd == (iconv_t)-1) return 10;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "Big5");
    if (cd == (iconv_t)-1) return 11;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "SHIFTJIS");
    if (cd == (iconv_t)-1) return 12;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "EUCJP");
    if (cd == (iconv_t)-1) return 13;
    iconv_close(cd);

    if (iconv_open("UTF-8", "NONSENSE") != (iconv_t)-1) return 14;

    /* WCHAR_T must use unaligned-safe accesses: both directions with the
       wchar_t at offset 1 in the byte buffer. */
    cd = iconv_open("WCHAR_T", "UTF-8");
    if (cd == (iconv_t)-1) return 15;
    unsigned char outbuf[16];
    memset(outbuf, 0xcc, sizeof outbuf);
    pin = "A";
    pout = (char *)outbuf + 1;
    inleft = 1;
    outleft = sizeof outbuf - 1;
    r = iconv(cd, &pin, &inleft, &pout, &outleft);
    if (r == (size_t)-1 || inleft != 0 || outleft != sizeof outbuf - 1 - sizeof(wchar_t)) return 16;
    wchar_t wc;
    memcpy(&wc, outbuf + 1, sizeof wc);
    if (wc != L'A') return 17;
    iconv_close(cd);

    cd = iconv_open("UTF-8", "WCHAR_T");
    if (cd == (iconv_t)-1) return 18;
    unsigned char inbuf[16];
    memset(inbuf, 0xcc, sizeof inbuf);
    wc = L'A';
    memcpy(inbuf + 1, &wc, sizeof wc);
    char out2[8];
    memset(out2, 0xcc, sizeof out2);
    pin = (char *)inbuf + 1;
    pout = out2;
    inleft = sizeof wc;
    outleft = sizeof out2;
    r = iconv(cd, &pin, &inleft, &pout, &outleft);
    if (r != 0 || inleft != 0 || outleft != sizeof out2 - 1 || out2[0] != 'A') return 19;
    iconv_close(cd);

    printf("iconv ok\n");
    return 0;
}
