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
	file::{path::PathBuf, vfs, vfs::ResolutionSettings, File},
	memory::stack,
	process,
	process::{
		exec,
		exec::{ExecInfo, ProgramImage},
		mem_space::ptr::SyscallString,
		regs::Regs,
		Process,
	},
};
use core::ops::Range;
use macros::syscall;
use utils::{
	collections::{string::String, vec::Vec},
	errno,
	errno::{EResult, Errno},
	format,
	interrupt::cli,
	io::IO,
	lock::Mutex,
	ptr::arc::Arc,
};

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 257;
/// The maximum number of interpreter that can be used recursively for an
/// execution.
const INTERP_MAX: usize = 4;

// TODO Use ARG_MAX

/// Structure representing a shebang.
struct Shebang {
	/// The shebang's string.
	buff: [u8; SHEBANG_MAX],

	/// The range on the shebang's string which represents the location of the
	/// interpreter.
	interp: Range<usize>,
	/// The range on the shebang's string which represents the location of the
	/// optional argument.
	arg: Option<Range<usize>>,
}

/// Peeks the shebang in the file.
///
/// Arguments:
/// - `file` is the file from which the shebang is to be read.
/// - `buff` is the buffer to write the shebang into.
///
/// If the file has a shebang, the function returns its size in bytes + the
/// offset to the end of the interpreter.
///
/// If the string is longer than the interpreter's name, the remaining characters shall be used as
/// an argument.
fn peek_shebang(file: &mut File) -> EResult<Option<Shebang>> {
	let mut buff: [u8; SHEBANG_MAX] = [0; SHEBANG_MAX];

	let (size, _) = file.read(0, &mut buff)?;
	let size = size as usize;

	if size >= 2 && buff[0..2] == [b'#', b'!'] {
		// Getting the end of the shebang
		let shebang_end = buff[..size]
			.iter()
			.enumerate()
			.filter(|(_, c)| **c == b'\n')
			.map(|(off, _)| off)
			.next();
		let shebang_end = match shebang_end {
			Some(shebang_end) => shebang_end,
			None => return Ok(None),
		};

		// Getting the range of the interpreter
		let interp_end = buff[..size]
			.iter()
			.enumerate()
			.filter(|(_, c)| **c == b' ' || **c == b'\t' || **c == b'\n')
			.map(|(off, _)| off)
			.next()
			.unwrap_or(shebang_end);
		let interp = 2..interp_end;

		// Getting the range of the optional argument
		let arg = buff[..size]
			.iter()
			.enumerate()
			.skip(interp_end)
			.filter(|(_, c)| **c != b' ' && **c != b'\t')
			.map(|(off, _)| off..shebang_end)
			.find(|arg| !arg.is_empty());

		Ok(Some(Shebang {
			buff,
			interp,
			arg,
		}))
	} else {
		Ok(None)
	}
}

/// Performs the execution on the current process.
fn do_exec(program_image: ProgramImage) -> EResult<Regs> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	// Execute the program
	exec::exec(&mut proc, program_image)?;
	Ok(proc.regs.clone())
}

/// Builds a program image.
///
/// Arguments:
/// - `file` is the executable file
/// - `path_resolution` is settings for path resolution
/// - `argv` is the arguments list
/// - `envp` is the environment variables list
fn build_image(
	file: Arc<Mutex<File>>,
	path_resolution: &ResolutionSettings,
	argv: Vec<String>,
	envp: Vec<String>,
) -> EResult<ProgramImage> {
	let mut file = file.lock();
	if !path_resolution.access_profile.can_execute_file(&file) {
		return Err(errno!(EACCES));
	}

	let exec_info = ExecInfo {
		path_resolution,
		argv,
		envp,
	};
	exec::build_image(&mut file, exec_info)
}

#[syscall]
pub fn execve(
	pathname: SyscallString,
	argv: *const *const u8,
	envp: *const *const u8,
) -> Result<i32, Errno> {
	let (mut path, mut argv, envp, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let path = {
			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			let path = pathname
				.get(&mem_space_guard)?
				.ok_or_else(|| errno!(EFAULT))?;
			PathBuf::try_from(path)?
		};

		let argv = unsafe { super::util::get_str_array(&proc, argv)? };
		let envp = unsafe { super::util::get_str_array(&proc, envp)? };

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, argv, envp, rs)
	};

	// Handling shebang
	let mut i = 0;
	while i < INTERP_MAX + 1 {
		// The file
		let file = vfs::get_file_from_path(&path, &rs)?;
		let mut f = file.lock();

		if !rs.access_profile.can_execute_file(&f) {
			return Err(errno!(EACCES));
		}

		// If the file has a shebang, process it
		if let Some(shebang) = peek_shebang(&mut f)? {
			// If too many interpreter recursions, abort
			if i == INTERP_MAX {
				return Err(errno!(ELOOP));
			}

			// Add the script to arguments
			if argv.is_empty() {
				argv.push(format!("{path}")?)?;
			} else {
				argv[0] = format!("{path}")?;
			}

			// Set interpreter to arguments
			let interp = String::try_from(&shebang.buff[shebang.interp.clone()])?;
			argv.insert(0, interp)?;

			// Set optional argument if it exists
			if let Some(arg) = shebang.arg {
				let arg = String::try_from(&shebang.buff[arg])?;
				argv.insert(1, arg)?;
			}

			// Set interpreter's path
			path = PathBuf::try_from(&shebang.buff[shebang.interp])?;

			i += 1;
		} else {
			break;
		}
	}

	// The file
	let file = vfs::get_file_from_path(&path, &rs)?;

	// Drop path to avoid memory leak
	drop(path);

	// Disable interrupt to prevent stack switching while using a temporary stack,
	// preventing this temporary stack from being used as a signal handling stack
	cli();

	// Build the program's image
	let program_image =
		unsafe { stack::switch(None, move || build_image(file, &rs, argv, envp)).unwrap()? };

	// The temporary stack will not be used since the scheduler cannot be ticked when
	// interrupts are disabled
	// A temporary stack cannot be allocated since it wouldn't be possible to free
	// it on success
	let tmp_stack = {
		let core = 0; // TODO Get current kernel ID
		process::get_scheduler().lock().get_tmp_stack(core)
	};

	// Switch to another stack in order to avoid crashing when switching to the
	// new memory space
	unsafe {
		stack::switch(Some(tmp_stack), move || -> EResult<()> {
			let regs = do_exec(program_image)?;
			regs.switch(true);
		})
		// `unwrap` cannot fail since the stack is provided
		.unwrap()?;
	}

	// Cannot be reached since on success
	unreachable!();
}
