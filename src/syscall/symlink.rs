//! The `symlink` syscall allows to create a symbolic link.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::limits;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::string::String;
use macros::syscall;

#[syscall]
pub fn symlink(target: SyscallString, linkpath: SyscallString) -> Result<i32, Errno> {
	let (target, linkpath, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let target_slice = target
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		if target_slice.len() > limits::SYMLINK_MAX {
			return Err(errno!(ENAMETOOLONG));
		}
		let target = String::try_from(target_slice)?;

		let linkpath = linkpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let linkpath = Path::new(linkpath)?;

		(target, linkpath, proc.access_profile)
	};

	// Get the path of the parent directory
	let parent_path = linkpath.parent().unwrap_or(Path::root());
	// The file's basename
	let name = linkpath.file_name().ok_or_else(|| errno!(ENOENT))?;

	// The parent directory
	let parent_mutex = vfs::get_file_from_path(parent_path, &ap, true)?;
	let mut parent = parent_mutex.lock();

	vfs::create_file(
		&mut parent,
		name.try_into()?,
		&ap,
		0o777,
		FileContent::Link(target),
	)?;

	Ok(0)
}
