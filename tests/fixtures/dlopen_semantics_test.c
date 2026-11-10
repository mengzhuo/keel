/* Copyright (c) 2026 Meng Zhuo */
/* SPDX-License-Identifier: BSD-3-Clause */

#define _GNU_SOURCE
#include <dlfcn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int who(void) { return 100; }

static int fails;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); fails++; } \
} while (0)

static void *must_sym(void *h, const char *name) {
    dlerror();
    void *p = dlsym(h, name);
    if (!p) {
        fprintf(stderr, "dlsym(%s): %s\n", name, dlerror());
        fails++;
    }
    return p;
}

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: %s <libmissing> <liba> <libb>\n", argv[0]);
        return 2;
    }
    const char *libmissing = argv[1];
    const char *liba = argv[2];
    const char *libb = argv[3];

    /* A missing DT_NEEDED must fail the dlopen, not the process. */
    dlerror();
    CHECK(dlopen(libmissing, RTLD_NOW) == NULL, "missing dependency open succeeded");
    CHECK(dlerror() != NULL, "missing dependency did not set dlerror");

    /* RTLD_NOLOAD on a library that is not loaded must fail. */
    dlerror();
    CHECK(dlopen(liba, RTLD_NOW | RTLD_NOLOAD) == NULL, "NOLOAD succeeded before load");
    CHECK(dlerror() != NULL, "NOLOAD did not set dlerror");

    /* Local load: symbols and dependency symbols are not global yet. */
    void *a = dlopen(liba, RTLD_NOW | RTLD_LOCAL);
    CHECK(a != NULL, "dlopen liba");
    CHECK(dlsym(RTLD_DEFAULT, "a_value") == NULL, "local a_value leaked global");
    CHECK(dlsym(RTLD_DEFAULT, "dep_value") == NULL, "local dep_value leaked global");

    /* Explicit handle sees the object and its dependency closure. */
    void *dep = must_sym(a, "dep_value");
    CHECK(dep && ((int (*)(void))dep)() == 77, "dependency dlsym value");

    /* TLS symbols are returned as thread-local addresses. */
    int *tls = (int *)must_sym(a, "a_tls");
    CHECK(tls != NULL, "dlsym a_tls");
    if (tls) {
        CHECK(*tls == 5, "initial TLS value");
        *tls = 42;
        int (*get)(void) = (int (*)(void))must_sym(a, "a_tls_get");
        CHECK(get && get() == 42, "TLS value through DSO");
    }

    /* Reopening with NOLOAD|GLOBAL returns the same object and makes the
     * object and its dependency closure visible to RTLD_DEFAULT. */
    void *a2 = dlopen(liba, RTLD_NOW | RTLD_NOLOAD | RTLD_GLOBAL);
    CHECK(a2 == a, "NOLOAD returned a different handle");
    CHECK(dlsym(RTLD_DEFAULT, "a_value") != NULL, "GLOBAL promotion of object");
    CHECK(dlsym(RTLD_DEFAULT, "dep_value") != NULL, "GLOBAL promotion of dependency");
    CHECK(dlsym(RTLD_DEFAULT, "a_tls") == tls, "GLOBAL TLS symbol");

    /* Loaded by absolute path, but still findable by basename. */
    CHECK(dlopen("libdlopen_a.so", RTLD_NOW | RTLD_NOLOAD) == a,
          "basename NOLOAD lookup");

    int (*ctor)(void) = (int (*)(void))must_sym(a, "a_ctor_count");
    CHECK(ctor && ctor() == 1, "constructor ran more than once");

    /* Load libb after liba: RTLD_NEXT from liba must find libb's who. */
    void *b = dlopen(libb, RTLD_NOW | RTLD_GLOBAL);
    if (!b) fprintf(stderr, "dlopen libb: %s\n", dlerror());
    CHECK(b != NULL, "dlopen libb");
    int *btls = (int *)must_sym(b, "b_tls");
    CHECK(btls != NULL && *btls == 7, "second module TLS value");
    if (btls) {
        *btls = 9;
        int (*bget)(void) = (int (*)(void))must_sym(b, "b_tls_get");
        CHECK(bget && bget() == 9, "second module TLS readback");
    }
    void *(*next)(void) = (void *(*)(void))must_sym(a, "a_next");
    void *p = next ? next() : NULL;
    CHECK(p != NULL, "RTLD_NEXT found nothing");
    if (p) CHECK(((int (*)(void))p)() == 2, "RTLD_NEXT returned wrong who");

    /* RTLD_NEXT from the main program skips main and starts at liba. */
    void *mnext = dlsym(RTLD_NEXT, "who");
    CHECK(mnext != NULL, "main RTLD_NEXT found nothing");
    if (mnext) CHECK(((int (*)(void))mnext)() == 3, "main RTLD_NEXT wrong who");

    /* Invalid handles are rejected; valid handles and dlopen(NULL) succeed. */
    CHECK(dlclose((void *)0x1234) != 0, "dlclose accepted invalid handle");
    CHECK(dlerror() != NULL, "dlclose did not set dlerror");
    CHECK(dlclose(a) == 0, "dlclose liba");
    CHECK(dlclose(b) == 0, "dlclose libb");

    void *self = dlopen(NULL, RTLD_NOW);
    CHECK(self != NULL, "dlopen(NULL)");
    CHECK(dlclose(self) == 0, "dlclose(dlopen(NULL))");

    if (!fails) puts("dlopen semantics ok");
    return fails ? 1 : 0;
}
