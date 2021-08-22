//! This module contains the system limits.

use crate::memory;

/// Maximum number of I/O operations in a single list I/O call supported by the implementation.
pub const AIO_LISTIO_MAX: usize = 2;
/// Maximum number of outstanding asynchronous I/O operations supported by the implementation.
pub const AIO_MAX: usize = 1;
/// The maximum amount by which a process can decrease its asynchronous I/O priority level from its
/// own scheduling priority.
pub const AIO_PRIO_DELTA_MAX: usize = 1024;
/// Maximum length of argument to the exec functions including environment data.
pub const ARG_MAX: usize = 4096;
/// Maximum number of functions that may be registered with atexit().
pub const ATEXIT_MAX: usize = 8;
/// Maximum number of simultaneous processes per real user ID.
pub const CHILD_MAX: usize = 25;
/// Maximum number of timer expiration overruns.
pub const DELAYTIMER_MAX: usize = 32;
/// Maximum length of a host name (not including the terminating null) as returned from the
/// gethostname() function.
pub const HOST_NAME_MAX: usize = 255;
/// Maximum number of iovec structures that one process has available for use with readv() or
/// writev().
pub const IOV_MAX: usize = 16;
/// Maximum length of a login name.
pub const LOGIN_NAME_MAX: usize = 255;
/// The maximum number of open message queue descriptors a process may hold.
pub const MQ_OPEN_MAX: usize = 8;
/// The maximum number of message priorities supported by the implementation.
pub const MQ_PRIO_MAX: usize = 32;
/// A value one greater than the maximum value that the system may assign to a newly-created file
/// descriptor.
pub const OPEN_MAX: usize = 1024;
/// Size in bytes of a page.
pub const PAGESIZE: usize = memory::PAGE_SIZE;
/// Equivalent to {PAGESIZE}. If either {PAGESIZE} or {PAGE_SIZE} is defined, the other is defined
/// with the same value.
pub const PAGE_SIZE: usize = PAGESIZE;
/// Maximum number of attempts made to destroy a thread's thread-specific data values on thread
/// exit.
pub const PTHREAD_DESTRUCTOR_ITERATIONS: usize = 4;
/// Maximum number of data keys that can be created by a process.
pub const PTHREAD_KEYS_MAX: usize = 128;
/// Minimum size in bytes of thread stack storage.
pub const PTHREAD_STACK_MIN: usize = PAGE_SIZE;
/// Maximum number of threads that can be created per process.
pub const PTHREAD_THREADS_MAX: usize = 64;
/// Maximum number of realtime signals reserved for application use in this implementation.
pub const RTSIG_MAX: usize = 8;
/// Maximum number of semaphores that a process may have.
pub const SEM_NSEMS_MAX: usize = 256;
/// The maximum value a semaphore may have.
pub const SEM_VALUE_MAX: usize = 32767;
/// Maximum number of queued signals that a process may send and have pending at the receiver(s) at
/// any time.
pub const SIGQUEUE_MAX: usize = 32;
/// The maximum number of replenishment operations that may be simultaneously pending for a
/// particular sporadic server scheduler.
pub const SS_REPL_MAX: usize = 4;
/// Maximum number of streams that one process can have open at one time. If defined, it has the
/// same value as {FOPEN_MAX} (see <stdio.h>).
pub const STREAM_MAX: usize = 8;
/// Maximum number of symbolic links that can be reliably traversed in the resolution of a pathname
/// in the absence of a loop.
pub const SYMLOOP_MAX: usize = 8;
/// Maximum number of timers per process supported by the implementation.
pub const TIMER_MAX: usize = 32;
/// Maximum length of the trace event name (not including the terminating null).
pub const TRACE_EVENT_NAME_MAX: usize = 30;
/// Maximum length of the trace generation version string or of the trace stream name (not
/// including the terminating null).
pub const TRACE_NAME_MAX: usize = 8;
/// Maximum number of trace streams that may simultaneously exist in the system.
pub const TRACE_SYS_MAX: usize = 8;
/// Maximum number of user trace event type identifiers that may simultaneously exist in a traced
/// process, including the predefined user trace event POSIX_TRACE_UNNAMED_USER_EVENT.
pub const TRACE_USER_EVENT_MAX: usize = 32;
/// Maximum length of terminal device name.
pub const TTY_NAME_MAX: usize = 9;
/// Maximum number of bytes supported for the name of a timezone (not of the TZ variable).
pub const TZNAME_MAX: usize = 6;

/// Minimum number of bits needed to represent, as a signed integer value, the maximum size of a
/// regular file allowed in the specified directory.
pub const FILESIZEBITS: usize = 32;
/// Maximum number of links to a single file.
pub const LINK_MAX: usize = 8;
/// Maximum number of bytes in a terminal canonical input line.
pub const MAX_CANON: usize = 255;
/// Minimum number of bytes for which space is available in a terminal input queue; therefore, the
/// maximum number of bytes a conforming application may require to be typed as input before
/// reading them.
pub const MAX_INPUT: usize = 255;
/// Maximum number of bytes in a filename (not including the terminating null of a filename
/// string).
pub const NAME_MAX: usize = 255;
/// Maximum number of bytes the implementation will store as a pathname in a user-supplied buffer
/// of unspecified size, including the terminating null character.
pub const PATH_MAX: usize = 4096;
/// Maximum number of bytes that is guaranteed to be atomic when writing to a pipe.
pub const PIPE_BUF: usize = 512;
/// Minimum number of bytes of storage actually allocated for any portion of a file.
pub const POSIX_ALLOC_SIZE_MIN: usize = 0;
/// Recommended increment for file transfer sizes between the {POSIX_REC_MIN_XFER_SIZE} and
/// {POSIX_REC_MAX_XFER_SIZE} values.
pub const POSIX_REC_INCR_XFER_SIZE: usize = 4096;
/// Maximum recommended file transfer size.
pub const POSIX_REC_MAX_XFER_SIZE: usize = 65536;
/// Minimum recommended file transfer size.
pub const POSIX_REC_MIN_XFER_SIZE: usize = 1024;
/// Recommended file transfer buffer alignment.
pub const POSIX_REC_XFER_ALIGN: usize = 4096;
/// Maximum number of bytes in a symbolic link.
pub const SYMLINK_MAX: usize = 4096;
