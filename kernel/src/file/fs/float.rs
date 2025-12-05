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

//! floatfs is a filesystem for "floating" files. Those are pipes and sockets created by system
//! calls, without any link to regular filesystems.

use crate::{
	file::{
		FileType, Stat,
		fs::{DummyOps, FileOps, Filesystem, FilesystemOps, Statfs},
		vfs,
		vfs::node::Node,
	},
	sync::once::OnceInit,
};
use utils::{boxed::Box, collections::string::String, errno, errno::EResult, ptr::arc::Arc};

/// Float filesystem
#[derive(Debug)]
pub struct FloatFs {
	_private: (),
}

impl FilesystemOps for FloatFs {
	fn get_name(&self) -> &[u8] {
		b"floatfs"
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

	fn root(&self, _fs: &Arc<Filesystem>) -> EResult<Arc<Node>> {
		Err(errno!(EINVAL))
	}

	fn destroy_node(&self, _node: &Node) -> EResult<()> {
		Err(errno!(EINVAL))
	}
}

/// The floatfs
static FLOAT_FS: OnceInit<Arc<Filesystem>> = unsafe { OnceInit::new() };

/// Initializes the floating filesystem
pub(crate) fn init() -> EResult<()> {
	let fs = Arc::new(Filesystem {
		dev: 0,
		ops: Box::new(FloatFs {
			_private: (),
		})?,
		flags: 0,

		nodes: Default::default(),
		buffers: Default::default(),
	})?;
	unsafe {
		OnceInit::init(&FLOAT_FS, fs);
	}
	Ok(())
}

/// Returns a VFS entry for a floating file
pub fn get_entry<O: FileOps>(ops: O, file_type: FileType) -> EResult<Arc<vfs::Entry>> {
	let node = Arc::new(Node::new(
		1,
		FLOAT_FS.clone(),
		Stat {
			mode: file_type.to_mode() | 0o666,
			..Default::default()
		},
		Box::new(DummyOps)?,
		Box::new(ops)?,
	))?;
	let ent = Arc::new(vfs::Entry::new(String::new(), None, Some(node)))?;
	Ok(ent)
}
