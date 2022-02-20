//! An inode is an identifier allowing to locate a file on a filesystem.

/// Trait representing an INode. A trait is required because each filesystem can store
/// filesystem-specific informations.
pub trait INode {}

/// Structure representing a Unix inode, which is just an unsigned integer.
pub struct UnixINode {
	/// The inode's value.
	inode: u32,
}

impl UnixINode {
	/// Returns the inode number.
	pub fn get(&self) -> u32 {
		self.inode
	}
}

impl INode for UnixINode {}

impl From<u32> for UnixINode {
	fn from(inode: u32) -> Self {
		Self {
			inode,
		}
	}
}

impl Into<u32> for UnixINode {
	fn into(self) -> u32 {
		self.inode
	}
}
