//! The open system call allows a process to open a file and get a file descriptor.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileType;
use crate::file::path::Path;
use crate::file;
use crate::limits;
use crate::process::Process;
use crate::util::ptr::SharedPtr;
use crate::util;

/// At each write operations on the file descriptor, the cursor is placed at the end of the file so
/// the data is appended.
pub const O_APPEND: u32 =    0b00000000000001;
/// TODO doc
pub const O_ASYNC: u32 =     0b00000000000010;
/// TODO doc
pub const O_CLOEXEC: u32 =   0b00000000000100;
/// If the file doesn't exist, create it.
pub const O_CREAT: u32 =     0b00000000001000;
/// TODO doc
pub const O_DIRECT: u32 =    0b00000000010000;
/// TODO doc
pub const O_DIRECTORY: u32 = 0b00000000100000;
/// TODO doc
pub const O_EXCL: u32 =      0b00000001000000;
/// TODO doc
pub const O_LARGEFILE: u32 = 0b00000010000000;
/// TODO doc
pub const O_NOATIME: u32 =   0b00000100000000;
/// TODO doc
pub const O_NOCTTY: u32 =    0b00001000000000;
/// Tells `open` not to follow symbolic links.
pub const O_NOFOLLOW: u32 =  0b00010000000000;
/// TODO doc
pub const O_NONBLOCK: u32 =  0b00100000000000;
/// TODO doc
pub const O_SYNC: u32 =      0b01000000000000;
/// TODO doc
pub const O_TRUNC: u32 =     0b10000000000000;

/// Returns the absolute path to the file.
fn get_file_absolute_path(process: &Process, path_str: &str) -> Result<Path, Errno> {
	let path = Path::from_string(path_str)?;
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		let mut absolute_path = cwd.concat(&path)?;
		absolute_path.reduce()?;
		Ok(absolute_path)
	} else {
		Ok(path)
	}
}

/// Returns the file at the given path `path`.
/// If the file doesn't exist and the O_CREAT flag is set, the file is created, then the function
/// returns it. If the flag is not set, the function returns an error with the appropriate errno.
/// If the file is to be created, the function uses `mode` to set its permissions.
fn get_file(path: Path, flags: u32, mode: u16) -> Result<SharedPtr<File>, Errno> {
	let mutex = file::get_files_cache();
	let mut guard = mutex.lock(true);
	let files_cache = guard.get_mut();

	if let Ok(file) = files_cache.get_file_from_path(&path) {
		Ok(file)
	} else if flags & O_CREAT != 0 {
		let name = crate::util::container::string::String::from("TODO")?; // TODO
		let uid = 0; // TODO
		let gid = 0; // TODO
		let file = File::new(name, FileType::Regular, uid, gid, mode)?;
		files_cache.create_file(&path, file)?;

		files_cache.get_file_from_path(&path)
	} else {
		Err(-errno::ENOENT as _)
	}
}

/// Resolves symbolic links and returns the final file. If too many links are to be resolved, the
/// function returns an error.
/// `file` is the starting file. If not a link, the function returns the same file directly.
/// `flags` are the system call's flag.
/// `mode` is used in case the file has to be created and represents its permissions to be set.
fn resolve_links(file: SharedPtr<File>, flags: u32, mode: u16) -> Result<SharedPtr<File>, Errno> {
	let mut resolve_count = 0;
	let mut file = file;

	loop {
		let file_guard = file.lock(true);
		let f = file_guard.get();
		if f.get_file_type() != FileType::Link {
			break;
		}

		let mut parent_path = f.get_path()?;
		parent_path.pop();

		let mut path = (parent_path + Path::from_string(f.get_link_target().as_str())?)?;
		path.reduce()?;
		drop(file_guard);

		file = get_file(path, flags, mode)?;

		resolve_count += 1;
		if resolve_count > limits::SYMLOOP_MAX {
			return Err(errno::ELOOP);
		}
	}

	Ok(file)
}

/// The implementation of the `open` syscall.
pub fn open(regs: &util::Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let flags = regs.ecx;
	let mode = regs.edx as u16;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	let len = proc.get_mem_space().can_access_string(pathname as _, true, false);
	if len.is_none() {
		return Err(errno::EFAULT);
	}
	let len = len.unwrap();
	if len > limits::PATH_MAX {
		return Err(errno::ENAMETOOLONG);
	}

	let path_str = unsafe {
		util::ptr_to_str(pathname as _)
	};

	let mode = mode & !proc.get_umask();
	let mut file = get_file(get_file_absolute_path(&proc, path_str)?, flags, mode)?;
	if flags & O_NOFOLLOW == 0 {
		file = resolve_links(file, flags, mode)?;
	}

	let fd = proc.open_file(file)?;
	Ok(fd.get_id() as _)
}
