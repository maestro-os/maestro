//! The content of a kernfs node can either be owned by the node, in which case the node shall
//! return a reference, or it can be dynamic, meaning it is generated on the fly by the node when
//! requested.
//!
//! On the fly generation is useful in special cases. For example, when the content changes
//! depending on the process calling the kernfs.

use crate::errno::AllocResult;
use crate::file::FileContent;
use crate::util::TryClone;
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use core::ops::Deref;
use core::ops::DerefMut;

/// Content of a kernfs node.
pub enum KernFSContent<'node> {
	/// A content owned by the node.
	Owned(&'node mut FileContent),
	/// A dynamic content generated on the fly.
	Dynamic(FileContent),
}

impl From<FileContent> for KernFSContent<'static> {
	fn from(val: FileContent) -> Self {
		Self::Dynamic(val)
	}
}

impl<'node> From<&'node mut FileContent> for KernFSContent<'node> {
	fn from(val: &'node mut FileContent) -> Self {
		Self::Owned(val)
	}
}

impl KernFSContent<'_> {
	/// Returns an owned version of the content.
	///
	/// This function may clone the original content.
	pub fn to_owned(self) -> AllocResult<FileContent> {
		match self {
			Self::Owned(c) => c.try_clone(),
			Self::Dynamic(c) => Ok(c),
		}
	}
}

impl Borrow<FileContent> for KernFSContent<'_> {
	fn borrow(&self) -> &FileContent {
		match self {
			Self::Owned(c) => c,
			Self::Dynamic(c) => c,
		}
	}
}

impl BorrowMut<FileContent> for KernFSContent<'_> {
	fn borrow_mut(&mut self) -> &mut FileContent {
		match self {
			Self::Owned(c) => c,
			Self::Dynamic(c) => c,
		}
	}
}

impl<'node> Deref for KernFSContent<'node> {
	type Target = FileContent;

	fn deref(&self) -> &Self::Target {
		self.borrow()
	}
}

impl DerefMut for KernFSContent<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.borrow_mut()
	}
}
