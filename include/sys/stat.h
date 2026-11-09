#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <sys/types.h>
#include <time.h>

struct stat {
#if defined(__aarch64__) || defined(__riscv)
    dev_t st_dev;
    ino_t st_ino;
    mode_t st_mode;
    nlink_t st_nlink;
    uid_t st_uid;
    gid_t st_gid;
    dev_t st_rdev;
    unsigned long long __pad;
    off_t st_size;
    blksize_t st_blksize;
    int __pad2;
    blkcnt_t st_blocks;
    struct timespec st_atim;
    struct timespec st_mtim;
    struct timespec st_ctim;
    unsigned int __unused[2];
#else
    dev_t st_dev;
    ino_t st_ino;
    nlink_t st_nlink;
    mode_t st_mode;
    uid_t st_uid;
    gid_t st_gid;
    unsigned int __pad0;
    dev_t st_rdev;
    off_t st_size;
    blksize_t st_blksize;
    blkcnt_t st_blocks;
    struct timespec st_atim;
    struct timespec st_mtim;
    struct timespec st_ctim;
    long __unused[3];
#endif
};

#define st_atime st_atim.tv_sec
#define st_mtime st_mtim.tv_sec
#define st_ctime st_ctim.tv_sec

#define S_IFMT   0170000
#define S_IFDIR  0040000
#define S_IFCHR  0020000
#define S_IFBLK  0060000
#define S_IFREG  0100000
#define S_IFLNK  0120000
#define S_IFIFO  0010000
#define S_IFSOCK 0140000

#define S_TYPEISMQ(buf)  0
#define S_TYPEISSEM(buf) 0
#define S_TYPEISSHM(buf) 0
#define S_TYPEISTMO(buf) 0

#define S_ISDIR(m)  (((m) & S_IFMT) == S_IFDIR)
#define S_ISCHR(m)  (((m) & S_IFMT) == S_IFCHR)
#define S_ISBLK(m)  (((m) & S_IFMT) == S_IFBLK)
#define S_ISREG(m)  (((m) & S_IFMT) == S_IFREG)
#define S_ISLNK(m)  (((m) & S_IFMT) == S_IFLNK)
#define S_ISFIFO(m) (((m) & S_IFMT) == S_IFIFO)
#define S_ISSOCK(m) (((m) & S_IFMT) == S_IFSOCK)

#ifndef S_IRUSR
#define S_ISUID 04000
#define S_ISGID 02000
#define S_ISVTX 01000
#define S_IRUSR 0400
#define S_IWUSR 0200
#define S_IXUSR 0100
#define S_IRWXU 0700
#define S_IRGRP 0040
#define S_IWGRP 0020
#define S_IXGRP 0010
#define S_IRWXG 0070
#define S_IROTH 0004
#define S_IWOTH 0002
#define S_IXOTH 0001
#define S_IRWXO 0007
#endif

#define AT_FDCWD (-100)

#define UTIME_NOW  0x3fffffff
#define UTIME_OMIT 0x3ffffffe

int stat(const char *, struct stat *);
int fstat(int, struct stat *);
int lstat(const char *, struct stat *);
int fstatat(int, const char *, struct stat *, int);
int chmod(const char *, mode_t);
int fchmod(int, mode_t);
int fchmodat(int, const char *, mode_t, int);
mode_t umask(mode_t);
int mkdir(const char *, mode_t);
int mkfifo(const char *, mode_t);
int mkdirat(int, const char *, mode_t);
int mkfifoat(int, const char *, mode_t);
int mknod(const char *, mode_t, dev_t);
int mknodat(int, const char *, mode_t, dev_t);
int futimens(int, const struct timespec [2]);
int utimensat(int, const char *, const struct timespec [2], int);

#ifdef __cplusplus
}
#endif


/* musl 1.2.6 compatibility macros */
#define STATX_ALL 0xfffU
#define STATX_ATIME 0x20U
#define STATX_ATTR_APPEND 0x20
#define STATX_ATTR_AUTOMOUNT 0x1000
#define STATX_ATTR_COMPRESSED 0x4
#define STATX_ATTR_DAX 0x200000
#define STATX_ATTR_ENCRYPTED 0x800
#define STATX_ATTR_IMMUTABLE 0x10
#define STATX_ATTR_MOUNT_ROOT 0x2000
#define STATX_ATTR_NODUMP 0x40
#define STATX_ATTR_VERITY 0x100000
#define STATX_ATTR_WRITE_ATOMIC 0x400000
#define STATX_BASIC_STATS 0x7ffU
#define STATX_BLOCKS 0x400U
#define STATX_BTIME 0x800U
#define STATX_CTIME 0x80U
#define STATX_DIOALIGN 0x2000U
#define STATX_GID 0x10U
#define STATX_INO 0x100U
#define STATX_MNT_ID 0x1000U
#define STATX_MNT_ID_UNIQUE 0x4000U
#define STATX_MODE 2U
#define STATX_MTIME 0x40U
#define STATX_NLINK 4U
#define STATX_SIZE 0x200U
#define STATX_SUBVOL 0x8000U
#define STATX_TYPE 1U
#define STATX_UID 8U
#define STATX_WRITE_ATOMIC 0x10000U
#define S_IEXEC S_IXUSR
#define S_IREAD S_IRUSR
#define S_IWRITE S_IWUSR
#endif
