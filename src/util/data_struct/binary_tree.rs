/// This module implements a binary tree utility.

use core::cmp::Ordering;
use core::cmp::max;
use core::marker::PhantomData;
use core::ptr::NonNull;

/// The color of a binary tree node.
enum NodeColor {
	Black,
	Red,
}

/// TODO doc
pub struct BinaryTreeNode<T, O: Fn() -> usize> {
	/// Pointer to the parent node
	parent: Option::<NonNull<Self>>,
	/// Pointer to the left child
	left: Option::<NonNull<Self>>,
	/// Pointer to the right child
	right: Option::<NonNull<Self>>,
	/// The color of the node
	color: NodeColor,

	/// Closure storing the offset of the node into the structure storing it
	offset_data: O,
	/// Phantom data to keep `T`
	_phantom: PhantomData<T>,
}

impl<T, O: Fn() -> usize> BinaryTreeNode::<T, O> {
	// TODO new

	/// Unwraps the given pointer option into a reference option.
	fn unwrap_pointer(ptr: &Option::<NonNull<Self>>) -> Option::<&Self> {
		if let Some(p) = ptr {
			unsafe { // Call to unsafe function
				Some(p.as_ref())
			}
		} else {
			None
		}
	}

	/// Same as `unwrap_pointer` but returns a mutable reference.
	fn unwrap_pointer_mut(ptr: &mut Option::<NonNull<Self>>) -> Option::<&mut Self> {
		if let Some(p) = ptr {
			unsafe { // Call to unsafe function
				Some(p.as_mut())
			}
		} else {
			None
		}
	}

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
	pub fn get_parent(&self) -> Option::<&Self> {
		Self::unwrap_pointer(&self.parent)
	}

	/// Returns a reference to the parent child node.
	pub fn get_parent_mut(&mut self) -> Option::<&mut Self> {
		Self::unwrap_pointer_mut(&mut self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	pub fn get_left(&self) -> Option::<&Self> {
		Self::unwrap_pointer(&self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_left_mut(&mut self) -> Option::<&mut Self> {
		Self::unwrap_pointer_mut(&mut self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_right(&self) -> Option::<&Self> {
		Self::unwrap_pointer(&self.right)
	}

	/// Returns a reference to the left child node.
	pub fn get_right_mut(&mut self) -> Option::<&mut Self> {
		Self::unwrap_pointer_mut(&mut self.right)
	}

	/// Returns a reference to the grandparent node.
	pub fn get_grandparent(&self) -> Option::<&Self> {
		if let Some(p) = self.get_parent() {
			p.get_parent()
		} else {
			None
		}
	}

	/// Returns a mutable reference to the grandparent node.
	pub fn get_grandparent_mut(&mut self) -> Option::<&mut Self> {
		if let Some(p) = self.get_parent_mut() {
			p.get_parent_mut()
		} else {
			None
		}
	}

	/// Returns a reference to the sibling node.
	pub fn get_sibling(&self) -> Option::<&Self> {
		let self_ptr = self as *const _;
		let p = self.get_parent();
		if p.is_none() {
			return None;
		}

		let parent = p.unwrap();
		let left = parent.get_left();
		if left.is_some() && left.unwrap() as *const _ == self_ptr {
			parent.get_right()
		} else {
			parent.get_left()
		}
	}

	/// Returns a mutable reference to the sibling node.
	pub fn get_sibling_mut(&mut self) -> Option::<&mut Self> {
		let self_ptr = self as *const _;
		let p = self.get_parent_mut();
		if p.is_none() {
			return None;
		}

		let parent = p.unwrap();
		let left = parent.get_left_mut();
		if left.is_some() && left.unwrap() as *const _ == self_ptr {
			parent.get_right_mut()
		} else {
			parent.get_left_mut()
		}
	}

	/// Returns a reference to the uncle node.
	pub fn get_uncle(&mut self) -> Option::<&Self> {
		if let Some(parent) = self.get_parent() {
			parent.get_sibling()
		} else {
			None
		}
	}

	/// Returns a mutable reference to the uncle node.
	pub fn get_uncle_mut(&mut self) -> Option::<&mut Self> {
		if let Some(parent) = self.get_parent_mut() {
			parent.get_sibling_mut()
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
