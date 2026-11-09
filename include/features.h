#ifndef _FEATURES_H
#define _FEATURES_H

#if defined(_GNU_SOURCE) || defined(_BSD_SOURCE) || defined(_SVID_SOURCE)
#define __USE_GNU 1
#define __USE_BSD 1
#define __USE_MISC 1
#define __USE_XOPEN2K8 1
#endif
#if defined(_POSIX_C_SOURCE) && _POSIX_C_SOURCE >= 200809L
#define __USE_XOPEN2K8 1
#endif
#if defined(_XOPEN_SOURCE) && _XOPEN_SOURCE >= 700
#define __USE_XOPEN2K8 1
#endif

#define __BEGIN_DECLS
#define __END_DECLS
#define __THROW
#define __NTH(x) x
#define __attribute__(x) __attribute__(x)
#if !defined(__cplusplus) && __STDC_VERSION__ >= 199901L
#define __restrict restrict
#define __restrict_arr restrict
#endif
#define __inline inline
#define __always_inline inline __attribute__((always_inline))
#define __hidden __attribute__((visibility("hidden")))
#define __strong_alias(a,b) extern __typeof(a) b __attribute__((alias(#a)))
#define weak_alias(a,b) extern __typeof(a) b __attribute__((weak, alias(#a)))
#define __cdecl
#define __stdcall

#ifndef _Noreturn
#if __STDC_VERSION__ >= 201112L
/* C11 keyword. */
#elif defined(__GNUC__)
#define _Noreturn __attribute__((__noreturn__))
#else
#define _Noreturn
#endif
#endif

#define __GLIBC_USE(x) 0
#define __GLIBC_PREREQ(a,b) 0

#if defined(__x86_64__)
#define __WORDSIZE 64
#define __WORDSIZE_COMPAT32 0
#else
#define __WORDSIZE 64
#endif

#endif
