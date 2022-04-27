//! The `execve` system call allows to execute a program from a file.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::path::Path;
use crate::memory::stack;
use crate::process::Process;
use crate::process::exec::exec;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process;
use crate::util::IO;
use crate::util::container::vec::Vec;

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 257;

/// Tells whether the given file has a shebang.
/// `file` is the file from which the shebang is to be read.
/// `buff` is the buffer to write the shebang on.
/// If the file has a shebang, the function returns its size in bytes.
pub fn read_shebang(file: &mut File, buff: &mut [u8; SHEBANG_MAX]) -> Result<Option<u64>, Errno> {
	let size = file.read(0, buff)?;

	if size >= 2 && buff[0..1] == [b'#', b'!'] {
		Ok(Some(size))
	} else {
		Ok(None)
	}
}

/// Performs the execution on the current process.
fn do_exec(pathname: SyscallString, argv: *const *const u8, envp: *const *const u8)
	-> Result<Regs, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let mut proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	let path = {
		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		Path::from_str(pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?, true)?
	};

	let (argv, envp) = unsafe {
		(super::util::get_str_array(proc, argv)?, super::util::get_str_array(proc, envp)?)
	};

	let uid = proc.get_euid();
	let gid = proc.get_egid();

	// The file
	let file = {
		let files_mutex = fcache::get();
		let mut files_guard = files_mutex.lock();
		let files_cache = files_guard.get_mut();

		files_cache.as_mut().unwrap().get_file_from_path(&path, uid, gid, true)?
	};

	// Handling shebang
	let mut i = 0;
	while i < 4 {
		// Locking file
		let mut guard = file.lock();
		let f = guard.get_mut();

		// Checking execute permission
		if !f.can_execute(uid, gid) {
			return Err(errno!(EACCES));
		}

		let mut shebang: [u8; SHEBANG_MAX] = [0; SHEBANG_MAX];

		// If the file has a shebang, process it
		if let Some(_shebang_len) = read_shebang(f, &mut shebang)? {
			// TODO Split shebang
			// TODO Get interpreters recursively (up to a limit)
			// TODO Execute with optional arguments

			i += 1;
		} else {
			break;
		}
	}

	// TODO Find a better solution
	let mut argv_ = Vec::new();
	for a in &argv {
		argv_.push(a.as_bytes())?;
	}
	let mut envp_ = Vec::new();
	for e in &envp {
		envp_.push(e.as_bytes())?;
	}

	// Executing the program
	exec(proc, &path, &*argv_, &*envp_)?;
	Ok(*proc.get_regs())
}

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	cli!();
	// The tmp stack will not be used since the scheduler cannot be ticked when interrupts are
	// disabled
	let tmp_stack = {
		let core = 0; // TODO Get current core ID
		process::get_scheduler().lock().get_mut().get_tmp_stack(core)
	};

	// Switching to another stack in order to avoid crashing when switching to the new memory
	// space
	let mut result = Err(errno!(EINVAL));
	unsafe {
		stack::switch(tmp_stack, || {
			let r = (|| {
				let pathname: SyscallString = (regs.ebx as usize).into();
				let argv = regs.ecx as *const () as *const *const u8;
				let envp = regs.edx as *const () as *const *const u8;

				do_exec(pathname, argv, envp)
			})();

			if let Ok(regs) = r {
				regs.switch(true);
			} else {
				result = r;
			}
		});
	}
	result?;

	// Cannot be reached since `do_exec` won't return on success
	unreachable!();
}
