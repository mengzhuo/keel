/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

static int only_a(const struct dirent *e)
{
    return strcmp(e->d_name, "a") == 0;
}

static int make_file(const char *dir, const char *name)
{
    char path[256];
    snprintf(path, sizeof path, "%s/%s", dir, name);
    FILE *f = fopen(path, "w");
    if (!f) return -1;
    fclose(f);
    return 0;
}

int main(void)
{
    char dir[] = "/tmp/keel-scandir-XXXXXX";
    if (!mkdtemp(dir)) return 1;
    const char *names[] = {"b", "a", "c"};
    for (int i = 0; i < 3; i++)
        if (make_file(dir, names[i])) return 2;

    struct dirent **list = NULL;
    int n = scandir(dir, &list, NULL, alphasort);
    if (n < 3 || !list) return 3;
    for (int i = 0; i < n; i++) {
        if (list[i]->d_ino == 0) return 4;
        if (strcmp(list[i]->d_name, ".") && strcmp(list[i]->d_name, "..")) {
            /* names must be in sorted order */
            for (int j = i + 1; j < n; j++) {
                if (strcmp(list[j]->d_name, ".") && strcmp(list[j]->d_name, "..") &&
                    strcmp(list[i]->d_name, list[j]->d_name) > 0) return 5;
            }
        }
        free(list[i]);
    }
    free(list);

    /* The selector receives a real struct dirent, including d_name. */
    list = NULL;
    n = scandir(dir, &list, only_a, alphasort);
    if (n != 1 || !list || strcmp(list[0]->d_name, "a")) return 6;
    free(list[0]);
    free(list);

    /* versionsort() sorts embedded numbers numerically, not lexically. */
    char vdir[] = "/tmp/keel-versionsort-XXXXXX";
    if (!mkdtemp(vdir)) return 7;
    const char *vnames[] = {"a10", "a1", "a2"};
    for (int i = 0; i < 3; i++)
        if (make_file(vdir, vnames[i])) return 8;
    list = NULL;
    n = scandir(vdir, &list, NULL, versionsort);
    if (n != 5 || !list) return 9;
    const char *expect[] = {".", "..", "a1", "a2", "a10"};
    for (int i = 0; i < n; i++) {
        if (strcmp(list[i]->d_name, expect[i])) return 10;
        free(list[i]);
    }
    free(list);
    for (int i = 0; i < 3; i++) {
        char path[256];
        snprintf(path, sizeof path, "%s/%s", vdir, vnames[i]);
        unlink(path);
    }
    rmdir(vdir);

    for (int i = 0; i < 3; i++) {
        char path[256];
        snprintf(path, sizeof path, "%s/%s", dir, names[i]);
        unlink(path);
    }
    rmdir(dir);
    puts("scandir ok");
    return 0;
}
