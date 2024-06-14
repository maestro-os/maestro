/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The proc is a virtual filesystem which provides information about
//! processes.

mod mem_info;
mod proc_dir;
mod self_link;
mod sys_dir;
mod uptime;
mod version;

use super::{kernfs, Filesystem, FilesystemType, NodeOps};
use crate::{
	file::{
		fs::{
			kernfs::{
				box_wrap, entry_init_default, entry_init_from, OwnedNode, StaticDir,
				StaticEntryBuilder, StaticLink,
			},
			Statfs,
		},
		path::PathBuf,
		perm::{Gid, Uid},
		DirEntry, FileType, INode, Stat,
	},
	process::{pid::Pid, scheduler::SCHEDULER, Process},
};
use mem_info::MemInfo;
use proc_dir::{
	cmdline::Cmdline, cwd::Cwd, exe::Exe, mounts::Mounts, stat::StatNode, status::Status,
};
use self_link::SelfNode;
use sys_dir::OsRelease;
use uptime::Uptime;
use utils::{
	boxed::Box,
	errno,
	errno::{AllocResult, EResult},
	format,
	io::IO,
	lock::Mutex,
	ptr::{arc::Arc, cow::Cow},
};
use version::Version;

/// Returns the user ID and group ID of the process with the given PID.
///
/// If the process does not exist, the function returns `(0, 0)`.
fn get_proc_owner(pid: Pid) -> (Uid, Gid) {
	Process::get_by_pid(pid)
		.map(|proc_mutex| {
			let proc = proc_mutex.lock();
			let uid = proc.access_profile.get_euid();
			let gid = proc.access_profile.get_egid();
			(uid, gid)
		})
		.unwrap_or((0, 0))
}

/// The root directory of the proc.
#[derive(Clone, Debug)]
struct RootDir;

impl RootDir {
	// Entries offsets: The first `Pid::MAX` offsets are reserved for processes. Static entries are
	// located right after
	/// Static entries of the root directory, as opposed to the dynamic ones that represent
	/// processes.
	const STATIC: StaticDir = StaticDir {
		entries: &[
			StaticEntryBuilder {
				name: b"meminfo",
				entry_type: FileType::Regular,
				init: entry_init_default::<MemInfo>,
			},
			StaticEntryBuilder {
				name: b"mounts",
				entry_type: FileType::Link,
				init: entry_init_default::<StaticLink<b"self/mounts">>,
			},
			StaticEntryBuilder {
				name: b"self",
				entry_type: FileType::Link,
				init: entry_init_default::<SelfNode>,
			},
			StaticEntryBuilder {
				name: b"sys",
				entry_type: FileType::Directory,
				init: |_| {
					box_wrap(StaticDir {
						entries: &[(StaticEntryBuilder {
							name: b"kernel",
							entry_type: FileType::Directory,
							init: |_| {
								box_wrap(StaticDir {
									entries: &[StaticEntryBuilder {
										name: b"osrelease",
										entry_type: FileType::Regular,
										init: entry_init_default::<OsRelease>,
									}],
									data: (),
								})
							},
						})],
						data: (),
					})
				},
			},
			StaticEntryBuilder {
				name: b"uptime",
				entry_type: FileType::Regular,
				init: entry_init_default::<Uptime>,
			},
			StaticEntryBuilder {
				name: b"version",
				entry_type: FileType::Regular,
				init: entry_init_default::<Version>,
			},
		],
		data: (),
	};
}

impl OwnedNode for RootDir {
	fn detached(&self) -> AllocResult<Box<dyn NodeOps>> {
		Ok(Box::new(self.clone())? as _)
	}
}

impl NodeOps for RootDir {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Directory,
			mode: 0o555,
			..Default::default()
		})
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let pid = core::str::from_utf8(name).ok().and_then(|s| s.parse().ok());
		if let Some(pid) = pid {
			// Check the process exists
			if Process::get_by_pid(pid).is_some() {
				// Return the entry for the process
				Ok(Some((
					DirEntry {
						inode: 0,
						entry_type: FileType::Directory,
						name: Cow::Borrowed(name),
					},
					Box::new(StaticDir {
						entries: &[
							StaticEntryBuilder {
								name: b"cmdline",
								entry_type: FileType::Regular,
								init: entry_init_from::<Cmdline, Pid>,
							},
							StaticEntryBuilder {
								name: b"cwd",
								entry_type: FileType::Regular,
								init: entry_init_from::<Cwd, Pid>,
							},
							StaticEntryBuilder {
								name: b"exe",
								entry_type: FileType::Regular,
								init: entry_init_from::<Exe, Pid>,
							},
							StaticEntryBuilder {
								name: b"mounts",
								entry_type: FileType::Regular,
								init: entry_init_from::<Mounts, Pid>,
							},
							StaticEntryBuilder {
								name: b"stat",
								entry_type: FileType::Regular,
								init: entry_init_from::<StatNode, Pid>,
							},
							StaticEntryBuilder {
								name: b"status",
								entry_type: FileType::Regular,
								init: entry_init_from::<Status, Pid>,
							},
						],
						data: pid,
					})? as _,
				)))
			} else {
				Ok(None)
			}
		} else {
			Self::STATIC.entry_by_name_inner(name)
		}
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		// Iterate on processes
		if off < Pid::MAX as usize {
			// Find next process
			let sched = SCHEDULER.get().lock();
			// TODO start iterating at `off`
			let pid = sched
				.iter_process()
				.map(|(pid, _)| pid)
				.find(|pid| **pid >= off as Pid);
			if let Some(pid) = pid {
				return Ok(Some((
					DirEntry {
						inode: 0,
						entry_type: FileType::Directory,
						name: Cow::Owned(format!("{pid}")?),
					},
					*pid as u64 + 1,
				)));
			}
		}
		// No process left, go to static entries
		let off = off.saturating_sub(Pid::MAX as usize);
		let ent = Self::STATIC.next_entry_inner(off as _)?;
		Ok(ent.map(|(ent, next)| (ent, next + Pid::MAX as u64)))
	}
}

/// A proc.
#[derive(Debug)]
pub struct ProcFS;

impl Filesystem for ProcFS {
	fn get_name(&self) -> &[u8] {
		b"proc"
	}

	fn is_readonly(&self) -> bool {
		true
	}

	fn use_cache(&self) -> bool {
		false
	}

	fn get_root_inode(&self) -> INode {
		kernfs::ROOT_INODE
	}

	fn get_stat(&self) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0,
			f_bsize: 0,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: 0,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: 0,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn node_from_inode(&self, inode: INode) -> EResult<Box<dyn NodeOps>> {
		if inode == kernfs::ROOT_INODE {
			Ok(Box::new(RootDir)? as _)
		} else {
			Err(errno!(ENOENT))
		}
	}
}

/// The proc filesystem type.
pub struct ProcFsType {}

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &'static [u8] {
		b"procfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<Mutex<dyn IO>>>,
		_mountpath: PathBuf,
		_readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		Ok(Arc::new(ProcFS)?)
	}
}
