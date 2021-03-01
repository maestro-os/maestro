/// This module implements a binary tree utility.

use core::cmp::Ordering;
use core::cmp::max;
use core::marker::PhantomData;
use core::ptr::NonNull;

/// TODO doc
pub struct BinaryTreeNode<T, O: Fn() -> usize> {
	/// Pointer to the left child
	left: Option::<NonNull<BinaryTreeNode<T, O>>>,
	/// Pointer to the right child
	right: Option::<NonNull<BinaryTreeNode<T, O>>>,

	/// Closure storing the offset of the node into the structure storing it
	offset_data: O,
	/// Phantom data to keep `T`
	_phantom: PhantomData<T>,
}

impl<T, O: Fn() -> usize> BinaryTreeNode::<T, O> {
	// TODO new

	/// Returns a reference to the item that owns the node of the tree.
	pub fn get(&self) -> &T {
		let ptr = (self as *const _ as usize) - (self.offset_data)();
		unsafe { // Dereference of raw pointer
			&*(ptr as *const T)
		}
	}

	/// Returns a mutable reference to the item that owns the node of the tree.
	pub fn get_mut(&mut self) -> &mut T {
		let ptr = (self as *mut _ as usize) - (self.offset_data)();
		unsafe { // Dereference of raw pointer
			&mut *(ptr as *mut T)
		}
	}

	/// Returns a reference to the left child node.
	pub fn get_left(&self) -> Option::<&BinaryTreeNode::<T, O>> {
		if let Some(l) = self.left.as_ref() {
			unsafe { // Call to unsafe function
				Some(l.as_ref())
			}
		} else {
			None
		}
	}

	/// Returns a reference to the left child node.
	pub fn get_left_mut(&mut self) -> Option::<&mut BinaryTreeNode::<T, O>> {
		if let Some(l) = self.left.as_mut() {
			unsafe { // Call to unsafe function
				Some(l.as_mut())
			}
		} else {
			None
		}
	}

	/// Returns a reference to the left child node.
	pub fn get_right(&self) -> Option::<&BinaryTreeNode::<T, O>> {
		if let Some(r) = self.right.as_ref() {
			unsafe { // Call to unsafe function
				Some(r.as_ref())
			}
		} else {
			None
		}
	}

	/// Returns a reference to the left child node.
	pub fn get_right_mut(&mut self) -> Option::<&mut BinaryTreeNode::<T, O>> {
		if let Some(r) = self.right.as_mut() {
			unsafe { // Call to unsafe function
				Some(r.as_mut())
			}
		} else {
			None
		}
	}

	/// Returns the number of nodes in the subtree.
	pub fn nodes_count(&self) -> usize {
		let left_count = if let Some(l) = self.get_left() {
			l.nodes_count()
		} else {
			0
		};
		let right_count = if let Some(r) = self.get_right() {
			r.nodes_count()
		} else {
			0
		};
		1 + left_count + right_count
	}

	/// Returns the depth of the subtree.
	pub fn get_depth(&self) -> usize {
		let left_count = if let Some(l) = self.get_left() {
			l.nodes_count()
		} else {
			0
		};
		let right_count = if let Some(r) = self.get_right() {
			r.nodes_count()
		} else {
			0
		};
		1 + max(left_count, right_count)
	}
}

/// TODO doc
pub struct BinaryTree<T, O: Fn() -> usize> {
	/// The root node of the binary tree.
	root: Option::<NonNull<BinaryTreeNode<T, O>>>,
	/// Closure storing the offset of the node into the structure storing it
	offset_data: O,
}

impl<T, O: Fn() -> usize> BinaryTree<T, O> {
	/// Creates a new binary tree.
	pub fn new(offset_data: O) -> Self {
		Self {
			root: None,
			offset_data: offset_data,
		}
	}

	/// Returns a reference to the root node.
	pub fn get_root(&self) -> Option::<&BinaryTreeNode<T, O>> {
		if let Some(r) = self.root.as_ref() {
			unsafe { // Call to unsafe function
				Some(r.as_ref())
			}
		} else {
			None
		}
	}

	/// Returns a mutable reference to the root node.
	pub fn get_root_mut(&mut self) -> Option::<&mut BinaryTreeNode<T, O>> {
		if let Some(r) = self.root.as_mut() {
			unsafe { // Call to unsafe function
				Some(r.as_mut())
			}
		} else {
			None
		}
	}

	/// Returns the number of nodes in the tree.
	pub fn nodes_count(&self) -> usize {
		if let Some(r) = self.get_root() {
			r.nodes_count()
		} else {
			0
		}
	}

	/// Returns the depth of the tree.
	pub fn get_depth(&self) -> usize {
		if let Some(r) = self.get_root() {
			r.get_depth()
		} else {
			0
		}
	}

	/// Searches for a node with the given closure for comparison.
	/// `cmp` is the comparison function.
	pub fn get<F: Fn(&BinaryTreeNode::<T, O>) -> Ordering>(&self, cmp: F)
		-> Option<&BinaryTreeNode::<T, O>> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(n);
			if ord == Ordering::Less {
				node = n.get_left();
			} else if ord == Ordering::Greater {
				node = n.get_right();
			} else {
				break;
			}
		}

		node
	}

	/// Inserts a node in the tree.
	/// `node` is the node to insert.
	/// `cmp` is the comparison function.
	pub fn insert<F: Fn(&T) -> Ordering>(&mut self, _node: BinaryTreeNode<T, O>, _cmp: F) {
		// TODO
	}

	/// Removes a node from the tree.
	/// `node` is the node to remove.
	/// `cmp` is the comparison function.
	pub fn remove<F: Fn(&T) -> Ordering>(&mut self, _node: BinaryTreeNode<T, O>, _cmp: F) {
		// TODO
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn binary_tree0() {
		// TODO
	}

	// TODO
}
