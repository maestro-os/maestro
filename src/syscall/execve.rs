//! The `execve` system call allows to execute a program from a file.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::path::Path;
use crate::idt;
use crate::process::Process;
use crate::process::Regs;
use crate::util::IO;

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 257;

/// Tells whether the given file has a shebang.
/// `file` is the file from which the shebang is to be read.
/// `buff` is the buffer to write the shebang on.
/// If the file has a shebang, the function returns its size in bytes.
pub fn read_shebang(file: &File, buff: &mut [u8; SHEBANG_MAX]) -> Result<Option<usize>, Errno> {
	let size = file.read(0, buff)?;

	if size >= 2 && buff[0..1] == [b'#', b'!'] {
		Ok(Some(size))
	} else {
		Ok(None)
	}
}

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let _argv = regs.ecx as *const () as *const *const u8;
	let _envp = regs.edx as *const () as *const *const u8;

	// Checking that parameters are accessible by the process
	let (uid, gid, path) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let path = Path::from_str(super::util::get_str(proc, pathname)?, true)?;

		// TODO Check argv and envp

		// TODO Figure out if the real or effective id should be used
		(proc.get_euid(), proc.get_egid(), path)
	};

	let mutex = fcache::get();
	let mut guard = mutex.lock();
	let files_cache = guard.get_mut();

	// The file
	let file = files_cache.as_mut().unwrap().get_file_from_path(&path)?;

	// Iterating on script files' iterators
	let mut i = 0;
	while i < 4 {
		// Locking file
		let guard = file.lock();
		let f = guard.get();

		// Checking execute permission
		if !f.can_execute(uid, gid) {
			return Err(errno::EACCES);
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

	idt::wrap_disable_interrupts(|| {
		// TODO Execute with arguments and environment

		crate::enter_loop();
	})
}
