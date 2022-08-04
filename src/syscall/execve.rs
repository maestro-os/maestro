//! The `execve` system call allows to execute a program from a file.

use crate::errno;
use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::file::File;
use crate::file::Gid;
use crate::file::Uid;
use crate::memory::stack;
use crate::process;
use crate::process::exec;
use crate::process::exec::ExecInfo;
use crate::process::exec::ProgramImage;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::ptr::SharedPtr;
use core::ops::Range;

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 257;
/// The maximum number of interpreter that can be used recursively for an execution.
const INTERP_MAX: usize = 4;

// TODO Use ARG_MAX

/// Structure representing a shebang.
struct Shebang {
	/// The shebang's string.
	buff: [u8; SHEBANG_MAX],

	/// The range on the shebang's string which represents the location of the interpreter.
	interp: Range<usize>,
	/// The range on the shebang's string which represents the location of the optional argument.
	arg: Option<Range<usize>>,
}

/// Peeks the shebang in the file.
/// `file` is the file from which the shebang is to be read.
/// `buff` is the buffer to write the shebang into.
/// If the file has a shebang, the function returns its size in bytes + the offset to the end of
/// the interpreter. If the string is longer than the interpreter's name, the remaining characters
/// shall be used as an argument.
fn peek_shebang(file: &mut File) -> Result<Option<Shebang>, Errno> {
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
			.filter(|arg| !arg.is_empty())
			.next();

		Ok(Some(Shebang { buff, interp, arg }))
	} else {
		Ok(None)
	}
}

/// Performs the execution on the current process.
fn do_exec(program_image: ProgramImage) -> Result<Regs, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	// Executing the program
	exec::exec(proc, program_image)?;
	Ok(*proc.get_regs())
}

// TODO clean
/// TODO doc
fn build_image(
	file: SharedPtr<File>,
	uid: Uid,
	euid: Uid,
	gid: Gid,
	egid: Gid,
	argv: &[String],
	envp: &[String],
) -> Result<ProgramImage, Errno> {
	// TODO Find a better solution
	let mut argv_ = Vec::new();
	for a in argv {
		argv_.push(a.as_bytes())?;
	}
	let mut envp_ = Vec::new();
	for e in envp {
		envp_.push(e.as_bytes())?;
	}

	let file_guard = file.lock();
	let file = file_guard.get_mut();
	if !file.can_execute(euid, egid) {
		return Err(errno!(EACCES));
	}

	let exec_info = ExecInfo {
		uid,
		euid,
		gid,
		egid,

		argv: &argv_,
		envp: &envp_,
	};

	exec::build_image(file, exec_info)
}

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let argv = regs.ecx as *const () as *const *const u8;
	let envp = regs.edx as *const () as *const *const u8;

	let (mut path, mut argv, envp, uid, gid, euid, egid) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		let path = {
			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			Path::from_str(
				pathname
					.get(&mem_space_guard)?
					.ok_or_else(|| errno!(EFAULT))?,
				true,
			)?
		};
		let path = super::util::get_absolute_path(proc, path)?;

		let argv = unsafe { super::util::get_str_array(proc, argv)? };
		let envp = unsafe { super::util::get_str_array(proc, envp)? };

		let uid = proc.get_uid();
		let gid = proc.get_gid();
		let euid = proc.get_euid();
		let egid = proc.get_egid();

		(path, argv, envp, uid, gid, euid, egid)
	};

	// Handling shebang
	let mut i = 0;
	while i < INTERP_MAX + 1 {
		// The file
		let file = {
			let files_mutex = fcache::get();
			let files_guard = files_mutex.lock();
			let files_cache = files_guard.get_mut();

			files_cache
				.as_mut()
				.unwrap()
				.get_file_from_path(&path, uid, gid, true)?
		};
		let guard = file.lock();
		let f = guard.get_mut();

		// If the file has a shebang, process it
		if let Some(shebang) = peek_shebang(f)? {
			// If too many interpreter recursions, abort
			if i == INTERP_MAX {
				return Err(errno!(ELOOP));
			}

			// Adding the script to arguments
			if argv.is_empty() {
				argv.push(path.as_string()?)?;
			} else {
				argv[0] = path.as_string()?;
			}

			// Setting interpreter to arguments
			let interp = String::from(&shebang.buff[shebang.interp.clone()])?;
			argv.insert(0, interp)?;

			// Setting optional argument if it exists
			if let Some(arg) = shebang.arg {
				let arg = String::from(&shebang.buff[arg])?;
				argv.insert(1, arg)?;
			}

			// Setting interpreter's path
			path = Path::from_str(&shebang.buff[shebang.interp], true)?;

			i += 1;
		} else {
			break;
		}
	}

	// The file
	let file = {
		let files_mutex = fcache::get();
		let files_guard = files_mutex.lock();
		let files_cache = files_guard.get_mut();

		files_cache
			.as_mut()
			.unwrap()
			.get_file_from_path(&path, uid, gid, true)?
	};

	// Building the program's image
	let program_image = unsafe {
		stack::switch(None, move || {
			build_image(file, uid, euid, gid, egid, &argv, &envp)
		})
		.unwrap()?
	};

	cli!();
	// The tmp stack will not be used since the scheduler cannot be ticked when interrupts are
	// disabled
	// A temporary stack cannot be allocated since it wouldn't be possible to free it on success
	let tmp_stack = {
		let core = 0; // TODO Get current core ID
		process::get_scheduler()
			.lock()
			.get_mut()
			.get_tmp_stack(core)
	};

	// Switching to another stack in order to avoid crashing when switching to the new memory
	// space
	unsafe {
		stack::switch(Some(tmp_stack), move || -> Result<(), Errno> {
			let regs = do_exec(program_image)?;
			regs.switch(true);
		})
		.unwrap()?;
	}

	// Cannot be reached since `do_exec` won't return on success
	unreachable!();
}
