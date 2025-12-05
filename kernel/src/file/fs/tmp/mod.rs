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
			create_file_ids, downcast_fs, generic_file_read, generic_file_write, kernfs,
			kernfs::NodeStorage,
		},
		perm::{ROOT_GID, ROOT_UID},
		vfs,
		vfs::{Entry, RENAME_EXCHANGE, node::Node},
	},
	memory::{
		cache::{FrameOwner, RcFrame},
		user::{UserSlice, UserString},
	},
	sync::mutex::Mutex,
	time::clock::{Clock, current_time_sec},
};
use core::{any::Any, ffi::c_int, hint::unlikely, mem};
use utils::{
	TryClone, TryToOwned,
	boxed::Box,
	collections::{path::PathBuf, string::String, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	limits::{NAME_MAX, PAGE_SIZE, SYMLINK_MAX},
	ptr::{arc::Arc, cow::Cow},
};

// TODO use rwlock
#[derive(Debug, Default)]
struct RegularContent(Mutex<Vec<RcFrame>, false>);

impl FileOps for RegularContent {
	fn read(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		generic_file_read(file, off, buf)
	}

	fn write(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		generic_file_write(file, off, buf)
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node = file.node();
		// Validation
		let size: usize = size.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let new_pages_count = size.div_ceil(PAGE_SIZE);
		let mut pages = self.0.lock();
		// Allocate or free pages
		if let Some(count) = new_pages_count.checked_sub(pages.len()) {
			pages.reserve(count)?;
			for _ in 0..count {
				// The offset is not necessary since `writeback` is a no-op
				let frame = RcFrame::new_zeroed(0, FrameOwner::Node(node.clone()), 0)?;
				pages.push(frame)?;
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

impl NodeOps for RegularContent {
	fn read_page(&self, _node: &Arc<Node>, off: u64) -> EResult<RcFrame> {
		let i: usize = off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		self.0.lock().get(i).cloned().ok_or_else(|| errno!(EINVAL))
	}

	fn write_frame(&self, _node: &Node, _frame: &RcFrame) -> EResult<()> {
		Ok(())
	}
}

#[derive(Debug)]
struct TmpfsDirEntry {
	name: Cow<'static, [u8]>,
	node: Arc<Node>,
}

#[derive(Debug, Default)]
struct DirInner {
	// Using a `Vec` with holes is necessary for `iter_entries` so we can keep the same offsets
	// for entries in between each calls
	entries: Vec<Option<TmpfsDirEntry>>,
	used_slots: usize,
	// TODO: add a structure to map entry names to slots in `entries`
}

impl DirInner {
	/// Returns the node of the entry with the given `name`.
	///
	/// If no such entry exist, the function returns `None`.
	fn find(&self, name: &[u8]) -> Option<&Arc<Node>> {
		self.entries
			.iter()
			.filter_map(Option::as_ref)
			.find(|e| e.name.as_ref() == name)
			.map(|e| &e.node)
	}

	/// Same as [`Self::find`], but mutable.
	fn find_entry_mut(&mut self, name: &[u8]) -> Option<&mut TmpfsDirEntry> {
		self.entries
			.iter_mut()
			.filter_map(Option::as_mut)
			.find(|e| e.name.as_ref() == name)
	}

	/// Inserts a new entry.
	///
	/// The function returns a reference to the inserted entry.
	fn insert(&mut self, ent: TmpfsDirEntry) -> AllocResult<&mut TmpfsDirEntry> {
		let slot = if self.used_slots == self.entries.len() {
			self.entries.push(Some(ent))?;
			self.entries.last_mut().unwrap().as_mut().unwrap()
		} else {
			let slot = self.entries.iter_mut().find(|e| e.is_none()).unwrap();
			slot.insert(ent)
		};
		self.used_slots += 1;
		Ok(slot)
	}

	/// Changes the node the entry with name `name` points to.
	///
	/// If no such entry exist, the function does nothing.
	fn set_inode(&mut self, name: &[u8], node: Arc<Node>) {
		let ent = self
			.entries
			.iter_mut()
			.filter_map(|e| e.as_mut())
			.find(|e| e.name.as_ref() == name);
		if let Some(ent) = ent {
			ent.node = node;
		}
	}

	/// Removes the entry with name `name`, if any.
	fn remove(&mut self, name: &[u8]) {
		let slots_count = self.entries.len();
		let slot = self
			.entries
			.iter_mut()
			.enumerate()
			.find(|(_, e)| matches!(e, Some(e) if e.name.as_ref() == name));
		let Some((index, slot)) = slot else {
			return;
		};
		if index == slots_count - 1 {
			self.entries.truncate(index);
		} else {
			*slot = None;
		}
		self.used_slots -= 1;
	}
}

// TODO use rwlock
#[derive(Debug, Default)]
struct DirectoryContent(Mutex<DirInner, false>);

impl DirectoryContent {
	/// Turns `ops` into a [`DirectoryContent`] if the node is a directory.
	#[inline]
	fn from_ops(ops: &dyn NodeOps) -> Option<&DirectoryContent> {
		(ops as &dyn Any).downcast_ref()
	}
}

impl NodeOps for DirectoryContent {
	fn lookup_entry(&self, _dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		ent.node = self.0.lock().find(ent.name.as_ref()).cloned();
		Ok(())
	}

	fn iter_entries(&self, _dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let off: usize = ctx.off.try_into().map_err(|_| errno!(EOVERFLOW))?;
		let inner = self.0.lock();
		let iter = inner.entries.iter().skip(off).filter_map(|e| e.as_ref());
		for e in iter {
			let ent = DirEntry {
				inode: e.node.inode,
				entry_type: e.node.stat.lock().get_type(),
				name: e.name.as_ref(),
			};
			if !(*ctx.write)(&ent)? {
				break;
			}
			ctx.off += 1;
		}
		Ok(())
	}

	fn create(&self, parent: &Node, ent: &mut Entry, stat: Stat) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		// Create inode
		let (uid, gid) = create_file_ids(&parent.stat());
		let ts = current_time_sec(Clock::Realtime);
		let (node_ops, file_ops): (Box<dyn NodeOps>, Box<dyn FileOps>) = match stat.get_type() {
			Some(FileType::Regular) => (
				Box::new(RegularContent::default())?,
				Box::new(RegularContent::default())?,
			),
			Some(FileType::Directory) => {
				(Box::new(DirectoryContent::default())?, Box::new(DummyOps)?)
			}
			_ => (Box::new(DummyOps)?, Box::new(DummyOps)?),
		};
		let node = {
			let mut nodes = fs.nodes.lock();
			let (inode, slot) = nodes.get_free_slot()?;
			let node = Arc::new(Node::new(
				inode,
				parent.fs.clone(),
				Stat {
					nlink: 1,
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
			*slot = Some(node.clone());
			node
		};
		// Check if an entry already exists
		let mut parent_inner = self.0.lock();
		if unlikely(parent_inner.find(ent.name.as_ref()).is_some()) {
			// TODO must undo
			return Err(errno!(EEXIST));
		}
		// Insert directory entry
		parent_inner.insert(TmpfsDirEntry {
			name: Cow::Owned(ent.name.try_clone()?),
			node: node.clone(),
		})?; // TODO on failure, must undo
		Ok(())
	}

	fn link(&self, parent: Arc<Node>, ent: &vfs::Entry) -> EResult<()> {
		let mut parent_inner = self.0.lock();
		// Check if an entry already exists
		if parent_inner.find(ent.name.as_ref()).is_some() {
			return Err(errno!(EEXIST));
		}
		// If this is a directory, create `.` and `..`
		let node = ent.node();
		if let Some(dir_inner) = DirectoryContent::from_ops(&*node.node_ops) {
			let mut dir_inner = dir_inner.0.lock();
			dir_inner.insert(TmpfsDirEntry {
				name: Cow::Borrowed(b"."),
				node: node.clone(),
			})?;
			dir_inner.insert(TmpfsDirEntry {
				name: Cow::Borrowed(b".."),
				node: parent.clone(),
			})?;
			// Update links count
			node.stat.lock().nlink += 1;
			parent.stat.lock().nlink += 1;
		}
		parent_inner.insert(TmpfsDirEntry {
			name: Cow::Owned(ent.name.try_clone()?),
			node: node.clone(),
		})?;
		node.stat.lock().nlink += 1;
		Ok(())
	}

	fn symlink(&self, parent: &Arc<Node>, ent: &mut Entry, target: UserString) -> EResult<()> {
		let fs = downcast_fs::<TmpFS>(&*parent.fs.ops);
		// Read target
		let target = target.copy_path_from_user()?;
		if unlikely(target.len() > SYMLINK_MAX) {
			return Err(errno!(ENAMETOOLONG));
		}
		// Create inode
		let (uid, gid) = create_file_ids(&parent.stat());
		let ts = current_time_sec(Clock::Realtime);
		let node = {
			let mut nodes = fs.nodes.lock();
			let (inode, slot) = nodes.get_free_slot()?;
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
			*slot = Some(node.clone());
			node
		};
		// Check if an entry already exists
		let mut parent_inner = self.0.lock();
		if unlikely(parent_inner.find(ent.name.as_ref()).is_some()) {
			// TODO must undo
			return Err(errno!(EEXIST));
		}
		// Insert directory entry
		parent_inner.insert(TmpfsDirEntry {
			name: Cow::Owned(ent.name.try_clone()?),
			node: node.clone(),
		})?; // TODO on failure, must undo
		ent.node = Some(node);
		Ok(())
	}

	fn unlink(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let mut parent_inner = self.0.lock();
		// Find entry
		let node = parent_inner
			.find(ent.name.as_ref())
			.ok_or_else(|| errno!(ENOENT))?;
		// Handle directory-specifics
		let content = DirectoryContent::from_ops(&*node.node_ops);
		if let Some(dir_inner) = content {
			// If not empty, error
			let mut dir_inner = dir_inner.0.lock();
			let not_empty = dir_inner.used_slots > 2
				|| dir_inner
					.entries
					.iter()
					.filter_map(|e| e.as_ref())
					.any(|e| !matches!(e.name.as_ref(), b"." | b".."));
			if not_empty {
				return Err(errno!(ENOTEMPTY));
			}
			// Remove `.` and `..` to break cycles
			dir_inner.entries.clear();
			// Decrement references count
			node.stat.lock().nlink -= 1;
			parent.stat.lock().nlink -= 1;
		}
		// Remove
		node.stat.lock().nlink -= 1;
		parent_inner.remove(ent.name.as_ref());
		Ok(())
	}

	fn rename(
		&self,
		old_parent: &vfs::Entry,
		old_entry: &vfs::Entry,
		new_parent: &vfs::Entry,
		new_entry: &vfs::Entry,
		flags: c_int,
	) -> EResult<()> {
		let old_node = old_entry.node();
		let old_parent_node = old_parent.node();
		let new_parent_node = new_parent.node();
		let Some(new_parent_inner) = DirectoryContent::from_ops(&*new_parent_node.node_ops) else {
			return Err(errno!(ENOTDIR));
		};
		// If source and destination parent are the same
		if old_parent_node.inode == new_parent_node.inode {
			// No need to check for cycles, hence no need to lock the rename mutex
			// TODO rename entry and return
		}
		// Prevent concurrent renames to safeguard cycle checking
		let fs = downcast_fs::<TmpFS>(&*old_parent_node.fs.ops);
		let _rename_guard = fs.rename_lock.lock();
		// Cannot make a directory a child of itself
		if unlikely(new_entry.is_child_of(old_entry)) {
			return Err(errno!(EINVAL));
		}
		let (mut old_parent_inner, mut new_parent_inner) =
			Mutex::lock_two(&self.0, &new_parent_inner.0);
		if let Some(new_ent) = new_parent_inner.find_entry_mut(&new_entry.name) {
			// Update entry
			let prev = mem::replace(&mut new_ent.node, old_node.clone());
			if flags & RENAME_EXCHANGE != 0 {
				// Set entry in the old directory. We are guaranteed that the entry already exists
				let old_ent = old_parent_inner
					.find_entry_mut(&old_entry.name)
					.ok_or_else(|| errno!(EUCLEAN))?;
				old_ent.node = prev;
				// TODO if the new file is a directory, update its `..`
			} else {
				// Decrement reference counter to the previous inode
				let mut stat = prev.stat.lock();
				stat.nlink = stat.nlink.saturating_sub(1);
			}
		} else {
			// Insert entry
			new_parent_inner.insert(TmpfsDirEntry {
				name: Cow::Owned(new_entry.name.try_to_owned()?),
				node: old_node.clone(),
			})?;
		}
		let content = DirectoryContent::from_ops(&*old_node.node_ops);
		if let Some(dir_inner) = content {
			// Update the `..` entry
			dir_inner.0.lock().set_inode(b"..", new_parent_node.clone());
			// Update links count
			let mut new_parent_stat = new_parent_node.stat.lock();
			if unlikely(new_parent_stat.nlink == u16::MAX) {
				return Err(errno!(EMFILE));
			}
			new_parent_stat.nlink += 1;
		}
		Ok(())
	}
}

// TODO remove mutex. the content should be immutable
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
	/// The inner kernfs.
	nodes: Mutex<NodeStorage, false>,
	/// Lock when renaming a file, to avoid concurrency issues when looking for cycles.
	rename_lock: Mutex<()>,
}

impl FilesystemOps for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn cache_entries(&self) -> bool {
		false
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

	fn root(&self, _fs: &Arc<Filesystem>) -> EResult<Arc<Node>> {
		self.nodes.lock().get_node(kernfs::ROOT_INODE).cloned()
	}

	fn destroy_node(&self, node: &Node) -> EResult<()> {
		self.nodes.lock().remove_node(node.inode);
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
				nodes: Mutex::new(NodeStorage::new()?),
				rename_lock: Mutex::new(()),
			})?,
			mount_flags,
		)?;
		let root = Arc::new(Node::new(
			0,
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
			Box::new(DirectoryContent::default())?,
			Box::new(DummyOps)?,
		))?;
		// Insert node
		downcast_fs::<TmpFS>(&*fs.ops).nodes.lock().set_root(root)?;
		Ok(fs)
	}
}
