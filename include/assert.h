#ifndef _ASSERT_H
#define _ASSERT_H

#ifdef __cplusplus
extern "C" {
#endif

#ifdef NDEBUG
#define assert(x) ((void)0)
#else
void __assert_fail(const char *, const char *, unsigned, const char *) __attribute__((noreturn));
#define assert(x) ((x) ? (void)0 : __assert_fail(#x, __FILE__, __LINE__, __func__))
#endif

#ifdef __cplusplus
}
#endif

#endif
