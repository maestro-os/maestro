/*
 * Copyright 2026 Luc Lenôtre
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

//! The `sysfs` is a virtual filesystem which exports information about devices and drivers to
//! userspace.
//!
//! Only the subset of the hierarchy required by userspace programs is implemented.

use super::{DummyOps, Filesystem, FilesystemOps, FilesystemType};
use crate::{
	device::BlkDev,
	file::{
		FileType, Stat,
		fs::{
			Statfs,
			kernfs::{EitherOps, StaticDir, StaticEntry, StaticLink, box_node, static_dir_stat},
		},
		vfs::node::Node,
	},
};
use utils::{boxed::Box, collections::path::PathBuf, errno, errno::EResult, ptr::arc::Arc};

/// Returns the status of a symbolic link.
#[inline]
fn link_stat(_: ()) -> Stat {
	Stat {
		mode: FileType::Link.to_mode() | 0o777,
		..Default::default()
	}
}

/// The root directory of the sysfs.
///
/// **Warning**: entries of each [`StaticDir`] must be kept sorted alphabetically by name.
const ROOT: StaticDir = StaticDir {
	entries: &[
		StaticEntry {
			name: b"bus",
			stat: |_| static_dir_stat(),
			init: EitherOps::Node(|_| {
				box_node(StaticDir {
					entries: &[StaticEntry {
						name: b"platform",
						stat: |_| static_dir_stat(),
						init: EitherOps::Node(|_| {
							box_node(StaticDir {
								entries: &[],
								data: (),
							})
						}),
					}],
					data: (),
				})
			}),
		},
		StaticEntry {
			name: b"class",
			stat: |_| static_dir_stat(),
			init: EitherOps::Node(|_| {
				box_node(StaticDir {
					entries: &[StaticEntry {
						name: b"graphics",
						stat: |_| static_dir_stat(),
						init: EitherOps::Node(|_| {
							box_node(StaticDir {
								entries: &[StaticEntry {
									name: b"fb0",
									stat: |_| static_dir_stat(),
									init: EitherOps::Node(|_| {
										box_node(StaticDir {
											entries: &[StaticEntry {
												name: b"device",
												stat: |_| static_dir_stat(),
												init: EitherOps::Node(|_| {
													box_node(StaticDir {
														entries: &[StaticEntry {
															name: b"subsystem",
															stat: link_stat,
															init: EitherOps::Node(|_| {
																box_node(StaticLink(
																	b"../../../../bus/platform",
																))
															}),
														}],
														data: (),
													})
												}),
											}],
											data: (),
										})
									}),
								}],
								data: (),
							})
						}),
					}],
					data: (),
				})
			}),
		},
	],
	data: (),
};

/// A sysfs.
#[derive(Debug)]
pub struct SysFS;

impl FilesystemOps for SysFS {
	fn get_name(&self) -> &[u8] {
		b"sysfs"
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
			static_dir_stat(),
			Box::new(ROOT)?,
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

/// The sysfs filesystem type.
pub struct SysFsType;

impl FilesystemType for SysFsType {
	fn get_name(&self) -> &'static [u8] {
		b"sysfs"
	}

	fn detect(&self, _dev: &Arc<BlkDev>) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_dev: Option<Arc<BlkDev>>,
		_mountpath: PathBuf,
		_readonly: bool,
	) -> EResult<Arc<Filesystem>> {
		Ok(Filesystem::new(0, Box::new(SysFS)?)?)
	}
}
