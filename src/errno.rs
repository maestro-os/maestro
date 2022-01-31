//! This module stores the errno utilities.

use core::fmt::Error;
use core::fmt::Formatter;
use core::fmt;

/// Structure representing a location at which an errno was raised.
#[cfg(config_debug_debug)]
#[derive(Clone, Copy, Debug)]
pub struct ErrnoLocation {
	/// The file in which the errno was raised.
	pub file: &'static str,
	/// The line at which the errno was raised.
	pub line: u32,
	/// The column at which the errno was raised.
	pub column: u32,
}

#[cfg(config_debug_debug)]
impl fmt::Display for ErrnoLocation {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
		write!(f, "file: {} line: {} col: {}", self.file, self.line, self.column)
	}
}

/// Structure representing an Unix errno.
#[derive(Clone, Copy, Debug)]
pub struct Errno {
	/// The errno number.
	errno: i32,

	/// The location at which the errno was raised.
	#[cfg(config_debug_debug)]
	location: ErrnoLocation,
}

impl Errno {
	/// Creates a new instance.
	/// This function should not be used directly but only through the `errno` macro.
	#[cfg(not(config_debug_debug))]
	pub fn new(errno: i32) -> Self {
		Self {
			errno,
		}
	}

	/// Creates a new instance.
	/// This function should not be used directly but only through the `errno` macro.
	#[cfg(config_debug_debug)]
	pub fn new(errno: i32, location: ErrnoLocation) -> Self {
		Self {
			errno,

			location,
		}
	}

	/// Returns the integer representation of the errno.
	pub fn as_int(&self) -> i32 {
		self.errno
	}

	/// Returns the error message for the given errno.
	pub fn strerror(&self) -> &'static str {
		match self.errno {
			E2BIG => "Argument list too long",
			EACCES => "Permission denied",
			EADDRINUSE => "Address in use",
			EADDRNOTAVAIL => "Address not available",
			EAFNOSUPPORT => "Address family not supported",
			EAGAIN => "Resource unavailable, try again",
			EALREADY => "Connection already in progress",
			EBADF => "Bad file descriptor",
			EBADMSG => "Bad message",
			EBUSY => "Device or resource busy",
			ECANCELED => "Operation canceled",
			ECHILD => "No child processes",
			ECONNABORTED => "Connection aborted",
			ECONNREFUSED => "Connection refused",
			ECONNRESET => "Connection reset",
			EDEADLK => "Resource deadlock would occur",
			EDESTADDRREQ => "Destination address required",
			EDOM => "Mathematics argument out of domain of function",
			EDQUOT => "Reserved",
			EEXIST => "File exists",
			EFAULT => "Bad address",
			EFBIG => "File too large",
			EHOSTUNREACH => "Host is unreachable",
			EIDRM => "Identifier removed",
			EILSEQ => "Illegal byte sequence",
			EINPROGRESS => "Operation in progress",
			EINTR => "Interrupted function",
			EINVAL => "Invalid argument",
			EIO => "I/O error",
			EISCONN => "Socket is connected",
			EISDIR => "Is a directory",
			ELOOP => "Too many levels of symbolic links",
			EMFILE => "File descriptor value too large",
			EMLINK => "Too many links",
			EMSGSIZE => "Message too large",
			EMULTIHOP => "Reserved",
			ENAMETOOLONG => "Filename too long",
			ENETDOWN => "Network is down",
			ENETRESET => "Connection aborted by network",
			ENETUNREACH => "Network unreachable",
			ENFILE => "Too many files open in system",
			ENOBUFS => "No buffer space available",
			ENODATA => "No message is available on the STREAM head read queue",
			ENODEV => "No such device",
			ENOENT => "No such file or directory",
			ENOEXEC => "Executable file format error",
			ENOLCK => "No locks available",
			ENOLINK => "Reserved",
			ENOMEM => "Not enough space",
			ENOMSG => "No message of the desired type",
			ENOPROTOOPT => "Protocol not available",
			ENOSPC => "No space left on device",
			ENOSR => "No STREAM resources",
			ENOSTR => "Not a STREAM",
			ENOSYS => "Functionality not supported",
			ENOTCONN => "The socket is not connected",
			ENOTDIR => "Not a directory or a symbolic link to a directory",
			ENOTEMPTY => "Directory not empty",
			ENOTRECOVERABLE => "State not recoverable",
			ENOTSOCK => "Not a socket",
			ENOTSUP => "Not supported",
			ENOTTY => "Inappropriate I/O control operation",
			ENXIO => "No such device or address",
			EOPNOTSUPP => "Operation not supported on socket",
			EOVERFLOW => "Value too large to be stored in data type",
			EOWNERDEAD => "Previous owner died",
			EPERM => "Operation not permitted",
			EPIPE => "Broken pipe",
			EPROTO => "Protocol error",
			EPROTONOSUPPORT => "Protocol not supported",
			EPROTOTYPE => "Protocol wrong type for socket",
			ERANGE => "Result too large",
			EROFS => "Read-only file system",
			ESPIPE => "Invalid seek",
			ESRCH => "No such process",
			ESTALE => "Reserved",
			ETIME => "Stream ioctl() timeout",
			ETIMEDOUT => "Connection timed out",
			ETXTBSY => "Text file busy",
			EWOULDBLOCK => "Operation would block",
			EXDEV => "Cross-device link",

			_ => "Unknown error",
		}
	}
}

impl PartialEq for Errno {
	fn eq(&self, rhs: &Self) -> bool {
		self.errno == rhs.errno
	}
}

#[cfg(not(config_debug_debug))]
impl fmt::Display for Errno {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
		write!(f, "errno: {}: {}", self.errno, self.strerror())
	}
}

#[cfg(config_debug_debug)]
impl fmt::Display for Errno {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
		write!(f, "errno: {}: {} (at: {})", self.errno, self.strerror(), self.location)
	}
}

/// Raises an errno.
/// `errno` is the name of the errno.
#[cfg(not(config_debug_debug))]
#[macro_export]
macro_rules! errno {
	($errno:ident) => {
		crate::errno::Errno::new(crate::errno::$errno)
	}
}

/// Raises an errno.
/// `errno` is the name of the errno.
#[cfg(config_debug_debug)]
#[macro_export]
macro_rules! errno {
	($errno:ident) => {
		crate::errno::Errno::new(crate::errno::$errno, crate::errno::ErrnoLocation {
			file: file!(),
			line: line!(),
			column: column!()
		})
	}
}

/// Argument list too long.
pub const E2BIG: i32 = 0;
/// Permission denied.
pub const EACCES: i32 = 1;
/// Address in use.
pub const EADDRINUSE: i32 = 2;
/// Address not available.
pub const EADDRNOTAVAIL: i32 = 3;
/// Address family not supported.
pub const EAFNOSUPPORT: i32 = 4;
/// Resource unavailable, try again.
pub const EAGAIN: i32 = 5;
/// Connection already in progress.
pub const EALREADY: i32 = 6;
/// Bad file descriptor.
pub const EBADF: i32 = 7;
/// Bad message.
pub const EBADMSG: i32 = 8;
/// Device or resource busy.
pub const EBUSY: i32 = 9;
/// Operation canceled.
pub const ECANCELED: i32 = 10;
/// No child processes.
pub const ECHILD: i32 = 11;
/// Connection aborted.
pub const ECONNABORTED: i32 = 12;
/// Connection refused.
pub const ECONNREFUSED: i32 = 13;
/// Connection reset.
pub const ECONNRESET: i32 = 14;
/// Resource deadlock would occur.
pub const EDEADLK: i32 = 15;
/// Destination address required.
pub const EDESTADDRREQ: i32 = 16;
/// Mathematics argument out of domain of function.
pub const EDOM: i32 = 17;
/// Reserved.
pub const EDQUOT: i32 = 18;
/// File exists.
pub const EEXIST: i32 = 19;
/// Bad address.
pub const EFAULT: i32 = 20;
/// File too large.
pub const EFBIG: i32 = 21;
/// Host is unreachable.
pub const EHOSTUNREACH: i32 = 22;
/// Identifier removed.
pub const EIDRM: i32 = 23;
/// Illegal byte sequence.
pub const EILSEQ: i32 = 24;
/// Operation in progress.
pub const EINPROGRESS: i32 = 25;
/// Interrupted function.
pub const EINTR: i32 = 26;
/// Invalid argument.
pub const EINVAL: i32 = 27;
/// I/O error.
pub const EIO: i32 = 28;
/// Socket is connected.
pub const EISCONN: i32 = 29;
/// Is a directory.
pub const EISDIR: i32 = 30;
/// Too many levels of symbolic links.
pub const ELOOP: i32 = 31;
/// File descriptor value too large.
pub const EMFILE: i32 = 32;
/// Too many links.
pub const EMLINK: i32 = 33;
/// Message too large.
pub const EMSGSIZE: i32 = 34;
/// Reserved.
pub const EMULTIHOP: i32 = 35;
/// Filename too long.
pub const ENAMETOOLONG: i32 = 36;
/// Network is down.
pub const ENETDOWN: i32 = 37;
/// Connection aborted by network.
pub const ENETRESET: i32 = 38;
/// Network unreachable.
pub const ENETUNREACH: i32 = 39;
/// Too many files open in system.
pub const ENFILE: i32 = 40;
/// No buffer space available.
pub const ENOBUFS: i32 = 41;
/// No message is available on the STREAM head read queue.
pub const ENODATA: i32 = 42;
/// No such device.
pub const ENODEV: i32 = 43;
/// No such file or directory.
pub const ENOENT: i32 = 44;
/// Executable file format error.
pub const ENOEXEC: i32 = 45;
/// No locks available.
pub const ENOLCK: i32 = 46;
/// Reserved.
pub const ENOLINK: i32 = 47;
/// Not enough space.
pub const ENOMEM: i32 = 48;
/// No message of the desired type.
pub const ENOMSG: i32 = 49;
/// Protocol not available.
pub const ENOPROTOOPT: i32 = 50;
/// No space left on device.
pub const ENOSPC: i32 = 51;
/// No STREAM resources.
pub const ENOSR: i32 = 52;
/// Not a STREAM.
pub const ENOSTR: i32 = 53;
/// Functionality not supported.
pub const ENOSYS: i32 = 54;
/// The socket is not connected.
pub const ENOTCONN: i32 = 55;
/// Not a directory or a symbolic link to a directory.
pub const ENOTDIR: i32 = 56;
/// Directory not empty.
pub const ENOTEMPTY: i32 = 57;
/// State not recoverable.
pub const ENOTRECOVERABLE: i32 = 58;
/// Not a socket.
pub const ENOTSOCK: i32 = 59;
/// Not supported.
pub const ENOTSUP: i32 = 60;
/// Inappropriate I/O control operation.
pub const ENOTTY: i32 = 61;
/// No such device or address.
pub const ENXIO: i32 = 62;
/// Operation not supported on socket.
pub const EOPNOTSUPP: i32 = 63;
/// Value too large to be stored in data type.
pub const EOVERFLOW: i32 = 64;
/// Previous owner died.
pub const EOWNERDEAD: i32 = 65;
/// Operation not permitted.
pub const EPERM: i32 = 66;
/// Broken pipe.
pub const EPIPE: i32 = 67;
/// Protocol error.
pub const EPROTO: i32 = 68;
/// Protocol not supported.
pub const EPROTONOSUPPORT: i32 = 69;
/// Protocol wrong type for socket.
pub const EPROTOTYPE: i32 = 70;
/// Result too large.
pub const ERANGE: i32 = 71;
/// Read-only file system.
pub const EROFS: i32 = 72;
/// Invalid seek.
pub const ESPIPE: i32 = 73;
/// No such process.
pub const ESRCH: i32 = 74;
/// Reserved.
pub const ESTALE: i32 = 75;
/// Stream ioctl() timeout.
pub const ETIME: i32 = 76;
/// Connection timed out.
pub const ETIMEDOUT: i32 = 77;
/// Text file busy.
pub const ETXTBSY: i32 = 78;
/// Operation would block.
pub const EWOULDBLOCK: i32 = 79;
/// Cross-device link.
pub const EXDEV: i32 = 80;
