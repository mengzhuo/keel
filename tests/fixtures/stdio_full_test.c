/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include "stdio.h"
#include "string.h"
#include "stdlib.h"
#include "fcntl.h"
#include "unistd.h"
#include "errno.h"
#include "limits.h"

static int test_fopen_fclose(void) {
    FILE *f = fopen("/tmp/test_stdio_fopen.txt", "w");
    if (!f) return 1;
    if (fileno(f) < 0) return 2;
    fputs("hello world\n", f);
    fclose(f);

    f = fopen("/tmp/test_stdio_fopen.txt", "r");
    if (!f) return 3;
    char buf[64];
    if (!fgets(buf, sizeof(buf), f)) return 4;
    if (strcmp(buf, "hello world\n") != 0) return 5;
    fclose(f);
    remove("/tmp/test_stdio_fopen.txt");
    return 0;
}

static int test_fwrite_fread(void) {
    FILE *f = fopen("/tmp/test_stdio_fwr.txt", "w+");
    if (!f) return 1;
    int data[] = {10, 20, 30, 40, 50};
    size_t n = fwrite(data, sizeof(int), 5, f);
    if (n != 5) return 2;
    rewind(f);
    int out[5] = {0};
    n = fread(out, sizeof(int), 5, f);
    if (n != 5) return 3;
    for (int i = 0; i < 5; i++) {
        if (out[i] != data[i]) return 4;
    }
    fclose(f);
    remove("/tmp/test_stdio_fwr.txt");
    return 0;
}

static int test_fseek_ftell(void) {
    FILE *f = fopen("/tmp/test_stdio_seek.txt", "w+");
    if (!f) return 1;
    fputs("abcdefghij", f);
    long pos = ftell(f);
    if (pos != 10) return 2;
    fseek(f, 0, SEEK_SET);
    pos = ftell(f);
    if (pos != 0) return 3;
    fseek(f, -3, SEEK_END);
    pos = ftell(f);
    if (pos != 7) return 4;
    fseek(f, 2, SEEK_CUR);
    pos = ftell(f);
    if (pos != 9) return 5;
    fclose(f);
    remove("/tmp/test_stdio_seek.txt");
    return 0;
}

static int test_fgetpos_fsetpos(void) {
    FILE *f = fopen("/tmp/test_stdio_fpos.txt", "w+");
    if (!f) return 1;
    fputs("0123456789", f);
    fpos_t pos;
    if (fgetpos(f, &pos) != 0) return 2;
    fseek(f, 0, SEEK_SET);
    if (fsetpos(f, &pos) != 0) return 3;
    if (ftell(f) != 10) return 4;
    fclose(f);
    remove("/tmp/test_stdio_fpos.txt");
    return 0;
}

static int test_feof_ferror_clearerr(void) {
    FILE *f = fopen("/tmp/test_stdio_err.txt", "w+");
    if (!f) return 1;
    fputs("x", f);
    rewind(f);
    if (feof(f)) return 2;
    if (ferror(f)) return 3;
    fgetc(f);
    fgetc(f);
    if (!feof(f)) return 4;
    clearerr(f);
    if (feof(f)) return 5;
    if (ferror(f)) return 6;
    fclose(f);
    remove("/tmp/test_stdio_err.txt");
    return 0;
}

static int test_fprintf_fscanf(void) {
    FILE *f = fopen("/tmp/test_stdio_fmt.txt", "w+");
    if (!f) return 1;
    int n = fprintf(f, "%d %u %x %s", 42, 100u, 0xff, "hi");
    if (n <= 0) return 2;
    rewind(f);
    int d; unsigned u, x; char s[8];
    n = fscanf(f, "%d %u %x %s", &d, &u, &x, s);
    if (n != 4) return 3;
    if (d != 42 || u != 100 || x != 0xff) return 4;
    if (strcmp(s, "hi") != 0) return 5;
    fclose(f);
    remove("/tmp/test_stdio_fmt.txt");
    return 0;
}

static int test_sprintf_snprintf(void) {
    char buf[64];
    int n = sprintf(buf, "%d+%d=%d", 1, 2, 3);
    if (n != 5 || strcmp(buf, "1+2=3") != 0) return 1;
    /* Truncation is the behaviour under test, not a bug. */
#pragma GCC diagnostic push
#pragma GCC diagnostic ignored "-Wformat-truncation"
    n = snprintf(buf, 4, "%s", "hello");
#pragma GCC diagnostic pop
    if (n != 5) return 2;
    if (strcmp(buf, "hel") != 0) return 3;
    n = snprintf(buf, sizeof buf, "%a", 0x1.092b2a541a68ep-19);
    if (n < 0 || strcmp(buf, "0x1.092b2a541a68ep-19") != 0) return 4;
    n = snprintf(buf, sizeof buf, "%.3a", 0x1.f8d125ff71ccp-6);
    if (n < 0 || strcmp(buf, "0x1.f8dp-6") != 0) return 5;
    n = snprintf(buf, sizeof buf, "%.0a", 1.5);
    if (n < 0 || strcmp(buf, "0x2p+0") != 0) return 6;
    n = snprintf(buf, sizeof buf, "%A", 0x1.001f1c62cf304p+0);
    if (n < 0 || strcmp(buf, "0X1.001F1C62CF304P+0") != 0) return 7;
    n = snprintf(buf, sizeof buf, "%a", 1.0);
    if (n < 0 || strcmp(buf, "0x1p+0") != 0) return 8;
    return 0;
}

static int test_sscanf(void) {
    int a, b;
    char s[16];
    int n = sscanf("42 hello 99", "%d %s %d", &a, s, &b);
    if (n != 3) return 1;
    if (a != 42 || b != 99) return 2;
    if (strcmp(s, "hello") != 0) return 3;
    /* A literal directive cannot match at end of input. */
    char empty[] = { 0 };
    n = sscanf(empty, "a");
    if (n != EOF) return 4;

    /* %m allocates the destination buffer. */
    char *ms = NULL;
    n = sscanf("hello world", "%ms", &ms);
    if (n != 1 || !ms || strcmp(ms, "hello") != 0) return 5;
    free(ms);
    ms = NULL;
    n = sscanf("abcdef", "%3ms", &ms);
    if (n != 1 || !ms || strcmp(ms, "abc") != 0) return 6;
    free(ms);
    ms = NULL;
    n = sscanf("abc123", "%m[abc]", &ms);
    if (n != 1 || !ms || strcmp(ms, "abc") != 0) return 7;
    free(ms);
    ms = NULL;
    n = sscanf("xyz", "%mc", &ms);
    if (n != 1 || !ms || ms[0] != 'x') return 8;
    free(ms);

    /* An unrepresentable field width is a format error. */
    int dummy = 0;
    n = sscanf("123", "%18446744073709551617d", &dummy);
    if (n != EOF) return 9;
    return 0;
}

static int test_getdelim_getline(void) {
    FILE *f = fopen("/tmp/test_stdio_gdl.txt", "w+");
    if (!f) return 1;
    fputs("line1\nline2\nline3", f);
    rewind(f);
    char *line = NULL;
    size_t cap = 0;
    long long len;
    len = getline(&line, &cap, f);
    if (len != 6 || strcmp(line, "line1\n") != 0) return 2;
    len = getdelim(&line, &cap, '2', f);
    if (len <= 0) return 3;
    if (line[len - 1] != '2') return 4;
    free(line);
    fclose(f);
    remove("/tmp/test_stdio_gdl.txt");
    return 0;
}

static int test_tmpfile(void) {
    FILE *f = tmpfile();
    if (!f) return 1;
    fputs("temp data", f);
    rewind(f);
    char buf[32];
    if (!fgets(buf, sizeof(buf), f)) return 2;
    if (strcmp(buf, "temp data") != 0) return 3;
    fclose(f);
    return 0;
}

static int test_tmpnam(void) {
    char name[L_tmpnam];
    char *r = tmpnam(name);
    if (!r) return 1;
    if (strlen(name) == 0) return 2;
    return 0;
}

static int test_rename_remove(void) {
    FILE *f = fopen("/tmp/test_stdio_rn1.txt", "w");
    if (!f) return 1;
    fputs("data", f);
    fclose(f);
    if (rename("/tmp/test_stdio_rn1.txt", "/tmp/test_stdio_rn2.txt") != 0) return 2;
    f = fopen("/tmp/test_stdio_rn2.txt", "r");
    if (!f) return 3;
    char buf[16];
    if (!fgets(buf, sizeof(buf), f)) return 4;
    if (strcmp(buf, "data") != 0) return 5;
    fclose(f);
    if (remove("/tmp/test_stdio_rn2.txt") != 0) return 6;
    if (remove("/tmp/test_stdio_rn2.txt") == 0) return 7;
    return 0;
}

static int test_popen_pclose(void) {
    FILE *f = popen("echo hello", "r");
    if (!f) return 1;
    char buf[32];
    if (!fgets(buf, sizeof(buf), f)) return 2;
    int rc = pclose(f);
    if (rc != 0) return 3;
    return 0;
}

static int test_setvbuf(void) {
    FILE *f = fopen("/tmp/test_stdio_svbuf.txt", "w+");
    if (!f) return 1;
    char buf[256];
    setvbuf(f, buf, _IOFBF, sizeof(buf));
    fputs("buffered", f);
    rewind(f);
    char out[32];
    if (!fgets(out, sizeof(out), f)) return 2;
    if (strcmp(out, "buffered") != 0) return 3;
    fclose(f);
    remove("/tmp/test_stdio_svbuf.txt");
    return 0;
}

static int test_fopen64(void) {
    FILE *f = fopen("/tmp/test_stdio_f64.txt", "w");
    if (!f) return 1;
    fputs("fopen64 ok", f);
    fclose(f);
    remove("/tmp/test_stdio_f64.txt");
    return 0;
}

static int test_perror_smoke(void) {
    FILE *f = fopen("/nonexistent/path", "r");
    if (f) return 1;
    perror("expected error");
    return 0;
}

static int test_fdopen(void) {
    FILE *f = fopen("/tmp/test_stdio_fd.txt", "w+");
    if (!f) return 1;
    fputs("fdopen", f);
    int fd = fileno(f);
    if (fd < 0) return 2;
    fclose(f);
    remove("/tmp/test_stdio_fd.txt");

    /* Reopening a file in append mode must leave ftello at the end, even
       before stdio flushes its buffer. */
    char path[] = "/tmp/test_stdio_fdopen_XXXXXX";
    fd = mkstemp(path);
    if (fd < 0) return 3;
    if (write(fd, "abcd", 4) != 4) return 4;
    if (close(fd) != 0) return 5;
    fd = open(path, O_WRONLY);
    if (fd < 0) return 6;
    f = fdopen(fd, "a");
    if (!f) return 7;
    if (fwrite("efg", 1, 3, f) != 3) return 8;
    if (ftello(f) != 7) return 9;
    if (fflush(f) != 0) return 10;
    if (ftello(f) != 7) return 11;
    if (fclose(f) != 0) return 12;
    if (unlink(path) != 0) return 13;
    return 0;
}

static int test_bulk_io(void) {
    const char *path = "/tmp/test_stdio_bulk.bin";
    FILE *f = fopen(path, "w+");
    if (!f) return 1;
    size_t n = 1u << 20;
    unsigned char *w = malloc(n);
    unsigned char *r = malloc(n);
    if (!w || !r) return 2;
    for (size_t i = 0; i < n; i++) w[i] = (unsigned char)(i * 31);
    if (fwrite(w, 1, n, f) != n) return 3;
    if (fseek(f, 0, SEEK_SET) != 0) return 4;
    if (fread(r, 1, n, f) != n) return 5;
    if (memcmp(w, r, n) != 0) return 6;
    if (fread(r, 1, 1, f) != 0) return 7;
    if (!feof(f)) return 8;

    /* ungetc bytes must be consumed before buffered read data. */
    if (fseek(f, 0, SEEK_SET) != 0) return 9;
    if (ungetc('Z', f) == EOF) return 10;
    char two[2];
    if (fread(two, 1, 2, f) != 2) return 11;
    if (two[0] != 'Z' || two[1] != (char)w[0]) return 12;
    fclose(f);
    remove(path);

    /* fgets returning NULL at EOF must leave the buffer untouched. */
    const char *eof_path = "/tmp/test_stdio_eof.txt";
    f = fopen(eof_path, "w");
    if (!f) return 13;
    fputs("test", f);
    fclose(f);
    f = fopen(eof_path, "r");
    if (!f) return 14;
    char line[8];
    strcpy(line, "keep");
    if (!fgets(line, sizeof line, f) || strcmp(line, "test") != 0) return 15;
    strcpy(line, "keep");
    if (fgets(line, sizeof line, f) != NULL) return 16;
    if (strcmp(line, "keep") != 0) return 17;
    fclose(f);
    remove(eof_path);
    free(w);
    free(r);
    return 0;
}

static int test_seek_overflow(void) {
    FILE *f = fopen("/tmp/test_stdio_seek.txt", "w+");
    if (!f) return 1;
    fputs("abcdef", f);
    if (fflush(f) != 0) return 2;
    rewind(f);
    if (fgetc(f) != 'a') return 3;
    errno = 0;
    if (fseeko(f, LLONG_MIN, SEEK_CUR) != -1 || errno != EOVERFLOW) return 4;
    fclose(f);
    remove("/tmp/test_stdio_seek.txt");
    return 0;
}

int main(void) {
    int r;
    if ((r = test_fopen_fclose())) { printf("fopen_fclose fail %d\n", r); return r; }
    if ((r = test_fwrite_fread())) { printf("fwrite_fread fail %d\n", r); return r + 10; }
    if ((r = test_fseek_ftell())) { printf("fseek_ftell fail %d\n", r); return r + 20; }
    if ((r = test_fgetpos_fsetpos())) { printf("fgetpos_fsetpos fail %d\n", r); return r + 30; }
    if ((r = test_feof_ferror_clearerr())) { printf("feof_ferror fail %d\n", r); return r + 40; }
    if ((r = test_fprintf_fscanf())) { printf("fprintf_fscanf fail %d\n", r); return r + 50; }
    if ((r = test_sprintf_snprintf())) { printf("sprintf_snprintf fail %d\n", r); return r + 60; }
    if ((r = test_sscanf())) { printf("sscanf fail %d\n", r); return r + 70; }
    if ((r = test_getdelim_getline())) { printf("getdelim_getline fail %d\n", r); return r + 80; }
    if ((r = test_tmpfile())) { printf("tmpfile fail %d\n", r); return r + 90; }
    if ((r = test_tmpnam())) { printf("tmpnam fail %d\n", r); return r + 100; }
    if ((r = test_rename_remove())) { printf("rename_remove fail %d\n", r); return r + 110; }
    if ((r = test_setvbuf())) { printf("setvbuf fail %d\n", r); return r + 130; }
    if ((r = test_fopen64())) { printf("fopen64 fail %d\n", r); return r + 140; }
    if ((r = test_perror_smoke())) { printf("perror fail %d\n", r); return r + 150; }
    if ((r = test_fdopen())) { printf("fdopen fail %d\n", r); return r + 160; }
    if ((r = test_seek_overflow())) { printf("seek_overflow fail %d\n", r); return r + 170; }
    if ((r = test_bulk_io())) { printf("bulk_io fail %d\n", r); return r + 180; }
    printf("stdio full ok\n");
    return 0;
}
