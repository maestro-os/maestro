/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Utility features.

use libc::{gid_t, mode_t, uid_t};
use std::{
	error::Error,
	ffi::{c_int, c_ulong, c_void, CStr, CString},
	io, mem,
	os::unix::ffi::OsStrExt,
	path::Path,
	process::{Command, Stdio},
};

pub struct TestError(pub String);

impl<E: Error> From<E> for TestError {
	fn from(err: E) -> Self {
		TestError(err.to_string())
	}
}

/// Result of a test.
pub type TestResult = Result<(), TestError>;

/// Test assertion.
#[macro_export]
macro_rules! test_assert {
	($predicate:expr) => {{
		let pred = ($predicate);
		if !pred {
			return Err($crate::util::TestError(format!(
				"Assertion failed: {}",
				stringify!($predicate)
			)));
		}
	}};
}

/// Test assertion with comparison.
#[macro_export]
macro_rules! test_assert_eq {
	($a:expr, $b:expr) => {{
		let a = ($a);
		let b = ($b);
		if a != b {
			return Err($crate::util::TestError(format!(
				"Assertion failed\n\tleft: `{:?}`\n\tright: `{:?}`",
				a, b
			)));
		}
	}};
}

/// Prints a log.
#[macro_export]
macro_rules! log {
	($($arg:tt)*) => {{
		println!("[LOG] {}", format_args!($($arg)*));
	}};
}

pub fn chmod<P: AsRef<Path>>(path: P, mode: mode_t) -> io::Result<()> {
	let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
	let res = unsafe { libc::chmod(path.as_ptr(), mode) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn fchmod(fd: c_int, mode: mode_t) -> io::Result<()> {
	let res = unsafe { libc::fchmod(fd, mode) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn chown<P: AsRef<Path>>(path: P, uid: uid_t, gid: gid_t) -> io::Result<()> {
	let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
	let res = unsafe { libc::chown(path.as_ptr(), uid, gid) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn stat<P: AsRef<Path>>(path: P) -> io::Result<libc::stat> {
	let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
	unsafe {
		let mut stat: libc::stat = mem::zeroed();
		let res = libc::stat(path.as_ptr(), &mut stat);
		if res >= 0 {
			Ok(stat)
		} else {
			Err(io::Error::last_os_error())
		}
	}
}

pub fn fstat(fd: c_int) -> io::Result<libc::stat> {
	unsafe {
		let mut stat: libc::stat = mem::zeroed();
		let res = libc::fstat(fd, &mut stat);
		if res >= 0 {
			Ok(stat)
		} else {
			Err(io::Error::last_os_error())
		}
	}
}

pub fn mkfifo<P: AsRef<Path>>(path: P, mode: mode_t) -> io::Result<()> {
	let path = CString::new(path.as_ref().as_os_str().as_bytes())?;
	let res = unsafe { libc::mkfifo(path.as_ptr(), mode) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn mount(
	src: &CStr,
	target: &CStr,
	fstype: &CStr,
	flags: c_ulong,
	data: *const c_void,
) -> io::Result<()> {
	let res = unsafe { libc::mount(src.as_ptr(), target.as_ptr(), fstype.as_ptr(), flags, data) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn seteuid(uid: uid_t) -> io::Result<()> {
	let res = unsafe { libc::seteuid(uid) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

pub fn setegid(gid: gid_t) -> io::Result<()> {
	let res = unsafe { libc::setegid(gid) };
	if res >= 0 {
		Ok(())
	} else {
		Err(io::Error::last_os_error())
	}
}

/// Executes the given code while unprivileged
pub fn unprivileged<F: FnOnce() -> R, R>(f: F) -> io::Result<R> {
	seteuid(1000)?;
	setegid(1000)?;
	let res = f();
	seteuid(0)?;
	setegid(0)?;
	Ok(res)
}

/// Executes the given command and returns a [`Result`] corresponding to the exit status.
pub fn exec(cmd: &mut Command) -> TestResult {
	// TODO capture output and compare to expected output?
	let cmd = cmd.stdout(Stdio::null()).stderr(Stdio::null());
	let status = cmd.status()?;
	if status.success() {
		Ok(())
	} else {
		Err(TestError(format!(
			"Command failed (status: {code}): {cmd:?}",
			code = status.code().unwrap(),
		)))
	}
}
