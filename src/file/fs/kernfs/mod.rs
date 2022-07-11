//! Kernfs implements utilities allowing to create a virtual filesystem.

pub mod node;

use crate::errno::Errno;
use crate::errno;
use crate::file::DirEntry;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::fs::Statfs;
use crate::file::path::Path;
use crate::memory;
use crate::process::oom;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use node::KernFSNode;

/// The index of the root inode.
const ROOT_INODE: INode = 0;

/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The path at which the filesystem is mounted.
	mountpath: Path,

	/// The list of nodes of the filesystem. The index in this vector is the inode.
	nodes: Vec<Option<KernFSNode>>,
	/// A list of free inodes.
	free_nodes: Vec<INode>,
}

impl KernFS {
	/// Creates a new instance.
	/// `name` is the name of the filesystem.
	/// `readonly` tells whether the filesystem is readonly.
	/// `mountpath` is the path at which the filesystem is mounted.
	pub fn new(name: String, readonly: bool, mountpath: Path) -> Result<Self, Errno> {
		let mut nodes = Vec::new();
		nodes.push(None)?;

		Ok(Self {
			name,
			readonly,

			mountpath,

			nodes,
			free_nodes: Vec::new(),
		})
	}

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, root: KernFSNode) -> Result<(), Errno> {
		if self.nodes.is_empty() {
			self.nodes.push(Some(root))?;
		} else {
			self.nodes[ROOT_INODE as _] = Some(root);
		}

		Ok(())
	}

	/// Returns an immutable reference to the node with inode `inode`. If the node doesn't exist,
	/// the function returns an error.
	pub fn get_node(&self, inode: INode) -> Result<&KernFSNode, Errno> {
		if inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as usize].as_ref().ok_or_else(|| errno!(ENOENT))
	}

	/// Returns a mutable reference to the node with inode `inode`. If the node doesn't exist, the
	/// function returns an error.
	pub fn get_node_mut(&mut self, inode: INode) -> Result<&mut KernFSNode, Errno> {
		if inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as usize].as_mut().ok_or_else(|| errno!(ENOENT))
	}

	/// Adds the given node `node` to the filesystem.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, node: KernFSNode) -> Result<INode, Errno> {
		if let Some(free_node) = self.free_nodes.pop() {
			// Using an existing slot
			self.nodes[free_node as _] = Some(node);

			Ok(free_node)
		} else {
			// Allocating a new node slot
			let inode = self.nodes.len();
			self.nodes.push(Some(node))?;

			Ok(inode as _)
		}
	}

	/// Removes the node with inode `inode`.
	pub fn remove_node(&mut self, inode: INode) -> Result<(), Errno> {
		if let Some(node) = &self.nodes[inode as _] {
			// If the node is a non-empty directory, return an error
			match node.get_content() {
				FileContent::Directory(entries) if !entries.is_empty() => {
					return Err(errno!(ENOTEMPTY));
				},
				_ => {},
			}

			self.nodes[inode as _] = None;
			self.free_nodes.push(inode)?;
		}

		Ok(())
	}
}

impl Filesystem for KernFS {
	fn get_name(&self) -> &[u8] {
		self.name.as_bytes()
	}

	fn is_readonly(&self) -> bool {
		self.readonly
	}

	fn must_cache(&self) -> bool {
		false
	}

	fn get_stat(&self, _io: &mut dyn IO) -> Result<Statfs, Errno> {
		Ok(Statfs {
			f_type: 0, // TODO
			f_bsize: memory::PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: self.nodes.len() as _,
			f_ffree: 0,
			f_fsid: 0, // TODO
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn get_root_inode(&self, _io: &mut dyn IO) -> Result<INode, Errno> {
		Ok(ROOT_INODE)
	}

	fn get_inode(&mut self, _io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno> {
		let parent = parent.unwrap_or(ROOT_INODE);

		// Getting the parent node
		let parent = self.get_node(parent)?;

		match parent.get_content() {
			FileContent::Directory(entries) => entries.get(name)
				.map(| dirent | dirent.inode)
				.ok_or_else(|| errno!(ENOENT)),

			_ => Err(errno!(ENOENT)),
		}
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: INode, name: String)
		-> Result<File, Errno> {
		let node = self.get_node(inode)?;

		let file_location = FileLocation::new(self.mountpath.failable_clone()?, inode);
		let file_content = node.get_content().failable_clone()?;

		let mut file = File::new(name, node.get_uid(), node.get_gid(), node.get_mode(),
			file_location, file_content)?;
		file.set_hard_links_count(node.get_hard_links_count());
		file.set_size(node.get_size());
		file.set_ctime(node.get_ctime());
		file.set_mtime(node.get_mtime());
		file.set_atime(node.get_atime());

		Ok(file)
	}

	fn add_file(&mut self, _: &mut dyn IO, parent_inode: INode, name: String, uid: Uid,
		gid: Gid, mode: Mode, content: FileContent) -> Result<File, Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		let file_type = content.get_file_type();
		let mountpath = self.mountpath.failable_clone()?;

		// Checking the parent exists
		self.get_node_mut(parent_inode)?;

		let node = KernFSNode::new(mode, uid, gid, content.failable_clone()?, None);
		let inode = self.add_node(node)?;

		// Getting entries from parent
		let parent = self.get_node_mut(parent_inode).unwrap();
		let entries = match parent.get_content_mut() {
			FileContent::Directory(entries) => entries,
			_ => return Err(errno!(ENOENT)),
		};

		oom::wrap(|| {
			entries.insert(name.failable_clone()?, DirEntry {
				inode,
				entry_type: file_type,
			})
		});

		let location = FileLocation::new(mountpath, inode);
		File::new(name, uid, gid, mode, location, content)
	}

	fn add_link(&mut self, _: &mut dyn IO, parent_inode: INode, name: &String, inode: INode)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		let entry_type = self.get_node(parent_inode)?.get_content().get_file_type();
		let parent = self.get_node_mut(parent_inode)?;

		match parent.get_content_mut() {
			FileContent::Directory(entries) => {
				entries.insert(name.failable_clone()?, DirEntry {
					inode,
					entry_type,
				})?;

				Ok(())
			},

			_ => Err(errno!(ENOTDIR)),
		}
	}

	fn update_inode(&mut self, _: &mut dyn IO, file: &File) -> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// Getting node
		let inode = file.get_location().get_inode();
		let node = self.get_node_mut(inode)?;

		// Changing file size if it has been truncated
		// TODO node.truncate(file.get_size())?;

		// Updating file attributes
		node.set_uid(file.get_uid());
		node.set_gid(file.get_gid());
		node.set_mode(file.get_mode());
		node.set_ctime(file.get_ctime());
		node.set_mtime(file.get_mtime());
		node.set_atime(file.get_atime());

		Ok(())
	}

	fn remove_file(&mut self, _: &mut dyn IO, parent_inode: INode, name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// Getting directory entry
		let parent = self.get_node_mut(parent_inode)?;
		let entry = match parent.get_content() {
			FileContent::Directory(entries) => if let Some(entry) = entries.get(name) {
				entry
			} else {
				return Err(errno!(ENOENT));
			},

			_ => return Err(errno!(ENOTDIR)),
		};
		let inode = entry.inode;

		let node = self.get_node(inode)?;
		match node.get_content() {
			FileContent::Directory(entries) if !entries.is_empty()
				=> return Err(errno!(ENOTEMPTY)),

			_ => {},
		}

		oom::wrap(|| self.remove_node(inode));

		// Removing directory entry
		let parent = self.get_node_mut(parent_inode).unwrap();
		match parent.get_content_mut() {
			FileContent::Directory(entries) => entries.remove(name),
			_ => unreachable!(),
		};

		Ok(())
	}

	fn read_node(&mut self, _: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		let node = self.get_node_mut(inode)?;
		node.read(off, buf)
	}

	fn write_node(&mut self, _: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		let node = self.get_node_mut(inode)?;
		node.write(off, buf)?;
		Ok(())
	}
}
