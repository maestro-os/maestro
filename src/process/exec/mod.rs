//! This module implements program execution.

pub mod elf;

use crate::errno::Errno;
use crate::file::path::Path;
use crate::process::Process;

/// Trait representing a program executor, whose role is to load a program and to preprare it for
/// execution.
pub trait Executor {
	/// Executes the program on the given process `process`.
	/// `argv` is the list of arguments.
	/// `envp` is the environment.
	/// If the process is not in running state, the behaviour is undefined.
	fn exec(&self, process: &mut Process, argv: &[&str], envp: &[&str]) -> Result<(), Errno>;
}

/// Executes the given program in the given process with the given arguments and environment.
/// `process` is the process that will execute the program.
/// `path` is the path to the program.
/// `argv` is the list of arguments.
/// `envp` is the environment.
pub fn exec(process: &mut Process, path: &Path, argv: &[&str], envp: &[&str])
	-> Result<(), Errno> {

	// TODO Support other formats than ELF (wasm?)

	let exec = elf::ELFExecutor::new(path, process.get_euid(), process.get_egid())?;
	exec.exec(process, argv, envp)
}
