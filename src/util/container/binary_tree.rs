//! This module implements a binary tree container.

use core::cmp::Ordering;
use core::cmp::max;
use core::fmt;
use core::mem::ManuallyDrop;
use core::mem::size_of;
use core::mem;
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
#[inline]
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
#[inline]
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
	#[inline]
	pub fn is_red(&self) -> bool {
		self.color == NodeColor::Red
	}

	/// Tells whether the node is black.
	#[inline]
	pub fn is_black(&self) -> bool {
		self.color == NodeColor::Black
	}

	/// Returns a reference to the left child node.
	#[inline]
	pub fn get_parent(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.parent)
	}

	/// Returns a reference to the parent child node.
	#[inline]
	pub fn get_parent_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	#[inline]
	pub fn get_left(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.left)
	}

	/// Returns a reference to the left child node.
	#[inline]
	pub fn get_left_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.left)
	}

	/// Returns a reference to the left child node.
	#[inline]
	pub fn get_right(&self) -> Option<&'static Self> {
		unwrap_pointer(&self.right)
	}

	/// Returns a reference to the left child node.
	#[inline]
	pub fn get_right_mut(&mut self) -> Option<&'static mut Self> {
		unwrap_pointer_mut(&mut self.right)
	}

	/// Tells whether the node is a left child.
	#[inline]
	pub fn is_left_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_left() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Tells whether the node is a right child.
	#[inline]
	pub fn is_right_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_right() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Tells whether the node and its parent and grandparent form a triangle.
	#[inline]
	pub fn is_triangle(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			return self.is_left_child() != parent.is_left_child();
		}

		false
	}

	/// Returns a reference to the grandparent node.
	#[inline]
	pub fn get_grandparent(&self) -> Option<&'static Self> {
		self.get_parent()?.get_parent()
	}

	/// Returns a mutable reference to the grandparent node.
	#[inline]
	pub fn get_grandparent_mut(&mut self) -> Option<&'static mut Self> {
		self.get_parent_mut()?.get_parent_mut()
	}

	/// Returns a reference to the sibling node.
	#[inline]
	pub fn get_sibling(&self) -> Option<&'static Self> {
		let parent = self.get_parent()?;

		if self.is_left_child() {
			parent.get_right()
		} else {
			parent.get_left()
		}
	}

	/// Returns a mutable reference to the sibling node.
	#[inline]
	pub fn get_sibling_mut(&mut self) -> Option<&'static mut Self> {
		let parent = self.get_parent_mut()?;

		if self.is_left_child() {
			parent.get_right_mut()
		} else {
			parent.get_left_mut()
		}
	}

	/// Returns a reference to the uncle node.
	#[inline]
	pub fn get_uncle(&mut self) -> Option<&'static Self> {
		self.get_parent()?.get_sibling()
	}

	/// Returns a mutable reference to the uncle node.
	#[inline]
	pub fn get_uncle_mut(&mut self) -> Option<&'static mut Self> {
		self.get_parent_mut()?.get_sibling_mut()
	}

	/// Tells whether the node has at least one red child.
	#[inline]
	pub fn has_red_child(&self) -> bool {
		if let Some(left) = self.get_left() {
			if left.is_red() {
				return true;
			}
		}

		if let Some(right) = self.get_right() {
			if right.is_red() {
				return true;
			}
		}

		false
	}

	/// Applies a left tree rotation with the current node as root.
	/// If the current node doesn't have a right child, the function does nothing.
	pub fn left_rotate(&mut self) {
		if let Some(pivot) = self.get_right_mut() {
			if let Some(parent) = self.get_parent_mut() {
				if self.is_right_child() {
					parent.right = NonNull::new(pivot);
				} else {
					parent.left = NonNull::new(pivot);
				}

				pivot.parent = NonNull::new(parent);
			} else {
				pivot.parent = None;
			}

			let left = pivot.get_left_mut();
			pivot.left = NonNull::new(self);
			self.parent = NonNull::new(pivot);

			if let Some(left) = left {
				self.right = NonNull::new(left);
				left.parent = NonNull::new(self);
			} else {
				self.right = None;
			}
		}
	}

	/// Applies a right tree rotation with the current node as root.
	/// If the current node doesn't have a left child, the function does nothing.
	pub fn right_rotate(&mut self) {
		if let Some(pivot) = self.get_left_mut() {
			if let Some(parent) = self.get_parent_mut() {
				if self.is_left_child() {
					parent.left = NonNull::new(pivot);
				} else {
					parent.right = NonNull::new(pivot);
				}

				pivot.parent = NonNull::new(parent);
			} else {
				pivot.parent = None;
			}

			let right = pivot.get_right_mut();
			pivot.right = NonNull::new(self);
			self.parent = NonNull::new(pivot);

			if let Some(right) = right {
				self.left = NonNull::new(right);
				right.parent = NonNull::new(self);
			} else {
				self.left = None;
			}
		}
	}

	/// Inserts the given node `node` to left of the current node. If the node already has a left
	/// child, the behaviour is undefined.
	#[inline]
	pub fn insert_left(&mut self, node: &mut BinaryTreeNode<K, V>) {
		debug_assert!(self.get_left().is_none());
		debug_assert!(node.get_parent().is_none());

		self.left = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Inserts the given node `node` to right of the current node. If the node already has a right
	/// child, the behaviour is undefined.
	#[inline]
	pub fn insert_right(&mut self, node: &mut BinaryTreeNode<K, V>) {
		debug_assert!(self.get_right().is_none());
		debug_assert!(node.get_parent().is_none());

		self.right = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Returns the number of nodes in the subtree.
	/// This function has `O(n)` complexity.
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
	/// This function has `O(log n)` complexity.
	pub fn get_node_depth(&self) -> usize {
		if let Some(p) = self.get_parent() {
			p.get_node_depth() + 1
		} else {
			0
		}
	}

	/// Returns the black depth of the node in the tree.
	/// This function has `O(log n)` complexity.
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
	/// This function has `O(log n)` complexity.
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
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.root.is_none()
	}

	/// Returns a reference to the root node.
	#[inline]
	fn get_root(&self) -> Option<&'static BinaryTreeNode<K, V>> {
		unsafe {
			Some(self.root.as_ref()?.as_ref())
		}
	}

	/// Returns a mutable reference to the root node.
	#[inline]
	fn get_root_mut(&mut self) -> Option<&'static mut BinaryTreeNode<K, V>> {
		unsafe {
			Some(self.root.as_mut()?.as_mut())
		}
	}

	/// Returns the number of elements in the tree.
	/// This function has `O(n)` complexity.
	pub fn count(&self) -> usize {
		if let Some(r) = self.get_root() {
			r.nodes_count()
		} else {
			0
		}
	}

	/// Returns the depth of the tree.
	/// This function has `O(log n)` complexity.
	pub fn get_depth(&self) -> usize {
		if let Some(r) = self.get_root() {
			r.get_depth()
		} else {
			0
		}
	}

	/// Searches for a node with the given key in the tree and returns a reference.
	/// `key` is the key to find.
	fn get_node(&self, key: &K) -> Option<&'static BinaryTreeNode<K, V>> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(key).unwrap().reverse();

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
	fn get_mut_node(&mut self, key: &K) -> Option<&'static mut BinaryTreeNode<K, V>> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(key).unwrap().reverse();

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
	#[inline]
	pub fn get<'a>(&'a self, key: K) -> Option<&'a V> {
		let node = self.get_node(&key)?;
		Some(&node.value)
	}

	/// Searches for the given key in the tree and returns a mutable reference.
	/// `key` is the key to find.
	#[inline]
	pub fn get_mut<'a>(&'a mut self, key: K) -> Option<&'a mut V> {
		let node = self.get_mut_node(&key)?;
		Some(&mut node.value)
	}

	/// Searches for a node in the tree using the given comparison function `cmp` instead of the
	/// Ord trait.
	pub fn cmp_get<'a, F: Fn(&K, &V) -> Ordering>(&'a self, cmp: F) -> Option<&'a V> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.key, &n.value);

			match ord {
				Ordering::Less => node = n.get_left(),
				Ordering::Greater => node = n.get_right(),
				Ordering::Equal => return Some(&n.value),
			}
		}

		None
	}

	/// Searches for a node in the tree using the given comparison function `cmp` instead of the
	/// Ord trait and returns a mutable reference.
	pub fn cmp_get_mut<'a, F: Fn(&K, &V) -> Ordering>(&'a mut self, cmp: F) -> Option<&'a mut V> {
		let mut node = self.get_root_mut();

		while node.is_some() {
			let n = node.unwrap();
			let ord = cmp(&n.key, &n.value);

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
	pub fn get_min<'a>(&'a self, key: K) -> Option<(&'a K, &'a V)> {
		let mut node = self.get_root();

		while node.is_some() {
			let n = node.unwrap();
			let ord = n.key.partial_cmp(&key).unwrap().reverse();

			if ord == Ordering::Greater {
				node = n.get_right();
			} else {
				return Some((&n.key, &n.value));
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
					parent.right_rotate();
				} else {
					parent.left_rotate();
				}

				node = parent;
			}

			let parent = node.get_parent_mut().unwrap();
			let grandparent = parent.get_parent_mut().unwrap();

			if node.is_right_child() {
				grandparent.left_rotate();
			} else {
				grandparent.right_rotate();
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
	pub fn insert<'a>(&'a mut self, key: K, val: V) -> Result<&'a mut V, Errno> {
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
		//#[cfg(config_debug_debug)]
		//self.check();
		self.update_root(n);

		Ok(&mut n.value)
	}

	/// Deletes the node at the given pointer.
	unsafe fn drop_node(node: &mut BinaryTreeNode<K, V>) {
		let ptr = node as *mut _ as *mut _;
		let mut n = ManuallyDrop::new(node);
		drop_in_place(&mut n.parent);
		drop_in_place(&mut n.left);
		drop_in_place(&mut n.right);
		drop_in_place(&mut n.color);
		drop_in_place(&mut n.key);

		malloc::free(ptr);
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

	/// Fixes the tree after deletion in the case where the deleted node and its replacement are
	/// both black.
	/// `node` is the node to fix.
	fn remove_fix_double_black(&mut self, node: &mut BinaryTreeNode<K, V>) {
		if let Some(parent) = node.get_parent_mut() {
			if let Some(sibling) = node.get_sibling_mut() {
				if sibling.is_red() {
					parent.color = NodeColor::Red;
					sibling.color = NodeColor::Black;

					if sibling.is_left_child() {
						parent.right_rotate();
					} else {
						parent.left_rotate();
					}

					self.remove_fix_double_black(node);
				} else {
					// `sibling` is black
					let s_left = sibling.get_left_mut();
					let s_right = sibling.get_right_mut();

					if s_left.is_some() && s_left.as_ref().unwrap().is_red() {
						let s_left = s_left.unwrap();

						if sibling.is_left_child() {
							s_left.color = sibling.color;
							sibling.color = parent.color;
							parent.right_rotate();
						} else {
							s_left.color = parent.color;
							sibling.right_rotate();
							parent.left_rotate();
						}

						parent.color = NodeColor::Black;
					} else if s_right.is_some() && s_right.as_ref().unwrap().is_red() {
						let s_right = s_right.unwrap();

						if sibling.is_left_child() {
							s_right.color = parent.color;
							sibling.left_rotate();
							parent.right_rotate();
						} else {
							s_right.color = sibling.color;
							sibling.color = parent.color;
							parent.left_rotate();
						}

						parent.color = NodeColor::Black;
					} else {
						// `sibling` has two black children
						sibling.color = NodeColor::Red;

						if parent.is_black() {
							self.remove_fix_double_black(parent);
						} else {
							parent.color = NodeColor::Black;
						}
					}
				}
			} else {
				self.remove_fix_double_black(parent);
			}
		}
	}

	/// Removes the given node `node` from the tree.
	fn remove_node(&mut self, node: &mut BinaryTreeNode<K, V>) {
		let mut replacement = {
			let left = node.get_left_mut();
			let right = node.get_right_mut();

			if left.is_some() && right.is_some() {
				// The node has two children
				// The leftmost node may have a child on the right
				Some(Self::get_leftmost_node(right.unwrap()))
			} else if left.is_some() {
				// The node has only one child on the left
				left
			} else if right.is_some() {
				// The node has only one child on the right
				right
			} else {
				// The node has no children
				None
			}
		};

		let both_black = node.is_black()
			&& (replacement.is_none() || replacement.as_ref().unwrap().is_black());

		if replacement.is_none() {
			if node.get_parent_mut().is_none() {
				// The node is root and has no children
				unsafe {
					debug_assert_eq!(self.root.unwrap().as_mut() as *mut BinaryTreeNode<K, V>,
						node as *mut _);
				}
				self.root = None;
			} else {
				if both_black {
					self.remove_fix_double_black(node);
					self.update_root(node);
				} else if let Some(sibling) = node.get_sibling_mut() {
					sibling.color = NodeColor::Red;
				}

				node.unlink();
			}

			unsafe {
				Self::drop_node(node);
			}
		} else if node.get_left().is_none() || node.get_right().is_none() {
			let replacement = replacement.as_mut().unwrap();

			if let Some(parent) = node.get_parent_mut() {
				replacement.parent = None;
				if node.is_left_child() {
					parent.left = None;
					parent.insert_left(replacement);
				} else {
					parent.right = None;
					parent.insert_right(replacement);
				}

				node.unlink();
				unsafe {
					Self::drop_node(node);
				}

				if both_black {
					self.remove_fix_double_black(replacement);
					self.update_root(replacement);
				} else {
					replacement.color = NodeColor::Black;
				}
			} else {
				// The node is the root
				node.key = unsafe {
					ptr::read(&replacement.key as _)
				};
				node.value = unsafe {
					ptr::read(&replacement.value as _)
				};

				node.left = None;
				node.right = None;

				replacement.unlink();
				unsafe {
					Self::drop_node(replacement);
				}
			}
		} else {
			let replacement = replacement.as_mut().unwrap();
			mem::swap(&mut node.key, &mut replacement.key);
			mem::swap(&mut node.value, &mut replacement.value);
			self.remove_node(replacement);
		}
	}

	/// Removes a value from the tree. If the value is present several times in the tree, only one
	/// node is removed.
	/// `key` is the key to select the node to remove.
	/// If the key exists, the function returns the value of the removed node.
	pub fn remove(&mut self, key: K) -> Option<V> {
		let node = self.get_mut_node(&key)?;
		let value = unsafe {
			ptr::read(&node.value)
		};

		self.remove_node(node);

		//#[cfg(config_debug_debug)]
		//self.check();
		Some(value)
	}

	/// Removes a value from the tree. This function is useful when several values have the same
	/// key since the given closure allows to select the node to remove.
	/// `key` is the key to select the node to remove.
	/// `f` the closure that selects the node to be removed. When returning `false`, the closure is
	/// called with the next node. When returning `true`, the node is removed and the closure isn't
	/// called anymore.
	/// If a node is removed, the function returns the value of the removed node.
	pub fn select_remove<F: FnMut(&V) -> bool>(&mut self, key: K, mut f: F) -> Option<V> {
		let node = {
			let mut n = self.get_mut_node(&key)?;

			loop {
				debug_assert_eq!(n.key.cmp(&key), Ordering::Equal);
				if f(&n.value) {
					break;
				}

				let left = n.get_left_mut();
				if left.is_some() && left.as_ref().unwrap().key == key {
					n = left.unwrap();
				} else {
					loop {
						let right = n.get_right_mut();
						if right.is_some() && right.as_ref().unwrap().key == key {
							n = right.unwrap();
							break;
						}

						n = n.get_parent_mut()?;
						if n.key != key {
							return None;
						}
					}

					break;
				}
			}

			n
		};
		let value = unsafe {
			ptr::read(&node.value)
		};

		self.remove_node(node);

		//#[cfg(config_debug_debug)]
		//self.check();
		Some(value)
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
			let mut explored_nodes = Vec::<*const c_void>::new();

			Self::foreach_nodes(unsafe {
				root.as_ref()
			}, &mut | n: &BinaryTreeNode<K, V> | {
				assert!(n as *const _ as usize >= memory::PROCESS_END as usize);

				for e in explored_nodes.iter() {
					assert_ne!(*e, n as *const _ as *const c_void);
				}
				explored_nodes.push(n as *const _ as *const c_void).unwrap();

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

	/// Returns an iterator for the current binary tree.
	pub fn iter(&self) -> BinaryTreeIterator::<K, V> {
		BinaryTreeIterator::new(self)
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

	/// Makes the iterator jump to the given key. If the key doesn't exist, the iterator ends.
	pub fn jump(&mut self, key: &K) {
		self.node = self.tree.get_node(key).and_then(| v | {
			NonNull::new(v as *const _ as *mut _)
		});
	}
}

impl<'a, K: 'static + Ord, V> Iterator for BinaryTreeIterator<'a, K, V> {
	type Item = (&'a K, &'a V);

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
		self.tree.count()
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

	/// Makes the iterator jump to the given key. If the key doesn't exist, the iterator ends.
	pub fn jump(&mut self, key: &K) {
		self.node = self.tree.get_mut_node(key).and_then(| v | NonNull::new(v));
	}
}

impl<'a, K: 'static + Ord, V> Iterator for BinaryTreeMutIterator<'a, K, V> {
	type Item = (&'a K, &'a mut V);

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
		self.tree.count()
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

impl<K: 'static + Ord + fmt::Display, V> fmt::Display for BinaryTree<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(mut n) = self.root {
			Self::foreach_nodes(unsafe {
				n.as_mut()
			}, &mut | n | {
				// TODO Optimize
				for _ in 0..n.get_node_depth() {
					let _ = write!(f, "\t");
				}

				let color = if n.color == NodeColor::Red {
					"red"
				} else {
					"black"
				};
				let _ = writeln!(f, "{} ({})", n.key, color);
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
			for i in i..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}

			b.remove(i);

			assert!(b.get(i).is_none());
			for i in (i + 1)..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}
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
		let b = BinaryTree::<i32, i32>::new();
		assert!(b.get_min(0).is_none());
	}

	#[test_case]
	fn binary_tree_get_min1() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		assert!(*b.get_min(0).unwrap().0 >= 0);
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
		assert!(*b.get_min(0).unwrap().0 >= 0);
	}

	#[test_case]
	fn binary_tree_get_min4() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap().0 >= 0);
	}

	#[test_case]
	fn binary_tree_get_min5() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap().0 >= 0);
	}

	#[test_case]
	fn binary_tree_get_min6() {
		let mut b = BinaryTree::<i32, i32>::new();
		b.insert(-1, -1).unwrap();
		b.insert(1, 1).unwrap();
		assert!(*b.get_min(0).unwrap().0 >= 0);
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
