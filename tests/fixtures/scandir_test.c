/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */
#include <dirent.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main(void)
{
    char dir[] = "/tmp/keel-scandir-XXXXXX";
    if (!mkdtemp(dir)) return 1;
    char path[256];
    const char *names[] = {"b", "a", "c"};
    for (int i = 0; i < 3; i++) {
        snprintf(path, sizeof path, "%s/%s", dir, names[i]);
        FILE *f = fopen(path, "w");
        if (!f) return 2;
        fclose(f);
    }

    struct dirent **list = NULL;
    int n = scandir(dir, &list, NULL, alphasort);
    if (n < 3 || !list) return 3;
    for (int i = 0; i < n; i++) {
        if (strcmp(list[i]->d_name, ".") && strcmp(list[i]->d_name, "..")) {
            /* names must be in sorted order */
            for (int j = i + 1; j < n; j++) {
                if (strcmp(list[j]->d_name, ".") && strcmp(list[j]->d_name, "..") &&
                    strcmp(list[i]->d_name, list[j]->d_name) > 0) return 4;
            }
        }
        free(list[i]);
    }
    free(list);
    for (int i = 0; i < 3; i++) {
        snprintf(path, sizeof path, "%s/%s", dir, names[i]);
        unlink(path);
    }
    rmdir(dir);
    puts("scandir ok");
    return 0;
}
