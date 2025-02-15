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

//! The `procfs` is a virtual filesystem which provides information about
//! processes.

mod mem_info;
mod proc_dir;
mod self_link;
mod sys_dir;
mod uptime;
mod version;

use super::{FileOps, FilesystemOps, FilesystemType, NodeOps};
use crate::{
	device::DeviceIO,
	file::{
		fs::{
			kernfs::{
				box_wrap, entry_init_default, entry_init_from, StaticDir, StaticEntryBuilder,
				StaticLink,
			},
			proc::proc_dir::environ::Environ,
			Statfs,
		},
		perm::{Gid, Uid},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, File, FileType, Stat,
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
	boxed::Box, collections::path::PathBuf, errno, errno::EResult, format, ptr::arc::Arc,
};
use version::Version;

/// Returns the user ID and group ID of the process with the given PID.
///
/// If the process does not exist, the function returns `(0, 0)`.
fn get_proc_owner(pid: Pid) -> (Uid, Gid) {
	Process::get_by_pid(pid)
		.map(|proc| {
			let fs = proc.fs.lock();
			(fs.access_profile.euid, fs.access_profile.egid)
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
				init: |_| box_wrap(StaticLink(b"self/mounts")),
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

impl NodeOps for RootDir {
	fn get_stat(&self, _node: &Node) -> EResult<Stat> {
		Ok(Stat {
			mode: FileType::Directory.to_mode() | 0o555,
			..Default::default()
		})
	}

	fn lookup_entry<'n>(&self, _node: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let pid = core::str::from_utf8(&ent.name)
			.ok()
			.and_then(|s| s.parse().ok());
		let Some(pid) = pid else {
			return Self::STATIC.lookup_entry_inner(ent);
		};
		ent.node = Process::get_by_pid(pid)
			.map(|_| {
				Arc::new(Node {
					inode: 0,
					fs: Arc {},
					node_ops: Box::new(StaticDir {
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
								name: b"environ",
								entry_type: FileType::Regular,
								init: entry_init_from::<Environ, Pid>,
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
					})?,
					file_ops: (),
					pages: Default::default(),
				})
			})
			.transpose()?;
		Ok(())
	}
}

impl FileOps for RootDir {
	fn iter_entries(&self, _dir: &File, ctx: &mut DirContext) -> EResult<()> {
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EINVAL))?;
		// Iterate on static entries
		let static_iter = Self::STATIC.entries.iter().skip(off);
		for e in static_iter {
			let ent = DirEntry {
				inode: 0,
				entry_type: e.entry_type,
				name: e.name,
			};
			ctx.off += 1;
			if !(ctx.write)(&ent)? {
				return Ok(());
			}
		}
		// Iterate on processes
		let off = ctx.off as usize - Self::STATIC.entries.len();
		let sched = SCHEDULER.lock();
		let proc_iter = sched.iter_process().skip(off);
		for (pid, _) in proc_iter {
			let name = format!("{pid}")?;
			let ent = DirEntry {
				inode: 0,
				entry_type: FileType::Directory,
				name: &name,
			};
			ctx.off += 1;
			if !(ctx.write)(&ent)? {
				return Ok(());
			}
		}
		Ok(())
	}
}

/// A proc.
#[derive(Debug)]
pub struct ProcFS;

impl FilesystemOps for ProcFS {
	fn get_name(&self) -> &[u8] {
		b"proc"
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

	fn root(&self) -> EResult<Arc<Node>> {
		Ok(Box::new(RootDir)? as _)
	}

	fn create_node(&self, _stat: &Stat) -> EResult<Arc<Node>> {
		Err(errno!(EINVAL))
	}

	fn destroy_node(&self, _node: &Node) -> EResult<()> {
		Err(errno!(EINVAL))
	}
}

/// The proc filesystem type.
pub struct ProcFsType;

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &'static [u8] {
		b"procfs"
	}

	fn detect(&self, _io: &dyn DeviceIO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<dyn DeviceIO>>,
		_mountpath: PathBuf,
		_readonly: bool,
	) -> EResult<Box<dyn FilesystemOps>> {
		Ok(Box::new(ProcFS)?)
	}
}
