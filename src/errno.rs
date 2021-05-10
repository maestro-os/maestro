//! This module stores the errno utilities.

/// Type representing an Unix errno.
pub type Errno = i32;

/// Argument list too long.
pub const E2BIG: Errno = 0;
/// Permission denied.
pub const EACCES: Errno = 1;
/// Address in use.
pub const EADDRINUSE: Errno = 2;
/// Address not available.
pub const EADDRNOTAVAIL: Errno = 3;
/// Address family not supported.
pub const EAFNOSUPPORT: Errno = 4;
/// Resource unavailable, try again.
pub const EAGAIN: Errno = 5;
/// Connection already in progress.
pub const EALREADY: Errno = 6;
/// Bad file descriptor.
pub const EBADF: Errno = 7;
/// Bad message.
pub const EBADMSG: Errno = 8;
/// Device or resource busy.
pub const EBUSY: Errno = 9;
/// Operation canceled.
pub const ECANCELED: Errno = 10;
/// No child processes.
pub const ECHILD: Errno = 11;
/// Connection aborted.
pub const ECONNABORTED: Errno = 12;
/// Connection refused.
pub const ECONNREFUSED: Errno = 13;
/// Connection reset.
pub const ECONNRESET: Errno = 14;
/// Resource deadlock would occur.
pub const EDEADLK: Errno = 15;
/// Destination address required.
pub const EDESTADDRREQ: Errno = 16;
/// Mathematics argument out of domain of function.
pub const EDOM: Errno = 17;
/// Reserved.
pub const EDQUOT: Errno = 18;
/// File exists.
pub const EEXIST: Errno = 19;
/// Bad address.
pub const EFAULT: Errno = 20;
/// File too large.
pub const EFBIG: Errno = 21;
/// Host is unreachable.
pub const EHOSTUNREACH: Errno = 22;
/// Identifier removed.
pub const EIDRM: Errno = 23;
/// Illegal byte sequence.
pub const EILSEQ: Errno = 24;
/// Operation in progress.
pub const EINPROGRESS: Errno = 25;
/// Interrupted function.
pub const EINTR: Errno = 26;
/// Invalid argument.
pub const EINVAL: Errno = 27;
/// I/O error.
pub const EIO: Errno = 28;
/// Socket is connected.
pub const EISCONN: Errno = 29;
/// Is a directory.
pub const EISDIR: Errno = 30;
/// Too many levels of symbolic links.
pub const ELOOP: Errno = 31;
/// File descriptor value too large.
pub const EMFILE: Errno = 32;
/// Too many links.
pub const EMLINK: Errno = 33;
/// Message too large.
pub const EMSGSIZE: Errno = 34;
/// Reserved.
pub const EMULTIHOP: Errno = 35;
/// Filename too long.
pub const ENAMETOOLONG: Errno = 36;
/// Network is down.
pub const ENETDOWN: Errno = 37;
/// Connection aborted by network.
pub const ENETRESET: Errno = 38;
/// Network unreachable.
pub const ENETUNREACH: Errno = 39;
/// Too many files open in system.
pub const ENFILE: Errno = 40;
/// No buffer space available.
pub const ENOBUFS: Errno = 41;
/// No message is available on the STREAM head read queue.
pub const ENODATA: Errno = 42;
/// No such device.
pub const ENODEV: Errno = 43;
/// No such file or directory.
pub const ENOENT: Errno = 44;
/// Executable file format error.
pub const ENOEXEC: Errno = 45;
/// No locks available.
pub const ENOLCK: Errno = 46;
/// Reserved.
pub const ENOLINK: Errno = 47;
/// Not enough space.
pub const ENOMEM: Errno = 48;
/// No message of the desired type.
pub const ENOMSG: Errno = 49;
/// Protocol not available.
pub const ENOPROTOOPT: Errno = 50;
/// No space left on device.
pub const ENOSPC: Errno = 51;
/// No STREAM resources.
pub const ENOSR: Errno = 52;
/// Not a STREAM.
pub const ENOSTR: Errno = 53;
/// Functionality not supported.
pub const ENOSYS: Errno = 54;
/// The socket is not connected.
pub const ENOTCONN: Errno = 55;
/// Not a directory or a symbolic link to a directory.
pub const ENOTDIR: Errno = 56;
/// Directory not empty.
pub const ENOTEMPTY: Errno = 57;
/// State not recoverable.
pub const ENOTRECOVERABLE: Errno = 58;
/// Not a socket.
pub const ENOTSOCK: Errno = 59;
/// Not supported.
pub const ENOTSUP: Errno = 60;
/// Inappropriate I/O control operation.
pub const ENOTTY: Errno = 61;
/// No such device or address.
pub const ENXIO: Errno = 62;
/// Operation not supported on socket.
pub const EOPNOTSUPP: Errno = 63;
/// Value too large to be stored in data type.
pub const EOVERFLOW: Errno = 64;
/// Previous owner died.
pub const EOWNERDEAD: Errno = 65;
/// Operation not permitted.
pub const EPERM: Errno = 66;
/// Broken pipe.
pub const EPIPE: Errno = 67;
/// Protocol error.
pub const EPROTO: Errno = 68;
/// Protocol not supported.
pub const EPROTONOSUPPORT: Errno = 69;
/// Protocol wrong type for socket.
pub const EPROTOTYPE: Errno = 70;
/// Result too large.
pub const ERANGE: Errno = 71;
/// Read-only file system.
pub const EROFS: Errno = 72;
/// Invalid seek.
pub const ESPIPE: Errno = 73;
/// No such process.
pub const ESRCH: Errno = 74;
/// Reserved.
pub const ESTALE: Errno = 75;
/// Stream ioctl() timeout.
pub const ETIME: Errno = 76;
/// Connection timed out.
pub const ETIMEDOUT: Errno = 77;
/// Text file busy.
pub const ETXTBSY: Errno = 78;
/// Operation would block.
pub const EWOULDBLOCK: Errno = 79;
/// Cross-device link.
pub const EXDEV: Errno = 80;
