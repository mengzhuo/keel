#include <dlfcn.h>

int who(void) { return 3; }

int a_value(void) { return 11; }

__thread int a_tls = 5;

int a_tls_get(void) { return a_tls; }

static int ctor_count;

__attribute__((constructor)) static void a_ctor(void) { ctor_count++; }

int a_ctor_count(void) { return ctor_count; }

void *a_next(void) { return dlsym(RTLD_NEXT, "who"); }
