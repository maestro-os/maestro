/// This module implements a binary tree container.

use core::cmp::Ordering;
use core::cmp::max;
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
use crate::memory::malloc;
use crate::util;

/// The color of a binary tree node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeColor {
	Red,
	Black,
}

/// TODO doc
struct BinaryTreeNode<T> {
	/// Pointer to the parent node
	parent: Option::<NonNull<Self>>,
	/// Pointer to the left child
	left: Option::<NonNull<Self>>,
	/// Pointer to the right child
	right: Option::<NonNull<Self>>,
	/// The color of the node
	color: NodeColor,

	value: T,
}

impl<T: 'static> BinaryTreeNode<T> {
	/// Creates a new node with the given `value`. The node is colored Red by default.
	pub fn new(value: T) -> Result::<NonNull::<Self>, ()> {
		let ptr = malloc::alloc(size_of::<Self>())? as *mut Self;
		let s = Self {
			parent: None,
			left: None,
			right: None,
			color: NodeColor::Red,

			value: value,
		};
		unsafe { // Call to unsafe function
			util::write_ptr(ptr, s);
		}
		Ok(NonNull::new(ptr).unwrap())
	}

	/// Tells whether the node is red.
	pub fn is_red(&self) -> bool {
		self.color == NodeColor::Red
	}

	/// Tells whether the node is black.
	pub fn is_black(&self) -> bool {
		self.color == NodeColor::Black
	}

	/// Unwraps the given pointer option into a reference option.
	fn unwrap_pointer(ptr: &Option::<NonNull::<Self>>) -> Option::<&'static Self> {
		if let Some(p) = ptr {
			unsafe { // Dereference of raw pointer
				Some(&*p.as_ptr())
			}
		} else {
			None
		}
	}

	/// Same as `unwrap_pointer` but returns a mutable reference.
	fn unwrap_pointer_mut(ptr: &mut Option::<NonNull::<Self>>) -> Option::<&'static mut Self> {
		if let Some(p) = ptr {
			unsafe { // Call to unsafe function
				Some(&mut *(p.as_ptr() as *mut _))
			}
		} else {
			None
		}
	}

	/// Returns a reference to the left child node.
	pub fn get_parent(&self) -> Option::<&'static Self> {
		Self::unwrap_pointer(&self.parent)
	}

	/// Returns a reference to the parent child node.
	pub fn get_parent_mut(&mut self) -> Option::<&'static mut Self> {
		Self::unwrap_pointer_mut(&mut self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	pub fn get_left(&self) -> Option::<&'static Self> {
		Self::unwrap_pointer(&self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_left_mut(&mut self) -> Option::<&'static mut Self> {
		Self::unwrap_pointer_mut(&mut self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_right(&self) -> Option::<&'static Self> {
		Self::unwrap_pointer(&self.right)
	}

	/// Returns a reference to the left child node.
	pub fn get_right_mut(&mut self) -> Option::<&'static mut Self> {
		Self::unwrap_pointer_mut(&mut self.right)
	}

	/// Returns a reference to the grandparent node.
	pub fn get_grandparent(&self) -> Option::<&'static Self> {
		if let Some(p) = self.get_parent() {
			p.get_parent()
		} else {
			None
		}
	}

	/// Returns a mutable reference to the grandparent node.
	pub fn get_grandparent_mut(&mut self) -> Option::<&'static mut Self> {
		if let Some(p) = self.get_parent_mut() {
			p.get_parent_mut()
		} else {
			None
		}
	}

	/// Returns a reference to the sibling node.
	pub fn get_sibling(&self) -> Option::<&'static Self> {
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
	pub fn get_sibling_mut(&mut self) -> Option::<&'static mut Self> {
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
	pub fn get_uncle(&mut self) -> Option::<&'static Self> {
		if let Some(parent) = self.get_parent() {
			parent.get_sibling()
		} else {
			None
		}
	}

	/// Returns a mutable reference to the uncle node.
	pub fn get_uncle_mut(&mut self) -> Option::<&'static mut Self> {
		if let Some(parent) = self.get_parent_mut() {
			parent.get_sibling_mut()
		} else {
			None
		}
	}

	/// Tells whether the node is a left child.
	pub fn is_left_child(&self) -> bool {
		// TODO
		false
	}

	/// Tells whether the node is a right child.
	pub fn is_right_child(&self) -> bool {
		// TODO
		false
	}

	/// Tells whether the node and its parent and grandparent form a triangle.
	pub fn is_triangle(&self) -> bool {
		// TODO
		false
	}

	/// Tells whether the node and its parent and grandparent form a line.
	pub fn is_line(&self) -> bool {
		// TODO
		false
	}

	/// Applies a left tree rotation with the current node as pivot.
	pub fn left_rotate(&mut self) {
		let root = self.parent;
		let root_ptr = unsafe { // Dereference of raw pointer
			&mut *(root.unwrap().as_ptr() as *mut Self)
		};
		let left = self.left;

		self.left = root;
		root_ptr.parent = NonNull::new(self);

		root_ptr.right = left;
		if left.is_some() {
			unsafe { // Dereference of raw pointer
				&mut *(left.unwrap().as_ptr() as *mut Self)
			}.parent = root;
		}
	}

	/// Applies a right tree rotation with the current node as pivot.
	pub fn right_rotate(&mut self) {
		let root = self.parent;
		let root_ptr = unsafe { // Dereference of raw pointer
			&mut *(root.unwrap().as_ptr() as *mut Self)
		};
		let right = self.right;

		self.right = root;
		root_ptr.parent = NonNull::new(self);

		root_ptr.left = right;
		if right.is_some() {
			unsafe { // Dereference of raw pointer
				&mut *(right.unwrap().as_ptr() as *mut Self)
			}.parent = root;
		}
	}

	/// Inserts the given node `node` to left of the current node.
	pub fn insert_left(&mut self, node: &mut BinaryTreeNode::<T>) {
		if let Some(n) = self.get_left_mut() {
			node.insert_left(n);
		}
		self.left = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Inserts the given node `node` to right of the current node.
	pub fn insert_right(&mut self, node: &mut BinaryTreeNode::<T>) {
		if let Some(n) = self.get_right_mut() {
			node.insert_right(n);
		}
		self.right = NonNull::new(node);
		node.parent = NonNull::new(self);
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

	/// Returns the depth of the node in the tree.
	pub fn get_node_depth(&self) -> usize {
		if let Some(p) = self.get_parent() {
			p.get_node_depth() + 1
		} else {
			0
		}
	}

	/// Returns the black depth of the node in the tree.
	pub fn get_node_black_depth(&self) -> usize {
		let parent = if let Some(p) = self.get_parent() {
			p.get_node_black_depth()
		} else {
			0
		};
		let curr = if self.is_black() {
			1
		} else {
			0
		};
		parent + curr
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

/// Specify the order in which the tree is traversed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalType {
	/// Accesses the data, then left child, then right child
	PreOrder,
	/// Accesses left child, then the data, then right child
	InOrder,
	/// Accesses right child, then the data, then left child
	ReverseInOrder,
	/// Accesses left child, then right child, then the data
	PostOrder,
}

/// TODO doc
pub struct BinaryTree<T: 'static> {
	/// The root node of the binary tree.
	root: Option::<NonNull<BinaryTreeNode::<T>>>,
}

impl<T: 'static> BinaryTree<T> {
	/// Creates a new binary tree.
	pub fn new() -> Self {
		Self {
			root: None,
		}
	}

	/// Returns a reference to the root node.
	fn get_root(&self) -> Option::<&BinaryTreeNode::<T>> {
		if let Some(r) = self.root.as_ref() {
			unsafe { // Call to unsafe function
				Some(r.as_ref())
			}
		} else {
			None
		}
	}

	/// Returns a mutable reference to the root node.
	fn get_root_mut(&mut self) -> Option::<&mut BinaryTreeNode::<T>> {
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

	/// Updates the root of the tree.
	/// `node` is a node of the tree.
	fn update_node(&mut self, node: &mut BinaryTreeNode::<T>) {
		let mut root = NonNull::new(node as *mut BinaryTreeNode::<T>);
		loop {
			let parent = unsafe { // Call to unsafe function
				root.unwrap().as_mut()
			}.parent;
			if parent.is_none() {
				break;
			}
			root = parent;
		}
		self.root = root;
	}

	/// Searches for a node with the given closure for comparison.
	/// `cmp` is the comparison function.
	fn get_node<F: Fn(&T) -> Ordering>(&mut self, cmp: F) -> Option::<&mut BinaryTreeNode::<T>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.value);
			if ord == Ordering::Less {
				node = n.get_left_mut();
			} else if ord == Ordering::Greater {
				node = n.get_right_mut();
			} else {
				return Some(n);
			}
		}

		None
	}

	/// Searches for a value with the given closure for comparison.
	/// `cmp` is the comparison function.
	pub fn get<F: Fn(&T) -> Ordering>(&mut self, cmp: F) -> Option::<&mut T> {
		if let Some(n) = self.get_node(cmp) {
			Some(&mut n.value)
		} else {
			None
		}
	}

	/// For value insertion, returns the parent node on which the value will be inserted.
	fn get_insert_node<F: Fn(&T) -> Ordering>(&mut self, cmp: F)
		-> Option::<&mut BinaryTreeNode::<T>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.value);
			let next = if ord == Ordering::Less {
				n.get_left_mut()
			} else if ord == Ordering::Greater {
				n.get_right_mut()
			} else {
				None
			};
			if next.is_none() {
				return Some(n);
			}
			node = next;
		}

		None
	}

	/// Equilibrates the tree after insertion of node `n`.
	fn insert_equilibrate(&mut self, n: &mut BinaryTreeNode::<T>) {
		let mut node = n;
		if node.parent.is_none() {
			node.color = NodeColor::Black;
			return;
		}

		loop {
			if let Some(parent) = node.get_parent_mut() {
				if let Some(grandparent) = node.get_grandparent_mut() {
					if let Some(uncle) = node.get_uncle_mut() {
						if uncle.is_red() {
							parent.color = NodeColor::Black;
							grandparent.color = NodeColor::Red;
							uncle.color = NodeColor::Black;

							node = grandparent;
							continue;
						}
					}

					if node.is_triangle() {
						if node.is_left_child() {
							parent.right_rotate();
						} else {
							parent.left_rotate();
						}

						node = parent;
						continue;
					}

					if node.is_line() {
						if node.is_left_child() {
							grandparent.right_rotate();
						} else {
							grandparent.left_rotate();
						}

						parent.color = NodeColor::Black;
						grandparent.color = NodeColor::Red;

						node = parent;
						continue;
					}
				}
			}

			break;
		}
	}

	/// Inserts a value in the tree.
	/// `value` is the value to insert.
	/// `cmp` is the comparison function.
	pub fn insert<F: Fn(&T, &T) -> Ordering>(&mut self, value: T, cmp: F) -> Result::<(), ()> {
		let mut node = BinaryTreeNode::new(value)?;
		let n = unsafe { // Call to unsafe function
			node.as_mut()
		};

		let parent = self.get_insert_node(| val | {
			cmp(&n.value, val)
		});

		if let Some(p) = parent {
			let order = cmp(&n.value, &p.value);
			if order == Ordering::Less {
				p.insert_left(n);
			} else {
				p.insert_right(n);
			}

			self.insert_equilibrate(n);
			self.update_node(n);
		} else {
			debug_assert!(self.root.is_none());
			self.root = Some(node);

			let n = unsafe { // Call to unsafe function
				node.as_mut()
			};
			self.insert_equilibrate(n);
			self.update_node(n);
		}

		Ok(())
	}

	/// Removes a value from the tree. If the value is present several times in the tree, only one
	/// node is removed.
	/// `value` is the value to remove.
	/// `cmp` is the comparison function.
	pub fn remove<F: Fn(&T) -> Ordering>(&mut self, _value: T, _cmp: F) {
		// TODO
	}

	/// Calls the given closure for every nodes in the subtree with root `root`.
	/// `traversal_type` defines the order in which the tree is traversed.
	fn foreach_nodes<F: FnMut(&BinaryTreeNode::<T>)>(root: &BinaryTreeNode::<T>, f: &mut F,
		traversal_type: TraversalType) {
		let (first, second) = if traversal_type == TraversalType::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};

		if traversal_type == TraversalType::PreOrder {
			f(root);
		}

		if let Some(mut n) = first {
			Self::foreach_nodes(unsafe { // Call to unsafe function
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::InOrder
			|| traversal_type == TraversalType::ReverseInOrder {
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes(unsafe { // Call to unsafe function
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::PostOrder {
			f(root);
		}
	}

	/// Calls the given closure for every nodes in the subtree with root `root`.
	/// `traversal_type` defines the order in which the tree is traversed.
	fn foreach_nodes_mut<F: FnMut(&mut BinaryTreeNode::<T>)>(root: &mut BinaryTreeNode::<T>,
		f: &mut F, traversal_type: TraversalType) {
		let (first, second) = if traversal_type == TraversalType::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};

		if traversal_type == TraversalType::PreOrder {
			f(root);
		}

		if let Some(mut n) = first {
			Self::foreach_nodes_mut(unsafe { // Call to unsafe function
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::InOrder
			|| traversal_type == TraversalType::ReverseInOrder {
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes_mut(unsafe { // Call to unsafe function
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::PostOrder {
			f(root);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach<F: FnMut(&T)>(&self, mut f: F, traversal_type: TraversalType) {
		if let Some(n) = self.root {
			Self::foreach_nodes(unsafe { // Call to unsafe function
				n.as_ref()
			}, &mut | n: &BinaryTreeNode::<T> | {
				f(&n.value);
			}, traversal_type);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach_mut<F: FnMut(&mut T)>(&mut self, mut f: F, traversal_type: TraversalType) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(unsafe { // Call to unsafe function
				n.as_mut()
			}, &mut | n: &mut BinaryTreeNode::<T> | {
				f(&mut n.value);
			}, traversal_type);
		}
	}
}

// TODO impl Clone?

impl<T: fmt::Display> fmt::Display for BinaryTree::<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(mut n) = self.root {
			Self::foreach_nodes(unsafe { // Call to unsafe function
				n.as_mut()
			}, &mut | n | {
				for _ in 0..n.get_node_depth() {
					let _ = write!(f, "\t");
				}

				let _ = write!(f, "{}\n", n.value);
			}, TraversalType::ReverseInOrder);
			Ok(())
		} else {
			write!(f, "<Empty tree>")
		}
	}
}

impl<T> Drop for BinaryTree::<T> {
	fn drop(&mut self) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(unsafe { // Call to unsafe function
				n.as_mut()
			}, &mut | n | {
				malloc::free(n as *mut _ as *mut _);
			}, TraversalType::PostOrder);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn binary_tree0() {
		let mut b = BinaryTree::<i32>::new();

		assert!(b.get(| val | {
			0.cmp(val)
		}).is_none());
	}

	#[test_case]
	fn binary_tree_insert0() {
		let mut b = BinaryTree::<i32>::new();

		b.insert(0, | v0, v1 | {
			v0.cmp(v1)
		}).unwrap();

		assert_eq!(*b.get(| val | {
			0.cmp(val)
		}).unwrap(), 0);
	}

	#[test_case]
	fn binary_tree_insert1() {
		let mut b = BinaryTree::<i32>::new();
		let cmp = | v0: &i32, v1: &i32 | {
			v0.cmp(v1)
		};

		for i in 0..10 {
			b.insert(i, cmp).unwrap();
		}

		for i in 0..10 {
			assert_eq!(*b.get(| val | {
				i.cmp(val)
			}).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert2() {
		let mut b = BinaryTree::<i32>::new();
		let cmp = | v0: &i32, v1: &i32 | {
			v0.cmp(v1)
		};

		for i in 0..10 {
			b.insert(i, cmp).unwrap();
			b.insert(-i, cmp).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(| val | {
				i.cmp(val)
			}).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_remove0() {
		let mut b = BinaryTree::<i32>::new();
		let cmp = | v0: &i32, v1: &i32 | {
			v0.cmp(v1)
		};

		for i in 0..10 {
			b.insert(i, cmp).unwrap();
			b.insert(-i, cmp).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(| val | {
				i.cmp(val)
			}).unwrap(), i);

			b.remove(i, | val | {
				i.cmp(val)
			});

			assert!(b.get(| val | {
				i.cmp(val)
			}).is_none());
		}
	}

	// TODO
}
