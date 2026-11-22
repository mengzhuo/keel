/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#include <stdio.h>
#include <string.h>

extern int relr_value(void);
extern const char *relr_message(void);

static const char *msg = "relr ok";

int main(void) {
    if (strcmp(msg, "relr ok") != 0) return 1;
    /* This pointer lives in the DSO, so it only works when the loader
     * applied the DSO's DT_RELR table. */
    if (strcmp(relr_message(), "dso relr") != 0) return 1;
    int v = relr_value();
    printf("%s %d\n", msg, v);
    return v != 42;
}
