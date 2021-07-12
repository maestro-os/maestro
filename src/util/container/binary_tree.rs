//! This module implements a binary tree container.

use core::cmp::Ordering;
use core::cmp::max;
use core::fmt;
use core::mem::size_of;
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use core::ptr;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::memory;
use crate::util::FailableClone;

#[cfg(config_debug_debug)]
use core::ffi::c_void;
#[cfg(config_debug_debug)]
use crate::util::container::vec::Vec;

/// The color of a binary tree node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeColor {
	Red,
	Black,
}

/// A node in the binary tree.
struct BinaryTreeNode<K, V> {
	/// Pointer to the parent node
	parent: Option<NonNull<Self>>,
	/// Pointer to the left child
	left: Option<NonNull<Self>>,
	/// Pointer to the right child
	right: Option<NonNull<Self>>,
	/// The color of the node
	color: NodeColor,

	/// The node's key.
	key: K,
	/// The node's value.
	value: V,
}

/// Unwraps the given pointer option into a reference option.
fn unwrap_pointer<K, V>(ptr: &Option<NonNull<BinaryTreeNode<K, V>>>)
	-> Option<&'static BinaryTreeNode<K, V>> {
	if let Some(p) = ptr {
		unsafe {
			debug_assert!(p.as_ptr() as usize >= memory::PROCESS_END as usize);
			Some(&*p.as_ptr())
		}
	} else {
		None
	}
}

/// Same as `unwrap_pointer` but returns a mutable reference.
fn unwrap_pointer_mut<K, V>(ptr: &mut Option<NonNull<BinaryTreeNode<K, V>>>)
	-> Option<&'static mut BinaryTreeNode<K, V>> {
	if let Some(p) = ptr {
		unsafe {
			debug_assert!(p.as_ptr() as usize >= memory::PROCESS_END as usize);
			Some(&mut *(p.as_ptr() as *mut _))
		}
	} else {
		None
	}
}

impl<K: 'static + Ord, V: 'static> BinaryTreeNode<K, V> {
	/// Creates a new node with the given `value`. The node is colored Red by default.
	pub fn new(key: K, value: V) -> Result<NonNull<Self>, Errno> {
		let ptr = unsafe {
			malloc::alloc(size_of::<Self>())? as *mut Self
		};
		let s = Self {
			parent: None,
			left: None,
			right: None,
			color: NodeColor::Red,

			key,
			value,
		};

		debug_assert!(ptr as usize >= memory::PROCESS_END as usize);
		unsafe { // Safe because the pointer is valid
			ptr::write_volatile(ptr, s);
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
	pub fn get_parent(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.parent)
	}

	/// Returns a reference to the parent child node.
	pub fn get_parent_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	pub fn get_left(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_left_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.left)
	}

	/// Returns a reference to the left child node.
	pub fn get_right(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.right)
	}

	/// Returns a reference to the left child node.
	pub fn get_right_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.right)
	}

	/// Tells whether the node is a left child.
	pub fn is_left_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_left() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Tells whether the node is a right child.
	pub fn is_right_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_right() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Tells whether the node and its parent and grandparent form a triangle.
	pub fn is_triangle(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			return self.is_left_child() != parent.is_left_child();
		}

		false
	}

	/// Returns a reference to the grandparent node.
	pub fn get_grandparent(&self) -> Option<&'static Self> {
		self.get_parent()?.get_parent()
	}

	/// Returns a mutable reference to the grandparent node.
	pub fn get_grandparent_mut(&mut self) -> Option<&'static mut Self> {
		self.get_parent_mut()?.get_parent_mut()
	}

	/// Returns a reference to the sibling node.
	pub fn get_sibling(&self) -> Option<&'static Self> {
		let parent = self.get_parent()?;

		if self.is_left_child() {
			parent.get_right()
		} else {
			parent.get_left()
		}
	}

	/// Returns a mutable reference to the sibling node.
	pub fn get_sibling_mut(&mut self) -> Option<&'static mut Self> {
		let parent = self.get_parent_mut()?;

		if self.is_left_child() {
			parent.get_right_mut()
		} else {
			parent.get_left_mut()
		}
	}

	/// Returns a reference to the uncle node.
	pub fn get_uncle(&mut self) -> Option<&'static Self> {
		self.get_parent()?.get_sibling()
	}

	/// Returns a mutable reference to the uncle node.
	pub fn get_uncle_mut(&mut self) -> Option<&'static mut Self> {
		self.get_parent_mut()?.get_sibling_mut()
	}

	/// Applies a left tree rotation with the current node as pivot.
	/// If the current node doesn't have a parent, the behaviour is undefined.
	pub fn left_rotate(&mut self) {
		let root = self.get_parent_mut().unwrap();
		let left = self.left;

		if let Some(mut p) = root.parent {
			let p = unsafe {
				p.as_mut()
			};

			if root.is_right_child() {
				p.right = NonNull::new(self);
			} else {
				p.left = NonNull::new(self);
			}
		}
		self.parent = root.parent;

		self.left = NonNull::new(root);
		root.parent = NonNull::new(self);

		root.right = left;
		if let Some(mut left) = left {
			unsafe {
				left.as_mut()
			}.parent = NonNull::new(root);
		}
	}

	/// Applies a right tree rotation with the current node as pivot.
	/// If the current node doesn't have a parent, the behaviour is undefined.
	pub fn right_rotate(&mut self) {
		let root = self.get_parent_mut().unwrap();
		let right = self.right;

		if let Some(mut p) = root.parent {
			let p = unsafe {
				p.as_mut()
			};

			if root.is_right_child() {
				p.right = NonNull::new(self);
			} else {
				p.left = NonNull::new(self);
			}
		}
		self.parent = root.parent;

		self.right = NonNull::new(root);
		root.parent = NonNull::new(self);

		root.left = right;
		if let Some(mut right) = right {
			unsafe {
				right.as_mut()
			}.parent = NonNull::new(root);
		}
	}

	/// Inserts the given node `node` to left of the current node.
	pub fn insert_left(&mut self, node: &mut BinaryTreeNode<K, V>) {
		if let Some(n) = self.get_left_mut() {
			node.insert_left(n);
		} else {
			self.left = NonNull::new(node);
			node.parent = NonNull::new(self);
		}
	}

	/// Inserts the given node `node` to right of the current node.
	pub fn insert_right(&mut self, node: &mut BinaryTreeNode<K, V>) {
		if let Some(n) = self.get_right_mut() {
			node.insert_right(n);
		} else {
			self.right = NonNull::new(node);
			node.parent = NonNull::new(self);
		}
	}

	/// Returns the number of nodes in the subtree.
	pub fn nodes_count(&self) -> usize {
		let left_count = {
			if let Some(l) = self.get_left() {
				l.nodes_count()
			} else {
				0
			}
		};
		let right_count = {
			if let Some(r) = self.get_right() {
				r.nodes_count()
			} else {
				0
			}
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
		let parent = {
			if let Some(p) = self.get_parent() {
				p.get_node_black_depth()
			} else {
				0
			}
		};
		let curr = {
			if self.is_black() {
				1
			} else {
				0
			}
		};

		parent + curr
	}

	/// Returns the depth of the subtree.
	pub fn get_depth(&self) -> usize {
		let left_count = {
			if let Some(l) = self.get_left() {
				l.get_depth()
			} else {
				0
			}
		};
		let right_count = {
			if let Some(r) = self.get_right() {
				r.get_depth()
			} else {
				0
			}
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
				if ptr::eq(p.as_ptr(), self as _) {
					left.parent = None;
				}
			}

			self.left = None;
		}

		if let Some(right) = self.get_right_mut() {
			if let Some(p) = right.parent {
				if ptr::eq(p.as_ptr(), self as _) {
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
pub struct BinaryTree<K: 'static + Ord, V: 'static> {
	/// The root node of the binary tree.
	root: Option<NonNull<BinaryTreeNode<K, V>>>,
}

impl<K: 'static + Ord, V: 'static> BinaryTree<K, V> {
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
	fn get_root(&self) -> Option<&BinaryTreeNode<K, V>> {
		unsafe {
			Some(self.root.as_ref()?.as_ref())
		}
	}

	/// Returns a mutable reference to the root node.
	fn get_root_mut(&mut self) -> Option<&mut BinaryTreeNode<K, V>> {
		unsafe {
			Some(self.root.as_mut()?.as_mut())
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

	/// Searches for a node with the given key in the tree and returns a reference.
	/// `key` is the key to find.
	fn get_node(&self, key: K) -> Option<&BinaryTreeNode<K, V>> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(&key).unwrap().reverse();

			match ord {
				Ordering::Less => node = n.get_left(),
				Ordering::Greater => node = n.get_right(),
				Ordering::Equal => return Some(n),
			}
		}

		None
	}

	/// Searches for a node with the given key in the tree and returns a mutable reference.
	/// `key` is the key to find.
	fn get_mut_node(&mut self, key: K) -> Option<&mut BinaryTreeNode<K, V>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(&key).unwrap().reverse();

			match ord {
				Ordering::Less => node = n.get_left_mut(),
				Ordering::Greater => node = n.get_right_mut(),
				Ordering::Equal => return Some(n),
			}
		}

		None
	}

	/// Searches for the given key in the tree and returns a reference.
	/// `key` is the key to find.
	pub fn get(&self, key: K) -> Option<&V> {
		let node = self.get_node(key)?;
		Some(&node.value)
	}

	/// Searches for the given key in the tree and returns a mutable reference.
	/// `key` is the key to find.
	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		let node = self.get_mut_node(key)?;
		Some(&mut node.value)
	}

	/// Searches for a node in the tree using the given comparison function `cmp` instead of the
	/// Ord trait.
	pub fn cmp_get<F: Fn(&K, &V) -> Ordering>(&mut self, cmp: F) -> Option<&mut V> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.key, &n.value).reverse();

			match ord {
				Ordering::Less => node = n.get_left_mut(),
				Ordering::Greater => node = n.get_right_mut(),
				Ordering::Equal => return Some(&mut n.value),
			}
		}

		None
	}

	/// Searches in the tree for a key greater or equal to the given key.
	/// `key` is the key to find.
	pub fn get_min(&mut self, key: K) -> Option<&mut V> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(&key).unwrap().reverse();

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
	fn update_root(&mut self, node: &mut BinaryTreeNode<K, V>) {
		let mut root = NonNull::new(node as *mut BinaryTreeNode<K, V>);

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

	/// For node insertion, returns the parent node on which it will be inserted.
	fn get_insert_node(&mut self, key: &K) -> Option<&mut BinaryTreeNode<K, V>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = key.cmp(&n.key);

			let next = match ord {
				Ordering::Less => n.get_left_mut(),
				Ordering::Greater => n.get_right_mut(),
				_ => None,
			};

			if next.is_none() {
				return Some(n);
			}
			node = next;
		}

		None
	}

	/// Equilibrates the tree after insertion of node `n`.
	fn insert_equilibrate(&mut self, n: &mut BinaryTreeNode<K, V>) {
		let mut node = n;

		if let Some(parent) = node.get_parent_mut() {
			if parent.is_black() {
				return;
			}

			// The node's parent exists and is red
			if let Some(uncle) = node.get_uncle_mut() {
				if uncle.is_red() {
					let grandparent = parent.get_parent_mut().unwrap();
					parent.color = NodeColor::Black;
					uncle.color = NodeColor::Black;
					grandparent.color = NodeColor::Red;

					self.insert_equilibrate(grandparent);
					return;
				}
			}

			if node.is_triangle() {
				if parent.is_right_child() {
					node.right_rotate();
				} else {
					node.left_rotate();
				}

				node = parent;
			}

			let parent = node.get_parent_mut().unwrap();
			let grandparent = parent.get_parent_mut().unwrap();

			if node.is_right_child() {
				parent.left_rotate();
			} else {
				parent.right_rotate();
			}

			parent.color = NodeColor::Black;
			grandparent.color = NodeColor::Red;
		} else {
			node.color = NodeColor::Black;
		}
	}

	/// Inserts a key/value pair in the tree and returns a mutable reference to the value.
	/// `key` is the key to insert.
	/// `val` is the value to insert.
	/// `cmp` is the comparison function.
	pub fn insert(&mut self, key: K, val: V) -> Result<&mut V, Errno> {
		let mut node = BinaryTreeNode::new(key, val)?;
		let n = unsafe {
			node.as_mut()
		};

		if let Some(p) = self.get_insert_node(&n.key) {
			let order = n.key.cmp(&p.key);
			if order == Ordering::Less {
				p.insert_left(n);
			} else {
				p.insert_right(n);
			}
		} else {
			debug_assert!(self.root.is_none());
			self.root = Some(node);
		}
		self.insert_equilibrate(n);
		self.update_root(n);

		#[cfg(config_debug_debug)]
		self.check();
		Ok(&mut n.value)
	}

	/// Returns the leftmost node in the tree.
	fn get_leftmost_node(node: &'static mut BinaryTreeNode<K, V>)
		-> &'static mut BinaryTreeNode<K, V> {
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
	/// If the key exists, the function returns the value of the removed node.
	pub fn remove(&mut self, key: K) -> Option<V> {
		let value = {
			if let Some(node) = self.get_mut_node(key) {
				let left = node.get_left_mut();
				let right = node.get_right_mut();

				let replacement: Option<NonNull<BinaryTreeNode<K, V>>> = {
					if left.is_some() && right.is_some() {
						let leftmost = Self::get_leftmost_node(right.unwrap());
						leftmost.unlink();
						NonNull::new(leftmost as *mut _)
					} else if let Some(left) = left {
						NonNull::new(left as *mut _)
					} else if let Some(right) = right {
						NonNull::new(right as *mut _)
					} else {
						None
					}
				};

				if let Some(mut r) = replacement {
					unsafe {
						r.as_mut()
					}.parent = node.parent;
				}

				let value = unsafe { // Safe because the pointer is valid
					ptr::read(&node.value)
				};

				if let Some(parent) = node.get_parent_mut() {
					if node.is_left_child() {
						parent.left = replacement;
					} else {
						parent.right = replacement;
					}

					node.unlink();
					unsafe {
						drop_in_place(node);
						malloc::free(node as *mut _ as *mut _);
					}
				} else {
					node.unlink();
					unsafe {
						drop_in_place(node);
						malloc::free(node as *mut _ as *mut _);
					}

					self.root = replacement;
				}

				Some(value)
			} else {
				None
			}
		};

		#[cfg(config_debug_debug)]
		self.check();

		value
	}
}

impl<K: 'static + Ord, V: 'static> BinaryTree<K, V> {
	/// Calls the given closure for every nodes in the subtree with root `root`.
	/// `traversal_type` defines the order in which the tree is traversed.
	fn foreach_nodes<F: FnMut(&BinaryTreeNode<K, V>)>(root: &BinaryTreeNode<K, V>, f: &mut F,
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
	fn foreach_nodes_mut<F: FnMut(&mut BinaryTreeNode<K, V>)>(root: &mut BinaryTreeNode<K, V>,
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
	pub fn foreach<F: FnMut(&K, &V)>(&self, mut f: F, traversal_type: TraversalType) {
		if let Some(n) = self.root {
			Self::foreach_nodes(unsafe {
				n.as_ref()
			}, &mut | n: &BinaryTreeNode<K, V> | {
				f(&n.key, &n.value);
			}, traversal_type);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach_mut<F: FnMut(&K, &mut V)>(&mut self, mut f: F, traversal_type: TraversalType) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(unsafe {
				n.as_mut()
			}, &mut | n: &mut BinaryTreeNode<K, V> | {
				f(&n.key, &mut n.value);
			}, traversal_type);
		}
	}

	/// Checks the integrity of the tree. If the tree is invalid, the function makes the kernel
	/// panic. This function is available only in debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		if let Some(root) = self.root {
			let mut expored_nodes = Vec::<*const c_void>::new();

			Self::foreach_nodes(unsafe {
				root.as_ref()
			}, &mut | n: &BinaryTreeNode<K, V> | {
				assert!(n as *const _ as usize >= memory::PROCESS_END as usize);

				for e in expored_nodes.iter() {
					assert_ne!(*e, n as *const _ as *const c_void);
				}
				expored_nodes.push(n as *const _ as *const c_void).unwrap();

				if let Some(left) = n.get_left() {
					assert!(left as *const _ as usize >= memory::PROCESS_END as usize);
					assert!(ptr::eq(left.get_parent().unwrap() as *const _, n as *const _));
					assert!(left.key <= n.key);
				}

				if let Some(right) = n.get_right() {
					assert!(right as *const _ as usize >= memory::PROCESS_END as usize);
					assert!(ptr::eq(right.get_parent().unwrap() as *const _, n as *const _));
					assert!(right.key >= n.key);
				}
			}, TraversalType::PreOrder);
		}
	}

	/// Returns a mutable iterator for the current binary tree.
	pub fn iter_mut(&mut self) -> BinaryTreeMutIterator::<K, V> {
		BinaryTreeMutIterator::new(self)
	}
}

/// An iterator for the BinaryTree structure. This iterator traverses the tree in pre order.
pub struct BinaryTreeIterator<'a, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'a BinaryTree::<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<BinaryTreeNode<K, V>>>,
}

impl<'a, K: Ord, V> BinaryTreeIterator<'a, K, V> {
	/// Creates a binary tree iterator for the given reference.
	fn new(tree: &'a BinaryTree::<K, V>) -> Self {
		BinaryTreeIterator {
			tree,
			node: tree.root,
		}
	}
}

impl<'a, K: 'static + Ord, V> Iterator for BinaryTreeIterator<'a, K, V> {
	type Item = (&'a K, &'a V);

	// TODO Implement every functions?

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.node;
		self.node = {
			self.node?;

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
			Some((&node.key, &node.value))
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.tree.nodes_count()
	}
}

impl<'a, K: 'static + Ord, V> IntoIterator for &'a BinaryTree<K, V> {
	type Item = (&'a K, &'a V);
	type IntoIter = BinaryTreeIterator<'a, K, V>;

	fn into_iter(self) -> Self::IntoIter {
		BinaryTreeIterator::new(&self)
	}
}

/// An iterator for the BinaryTree structure. This iterator traverses the tree in pre order.
pub struct BinaryTreeMutIterator<'a, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'a mut BinaryTree::<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<BinaryTreeNode<K, V>>>,
}

impl<'a, K: Ord, V> BinaryTreeMutIterator<'a, K, V> {
	/// Creates a binary tree iterator for the given reference.
	fn new(tree: &'a mut BinaryTree::<K, V>) -> Self {
		let root = tree.root;
		BinaryTreeMutIterator {
			tree,
			node: root,
		}
	}
}

impl<'a, K: 'static + Ord, V> Iterator for BinaryTreeMutIterator<'a, K, V> {
	type Item = (&'a K, &'a mut V);

	// TODO Implement every functions?

	fn next(&mut self) -> Option<Self::Item> {
		let mut node = self.node;
		self.node = {
			self.node?;

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
			Some((&node.key, &mut node.value))
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.tree.nodes_count()
	}
}

impl<K: 'static + FailableClone + Ord, V: FailableClone> FailableClone for BinaryTree<K, V> {
	fn failable_clone(&self) -> Result<Self, Errno> {
		let mut new = Self::new();
		for (k, v) in self {
			new.insert(k.failable_clone()?, v.failable_clone()?)?;
		}
		Ok(new)
	}
}

impl<K: 'static + Ord + fmt::Display, V: fmt::Display> fmt::Display for BinaryTree<K, V> {
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

impl<K: 'static + Ord, V> Drop for BinaryTree<K, V> {
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
		let b = BinaryTree::<i32, ()>::new();
		assert!(b.get(0).is_none());
	}

	#[test_case]
	fn binary_tree_insert0() {
		let mut b = BinaryTree::<i32, i32>::new();

		b.insert(0, 0).unwrap();
		assert_eq!(*b.get(0).unwrap(), 0);
	}

	#[test_case]
	fn binary_tree_insert1() {
		let mut b = BinaryTree::<i32, i32>::new();

		for i in 0..10 {
			b.insert(i, i).unwrap();
		}

		for i in 0..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert2() {
		let mut b = BinaryTree::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert3() {
		let mut b = BinaryTree::<u32, u32>::new();

		let mut val = 0;
		for _ in 0..100 {
			val = crate::util::math::pseudo_rand(val, 1664525, 1013904223, 0x100);
			b.insert(val, val).unwrap();
		}

		val = 0;
		for _ in 0..100 {
			val = crate::util::math::pseudo_rand(val, 1664525, 1013904223, 0x100);
			assert_eq!(*b.get(val).unwrap(), val);
		}
	}

	#[test_case]
	fn binary_tree_remove0() {
		let mut b = BinaryTree::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
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
		let mut b = BinaryTree::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
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
		let mut b = BinaryTree::<i32, i32>::new();

		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
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
		let mut b = BinaryTree::<i32, i32>::new();

		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
			assert_eq!(b.remove(i).unwrap(), i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove4() {
		let mut b = BinaryTree::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.remove(i).unwrap(), i);
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove5() {
		let mut b = BinaryTree::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			if i % 2 == 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				assert_eq!(b.remove(i).unwrap(), i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(!b.is_empty());

		for i in -9..10 {
			if i % 2 != 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				assert_eq!(b.remove(i).unwrap(), i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_get_min0() {
		let mut b = BinaryTree::<i32, i32>::new();
		assert!(b.get_min(0).is_none());
	}

	#[test_case]
	fn binary_tree_get_min1() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min2() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		assert!(b.get_min(1).is_none());
	}

	#[test_case]
	fn binary_tree_get_min3() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(-1, -1).unwrap();
		b.insert(0, 0).unwrap();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min4() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min5() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_get_min6() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(-1, -1).unwrap();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap() >= 0);
	}

	#[test_case]
	fn binary_tree_foreach0() {
		let b = BinaryTree::<i32, i32>::new();
		b.foreach(| _, _ | {
			assert!(false);
		}, TraversalType::PreOrder);
	}

	#[test_case]
	fn binary_tree_foreach1() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();

		let mut passed = false;
		b.foreach(| key, _ | {
			assert!(!passed);
			assert_eq!(*key, 0);
			passed = true;
		}, TraversalType::PreOrder);
		assert!(passed);
	}

	// TODO More foreach tests
}
