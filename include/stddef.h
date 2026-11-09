#ifndef _STDDEF_H
#define _STDDEF_H
#ifdef __cplusplus
extern "C" {
#endif
typedef __SIZE_TYPE__ size_t;
typedef __PTRDIFF_TYPE__ ptrdiff_t;
typedef __WCHAR_TYPE__ wchar_t;
#if defined(__cplusplus) && __cplusplus >= 201103L
typedef struct { long long __ll; long double __ld; } max_align_t;
#elif !defined(__cplusplus) && defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
typedef struct { long long __ll; long double __ld; } max_align_t;
#endif
#if defined(__cplusplus) && __cplusplus >= 201103L
#define NULL nullptr
#elif defined(__cplusplus)
#define NULL 0L
#else
#define NULL ((void *)0)
#endif
#define offsetof(type, member) __builtin_offsetof(type, member)
#ifdef __cplusplus
}
#endif
#endif
