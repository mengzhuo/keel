#ifndef _UTMP_H
#define _UTMP_H

#include <utmpx.h>

#ifdef __cplusplus
extern "C" {
#endif

#define UT_NAMESIZE 32
#define UT_HOSTSIZE 256
#define UT_LINESIZE 32

struct lastlog {
    time_t ll_time;
    char ll_line[UT_LINESIZE];
    char ll_host[UT_HOSTSIZE];
};

#define ut_time ut_tv.tv_sec
#define ut_name ut_user
#define ut_addr ut_addr_v6[0]
#define utmp utmpx

void endutent(void);
struct utmp *getutent(void);
struct utmp *getutid(const struct utmp *);
struct utmp *getutline(const struct utmp *);
struct utmp *pututline(const struct utmp *);
void setutent(void);
void updwtmp(const char *, const struct utmp *);
int utmpname(const char *);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define ACCOUNTING 9
#define UTMP_FILE _PATH_UTMP
#define UTMP_FILENAME _PATH_UTMP
#define WTMP_FILE _PATH_WTMP
#define WTMP_FILENAME _PATH_WTMP
#define e_exit __e_exit
#define e_termination __e_termination

/* musl 1.2.6 compatibility helpers */
#define _PATH_UTMP "/dev/null/utmp"
#define _PATH_WTMP "/dev/null/wtmp"
#endif
