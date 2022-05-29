//! The `execve` system call allows to execute a program from a file.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::path::Path;
use crate::memory::stack;
use crate::process::Process;
use crate::process::exec::ExecInfo;
use crate::process::exec::ProgramImage;
use crate::process::exec;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process;
use crate::util::IO;
use crate::util::container::vec::Vec;

/// The maximum length of the shebang.
const SHEBANG_MAX: usize = 257;

/// Peeks the shebang in the file.
/// `file` is the file from which the shebang is to be read.
/// `buff` is the buffer to write the shebang into.
/// If the file has a shebang, the function returns its size in bytes.
pub fn peek_shebang(file: &mut File, buff: &mut [u8; SHEBANG_MAX]) -> Result<Option<u64>, Errno> {
	let size = file.read(0, buff)?;

	if size >= 2 && buff[0..1] == [b'#', b'!'] {
		Ok(Some(size))
	} else {
		Ok(None)
	}
}

/// Performs the execution on the current process.
fn do_exec(program_image: ProgramImage) -> Result<Regs, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let mut proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	// Executing the program
	exec::exec(proc, program_image)?;
	Ok(*proc.get_regs())
}

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let argv = regs.ecx as *const () as *const *const u8;
	let envp = regs.edx as *const () as *const *const u8;

	let (path, argv, envp, uid, gid, euid, egid) = {
		let proc_mutex = Process::get_current().unwrap();
		let mut proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		let path = {
			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			Path::from_str(pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?, true)?
		};
		let argv = unsafe {
			super::util::get_str_array(proc, argv)?
		};
		let envp = unsafe {
			super::util::get_str_array(proc, envp)?
		};

		let uid = proc.get_uid();
		let gid = proc.get_gid();
		let euid = proc.get_euid();
		let egid = proc.get_egid();

		(path, argv, envp, uid, gid, euid, egid)
	};

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
		if let Some(_shebang_len) = peek_shebang(f, &mut shebang)? {
			// TODO Split shebang
			// TODO Get interpreters recursively (up to a limit)
			// TODO Execute with optional arguments

			i += 1;
		} else {
			break;
		}
	}

	let program_image = {
		// TODO Find a better solution
		let mut argv_ = Vec::new();
		for a in &argv {
			argv_.push(a.as_bytes())?;
		}
		let mut envp_ = Vec::new();
		for e in &envp {
			envp_.push(e.as_bytes())?;
		}

		// Building the program's image
		let mut file_guard = file.lock();
		let exec_info = ExecInfo {
			uid,
			euid,
			gid,
			egid,

			argv: &argv_,
			envp: &envp_,
		};
		exec::build_image(file_guard.get_mut(), exec_info)?
	};

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
			result = do_exec(program_image);
		});
	}

	let regs = result?;
	unsafe {
		regs.switch(true);
	}
}
