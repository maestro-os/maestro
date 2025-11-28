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

//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem.
//!
//! The files are stored on the kernel's memory and thus are removed when the
//! filesystem is unmounted.

// TODO count memory usage to enforce quota

use crate::{
	device::BlkDev,
	file::{
		DirContext, DirEntry, File, FileType, Stat,
		fs::{
			DummyOps, FileOps, Filesystem, FilesystemOps, FilesystemType, NodeOps, Statfs,
			create_file_ids, downcast_fs, generic_file_read, generic_file_write,
		},
		perm::{ROOT_GID, ROOT_UID},
		vfs,
		vfs::{CachePolicy, RENAME_NOREPLACE, node::Node},
	},
	memory::{
		cache::RcPage,
		user::{UserSlice, UserString},
	},
	sync::{atomic::AtomicU64, mutex::Mutex},
	time::clock::{Clock, current_time_sec},
};
use core::{any::Any, ffi::c_int, hint::unlikely, sync::atomic::Ordering::Release};
use utils::{
	boxed::Box,
	collections::{path::PathBuf, string::String, vec::Vec},
	errno,
	errno::EResult,
	limits::{NAME_MAX, PAGE_SIZE, SYMLINK_MAX},
	ptr::arc::Arc,
};

// TODO use rwlock
#[derive(Debug, Default)]
struct RegularContent(Mutex<Vec<RcPage>, false>);

impl NodeOps for RegularContent {
	fn read_page(&self, _node: &Arc<Node>, off: u64) -> EResult<RcPage> {
		let i: usize = off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		self.0.lock().get(i).cloned().ok_or_else(|| errno!(EINVAL))
	}
}

#[derive(Debug, Default)]
struct TmpFileOps;

impl FileOps for TmpFileOps {
	fn read(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		generic_file_read(file, off, buf)
	}

	fn write(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		generic_file_write(file, off, buf)
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node = file.node();
		let content = (node.node_ops.as_ref() as &dyn Any)
			.downcast_ref::<RegularContent>()
			.unwrap();
		// Validation
		let size: usize = size.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let new_pages_count = size.div_ceil(PAGE_SIZE);
		let mut pages = content.0.lock();
		// Allocate or free pages
		if let Some(count) = new_pages_count.checked_sub(pages.len()) {
			pages.reserve(count)?;
			for _ in 0..count {
				pages.push(RcPage::new_zeroed()?)?;
			}
		} else {
			pages.truncate(new_pages_count);
			// Zero the last page
			if let Some(page) = pages.last() {
				let inner_off = size % PAGE_SIZE;
				let slice = unsafe { page.slice_mut() };
				slice[inner_off..].fill(0);
			}
			// Clear cache
			node.mapped.truncate(new_pages_count as _);
		}
		// Update status
		node.stat.lock().size = size as _;
		Ok(())
	}
}

#[derive(Debug, Default)]
struct DirectoryContent;

impl NodeOps for DirectoryContent {
	fn lookup_entry(&self, _dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		// Since this is called only if the entry is not in cache, we can just make the entry
		// negative
		ent.node = None;
		Ok(())
	}

	fn iter_entries(&self, dir: &vfs::Entry, ctx: &mut DirContext) -> EResult<()> {
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let children = dir.children.lock();
		let iter = children.iter().filter_map(|ent| {
			let ent = ent.inner();
			let node = ent.node.as_ref()?;
			Some(DirEntry {
				inode: node.inode,
				entry_type: node.stat.lock().get_type(),
				name: ent.name.as_ref(),
			})
		});
		let iter = [
			DirEntry {
				inode: dir.node().inode,
				entry_type: Some(FileType::Directory),
				name: b".",
			},
			DirEntry {
				inode: dir.parent.as_deref().unwrap_or(dir).node().inode,
				entry_type: Some(FileType::Directory),
				name: b"..",
			},
		]
		.into_iter()
		.chain(iter)
		.skip(off);
		for ent in iter {
			if !(*ctx.write)(&ent)? {
				break;
			}
			ctx.off += 1;
		}
		Ok(())
	}

	fn create(&self, parent: &Node, ent: &mut vfs::Entry, stat: Stat) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		// Create inode
		let (uid, gid) = create_file_ids(&parent.stat());
		let ts = current_time_sec(Clock::Realtime);
		let file_type = stat.get_type();
		let (node_ops, file_ops): (Box<dyn NodeOps>, Box<dyn FileOps>) = match file_type {
			Some(FileType::Regular) => {
				(Box::new(RegularContent::default())?, Box::new(TmpFileOps)?)
			}
			Some(FileType::Directory) => (Box::new(DirectoryContent)?, Box::new(DummyOps)?),
			_ => (Box::new(DummyOps)?, Box::new(DummyOps)?),
		};
		let inode = fs.next_inode.fetch_add(1, Release);
		// Count `.` and `..` for directories
		let nlink = if file_type == Some(FileType::Directory) {
			2
		} else {
			1
		};
		let node = Arc::new(Node::new(
			inode,
			parent.fs.clone(),
			Stat {
				nlink,
				uid,
				gid,
				ctime: ts,
				mtime: ts,
				atime: ts,
				..stat
			},
			node_ops,
			file_ops,
		))?;
		ent.node = Some(node);
		// Add reference for `..`
		if file_type == Some(FileType::Directory) {
			parent.stat.lock().nlink += 1;
		}
		Ok(())
	}

	fn link(&self, parent: Arc<Node>, ent: &vfs::Entry) -> EResult<()> {
		if ent.stat().get_type() == Some(FileType::Directory) {
			parent.stat.lock().nlink += 1;
		}
		ent.node().stat.lock().nlink += 1;
		Ok(())
	}

	fn symlink(
		&self,
		parent: &Arc<Node>,
		ent: &mut vfs::Entry,
		target: UserString,
	) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		// Read target
		let target = target.copy_path_from_user()?;
		if unlikely(target.len() > SYMLINK_MAX) {
			return Err(errno!(ENAMETOOLONG));
		}
		// Create inode
		let (uid, gid) = create_file_ids(&parent.stat());
		let ts = current_time_sec(Clock::Realtime);
		let inode = fs.next_inode.fetch_add(1, Release);
		let node = Arc::new(Node::new(
			inode,
			parent.fs.clone(),
			Stat {
				mode: FileType::Link.to_mode() | 0o777,
				nlink: 1,
				uid,
				gid,
				size: target.len() as _,
				ctime: ts,
				mtime: ts,
				atime: ts,
				..Default::default()
			},
			Box::new(LinkContent(String::from(target).into_bytes()))?,
			Box::new(DummyOps)?,
		))?;
		ent.node = Some(node);
		Ok(())
	}

	fn unlink(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let node = ent.node();
		if node.get_type() == Some(FileType::Directory) {
			if !ent.children.lock().is_empty() {
				return Err(errno!(ENOTEMPTY));
			}
			parent.stat.lock().nlink -= 1;
		}
		node.stat.lock().nlink -= 1;
		Ok(())
	}

	// TODO implement RENAME_EXCHANGE
	fn rename(
		&self,
		old_parent: &vfs::Entry,
		old_entry: &vfs::Entry,
		new_parent: &vfs::Entry,
		new_entry: &vfs::Entry,
		flags: c_int,
	) -> EResult<()> {
		let old_parent_node = old_parent.node();
		let new_parent_node = new_parent.node();
		let fs = downcast_fs::<TmpFS>(&*old_parent_node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let new_parent_ops = NodeContent::from_ops(&*new_parent_node.node_ops);
		let NodeContent::Directory(new_parent_inner) = new_parent_ops else {
			return Err(errno!(ENOTDIR));
		};
		// If source and destination parent are the same
		if old_parent_node.inode == new_parent_node.inode {
			// No need to check for cycles, hence no need to lock the rename mutex
			// TODO rename entry and return
		}
		// Prevent concurrent renames to safeguard cycle checking
		let _rename_guard = fs.rename_lock.lock();
		// Cannot make a directory a child of itself
		if unlikely(new_entry.is_child_of(old_entry)) {
			return Err(errno!(EINVAL));
		}
		let mut new_parent_inner = new_parent_inner.lock();
		let new_ent = new_parent_inner.find_entry_mut(new_name);
		if new_ent.is_some() && flags & RENAME_NOREPLACE != 0 {
			return Err(errno!(EEXIST));
		}
		// If the source and destination are the same node, do nothing
		let node = entry.node();
		if let Some(new_ent) = new_ent
			&& new_ent.node.inode == node.inode
		{
			return Ok(());
		}
		let node_ops = NodeContent::from_ops(&*node.node_ops);
		if let NodeContent::Directory(inner) = node_ops {
			// Update the `..` entry
			inner.lock().set_inode(b"..", new_parent_node.clone());
			// Update links count
			let mut new_parent_stat = new_parent_node.stat.lock();
			if unlikely(new_parent_stat.nlink == u16::MAX) {
				return Err(errno!(EMFILE));
			}
			new_parent_stat.nlink += 1;
		}
		// Insert or replace entry
		let tmpfs_ent = TmpfsDirEntry {
			name: Cow::Owned(new_name.try_to_owned()?),
			node: node.clone(),
		};
		if let Some(new_ent) = new_ent {
			*new_ent = tmpfs_ent;
			// TODO update links count on previous entry
		} else {
			new_parent_inner.insert(tmpfs_ent)?;
		}
		drop(new_parent_inner);
		let old_parent_ops = NodeContent::from_ops(&*old_parent_node.node_ops);
		let NodeContent::Directory(old_parent_inner) = old_parent_ops else {
			unreachable!();
		};
		// Remove old entry
		let old_parent = entry.parent.as_ref().unwrap();
		old_parent_inner.lock().remove(&entry.name);
		// Update links count
		if let NodeContent::Directory(_) = node_ops {
			let mut old_parent_stat = old_parent_node.stat.lock();
			old_parent_stat.nlink = old_parent_stat.nlink.saturating_sub(1);
		}
		Ok(())
	}

}

#[derive(Debug)]
struct LinkContent(Vec<u8>);

impl NodeOps for LinkContent {
	fn readlink(&self, _node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		buf.copy_to_user(0, &self.0)
	}
}

/// A temporary file system.
///
/// On the inside, the tmpfs works using a kernfs.
#[derive(Debug)]
pub struct TmpFS {
	/// Inode ID bump allocator
	next_inode: AtomicU64,
}

impl FilesystemOps for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn cache_policy(&self) -> CachePolicy {
		CachePolicy::Keep
	}

	fn get_stat(&self) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0,
			f_bsize: PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: 0,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: NAME_MAX as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn root(&self, fs: &Arc<Filesystem>) -> EResult<Arc<Node>> {
		let root = Arc::new(Node::new(
			1,
			fs.clone(),
			Stat {
				mode: FileType::Directory.to_mode() | 0o1777,
				nlink: 2, // `.` and `..`
				uid: ROOT_UID,
				gid: ROOT_GID,
				size: 0,
				blocks: 0,
				dev_major: 0,
				dev_minor: 0,
				ctime: 0,
				mtime: 0,
				atime: 0,
			},
			Box::new(DirectoryContent)?,
			Box::new(DummyOps)?,
		))?;
		Ok(root)
	}

	fn destroy_node(&self, _node: &Node) -> EResult<()> {
		Ok(())
	}
}

/// The tmpfs filesystem type.
pub struct TmpFsType;

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &'static [u8] {
		b"tmpfs"
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
		let fs = Filesystem::new(
			0,
			Box::new(TmpFS {
				next_inode: AtomicU64::new(2),
			})?,
			mount_flags,
		)?;
		Ok(fs)
	}
}
