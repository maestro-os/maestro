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

use super::Args;
use crate::{
	arch::x86::idt::IntFrame,
	file::{vfs, vfs::ResolutionSettings, File, O_RDONLY},
	process::{
		exec,
		exec::{exec, ExecInfo, ProgramImage},
		mem_space::copy::{UserArray, UserSlice, UserString},
		scheduler::{switch::init_ctx, SCHEDULER},
		Process,
	},
};
use core::intrinsics::unlikely;
use utils::{
	collections::{
		path::{Path, PathBuf},
		string::String,
		vec::Vec,
	},
	errno,
	errno::{CollectResult, EResult, Errno},
	ptr::arc::Arc,
};

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 256;
/// The maximum number of interpreters that can be used recursively for an
/// execution.
const INTERP_MAX: usize = 4;

// TODO Use ARG_MAX

/// A buffer containing a shebang.
struct ShebangBuffer {
	/// The before to store the shebang read from file.
	buf: [u8; SHEBANG_MAX],
	/// The index of the end of the shebang in the buffer.
	end: usize,
}

impl Default for ShebangBuffer {
	fn default() -> Self {
		Self {
			buf: [0; SHEBANG_MAX],
			end: 0,
		}
	}
}

/// Returns the file for the given `path`.
///
/// The function also parses and eventual shebang string and builds the resulting **argv**.
///
/// Arguments:
/// - `path` is the path of the executable file.
/// - `rs` is the resolution settings to be used to open files.
/// - `argv` is an iterator over the arguments passed to the system call.
fn get_file<A: Iterator<Item = EResult<String>>>(
	path: &Path,
	rs: &ResolutionSettings,
	argv: A,
) -> EResult<(Arc<vfs::Entry>, Vec<String>)> {
	let mut shebangs: [ShebangBuffer; INTERP_MAX] = Default::default();
	// Read and parse shebangs
	let mut ent = vfs::get_file_from_path(path, rs)?;
	let mut i = 0;
	loop {
		// Check permission
		let stat = ent.stat();
		if !rs.access_profile.can_read_file(&stat) || !rs.access_profile.can_execute_file(&stat) {
			return Err(errno!(EACCES));
		}
		// Read file
		let shebang = &mut shebangs[i];
		let len = {
			let file = File::open_entry(ent.clone(), O_RDONLY)?;
			let buf = UserSlice::from_slice_mut(&mut shebang.buf);
			file.ops.read(&file, 0, buf)?
		};
		// Parse shebang
		shebang.end = shebang.buf[..len]
			.iter()
			.position(|b| *b == b'\n')
			.unwrap_or(len);
		let Some(interp_path) = shebang.buf[..shebang.end].strip_prefix(b"#!") else {
			break;
		};
		let interp_path = Path::new(interp_path)?;
		i += 1;
		// If there is still an interpreter but the limit has been reached
		if unlikely(i >= INTERP_MAX) {
			return Err(errno!(ELOOP));
		}
		// Read interpreter
		ent = vfs::get_file_from_path(interp_path, rs)?;
	}
	// Build arguments
	let final_argv = shebangs[..i]
		.iter()
		.rev()
		.enumerate()
		.flat_map(|(i, shebang)| {
			let mut words =
				shebang.buf[2..shebang.end].split(|b| (*b as char).is_ascii_whitespace());
			// Skip interpreters, except the first
			if i > 0 {
				words.next();
			}
			words
		})
		.map(|s| Ok(String::try_from(s)?))
		.chain(argv)
		.collect::<EResult<CollectResult<Vec<String>>>>()?
		.0?;
	Ok((ent, final_argv))
}

pub fn execve(
	Args((pathname, argv, envp)): Args<(UserString, UserArray, UserArray)>,
	rs: ResolutionSettings,
	frame: &mut IntFrame,
) -> EResult<usize> {
	// Use scope to drop everything before calling `init_ctx`
	{
		let path = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;
		let argv = argv.iter();
		let (file, argv) = get_file(&path, &rs, argv)?;
		let envp = envp.iter().collect::<EResult<CollectResult<Vec<_>>>>()?.0?;
		let program_image = exec::build_image(
			file,
			ExecInfo {
				path_resolution: &rs,
				argv,
				envp,
			},
		)?;
		let proc = Process::current();
		exec(&proc, frame, program_image)?;
	}
	// Use `init_ctx` to handle transition to compatibility mode
	unsafe {
		init_ctx(frame);
	}
}
