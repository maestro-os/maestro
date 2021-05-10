//! This module implements a binary tree container.

use core::cmp::Ordering;
use core::cmp::max;
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::FailableClone;
use crate::util;

// TODO Fix: an element in a tree might be at the wrong place after being modified by mutable
// reference
// TODO Use boxes for nodes?

/// The color of a binary tree node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeColor {
	Red,
	Black,
}

/// A node in the binary tree.
struct BinaryTreeNode<T> {
	/// Pointer to the parent node
	parent: Option<NonNull<Self>>,
	/// Pointer to the left child
	left: Option<NonNull<Self>>,
	/// Pointer to the right child
	right: Option<NonNull<Self>>,
	/// The color of the node
	color: NodeColor,

	value: T,
}

/// Unwraps the given pointer option into a reference option.
fn unwrap_pointer<T>(ptr: &Option::<NonNull::<BinaryTreeNode::<T>>>)
	-> Option::<&'static BinaryTreeNode::<T>> {
	if let Some(p) = ptr {
		unsafe {
			Some(&*p.as_ptr())
		}
	} else {
		None
	}
}

/// Same as `unwrap_pointer` but returns a mutable reference.
fn unwrap_pointer_mut<T>(ptr: &mut Option::<NonNull::<BinaryTreeNode::<T>>>)
	-> Option::<&'static mut BinaryTreeNode::<T>> {
	if let Some(p) = ptr {
		unsafe {
			Some(&mut *(p.as_ptr() as *mut _))
		}
	} else {
		None
	}
}

impl<T: 'static> BinaryTreeNode<T> {
	/// Creates a new node with the given `value`. The node is colored Red by default.
	pub fn new(value: T) -> Result<NonNull::<Self>, Errno> {
		let ptr = unsafe {
			malloc::alloc(size_of::<Self>())? as *mut Self
		};
		let s = Self {
			parent: None,
			left: None,
			right: None,
			color: NodeColor::Red,

			value: value,
		};
		unsafe {
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

	/// Returns a reference to the left child node.
	pub fn get_parent(&self) -> Option::<&'static Self> {
		unwrap_pointer(&self.parent)
	}

	/// Returns a reference to the parent child node.
	pub fn get_parent_mut(&mut self) -> Option::<&'static mut Self> {
		unwrap_pointer_mut(&mut self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	pub fn get_left(&self) -> Option::<&'static Self> {
		unwrap_pointer(&self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_left_mut(&mut self) -> Option::<&'static mut Self> {
		unwrap_pointer_mut(&mut self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_right(&self) -> Option::<&'static Self> {
		unwrap_pointer(&self.right)
	}

	/// Returns a reference to the left child node.
	pub fn get_right_mut(&mut self) -> Option::<&'static mut Self> {
		unwrap_pointer_mut(&mut self.right)
	}

	/// Tells whether the node is a left child.
	pub fn is_left_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_left() {
				return n as *const _ == self as *const _;
			}
		}

		false
	}

	/// Tells whether the node is a right child.
	pub fn is_right_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_right() {
				return n as *const _ == self as *const _;
			}
		}

		false
	}

	/// Tells whether the node and its parent and grandparent form a triangle.
	pub fn is_triangle(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(grandparent) = parent.get_parent() {
				return parent.is_left_child() != grandparent.is_left_child();
			}
		}

		false
	}

	/// Tells whether the node and its parent and grandparent form a line.
	pub fn is_line(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(grandparent) = parent.get_parent() {
				return parent.is_left_child() == grandparent.is_left_child();
			}
		}

		false
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
		if let Some(parent) = self.get_parent() {
			if self.is_left_child() {
				parent.get_right()
			} else {
				parent.get_left()
			}
		} else {
			None
		}
	}

	/// Returns a mutable reference to the sibling node.
	pub fn get_sibling_mut(&mut self) -> Option::<&'static mut Self> {
		if let Some(parent) = self.get_parent_mut() {
			if self.is_left_child() {
				parent.get_right_mut()
			} else {
				parent.get_left_mut()
			}
		} else {
			None
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

	/// Applies a left tree rotation with the current node as pivot.
	pub fn left_rotate(&mut self) {
		let root = self.get_parent_mut();
		let root_ptr = unsafe {
			&mut *(root.unwrap() as *mut Self)
		};
		let left = self.left;

		self.left = NonNull::new(root_ptr);
		root_ptr.parent = NonNull::new(self);

		root_ptr.right = left;
		if left.is_some() {
			unsafe {
				&mut *(left.unwrap().as_ptr() as *mut Self)
			}.parent = NonNull::new(root_ptr);
		}
	}

	/// Applies a right tree rotation with the current node as pivot.
	pub fn right_rotate(&mut self) {
		let root = self.get_parent_mut();
		let root_ptr = unsafe {
			&mut *(root.unwrap() as *mut Self)
		};
		let right = self.right;

		self.right = NonNull::new(root_ptr);
		root_ptr.parent = NonNull::new(self);

		root_ptr.left = right;
		if right.is_some() {
			unsafe {
				&mut *(right.unwrap().as_ptr() as *mut Self)
			}.parent = NonNull::new(root_ptr);
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

	/// Unlinks the node from its tree.
	pub fn unlink(&mut self) {
		if let Some(parent) = self.get_parent_mut() {
			if self.is_left_child() {
				parent.left = None;
			} else if self.is_right_child() {
				parent.right = None;
			}
			self.parent = None;
		}

		if let Some(left) = self.get_left_mut() {
			if let Some(p) = left.parent {
				if p.as_ptr() == self as _ {
					left.parent = None;
				}
			}
			self.left = None;
		}

		if let Some(right) = self.get_right_mut() {
			if let Some(p) = right.parent {
				if p.as_ptr() == self as _ {
					right.parent = None;
				}
			}
			self.right = None;
		}
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

/// A binary tree is a structure which allows, when properly balanced, to performs actions
/// (insertion, removal, searching) in O(log n) complexity.
pub struct BinaryTree<T: 'static> {
	/// The root node of the binary tree.
	root: Option<NonNull<BinaryTreeNode<T>>>,
}

impl<T: 'static + Ord> BinaryTree<T> {
	/// Creates a new binary tree.
	pub const fn new() -> Self {
		Self {
			root: None,
		}
	}

	/// Tells whether the tree is empty.
	pub fn is_empty(&self) -> bool {
		self.root.is_none()
	}

	/// Returns a reference to the root node.
	fn get_root(&self) -> Option::<&BinaryTreeNode::<T>> {
		if let Some(r) = self.root.as_ref() {
			unsafe {
				Some(r.as_ref())
			}
		} else {
			None
		}
	}

	/// Returns a mutable reference to the root node.
	fn get_root_mut(&mut self) -> Option::<&mut BinaryTreeNode::<T>> {
		if let Some(r) = self.root.as_mut() {
			unsafe {
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

	/// Searches for a node with the given value in the tree and returns a reference.
	/// `val` is the value to find.
	fn get_node<T_: 'static>(&self, val: T_) -> Option::<&BinaryTreeNode::<T>>
		where T: PartialOrd<T_> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.value.partial_cmp(&val).unwrap().reverse();
			if ord == Ordering::Less {
				node = n.get_left();
			} else if ord == Ordering::Greater {
				node = n.get_right();
			} else {
				return Some(n);
			}
		}

		None
	}

	/// Searches for a node with the given value in the tree and returns a mutable reference.
	/// `val` is the value to find.
	fn get_mut_node<T_: 'static>(&mut self, val: T_) -> Option::<&mut BinaryTreeNode::<T>>
		where T: PartialOrd<T_> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.value.partial_cmp(&val).unwrap().reverse();
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

	/// Searches for the given value in the tree and returns a reference.
	/// `val` is the value to find.
	pub fn get<T_: 'static>(&self, val: T_) -> Option::<&T> where T: PartialOrd<T_> {
		if let Some(n) = self.get_node(val) {
			Some(&n.value)
		} else {
			None
		}
	}

	/// Searches for the given value in the tree and returns a mutable reference.
	/// `val` is the value to find.
	pub fn get_mut<T_: 'static>(&mut self, val: T_) -> Option::<&mut T> where T: PartialOrd<T_> {
		if let Some(n) = self.get_mut_node(val) {
			Some(&mut n.value)
		} else {
			None
		}
	}

	/// Searches for a node in the tree using the given comparison function `cmp` instead of the
	/// Ord trait.
	pub fn cmp_get<F: Fn(&T) -> Ordering>(&mut self, cmp: F) -> Option::<&mut T> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.value).reverse();
			if ord == Ordering::Less {
				node = n.get_left_mut();
			} else if ord == Ordering::Greater {
				node = n.get_right_mut();
			} else {
				return Some(&mut n.value);
			}
		}

		None
	}

	/// Searches in the tree for a value greater or equal to the given value.
	/// `val` is the value to find.
	pub fn get_min<T_: 'static>(&mut self, val: T_) -> Option::<&mut T> where T: PartialOrd<T_> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.value.partial_cmp(&val).unwrap().reverse();
			if ord == Ordering::Greater {
				node = n.get_right_mut();
			} else {
				return Some(&mut n.value);
			}
		}

		None
	}

	// TODO get_max?

	/// Updates the root of the tree.
	/// `node` is a node of the tree.
	fn update_root(&mut self, node: &mut BinaryTreeNode::<T>) {
		let mut root = NonNull::new(node as *mut BinaryTreeNode::<T>);
		loop {
			let parent = unsafe {
				root.unwrap().as_mut()
			}.parent;
			if parent.is_none() {
				break;
			}
			root = parent;
		}
		self.root = root;
	}

	/// For value insertion, returns the parent node on which the value will be inserted.
	fn get_insert_node(&mut self, val: &T) -> Option::<&mut BinaryTreeNode::<T>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = val.cmp(&n.value);
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

	// TODO Fix
	/// Equilibrates the tree after insertion of node `n`.
	fn insert_equilibrate(&mut self, n: &mut BinaryTreeNode::<T>) {
		let mut node = n;
		while let Some(parent) = node.get_parent_mut() {
			if parent.is_black() {
				break;
			}

			let grandparent = parent.get_parent_mut().unwrap();
			if let Some(uncle) = node.get_uncle_mut() {
				if uncle.is_red() {
					parent.color = NodeColor::Black;
					uncle.color = NodeColor::Black;
					grandparent.color = NodeColor::Red;

					node = grandparent;
					continue;
				}
			}

			if parent.is_left_child() {
				if node.is_right_child() {
					node.left_rotate();
					node = parent;
				}
			} else {
				if node.is_left_child() {
					node.right_rotate();
					node = parent;
				}
			}

			let parent = node.get_parent_mut().unwrap();
			parent.color = NodeColor::Black;
			let grandparent = parent.get_parent_mut().unwrap();
			grandparent.color = NodeColor::Red;
		}
	}

	/// Inserts a value in the tree and returns a mutable reference to it.
	/// `val` is the value to insert.
	/// `cmp` is the comparison function.
	pub fn insert(&mut self, val: T) -> Result<(), Errno> {
		let mut node = BinaryTreeNode::new(val)?;
		let n = unsafe {
			node.as_mut()
		};

		if let Some(p) = self.get_insert_node(&n.value) {
			let order = n.value.cmp(&p.value);
			if order == Ordering::Less {
				p.insert_left(n);
			} else {
				p.insert_right(n);
			}

			// TODO self.insert_equilibrate(n);
			self.update_root(n);
		} else {
			debug_assert!(self.root.is_none());
			self.root = Some(node);

			let n = unsafe {
				node.as_mut()
			};
			// TODO self.insert_equilibrate(n);
			self.update_root(n);
		}
		unsafe {
			self.root.unwrap().as_mut()
		}.color = NodeColor::Black;

		#[cfg(config_debug_debug)]
		self.check();
		Ok(())
	}

	/// Returns the leftmost node in the tree.
	fn get_leftmost_node<T_: 'static>(node: &'static mut BinaryTreeNode::<T>)
		-> &'static mut BinaryTreeNode::<T> where T: PartialOrd<T_> {
		let mut n = node;
		while let Some(left) = n.get_left_mut() {
			n = left;
		}
		n
	}

	// TODO Clean
	/// Removes a value from the tree. If the value is present several times in the tree, only one
	/// node is removed.
	/// `val` is the value to select the node to remove.
	pub fn remove<T_: 'static>(&mut self, val: T_) where T: PartialOrd<T_> {
		if let Some(node) = self.get_mut_node(val) {
			let left = node.get_left_mut();
			let right = node.get_right_mut();

			let replacement: Option::<NonNull::<BinaryTreeNode::<T>>>
				= if left.is_some() && right.is_some() {
				let leftmost = Self::get_leftmost_node::<T_>(right.unwrap());
				leftmost.unlink();
				NonNull::new(leftmost as *mut _)
			} else if left.is_some() {
				NonNull::new(left.unwrap() as *mut _)
			} else if right.is_some() {
				NonNull::new(right.unwrap() as *mut _)
			} else {
				None
			};

			if let Some(mut r) = replacement {
				unsafe {
					r.as_mut()
				}.parent = node.parent;
			}

			if let Some(parent) = node.get_parent_mut() {
				*if node.is_left_child() {
					&mut parent.left
				} else {
					&mut parent.right
				} = replacement;

				node.unlink();
				unsafe {
					malloc::free(node as *mut _ as *mut _);
				}
			} else {
				node.unlink();
				unsafe {
					malloc::free(node as *mut _ as *mut _);
				}

				self.root = replacement;
			}
		}

		#[cfg(config_debug_debug)]
		self.check();
	}
}

impl<T: 'static> BinaryTree::<T> {
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
			Self::foreach_nodes(unsafe {
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::InOrder
			|| traversal_type == TraversalType::ReverseInOrder {
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes(unsafe {
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
			Self::foreach_nodes_mut(unsafe {
				n.as_mut()
			}, f, traversal_type);
		}

		if traversal_type == TraversalType::InOrder
			|| traversal_type == TraversalType::ReverseInOrder {
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes_mut(unsafe {
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
			Self::foreach_nodes(unsafe {
				n.as_ref()
			}, &mut | n: &BinaryTreeNode::<T> | {
				f(&n.value);
			}, traversal_type);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach_mut<F: FnMut(&mut T)>(&mut self, mut f: F, traversal_type: TraversalType) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(unsafe {
				n.as_mut()
			}, &mut | n: &mut BinaryTreeNode::<T> | {
				f(&mut n.value);
			}, traversal_type);
		}
	}

	/// Checks the integrity of the tree. If the tree is invalid, the function makes the kernel
	/// panic. This function is available only in debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		if let Some(root) = self.root {
			Self::foreach_nodes(unsafe {
				root.as_ref()
			}, &mut | n: &BinaryTreeNode::<T> | {
				if let Some(left) = n.get_left() {
					debug_assert!(left.get_parent().unwrap() as *const _ == n as *const _);
				}
				if let Some(right) = n.get_right() {
					debug_assert!(right.get_parent().unwrap() as *const _ == n as *const _);
				}
			}, TraversalType::PreOrder);
		}
	}

	/// Returns a mutable iterator for the current binary tree.
	pub fn iter_mut(&mut self) -> BinaryTreeMutIterator::<T> {
		BinaryTreeMutIterator::new(self)
	}
}

/// An iterator for the BinaryTree structure. This iterator traverses the tree in pre order.
pub struct BinaryTreeIterator<'a, T: 'static> {
	/// The binary tree to iterate into.
	tree: &'a BinaryTree::<T>,
	/// The current node of the iterator.
	node: Option::<NonNull::<BinaryTreeNode::<T>>>,
}

impl<'a, T> BinaryTreeIterator<'a, T> {
	/// Creates a binary tree iterator for the given reference.
	fn new(tree: &'a BinaryTree::<T>) -> Self {
		BinaryTreeIterator {
			tree: tree,
			node: tree.root,
		}
	}
}

impl<'a, T: Ord> Iterator for BinaryTreeIterator<'a, T> {
	type Item = &'a T;

	// TODO Implement every functions?

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.node;
		self.node = {
			if self.node.is_none() {
				return None;
			}

			let n = unwrap_pointer(&node).unwrap();
			if let Some(left) = n.get_left() {
				NonNull::new(left as *const _ as *mut _)
			} else if let Some(right) = n.get_right() {
				NonNull::new(right as *const _ as *mut _)
			} else {
				let mut n = n;
				while n.is_right_child() {
					n = n.get_parent().unwrap();
				}

				if n.is_left_child() {
					if let Some(sibling) = n.get_sibling() {
						NonNull::new(sibling as *const _ as *mut _)
					} else {
						None
					}
				} else {
					None
				}
			}
		};

		if let Some(node) = unwrap_pointer(&node) {
			Some(&node.value)
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.tree.nodes_count()
	}
}

impl<'a, T: Ord> IntoIterator for &'a BinaryTree<T> {
	type Item = &'a T;
	type IntoIter = BinaryTreeIterator<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		BinaryTreeIterator::new(&self)
	}
}

/// An iterator for the BinaryTree structure. This iterator traverses the tree in pre order.
pub struct BinaryTreeMutIterator<'a, T: 'static> {
	/// The binary tree to iterate into.
	tree: &'a mut BinaryTree::<T>,
	/// The current node of the iterator.
	node: Option::<NonNull::<BinaryTreeNode::<T>>>,
}

impl<'a, T> BinaryTreeMutIterator<'a, T> {
	/// Creates a binary tree iterator for the given reference.
	fn new(tree: &'a mut BinaryTree::<T>) -> Self {
		let root = tree.root;
		BinaryTreeMutIterator {
			tree: tree,
			node: root,
		}
	}
}

impl<'a, T: Ord> Iterator for BinaryTreeMutIterator<'a, T> {
	type Item = &'a mut T;

	// TODO Implement every functions?

	fn next(&mut self) -> Option<Self::Item> {
		let mut node = self.node;
		self.node = {
			if self.node.is_none() {
				return None;
			}

			let n = unwrap_pointer(&node).unwrap();
			if let Some(left) = n.get_left() {
				NonNull::new(left as *const _ as *mut _)
			} else if let Some(right) = n.get_right() {
				NonNull::new(right as *const _ as *mut _)
			} else {
				let mut n = n;
				while n.is_right_child() {
					n = n.get_parent().unwrap();
				}

				if n.is_left_child() {
					if let Some(sibling) = n.get_sibling() {
						NonNull::new(sibling as *const _ as *mut _)
					} else {
						None
					}
				} else {
					None
				}
			}
		};

		if let Some(node) = unwrap_pointer_mut(&mut node) {
			Some(&mut node.value)
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.tree.nodes_count()
	}
}

impl<T: FailableClone + Ord> FailableClone for BinaryTree::<T> {
	fn failable_clone(&self) -> Result<Self, Errno> {
		let mut new = Self::new();
		for n in self {
			new.insert(n.failable_clone()?)?;
		}
		Ok(new)
	}
}

impl<T: fmt::Display> fmt::Display for BinaryTree::<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(mut n) = self.root {
			Self::foreach_nodes(unsafe {
				n.as_mut()
			}, &mut | n | {
				for _ in 0..n.get_node_depth() {
					let _ = write!(f, "\t");
				}

				let color = if n.color == NodeColor::Red {
					"red"
				} else {
					"black"
				};
				let _ = writeln!(f, "{} ({})", n.value, color);
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
			Self::foreach_nodes_mut(unsafe {
				n.as_mut()
			}, &mut | n | {
				unsafe {
					drop_in_place(&mut n.value);
					malloc::free(n as *mut _ as *mut _);
				}
			}, TraversalType::PostOrder);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn binary_tree0() {
		let b = BinaryTree::<i32>::new();
		assert!(b.get(0).is_none());
	}

	#[test_case]
	fn binary_tree_insert0() {
		let mut b = BinaryTree::<i32>::new();

		b.insert(0).unwrap();
		assert_eq!(*b.get(0).unwrap(), 0);
	}

	#[test_case]
	fn binary_tree_insert1() {
		let mut b = BinaryTree::<i32>::new();

		for i in 0..10 {
			b.insert(i).unwrap();
		}

		for i in 0..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert2() {
		let mut b = BinaryTree::<i32>::new();

		for i in -9..10 {
			b.insert(i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_remove0() {
		let mut b = BinaryTree::<i32>::new();

		for i in -9..10 {
			b.insert(i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove1() {
		let mut b = BinaryTree::<i32>::new();

		for i in -9..10 {
			b.insert(i).unwrap();
		}

		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove2() {
		let mut b = BinaryTree::<i32>::new();

		for i in (-9..10).rev() {
			b.insert(i).unwrap();
		}

		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove3() {
		let mut b = BinaryTree::<i32>::new();

		for i in (-9..10).rev() {
			b.insert(i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove4() {
		let mut b = BinaryTree::<i32>::new();

		for i in -9..10 {
			b.insert(i).unwrap();
			b.remove(i);
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove5() {
		let mut b = BinaryTree::<i32>::new();

		for i in -9..10 {
			b.insert(i).unwrap();
		}

		for i in -9..10 {
			if i % 2 == 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				b.remove(i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(!b.is_empty());

		for i in -9..10 {
			if i % 2 != 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				b.remove(i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_get_min0() {
		let mut b = BinaryTree::<i32>::new();
		assert!(b.get_min(0).is_none());
	}

	#[test_case]
	fn binary_tree_get_min1() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(0).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min2() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(0).unwrap();
		assert!(b.get_min(1).is_none());
	}

	#[test_case]
	fn binary_tree_get_min3() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(-1).unwrap();
		b.insert(0).unwrap();
		b.insert(1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min4() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(0).unwrap();
		b.insert(1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min5() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min6() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(-1).unwrap();
		b.insert(1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_foreach0() {
		let b = BinaryTree::<i32>::new();
		b.foreach(| _ | {
			assert!(false);
		}, TraversalType::PreOrder);
	}

	#[test_case]
	fn binary_tree_foreach1() {
		let mut b = BinaryTree::<i32>::new();
		b.insert(0).unwrap();

		let mut passed = false;
		b.foreach(| val | {
			assert_eq!(passed, false);
			assert_eq!(*val, 0);
			passed = true;
		}, TraversalType::PreOrder);
	}

	// TODO More foreach tests
}
