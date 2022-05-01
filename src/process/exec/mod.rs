//! This module implements program execution.

pub mod elf;

use core::ffi::c_void;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::Gid;
use crate::process::Process;
use crate::process::Uid;
use crate::process::mem_space::MemSpace;
use crate::process::regs::Regs;
use crate::process::signal::SignalHandler;
use crate::util::ptr::IntSharedPtr;

/// Structure storing informations to prepare a program image to be executed.
pub struct ExecInfo<'a> {
	/// The process's uid.
	uid: Uid,
	/// The process's euid.
	euid: Uid,
	/// The process's gid.
	gid: Gid,
	/// The process's egid.
	egid: Gid,

	/// The list of arguments.
	argv: &'a [&'a [u8]],
	/// The list of environment variables.
	envp: &'a [&'a [u8]],
}

/// Structure representing the loaded image of a program.
pub struct ProgramImage {
	/// The image's memory space.
	mem_space: MemSpace,

	/// A pointer to the entry point of the program.
	entry_point: *const c_void,

	/// A pointer to the process's user stack.
	user_stack: *const c_void,
	/// A pointer to the initial value of the user stack pointer.
	user_stack_begin: *const c_void,

	/// A pointer to the process's kernel stack.
	kernel_stack: *const c_void,
}

/// Trait representing a program executor, whose role is to load a program and to preprare it for
/// execution.
pub trait Executor<'a> {
	/// Builds a program image.
	/// `path` is the path to the program.
	fn build_image(&'a self, path: &Path) -> Result<ProgramImage, Errno>;
}

/// Builds a program image from the given executable file.
/// `path` is the path to the executable file.
/// `argv` is the list of arguments.
/// `envp` is the environment.
/// The function returns a memory space containing the program image and the pointer to the entry
/// point.
pub fn build_image(path: &Path, info: ExecInfo)
	-> Result<ProgramImage, Errno> {

	// TODO Support other formats than ELF (wasm?)

	let exec = elf::ELFExecutor::new(info)?;
	exec.build_image(path)
}

// TODO Find a way to avoid locking the process while parsing the ELF
/// TODO doc
pub fn exec(proc: &mut Process, path: &Path, argv: &[&[u8]], envp: &[&[u8]]) -> Result<(), Errno> {
	// Building the program's image
	let program_image = build_image(&path, ExecInfo {
		uid: proc.get_uid(),
		euid: proc.get_euid(),
		gid: proc.get_gid(),
		egid: proc.get_egid(),

		argv,
		envp,
	})?;

	// Setting the new memory space to the process
	proc.set_mem_space(Some(IntSharedPtr::new(program_image.mem_space)?));
	// Setting the process's stacks
	proc.user_stack = Some(program_image.user_stack);
	proc.kernel_stack = Some(program_image.kernel_stack);
	proc.update_tss();

	// TODO Duplicate file descriptors (but not open file descriptions)
	// TODO Close fds with CLOEXEC flag

	// Resetting signals
	proc.sigmask.clear_all();
	{
		let mut handlers_guard = proc.signal_handlers.lock();
		let handlers = handlers_guard.get_mut();

		for i in 0..handlers.len() {
			handlers[i] = SignalHandler::Default;
		}
	}

	proc.reset_vfork();

	// Setting the proc's registers
	let regs = Regs {
		esp: program_image.user_stack_begin as _,
		eip: program_image.entry_point as _,
		..Default::default()
	};
	proc.regs = regs;

	Ok(())
}
