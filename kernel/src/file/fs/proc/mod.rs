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

use super::{DummyOps, Filesystem, FilesystemOps, FilesystemType, NodeOps};
use crate::{
	device::BlkDev,
	file::{
		DirContext, DirEntry, FileType, Mode, Stat,
		fs::{
			Statfs,
			kernfs::{
				EitherOps, StaticDir, StaticEntry, StaticLink, box_file, box_node, static_dir_stat,
			},
			proc::proc_dir::{environ::Environ, maps::Maps},
		},
		perm::{Gid, Uid},
		vfs,
		vfs::node::Node,
	},
	process::{PROCESSES, Process, pid::Pid},
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
		.as_ref()
		.map(|proc| {
			let fs = proc.fs.lock();
			(fs.ap.euid, fs.ap.egid)
		})
		.unwrap_or((0, 0))
}

/// Returns the status of a file in a process's directory.
fn proc_file_stat(pid: Pid, mode: Mode) -> Stat {
	let (uid, gid) = get_proc_owner(pid);
	Stat {
		mode,
		uid,
		gid,
		..Default::default()
	}
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
			StaticEntry {
				name: b"meminfo",
				stat: |_| Stat {
					mode: FileType::Regular.to_mode() | 0o444,
					..Default::default()
				},
				init: EitherOps::File(|_| box_file(MemInfo)),
			},
			StaticEntry {
				name: b"mounts",
				stat: |_| Stat {
					mode: FileType::Link.to_mode() | 0o777,
					..Default::default()
				},
				init: EitherOps::Node(|_| box_node(StaticLink(b"self/mounts"))),
			},
			StaticEntry {
				name: b"self",
				stat: |_| Stat {
					mode: FileType::Link.to_mode() | 0o777,
					..Default::default()
				},
				init: EitherOps::Node(|_| box_node(SelfNode)),
			},
			StaticEntry {
				name: b"sys",
				stat: |_| static_dir_stat(),
				init: EitherOps::Node(|_| {
					box_node(StaticDir {
						entries: &[(StaticEntry {
							name: b"kernel",
							stat: |_| static_dir_stat(),
							init: EitherOps::Node(|_| {
								box_node(StaticDir {
									entries: &[StaticEntry {
										name: b"osrelease",
										stat: |_| static_dir_stat(),
										init: EitherOps::File(|_| box_file(OsRelease)),
									}],
									data: (),
								})
							}),
						})],
						data: (),
					})
				}),
			},
			StaticEntry {
				name: b"uptime",
				stat: |_| Stat {
					mode: FileType::Regular.to_mode() | 0o444,
					..Default::default()
				},
				init: EitherOps::File(|_| box_file(Uptime)),
			},
			StaticEntry {
				name: b"version",
				stat: |_| Stat {
					mode: FileType::Regular.to_mode() | 0o444,
					..Default::default()
				},
				init: EitherOps::File(|_| box_file(Version)),
			},
		],
		data: (),
	};

	/// Returns the directory's status.
	#[inline]
	fn stat() -> Stat {
		Stat {
			mode: FileType::Directory.to_mode() | 0o555,
			..Default::default()
		}
	}
}

impl NodeOps for RootDir {
	fn lookup_entry<'n>(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let pid = core::str::from_utf8(&ent.name)
			.ok()
			.and_then(|s| s.parse().ok());
		let Some(pid) = pid else {
			return Self::STATIC.lookup_entry(dir, ent);
		};
		ent.node = Process::get_by_pid(pid)
			.map(|_| {
				Arc::new(Node::new(
					0,
					dir.fs.clone(),
					static_dir_stat(),
					Box::new(StaticDir {
						entries: &[
							StaticEntry {
								name: b"cmdline",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(Cmdline(pid))),
							},
							StaticEntry {
								name: b"cwd",
								stat: |pid| proc_file_stat(pid, FileType::Link.to_mode() | 0o777),
								init: EitherOps::Node(|pid| box_node(Cwd(pid))),
							},
							StaticEntry {
								name: b"environ",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(Environ(pid))),
							},
							StaticEntry {
								name: b"exe",
								stat: |pid| proc_file_stat(pid, FileType::Link.to_mode() | 0o444),
								init: EitherOps::Node(|pid| box_node(Exe(pid))),
							},
							StaticEntry {
								name: b"maps",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(Maps(pid))),
							},
							StaticEntry {
								name: b"mounts",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(Mounts(pid))),
							},
							StaticEntry {
								name: b"stat",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(StatNode(pid))),
							},
							StaticEntry {
								name: b"status",
								stat: |pid| {
									proc_file_stat(pid, FileType::Regular.to_mode() | 0o400)
								},
								init: EitherOps::File(|pid| box_file(Status(pid))),
							},
						],
						data: pid,
					})?,
					Box::new(DummyOps)?,
				))
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, _dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EINVAL))?;
		// Iterate on static entries
		let static_iter = Self::STATIC.entries.iter().skip(off);
		for e in static_iter {
			let stat = (e.stat)(());
			let ent = DirEntry {
				inode: 0,
				entry_type: stat.get_type(),
				name: e.name,
			};
			if !(ctx.write)(&ent)? {
				return Ok(());
			}
			ctx.off += 1;
		}
		// Iterate on processes
		let off = ctx.off as usize - Self::STATIC.entries.len();
		let processes = PROCESSES.read();
		for (pid, _) in processes.iter().skip(off) {
			let name = format!("{pid}")?;
			let ent = DirEntry {
				inode: 0,
				entry_type: Some(FileType::Directory),
				name: &name,
			};
			if !(ctx.write)(&ent)? {
				return Ok(());
			}
			ctx.off += 1;
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

	fn cache_entries(&self) -> bool {
		false
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

	fn root(&self, fs: &Arc<Filesystem>) -> EResult<Arc<Node>> {
		Ok(Arc::new(Node::new(
			0,
			fs.clone(),
			RootDir::stat(),
			Box::new(RootDir)?,
			Box::new(DummyOps)?,
		))?)
	}

	fn create_node(&self, _fs: &Arc<Filesystem>, _stat: Stat) -> EResult<Arc<Node>> {
		Err(errno!(EINVAL))
	}

	fn destroy_node(&self, _node: &Node) -> EResult<()> {
		Ok(())
	}
}

/// The proc filesystem type.
pub struct ProcFsType;

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &'static [u8] {
		b"procfs"
	}

	fn detect(&self, _dev: &Arc<BlkDev>) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_dev: Option<Arc<BlkDev>>,
		_mountpath: PathBuf,
		mount_flags: u32,
	) -> EResult<Arc<Filesystem>> {
		Ok(Filesystem::new(0, Box::new(ProcFS)?, mount_flags)?)
	}
}
