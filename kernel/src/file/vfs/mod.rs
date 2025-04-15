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

//! The VFS (Virtual FileSystem) aggregates every mounted filesystems into one.
//!
//! To manipulate files, the VFS should be used instead of
//! calling the filesystems' directly.
//!
//! # Note about creating files
//!
//! When creating a file of any type, the following fields in the provided [`Stat`] structure are
//! ignored:
/// - `nlink`
/// - `uid`
/// - `gid`
///
/// `uid` and `gid` are set according to `ap`
pub mod mountpoint;
pub mod node;

use super::{
	perm,
	perm::{AccessProfile, S_ISVTX},
	FileType, Stat,
};
use crate::{
	file::fs::StatSet,
	process::Process,
	sync::{mutex::Mutex, once::OnceInit},
};
use core::{
	borrow::Borrow,
	hash::{Hash, Hasher},
	intrinsics::unlikely,
	sync::atomic::Ordering::Release,
};
use node::Node;
use utils::{
	collections::{
		hashset::HashSet,
		list::ListNode,
		path::{Component, Path, PathBuf},
		string::String,
	},
	errno,
	errno::{AllocResult, EResult},
	limits::{LINK_MAX, PATH_MAX, SYMLOOP_MAX},
	list, list_type,
	ptr::arc::Arc,
	vec,
};

/// A child of a VFS entry.
///
/// The [`Hash`] and [`PartialEq`] traits are forwarded to the entry's name.
#[derive(Debug)]
struct EntryChild(Arc<Entry>);

impl Borrow<[u8]> for EntryChild {
	fn borrow(&self) -> &[u8] {
		&self.0.name
	}
}

impl Eq for EntryChild {}

impl PartialEq for EntryChild {
	fn eq(&self, other: &Self) -> bool {
		self.0.name.eq(&other.0.name)
	}
}

impl Hash for EntryChild {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.name.hash(state)
	}
}

/// A VFS entry, representing a directory entry cached in memory.
///
/// An entry can be negative. That is, represent a non-existent file.
#[derive(Debug)]
pub struct Entry {
	/// Filename.
	pub name: String,
	/// The parent of the entry.
	///
	/// If `None`, the current entry is the root of the VFS.
	pub parent: Option<Arc<Entry>>,
	/// The list of cached file entries.
	///
	/// This is not an exhaustive list of the file's entries. Only those that are loaded.
	children: Mutex<HashSet<EntryChild>>,
	/// The node associated with the entry.
	///
	/// If `None`, the entry is negative.
	pub node: Option<Arc<Node>>,

	/// Node for the LRU
	lru: ListNode,
}

impl Entry {
	/// Creates a new instance.
	pub fn new(name: String, parent: Option<Arc<Entry>>, node: Option<Arc<Node>>) -> Self {
		Self {
			name,
			parent,
			children: Default::default(),
			node,

			lru: Default::default(),
		}
	}

	/// Tells whether the entry is negative. That is, if it represents a non-existent entry.
	#[inline]
	pub fn is_negative(&self) -> bool {
		self.node.is_none()
	}

	/// Returns a reference to the underlying node.
	///
	/// If the entry represents a non-existent file, the function panics.
	#[inline]
	pub fn node(&self) -> &Arc<Node> {
		self.node
			.as_ref()
			.expect("trying to access a non-existent node")
	}

	/// Helper returning the status of the underlying node.
	///
	/// If the entry represents a non-existent file, the function panics.
	#[inline]
	pub fn stat(&self) -> Stat {
		self.node().stat.lock().clone()
	}

	/// Returns the file's type.
	#[inline]
	pub fn get_type(&self) -> EResult<FileType> {
		FileType::from_mode(self.stat().mode).ok_or_else(|| errno!(EUCLEAN))
	}

	/// Returns the absolute path to reach the entry.
	pub fn get_path(this: &Arc<Self>) -> EResult<PathBuf> {
		if this.parent.is_none() {
			return Ok(PathBuf::root()?);
		}
		let mut buf = vec![0u8; PATH_MAX]?;
		let mut off = PATH_MAX;
		let mut cur = this;
		while let Some(parent) = &cur.parent {
			let len = cur.name.len();
			off = off
				.checked_sub(len + 1)
				.ok_or_else(|| errno!(ENAMETOOLONG))?;
			buf[off] = b'/';
			buf[(off + 1)..(off + len + 1)].copy_from_slice(&cur.name);
			cur = parent;
		}
		buf.rotate_left(off);
		buf.truncate(buf.len() - off);
		Ok(PathBuf::new_unchecked(String::from(buf)))
	}

	/// Makes `self` a child of its parent, if any. The entry is also inserted in the LRU.
	///
	/// The function returns `self` wrapped into an [`Arc`].
	pub fn link_parent(self) -> AllocResult<Arc<Self>> {
		let entry = Arc::new(self)?;
		if let Some(parent) = &entry.parent {
			parent.children.lock().insert(EntryChild(entry.clone()))?;
		}
		LRU.lock().insert_front(entry.clone());
		Ok(entry)
	}

	/// Releases the entry, removing it the underlying node if no link remain and this was the last
	/// use of it.
	pub fn release(this: Arc<Self>) -> EResult<()> {
		let Some(parent) = this.parent.clone() else {
			// This is the root of the VFS, stop
			return Ok(());
		};
		// Lock now to avoid a race condition with `strong_count`
		let mut parent_children = parent.children.lock();
		/*
		 * If this is **not** the last reference to the entry, we cannot remove it.
		 *
		 * The reference held by its own parent + the one held by the LRU + the one that we
		 * hold here = 3
		 *
		 * We cannot release an entry with at least one cached child. Fortunately, a child
		 * entry refers to its parent, so the condition below is sufficient.
		 */
		if Arc::strong_count(&this) > 3 {
			return Ok(());
		}
		// Remove other references
		parent_children.remove(&*this.name);
		unsafe {
			LRU.lock().remove(&this);
		}
		// If other references remain, we cannot go further
		let Some(entry) = Arc::into_inner(this) else {
			return Ok(());
		};
		drop(parent_children);
		// Release the inner node if present
		if let Some(node) = entry.node {
			Node::release(node)?;
		}
		Ok(())
	}
}

/// Directory entries LRU.
static LRU: Mutex<list_type!(Entry, lru)> = Mutex::new(list!(Entry, lru));

/// Attempts to shrink the directory entries cache.
///
/// If the cache cannot shrink, the function returns `false`.
pub fn shrink_entries() -> bool {
	let mut lru = LRU.lock();
	for cursor in lru.iter().rev() {
		let entry = cursor.arc();
		// The following is the same as the implementation of `Entry::release`. We don't call
		// directly to reuse the lock on `LRU`
		let Some(parent) = entry.parent.clone() else {
			continue;
		};
		let mut parent_children = parent.children.lock();
		if Arc::strong_count(&entry) > 3 {
			continue;
		}
		parent_children.remove(&*entry.name);
		cursor.remove();
		let Some(entry) = Arc::into_inner(entry) else {
			continue;
		};
		drop(parent_children);
		if let Some(node) = entry.node {
			// TODO log I/O errors?
			let _ = Node::release(node);
		}
		return true;
	}
	false
}

/// The root entry of the VFS.
pub static ROOT: OnceInit<Arc<Entry>> = unsafe { OnceInit::new() };

/// Settings for a path resolution operation.
#[derive(Clone, Debug)]
pub struct ResolutionSettings {
	/// The location of the root directory for the operation.
	///
	/// Contrary to the `start` field, resolution *cannot* access a parent of this path.
	pub root: Arc<Entry>,
	/// The current working directory, from which the resolution starts.
	///
	/// If `None`, resolution starts from `root`.
	pub cwd: Option<Arc<Entry>>,

	/// The access profile to use for resolution.
	pub access_profile: AccessProfile,

	/// If `true`, the path is resolved for creation, meaning the operation will not fail if the
	/// file does not exist.
	pub create: bool,
	/// If `true` and if the last component of the path is a symbolic link, path resolution
	/// follows it.
	pub follow_link: bool,
}

impl ResolutionSettings {
	/// Kernel access, following symbolic links.
	pub fn kernel_follow() -> Self {
		Self {
			root: ROOT.clone(),
			cwd: None,

			access_profile: AccessProfile::KERNEL,

			create: false,
			follow_link: true,
		}
	}

	/// Kernel access, without following symbolic links.
	pub fn kernel_nofollow() -> Self {
		Self {
			follow_link: false,
			..Self::kernel_follow()
		}
	}

	/// Returns the default for the given process.
	///
	/// `follow_link` tells whether symbolic links are followed.
	pub fn for_process(proc: &Process, follow_link: bool) -> Self {
		let fs = proc.fs.lock();
		Self {
			root: fs.chroot.clone(),
			cwd: Some(fs.cwd.clone()),

			access_profile: fs.access_profile,

			create: false,
			follow_link,
		}
	}
}

/// The resolute of the path resolution operation.
#[derive(Debug)]
pub enum Resolved<'s> {
	/// The file has been found.
	Found(Arc<Entry>),
	/// The file can be created.
	///
	/// This variant can be returned only if the `create` field is set to `true` in
	/// [`ResolutionSettings`].
	Creatable {
		/// The parent directory in which the file is to be created.
		parent: Arc<Entry>,
		/// The name of the file to be created.
		name: &'s [u8],
	},
}

/// Resolves an entry with the given `name`, in the given `lookup_dir`.
///
/// If the entry does not exist in cache or on the filesystem, the function returns a negative
/// entry.
fn resolve_entry(lookup_dir: &Arc<Entry>, name: &[u8]) -> EResult<Arc<Entry>> {
	let mut children = lookup_dir.children.lock();
	// Try to get from cache first
	if let Some(ent) = children.get(name) {
		let ent = ent.0.clone();
		drop(children);
		// Promote the entry in the LRU
		unsafe {
			LRU.lock().lru_promote(&ent);
		}
		return Ok(ent);
	}
	// Not in cache. Try to get from the filesystem
	let mut entry = Entry::new(String::try_from(name)?, Some(lookup_dir.clone()), None);
	lookup_dir
		.node()
		.node_ops
		.lookup_entry(lookup_dir.node(), &mut entry)?;
	// Insert in cache. Do not use `link_parent` to keep `children` locked
	let entry = Arc::new(entry)?;
	children.insert(EntryChild(entry.clone()))?;
	drop(children);
	LRU.lock().insert_front(entry.clone());
	Ok(entry)
}

/// Resolves the symbolic link `link` and returns the target.
///
/// Arguments:
/// - `root` is the root directory
/// - `lookup_dir` is the directory from which the resolution of the target starts
/// - `access_profile` is the access profile used for resolution
/// - `symlink_rec` is the number of recursions so far
///
/// Symbolic links are followed recursively, including the last element of the target path.
fn resolve_link(
	link: Arc<Entry>,
	root: Arc<Entry>,
	lookup_dir: Arc<Entry>,
	access_profile: AccessProfile,
	symlink_rec: usize,
) -> EResult<Arc<Entry>> {
	// If too many recursions occur, error
	if unlikely(symlink_rec + 1 > SYMLOOP_MAX) {
		return Err(errno!(ELOOP));
	}
	let target = link.node().readlink()?;
	// Resolve link
	let rs = ResolutionSettings {
		root,
		cwd: Some(lookup_dir),
		access_profile,
		create: false,
		follow_link: true,
	};
	let resolved = resolve_path_impl(&target, &rs, symlink_rec + 1)?;
	let Resolved::Found(target) = resolved else {
		// Because `create` is set to `false`
		unreachable!();
	};
	Ok(target)
}

/// Implementation of [`resolve_path`].
///
/// `symlink_rec` is the number of recursions due to symbolic links resolution.
fn resolve_path_impl<'p>(
	path: &'p Path,
	settings: &ResolutionSettings,
	symlink_rec: usize,
) -> EResult<Resolved<'p>> {
	// Get start lookup directory
	let mut lookup_dir = match (path.is_absolute(), &settings.cwd) {
		(false, Some(start)) => start.clone(),
		_ => settings.root.clone(),
	};
	let mut components = path.components();
	let Some(final_component) = components.next_back() else {
		return Ok(Resolved::Found(lookup_dir));
	};
	// Iterate on intermediate components
	for comp in components {
		// Check lookup permission
		let lookup_dir_stat = lookup_dir.stat();
		if !settings
			.access_profile
			.can_search_directory(&lookup_dir_stat)
		{
			return Err(errno!(EACCES));
		}
		// Get the name of the next entry
		let name = match comp {
			Component::ParentDir => {
				if let Some(parent) = &lookup_dir.parent {
					lookup_dir = parent.clone();
				}
				continue;
			}
			Component::Normal(name) => name,
			// Ignore
			_ => continue,
		};
		// Get entry
		let entry = resolve_entry(&lookup_dir, name)?;
		if entry.is_negative() {
			return Err(errno!(ENOENT));
		}
		match entry.get_type()? {
			FileType::Directory => lookup_dir = entry,
			FileType::Link => {
				lookup_dir = resolve_link(
					entry,
					settings.root.clone(),
					lookup_dir,
					settings.access_profile,
					symlink_rec,
				)?;
			}
			_ => return Err(errno!(ENOTDIR)),
		}
	}
	// Final component lookup
	let name = match final_component {
		Component::RootDir | Component::CurDir => {
			// If the component is `RootDir`, the entire path equals `/` and `lookup_dir` can only
			// be the root. If the component is `CurDir`, the `lookup_dir` is the target
			return Ok(Resolved::Found(lookup_dir));
		}
		Component::ParentDir => {
			if let Some(parent) = &lookup_dir.parent {
				lookup_dir = parent.clone();
			}
			return Ok(Resolved::Found(lookup_dir));
		}
		Component::Normal(name) => name,
	};
	// Check lookup permission
	let lookup_dir_stat = lookup_dir.stat();
	if !settings
		.access_profile
		.can_search_directory(&lookup_dir_stat)
	{
		return Err(errno!(EACCES));
	}
	// Get entry
	let entry = resolve_entry(&lookup_dir, name)?;
	if entry.is_negative() {
		// The file does not exist
		return if settings.create {
			Ok(Resolved::Creatable {
				parent: lookup_dir,
				name,
			})
		} else {
			Err(errno!(ENOENT))
		};
	}
	// Resolve symbolic link if necessary
	if settings.follow_link && entry.get_type()? == FileType::Link {
		Ok(Resolved::Found(resolve_link(
			entry,
			settings.root.clone(),
			lookup_dir,
			settings.access_profile,
			symlink_rec,
		)?))
	} else {
		Ok(Resolved::Found(entry))
	}
}

/// Resolves the given `path` with the given `settings`.
///
/// The following conditions can cause errors:
/// - If the path is empty, the function returns [`errno::ENOMEM`].
/// - If a component of the path cannot be accessed with the provided access profile, the function
///   returns [`errno::EACCES`].
/// - If a component of the path (excluding the last) is not a directory nor a symbolic link, the
///   function returns [`errno::ENOTDIR`].
/// - If a component of the path (excluding the last) is a symbolic link and following them is
///   disabled, the function returns [`errno::ENOTDIR`].
/// - If the resolution of the path requires more symbolic link indirections than [`SYMLOOP_MAX`],
///   the function returns [`errno::ELOOP`].
pub fn resolve_path<'p>(path: &'p Path, settings: &ResolutionSettings) -> EResult<Resolved<'p>> {
	// Required by POSIX
	if settings.cwd.is_none() && path.is_empty() {
		return Err(errno!(ENOENT));
	}
	resolve_path_impl(path, settings, 0)
}

/// Like [`get_file_from_path`], but returns `None` is the file does not exist.
pub fn get_file_from_path_opt(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<Option<Arc<Entry>>> {
	let file = match resolve_path(path, resolution_settings)? {
		Resolved::Found(file) => Some(file),
		_ => None,
	};
	Ok(file)
}

/// Returns the file at the given `path`.
///
/// If the file does not exist, the function returns [`errno::ENOENT`].
pub fn get_file_from_path(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<Arc<Entry>> {
	get_file_from_path_opt(path, resolution_settings)?.ok_or_else(|| errno!(ENOENT))
}

/// Updates status of a node.
pub fn set_stat(node: &Node, set: &StatSet) -> EResult<()> {
	let mut stat = node.stat.lock();
	if let Some(mode) = set.mode {
		stat.mode = (stat.mode & !0o7777) | (mode & 0o7777);
	}
	if let Some(uid) = set.uid {
		stat.uid = uid;
	}
	if let Some(gid) = set.gid {
		stat.gid = gid;
	}
	if let Some(ctime) = set.ctime {
		stat.ctime = ctime;
	}
	if let Some(mtime) = set.mtime {
		stat.mtime = mtime;
	}
	if let Some(atime) = set.atime {
		stat.atime = atime;
	}
	node.dirty.store(true, Release);
	Ok(())
}

/// Creates a file, adds it to the VFS, then returns it.
///
/// Arguments:
/// - `parent` is the parent directory of the file to be created
/// - `name` is the name of the file to be created
/// - `ap` is access profile to check permissions. This also determines the UID and GID to be used
///   for the created file
/// - `stat` is the status of the newly created file
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - Permissions to create the file are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - `parent` is not a directory: [`errno::ENOTDIR`]
/// - The file already exists: [`errno::EEXIST`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn create_file(
	parent: Arc<Entry>,
	name: &[u8],
	ap: &AccessProfile,
	mut stat: Stat,
) -> EResult<Arc<Entry>> {
	let parent_stat = parent.stat();
	// Validation
	if parent_stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(&parent_stat) {
		return Err(errno!(EACCES));
	}
	stat.nlink = 0;
	stat.uid = ap.euid;
	stat.gid = if parent_stat.mode & perm::S_ISGID != 0 {
		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		parent_stat.gid
	} else {
		ap.egid
	};
	// Add file to filesystem
	let parent_node = parent.node();
	let node = parent_node
		.fs
		.ops
		.create_node(parent_node.fs.clone(), stat)?;
	// Add link to filesystem
	let ent = Entry::new(String::try_from(name)?, Some(parent.clone()), Some(node));
	parent_node.node_ops.link(parent_node.clone(), &ent)?;
	Ok(ent.link_parent()?)
}

/// Creates a new hard link to the given target file.
///
/// Arguments:
/// - `parent` is the parent directory where the new link will be created
/// - `name` is the name of the link
/// - `target` is the target file
/// - `ap` is the access profile to check permissions
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - Permissions to create the link are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - The number of links to the file is larger than [`LINK_MAX`]: [`errno::EMLINK`]
/// - `target` is a directory: [`errno::EPERM`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn link(
	parent: &Arc<Entry>,
	name: String,
	target: Arc<Node>,
	ap: &AccessProfile,
) -> EResult<()> {
	let parent_stat = parent.stat();
	// Validation
	if parent_stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	let target_stat = target.stat();
	if target_stat.get_type() == Some(FileType::Directory) {
		return Err(errno!(EPERM));
	}
	if target_stat.nlink >= LINK_MAX as u16 {
		return Err(errno!(EMLINK));
	}
	if !ap.can_write_directory(&parent_stat) {
		return Err(errno!(EACCES));
	}
	if !parent.node().is_same_fs(&target) {
		return Err(errno!(EXDEV));
	}
	// Add link to the filesystem
	let ent = Entry::new(name, Some(parent.clone()), Some(target));
	parent.node().node_ops.link(parent.node().clone(), &ent)?;
	ent.link_parent()?;
	Ok(())
}

/// Removes a hard link to a file.
///
/// Arguments:
/// - `entry` is the entry to remove
/// - `ap` is the access profile to check permissions
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - The link does not exist: [`errno::ENOENT`]
/// - Permissions to remove the link are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - The file to remove is a mountpoint: [`errno::EBUSY`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn unlink(entry: &Entry, ap: &AccessProfile) -> EResult<()> {
	// Get parent
	let Some(parent) = &entry.parent else {
		// Cannot unlink root of the VFS
		return Err(errno!(EBUSY));
	};
	// Validation
	let parent_stat = parent.stat();
	if parent_stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(&parent_stat) {
		return Err(errno!(EACCES));
	}
	let stat = entry.stat();
	let has_sticky_bit = parent_stat.mode & S_ISVTX != 0;
	if has_sticky_bit && ap.euid != stat.uid && ap.euid != parent_stat.uid {
		return Err(errno!(EACCES));
	}
	// If the file to remove is a mountpoint, error
	if mountpoint::from_entry(entry).is_some() {
		return Err(errno!(EBUSY));
	}
	// Lock now to avoid race conditions
	let mut children = parent.children.lock();
	// Remove link from filesystem
	let dir_node = parent.node();
	dir_node.node_ops.unlink(dir_node, entry)?;
	// Remove link from cache
	let EntryChild(ent) = children.remove(entry.name.as_bytes()).unwrap();
	// Drop to avoid deadlock
	drop(children);
	Entry::release(ent)?;
	Ok(())
}

/// Creates a symbolic link.
///
/// Arguments:
/// - `parent` is the parent directory of where the new symbolic link will be created
/// - `name` is the name of the symbolic link
/// - `target` is the path the link points to
/// - `ap` is the access profile to check permissions
/// - `stat` is the status of the newly created file. Note that the `mode` is field is ignored and
///   replaced with the appropriate value
///
/// TODO: detail errors
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn symlink(
	parent: &Arc<Entry>,
	name: &[u8],
	target: &[u8],
	ap: &AccessProfile,
	mut stat: Stat,
) -> EResult<()> {
	let parent_stat = parent.stat();
	// Validation
	if parent_stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(&parent_stat) {
		return Err(errno!(EACCES));
	}
	stat.mode = FileType::Link.to_mode() | 0o777;
	stat.nlink = 0;
	stat.uid = ap.euid;
	stat.gid = if parent_stat.mode & perm::S_ISGID != 0 {
		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		parent_stat.gid
	} else {
		ap.egid
	};
	// Create node
	let parent_node = parent.node();
	let fs = parent_node.fs.clone();
	let node = fs.ops.create_node(fs.clone(), stat)?;
	node.node_ops.writelink(&node, target)?;
	// Add link to the filesystem
	let ent = Entry::new(String::try_from(name)?, Some(parent.clone()), Some(node));
	parent_node.node_ops.link(parent_node.clone(), &ent)?;
	ent.link_parent()?;
	Ok(())
}

/// Moves a file `old` to the directory `new_parent`, **on the same filesystem**.
///
/// If `old` is a directory, the destination shall not exist or be an empty directory.
///
/// Arguments:
/// - `old` is the file to move
/// - `new_parent` is the new parent directory for the file
/// - `new_name` is new name of the file
/// - `ap` is the access profile to check permissions
///
/// TODO: detail errors
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn rename(
	old: Arc<Entry>,
	new_parent: Arc<Entry>,
	new_name: &[u8],
	ap: &AccessProfile,
) -> EResult<()> {
	// If `old` has no parent, it's the root, so it's a mountpoint
	let old_parent = old.parent.as_ref().ok_or_else(|| errno!(EBUSY))?;
	// Parents validation
	if !new_parent.node().is_same_fs(old.node()) {
		return Err(errno!(EXDEV));
	}
	if mountpoint::from_entry(&old).is_some() {
		return Err(errno!(EBUSY));
	}
	// Check permissions on `old`
	let old_parent_stat = old_parent.stat();
	if !ap.can_write_directory(&old_parent_stat) {
		return Err(errno!(EACCES));
	}
	let old_stat = old.stat();
	if old_stat.mode & S_ISVTX != 0 && ap.euid != old_stat.uid && ap.euid != old_parent_stat.uid {
		return Err(errno!(EACCES));
	}
	// Check permissions on `new`
	let new_parent_stat = new_parent.stat();
	if !ap.can_write_directory(&new_parent_stat) {
		return Err(errno!(EACCES));
	}
	let new = resolve_entry(&new_parent, new_name)?;
	// Validation
	if !new.is_negative() {
		if mountpoint::from_entry(&new).is_some() {
			return Err(errno!(EBUSY));
		}
		let new_stat = new.stat();
		if new_stat.mode & S_ISVTX != 0
			&& ap.euid != new_stat.uid
			&& ap.euid != new_parent_stat.uid
		{
			return Err(errno!(EACCES));
		}
	}
	// Perform rename
	old.node().node_ops.rename(&old, &new_parent, new_name)?;
	// Invalidate cache
	old_parent.children.lock().remove(&*old.name);
	new_parent.children.lock().remove(new_name);
	Ok(())
}
