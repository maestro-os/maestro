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

//! The content of a kernfs node can either be owned by the node, in which case the node shall
//! return a reference, or it can be dynamic, meaning it is generated on the fly by the node when
//! requested.
//!
//! On the fly generation is useful in special cases. For example, when the content changes
//! depending on the process calling the kernfs.

use core::{
	borrow::{Borrow, BorrowMut},
	ops::{Deref, DerefMut},
};
use utils::{errno::AllocResult, TryClone};

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
