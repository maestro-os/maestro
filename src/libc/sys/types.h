#ifndef TYPES_H
# define TYPES_H

typedef int blkcount_t;
typedef int blksize_t;
typedef int clock_t;
typedef int clockid_t;
typedef int dev_t;
typedef unsigned fsblkcnt_t;
typedef unsigned fsfilcnt_t;
typedef int gid_t;
typedef int id_t;
typedef int ino_t;
typedef int key_t;
typedef int mode_t;
typedef int nlink_t;
typedef int off_t;
typedef int pid_t;
typedef int pthread_attr_t;
typedef int pthread_barrier_t;
typedef int pthread_barrierattr_t;
typedef int pthread_cond_t;
typedef int pthread_condattr_t;
typedef int pthread_key_t;
typedef int pthread_mutex_t;
typedef int pthread_mutexattr_t;
typedef int pthread_once_t;
typedef int pthread_rwlock_t;
typedef int pthread_rwlockattr_t;
typedef int pthread_spinlock_t;
typedef int pthread_t;
// TODO Already defined in C std library typedef uintmax_t size_t;
typedef intmax_t ssize_t;
typedef long suseconds_t;
typedef int time_t;
typedef int timer_t;
typedef int trace_attr_t;
typedef int trace_event_id_t;
typedef int trace_event_set_t;
typedef int trace_id_t;
typedef int uid_t;

#endif
