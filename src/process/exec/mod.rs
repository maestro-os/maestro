//! This module implements program execution.

pub mod elf;

use crate::errno::Errno;
use crate::file::File;
use crate::process::mem_space::MemSpace;
use crate::process::regs::Regs;
use crate::process::signal::SignalHandler;
use crate::process::Gid;
use crate::process::Process;
use crate::process::Uid;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::ptr::IntSharedPtr;
use core::ffi::c_void;

/// Structure storing informations to prepare a program image to be executed.
pub struct ExecInfo {
	/// The process's uid.
	pub uid: Uid,
	/// The process's euid.
	pub euid: Uid,
	/// The process's gid.
	pub gid: Gid,
	/// The process's egid.
	pub egid: Gid,

	/// The list of arguments.
	pub argv: Vec<String>,
	/// The list of environment variables.
	pub envp: Vec<String>,
}

/// Structure representing the loaded image of a program.
pub struct ProgramImage {
	/// The argv of the program.
	argv: Vec<String>,

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

/// Trait representing a program executor, whose role is to load a program and
/// to preprare it for execution.
pub trait Executor {
	/// Builds a program image.
	/// `file` is the program's file.
	fn build_image(&self, file: &mut File) -> Result<ProgramImage, Errno>;
}

/// Builds a program image from the given executable file.
/// `file` is the program's file.
/// `info` is the set execution informations for the program.
/// The function returns a memory space containing the program image and the
/// pointer to the entry point.
pub fn build_image(file: &mut File, info: ExecInfo) -> Result<ProgramImage, Errno> {
	// TODO Support other formats than ELF (wasm?)

	let exec = elf::ELFExecutor::new(info)?;
	exec.build_image(file)
}

/// Executes the program image `image` on the process `proc`.
pub fn exec(proc: &mut Process, image: ProgramImage) -> Result<(), Errno> {
	proc.set_argv(image.argv);
	// TODO Set exec path

	// Duplicate file descriptors
	proc.duplicate_fds()?; // TODO Undo on fail
					   // Setting the new memory space to the process
	proc.set_mem_space(Some(IntSharedPtr::new(image.mem_space)?));

	// Setting the process's stacks
	proc.user_stack = Some(image.user_stack);
	proc.kernel_stack = Some(image.kernel_stack);
	proc.update_tss();

	// Resetting signals
	proc.sigmask.clear_all();
	{
		let handlers_guard = proc.signal_handlers.lock();
		let handlers = handlers_guard.get_mut();

		for i in 0..handlers.len() {
			handlers[i] = SignalHandler::Default;
		}
	}

	proc.reset_vfork();
	proc.clear_tls_entries();

	// Setting the proc's registers
	let regs = Regs {
		esp: image.user_stack_begin as _,
		eip: image.entry_point as _,
		..Default::default()
	};
	proc.regs = regs;

	Ok(())
}
