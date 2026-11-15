/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <limits.h>
#include <stdio.h>
#include <string.h>
#include <wchar.h>

#define CHECK(x) do { if (!(x)) { printf("fail %d: %s\n", __LINE__, #x); return 1; } } while (0)

int main(void)
{
    char buf[8192];
    char small[64];
    wchar_t wbuf[8192];
    int n;

    /* Negative * width is left-justified, not a huge usize. */
    n = snprintf(buf, sizeof buf, "[%*d]", -5, 1);
    CHECK(n == 7);
    CHECK(strcmp(buf, "[1    ]") == 0);

    /* INT_MIN must not overflow the absolute value. */
    n = snprintf(NULL, 0, "%*d", INT_MIN, 1);
    CHECK(n == 4096);

    /* Huge widths/precisions are capped instead of overflowing the
       formatter's stack buffers. */
    n = snprintf(NULL, 0, "%10000d", 1);
    CHECK(n == 4096);
    n = snprintf(buf, sizeof buf, "%10000d", 1);
    CHECK(n == 4096);
    CHECK(strlen(buf) == 4096);
    CHECK(buf[4095] == '1');

    n = snprintf(buf, sizeof buf, "%.*u", 2000, 0);
    CHECK(n == 1022);
    CHECK(strlen(buf) == 1022);

    n = snprintf(buf, sizeof buf, "%.2000f", 1.0);
    CHECK(n == 1101);
    CHECK(strlen(buf) == 1101);
    CHECK(buf[0] == '1' && buf[1] == '.');

    n = snprintf(buf, sizeof buf, "%.*e", 2000, 1.0);
    CHECK(n > 0 && n < 100);

    /* The stream formatter has its own width parser. */
    FILE *f = tmpfile();
    CHECK(f != NULL);
    CHECK(fprintf(f, "[%*d]", -5, 1) == 7);
    rewind(f);
    memset(small, 0, sizeof small);
    CHECK(fgets(small, sizeof small, f) != NULL);
    CHECK(strcmp(small, "[1    ]") == 0);
    fclose(f);

    /* The stream float path used to write width padding at fbuf+4096. */
    f = tmpfile();
    CHECK(f != NULL);
    CHECK(fprintf(f, "%200f", 1.0) == 200);
    fclose(f);

    /* Wide printf shares format_int/format_f64_full. */
    n = swprintf(wbuf, 8192, L"%.2000f", 1.0);
    CHECK(n == 1101);
    CHECK(wcslen(wbuf) == 1101);
    n = swprintf(wbuf, 8192, L"%10000d", 1);
    CHECK(n == 4096);
    CHECK(wcslen(wbuf) == 4096);

    /* %lc honours width and left-justification like %c. */
    n = snprintf(buf, sizeof buf, "[%3lc][%-3lc]", (wint_t)L'x', (wint_t)L'y');
    CHECK(n == 10);
    CHECK(strcmp(buf, "[  x][y  ]") == 0);

    /* %h/%hh/%j length modifiers narrow like musl. */
    n = snprintf(buf, sizeof buf, "%hd %hhd %hu %hhu %hx %hhx %hX %#ho",
                 65536 + 65, 256 + 7, 65536 + 9, 256 + 3, 0x12345, 0x234,
                 0xbeef, 0x1ff);
    CHECK(n > 0);
    CHECK(strcmp(buf, "65 7 9 3 2345 34 BEEF 0777") == 0);
    n = snprintf(buf, sizeof buf, "%jd %ju %jx",
                 (long long)-1234567890123LL,
                 (unsigned long long)18446744073709551615ULL,
                 0xdeadbeefcafebabeULL);
    CHECK(n > 0);
    CHECK(strcmp(buf, "-1234567890123 18446744073709551615 deadbeefcafebabe") == 0);
    n = swprintf(wbuf, 8192, L"%hd %hhu %jd", 65536 + 66, 256 + 4, (long long)-5);
    CHECK(n > 0);
    CHECK(wcscmp(wbuf, L"66 4 -5") == 0);
    f = tmpfile();
    CHECK(f != NULL);
    CHECK(fprintf(f, "%hd %hhu", 65536 + 68, 256 + 6) == 4);
    rewind(f);
    memset(small, 0, sizeof small);
    CHECK(fgets(small, sizeof small, f) != NULL);
    CHECK(strcmp(small, "68 6") == 0);
    fclose(f);

    printf("printf bounds ok\n");
    return 0;
}
