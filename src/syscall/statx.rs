//! The statx system call returns the extended status of a file.

use super::util;
use crate::errno::Errno;
use crate::file::mountpoint::MountSource;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::io::IO;
use core::ffi::c_int;
use core::ffi::c_uint;
use macros::syscall;

/// Structure representing a timestamp with the statx syscall.
#[repr(C)]
#[derive(Debug)]
struct StatxTimestamp {
	/// Seconds since the Epoch (UNIX time)
	tv_sec: i64,
	/// Nanoseconds since tv_sec
	tv_nsec: u32,
	/// Reserved field.
	__reserved: i32,
}

/// Structure containing the extended attributes for a file.
#[repr(C)]
#[derive(Debug)]
struct Statx {
	/// Mask of bits indicating filled fields
	stx_mask: u32,
	/// Block size for filesystem I/O
	stx_blksize: u32,
	/// Extra file attribute indicators
	stx_attributes: u64,
	/// Number of hard links
	stx_nlink: u32,
	/// User ID of owner
	stx_uid: u32,
	/// Group ID of owner
	stx_gid: u32,
	/// File type and mode
	stx_mode: u16,

	/// Padding.
	__padding0: u16,

	/// Inode number
	stx_ino: u64,
	/// Total size in bytes
	stx_size: u64,
	/// Number of 512B blocks allocated
	stx_blocks: u64,
	/// Mask to show what's supported in stx_attributes
	stx_attributes_mask: u64,

	/// Last access
	stx_atime: StatxTimestamp,
	/// Creation
	stx_btime: StatxTimestamp,
	/// Last status change
	stx_ctime: StatxTimestamp,
	/// Last modification
	stx_mtime: StatxTimestamp,

	/// Major ID (if the file is a device)
	stx_rdev_major: u32,
	/// Minor ID (if the file is a device)
	stx_rdev_minor: u32,
	/// Major ID of the device containing the filesystem where the file resides
	stx_dev_major: u32,
	/// Minor ID of the device containing the filesystem where the file resides
	stx_dev_minor: u32,

	/// Mount ID.
	stx_mnt_id: u64,

	/// Padding.
	__padding1: [u64; 13],
}

#[syscall]
pub fn statx(
	dirfd: c_int,
	pathname: SyscallString,
	flags: c_int,
	_mask: c_uint,
	statxbuff: SyscallPtr<Statx>,
) -> Result<i32, Errno> {
	if pathname.is_null() || statxbuff.is_null() {
		return Err(errno!(EINVAL));
	}

	// TODO Implement all flags

	// Whether symbolic links are to be followed
	let follow_links = flags & super::access::AT_SYMLINK_NOFOLLOW == 0;

	// Getting the file
	let file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		util::get_file_at(proc, follow_links, dirfd, pathname, flags)?
	};
	let file = file_mutex.lock();

	// TODO Use mask?

	// If the file is a device, get the major and minor numbers
	let (stx_rdev_major, stx_rdev_minor) = match file.get_content() {
		FileContent::BlockDevice {
			major,
			minor,
		}
		| FileContent::CharDevice {
			major,
			minor,
		} => (*major, *minor),
		_ => (0, 0),
	};

	// Getting the major and minor numbers of the device of the file's filesystem
	let (stx_dev_major, stx_dev_minor) = {
		if let Some(mountpoint_mutex) = file.get_location().get_mountpoint() {
			// TODO Clean: This is a quick fix to avoid a deadlock because vfs is also using
			// the mountpoint and locking vfs requires disabling interrupts
			crate::idt::wrap_disable_interrupts(|| {
				let mountpoint = mountpoint_mutex.lock();

				match mountpoint.get_source() {
					MountSource::Device {
						major,
						minor,
						..
					} => (*major, *minor),

					_ => (0, 0),
				}
			})
		} else {
			(0, 0)
		}
	};

	let inode = file.get_location().get_inode();

	// Filling the structure
	let statx_val = Statx {
		stx_mask: !0,      // TODO
		stx_blksize: 512,  // TODO
		stx_attributes: 0, // TODO
		stx_nlink: file.get_hard_links_count() as _,
		stx_uid: file.get_uid() as _,
		stx_gid: file.get_gid() as _,
		stx_mode: file.get_mode() as _,

		__padding0: 0,

		stx_ino: inode,
		stx_size: file.get_size(),
		stx_blocks: file.get_blocks_count(),
		stx_attributes_mask: 0, // TODO

		stx_atime: StatxTimestamp {
			tv_sec: file.atime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_btime: StatxTimestamp {
			tv_sec: 0,  // TODO
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_ctime: StatxTimestamp {
			tv_sec: file.ctime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_mtime: StatxTimestamp {
			tv_sec: file.mtime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},

		stx_rdev_major,
		stx_rdev_minor,
		stx_dev_major,
		stx_dev_minor,

		stx_mnt_id: 0, // TODO

		__padding1: [0; 13],
	};

	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let statx = statxbuff
			.get_mut(&mut mem_space_guard)?
			.ok_or(errno!(EFAULT))?;
		*statx = statx_val;
	}

	Ok(0)
}
