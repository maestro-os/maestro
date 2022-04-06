//! The `execve` system call allows to execute a program from a file.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::exec::exec;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::IO;

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

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let argv = regs.ecx as *const () as *const *const u8;
	let envp = regs.edx as *const () as *const *const u8;

	let proc_mutex = Process::get_current().unwrap();
	let mut proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	let mem_space_guard = proc.get_mem_space().unwrap().lock();

	let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
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

	// Iterating on script files' iterators
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

	// Running the process
	exec(proc, &path, &*argv, &*envp)?;

	drop(proc_guard);
	crate::enter_loop();
}
