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

//! The `execve` system call allows to execute a program from a file.

use crate::{
	arch::x86::idt::IntFrame,
	file::{
		File, O_RDONLY,
		perm::{can_execute_file, can_read_file},
		vfs,
		vfs::Resolved,
	},
	memory::user::{UserArray, UserSlice, UserString},
	process::{
		Process,
		exec::{elf, exec},
		scheduler::switch::init_ctx,
	},
	syscall::util::{at, at::AT_FDCWD},
};
use core::{ffi::c_int, hint::unlikely};
use utils::{
	collections::{
		path::{Path, PathBuf},
		string::String,
		vec::Vec,
	},
	errno,
	errno::{CollectResult, EResult},
	ptr::arc::Arc,
};

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 256;
/// The maximum number of interpreters that can be used recursively for an
/// execution.
const INTERP_MAX: usize = 4;

// TODO Use ARG_MAX

struct Shebang<'b> {
	interp_path: &'b Path,
	optional_arg: Option<&'b [u8]>,
}

impl Shebang<'_> {
	/// Pushes as arguments, in reversed order
	fn push_args(&self, args: &mut Vec<String>) -> EResult<()> {
		if let Some(arg) = self.optional_arg {
			args.push(arg.try_into()?)?;
		}
		args.push(self.interp_path.try_into()?)?;
		Ok(())
	}
}

fn parse_shebang(buf: &mut [u8]) -> Option<Shebang> {
	let mut split = buf
		.strip_prefix(b"#!")?
		.split(|b| matches!(*b, b' ' | b'\t'))
		.filter(|s| !s.is_empty());
	Some(Shebang {
		interp_path: split.next().map(Path::new_unbounded)?,
		optional_arg: split.next(),
	})
}

fn read_shebang(buf: &mut [u8; SHEBANG_MAX], ent: Arc<vfs::Entry>) -> EResult<Option<Shebang>> {
	// Check permission
	let stat = ent.stat();
	if !can_read_file(&stat, true) || !can_execute_file(&stat, true) {
		return Err(errno!(EACCES));
	}
	// Read file
	let file = File::open(ent, O_RDONLY)?;
	let ptr = UserSlice::from_slice_mut(buf);
	let len = file.ops.read(&file, 0, ptr)?;
	// Find the end of the shebang
	let end = buf.iter().position(|b| *b == b'\n').unwrap_or(len);
	Ok(parse_shebang(&mut buf[..end]))
}

/// Returns the file for the given `path`.
///
/// The function also parses any eventual shebang strings and builds the resulting **argv**.
///
/// Arguments:
/// - `ent` is the file to execute
/// - `path` is the path to `ent`
/// - `argv` is the list of arguments passed to the system call
fn get_file(
	mut ent: Arc<vfs::Entry>,
	path: PathBuf,
	argv: UserArray,
) -> EResult<(Arc<vfs::Entry>, Vec<String>)> {
	// Collect arguments
	let mut final_argv = argv.iter().collect::<EResult<CollectResult<Vec<_>>>>()?.0?;
	let mut shebang_buf: [u8; SHEBANG_MAX] = [0; SHEBANG_MAX];
	let Some(shebang) = read_shebang(&mut shebang_buf, ent.clone())? else {
		// No shebang, stop here
		return Ok((ent, final_argv));
	};
	// Reverse the list, to avoid shifting everything at each push
	final_argv.reverse();
	// Swap `argv[0]` for `path`
	let path = path.into();
	if let Some(a) = final_argv.last_mut() {
		*a = path;
	} else {
		final_argv.push(path)?;
	}
	shebang.push_args(&mut final_argv)?;
	ent = vfs::get_file_from_path(shebang.interp_path, true)?;
	// Handle further shebangs
	let mut i = 1;
	while let Some(shebang) = read_shebang(&mut shebang_buf, ent.clone())? {
		i += 1;
		if unlikely(i > INTERP_MAX) {
			return Err(errno!(ELOOP));
		}
		shebang.push_args(&mut final_argv)?;
		ent = vfs::get_file_from_path(shebang.interp_path, true)?;
	}
	// Put back in the original order
	final_argv.reverse();
	Ok((ent, final_argv))
}

pub fn execve(
	pathname: UserString,
	argv: UserArray,
	envp: UserArray,
	frame: &mut IntFrame,
) -> EResult<usize> {
	execveat(AT_FDCWD, pathname, argv, envp, 0, frame)
}

pub fn execveat(
	dirfd: c_int,
	path: UserString,
	argv: UserArray,
	envp: UserArray,
	flags: c_int,
	frame: &mut IntFrame,
) -> EResult<usize> {
	// Use scope to drop everything before calling `init_ctx`
	{
		let path = path.copy_path_from_user()?;
		let Resolved::Found(ent) = at::get_file(dirfd, &path, flags, false, true)? else {
			unreachable!();
		};
		let (file, argv) = get_file(ent, path, argv)?;
		let envp = envp.iter().collect::<EResult<CollectResult<Vec<_>>>>()?.0?;
		let program_image = elf::exec(file, argv, envp)?;
		let proc = Process::current();
		exec(&proc, frame, program_image)?;
	}
	// Use `init_ctx` to handle transition to compatibility mode
	unsafe {
		init_ctx(frame);
	}
}
