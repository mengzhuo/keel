/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

/* The string pointer forces a packed relative relocation in the DSO's
 * .data.rel.ro; the executable test asserts the loader applied it. */
static const char *relr_message_v = "dso relr";
static int value = 42;

const char *relr_message(void) { return relr_message_v; }
int relr_value(void) { return value; }
