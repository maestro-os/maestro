//! This module implements a binary tree container.

use crate::errno::Errno;
use crate::memory;
use crate::memory::malloc;
use crate::util::TryClone;
use core::cmp::max;
use core::cmp::Ordering;
use core::fmt;
use core::mem;
use core::mem::size_of;
use core::ops::Bound;
use core::ops::RangeBounds;
use core::ptr;
use core::ptr::NonNull;

#[cfg(config_debug_debug)]
use crate::util::container::vec::Vec;
#[cfg(config_debug_debug)]
use core::ffi::c_void;

// FIXME abusive use of `'static` lifetime results in UBs

/// The color of a binary tree node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NodeColor {
	Red,
	Black,
}

/// A node in the binary tree.
struct Node<K, V> {
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

/// Deletes the node at the given pointer, except the key and value fields which
/// are returned.
///
/// # Safety
///
/// The caller must ensure the pointer points to a valid node and must not use it after calling
/// this function since it will be dropped.
#[inline]
unsafe fn drop_node<K, V>(ptr: *mut Node<K, V>) -> (K, V) {
	let node = ptr::read(ptr);
	malloc::free(ptr as _);

	let Node::<K, V> {
		key,
		value,
		..
	} = node;

	(key, value)
}

/// Unwraps the given pointer option into a reference option.
#[inline]
fn unwrap_pointer<K, V>(ptr: &Option<NonNull<Node<K, V>>>) -> Option<&'static Node<K, V>> {
	ptr.map(|p| unsafe {
		debug_assert!(p.as_ptr() as usize >= memory::PROCESS_END as usize);
		&*p.as_ptr()
	})
}

/// Same as `unwrap_pointer` but returns a mutable reference.
#[inline]
fn unwrap_pointer_mut<K, V>(
	ptr: &mut Option<NonNull<Node<K, V>>>,
) -> Option<&'static mut Node<K, V>> {
	ptr.map(|mut p| unsafe {
		debug_assert!(p.as_ptr() as usize >= memory::PROCESS_END as usize);
		p.as_mut()
	})
}

impl<K: 'static + Ord, V: 'static> Node<K, V> {
	/// Creates a new node with the given `value`.
	///
	/// The node is colored `Red` by default.
	pub fn new(key: K, value: V) -> Result<NonNull<Self>, Errno> {
		let ptr = unsafe { malloc::alloc(size_of::<Self>())? as *mut Self };
		let s = Self {
			parent: None,
			left: None,
			right: None,
			color: NodeColor::Red,

			key,
			value,
		};

		debug_assert!(ptr as usize >= memory::PROCESS_END as usize);
		unsafe {
			// Safe because the pointer is valid
			ptr::write(ptr, s);
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

	/// Tells whether the node and its parent and grandparent form a triangle.
	#[inline]
	pub fn is_triangle(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			return self.is_left_child() != parent.is_left_child();
		}

		false
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
	///
	/// If the current node doesn't have a right child, the function does
	/// nothing.
	pub fn left_rotate(&mut self) {
		let Some(pivot) = self.get_right_mut() else {
            return;
		};

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

	/// Applies a right tree rotation with the current node as root.
	///
	/// If the current node doesn't have a left child, the function does
	/// nothing.
	pub fn right_rotate(&mut self) {
		let Some(pivot) = self.get_left_mut() else {
            return;
		};

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

	/// Inserts the given node `node` to left of the current node.
	///
	/// If the node already has a left child, the behaviour is undefined.
	#[inline]
	pub fn insert_left(&mut self, node: &mut Node<K, V>) {
		debug_assert!(self.left.is_none());
		debug_assert!(node.parent.is_none());

		self.left = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Inserts the given node `node` to right of the current node.
	///
	/// If the node already has a right child, the behaviour is undefined.
	#[inline]
	pub fn insert_right(&mut self, node: &mut Node<K, V>) {
		debug_assert!(self.right.is_none());
		debug_assert!(node.parent.is_none());

		self.right = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Returns the number of nodes in the subtree.
	///
	/// This function has `O(n)` complexity.
	pub fn nodes_count(&self) -> usize {
		let left_count = self.get_left().map(|n| n.nodes_count()).unwrap_or(0);
		let right_count = self.get_right().map(|n| n.nodes_count()).unwrap_or(0);

		1 + left_count + right_count
	}

	/// Returns the depth of the node in the tree.
	///
	/// This function has `O(log n)` complexity.
	pub fn get_node_depth(&self) -> usize {
		self.get_parent()
			.map(|n| n.get_node_depth() + 1)
			.unwrap_or(0)
	}

	/// Returns the black depth of the node in the tree.
	///
	/// This function has `O(log n)` complexity.
	pub fn get_node_black_depth(&self) -> usize {
		let parent = self
			.get_parent()
			.map(|n| n.get_node_black_depth())
			.unwrap_or(0);

		if self.is_black() {
			1 + parent
		} else {
			parent
		}
	}

	/// Returns the depth of the subtree.
	///
	/// This function has `O(log n)` complexity.
	pub fn get_depth(&self) -> usize {
		let left_count = self.get_left().map(|n| n.get_depth()).unwrap_or(0);
		let right_count = self.get_right().map(|n| n.get_depth()).unwrap_or(0);

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
pub enum TraveralOrder {
	/// Accesses the data, then left child, then right child
	PreOrder,
	/// Accesses left child, then the data, then right child
	InOrder,
	/// Accesses right child, then the data, then left child
	ReverseInOrder,
	/// Accesses left child, then right child, then the data
	PostOrder,
}

/// A binary tree is a structure which allows, when properly balanced, to
/// performs actions (insertion, removal, searching) in `O(log n)` complexity.
pub struct Map<K: 'static + Ord, V: 'static> {
	/// The root node of the binary tree.
	root: Option<NonNull<Node<K, V>>>,
}

impl<K: 'static + Ord, V: 'static> Default for Map<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: 'static + Ord, V: 'static> Map<K, V> {
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
	fn get_root(&self) -> Option<&'static Node<K, V>> {
		unsafe { Some(self.root.as_ref()?.as_ref()) }
	}

	/// Returns a mutable reference to the root node.
	#[inline]
	fn get_root_mut(&mut self) -> Option<&'static mut Node<K, V>> {
		unsafe { Some(self.root.as_mut()?.as_mut()) }
	}

	/// Returns the number of elements in the tree.
	///
	/// This function has `O(n)` complexity.
	pub fn count(&self) -> usize {
		self.get_root().map(|n| n.nodes_count()).unwrap_or(0)
	}

	/// Returns the depth of the tree.
	///
	/// This function has `O(log n)` complexity.
	pub fn get_depth(&self) -> usize {
		self.get_root().map(|n| n.get_depth()).unwrap_or(0)
	}

	/// Searches for a node with the given key in the tree and returns a
	/// reference.
	///
	/// `key` is the key to find.
	fn get_node(&self, key: &K) -> Option<&'static Node<K, V>> {
		let mut node = self.get_root();

		while let Some(n) = node {
			let ord = key.cmp(&n.key);

			match ord {
				Ordering::Less => node = n.get_left(),
				Ordering::Greater => node = n.get_right(),
				Ordering::Equal => return Some(n),
			}
		}

		None
	}

	/// Searches for a node with the given key in the tree and returns a mutable
	/// reference.
	///
	/// `key` is the key to find.
	fn get_mut_node(&mut self, key: &K) -> Option<&'static mut Node<K, V>> {
		let mut node = self.get_root_mut();

		while let Some(n) = node {
			let ord = key.cmp(&n.key);

			match ord {
				Ordering::Less => node = n.get_left_mut(),
				Ordering::Greater => node = n.get_right_mut(),
				Ordering::Equal => return Some(n),
			}
		}

		None
	}

	/// Returns the start node for a range iterator starting at `start`.
	fn get_start_node(&self, start: Bound<&K>) -> Option<NonNull<Node<K, V>>> {
		let mut node = self.root.map(|mut p| unsafe { p.as_mut() });

		match start {
			Bound::Included(key) => {
				while let Some(n) = node {
					if key.cmp(&n.key) == Ordering::Greater {
						node = n.get_right_mut();
					} else {
						return NonNull::new(n);
					}
				}

				None
			}

			Bound::Excluded(_key) => {
				// TODO
				todo!();
			}

			Bound::Unbounded => NonNull::new(Self::get_leftmost_node(node?)),
		}
	}

	/// Searches for the given key in the tree and returns a reference.
	///
	/// `key` is the key to find.
	#[inline]
	pub fn get(&self, key: K) -> Option<&V> {
		let node = self.get_node(&key)?;
		Some(&node.value)
	}

	/// Searches for the given key in the tree and returns a mutable reference.
	///
	/// `key` is the key to find.
	#[inline]
	pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
		let node = self.get_mut_node(&key)?;
		Some(&mut node.value)
	}

	/// Searches for a node in the tree using the given comparison function
	/// `cmp` instead of the `Ord` trait.
	pub fn cmp_get<F: Fn(&K, &V) -> Ordering>(&self, cmp: F) -> Option<&V> {
		let mut node = self.get_root();

		while let Some(n) = node {
			let ord = cmp(&n.key, &n.value);

			match ord {
				Ordering::Less => node = n.get_left(),
				Ordering::Greater => node = n.get_right(),
				Ordering::Equal => return Some(&n.value),
			}
		}

		None
	}

	/// Searches for a node in the tree using the given comparison function
	/// `cmp` instead of the `Ord` trait and returns a mutable reference.
	pub fn cmp_get_mut<F: Fn(&K, &V) -> Ordering>(&mut self, cmp: F) -> Option<&mut V> {
		let mut node = self.get_root_mut();

		while let Some(n) = node {
			let ord = cmp(&n.key, &n.value);

			match ord {
				Ordering::Less => node = n.get_left_mut(),
				Ordering::Greater => node = n.get_right_mut(),
				Ordering::Equal => return Some(&mut n.value),
			}
		}

		None
	}

	/// Updates the root of the tree.
	///
	/// `node` is a node of the tree.
	fn update_root(&mut self, node: &mut Node<K, V>) {
		let mut root = NonNull::new(node as *mut Node<K, V>);

		loop {
			let parent = unsafe { root.unwrap().as_mut() }.parent;

			if parent.is_none() {
				break;
			}
			root = parent;
		}

		self.root = root;
	}

	/// For node insertion, returns the parent node on which it will be
	/// inserted.
	fn get_insert_node(&mut self, key: &K) -> Option<&'static mut Node<K, V>> {
		let mut node = self.get_root_mut();

		while let Some(n) = node {
			let ord = key.cmp(&n.key);

			let next = match ord {
				Ordering::Less => n.get_left_mut(),
				Ordering::Greater => n.get_right_mut(),
				Ordering::Equal => return Some(n),
			};

			if next.is_none() {
				return Some(n);
			}
			node = next;
		}

		None
	}

	/// Equilibrates the tree after insertion of node `node`.
	fn insert_equilibrate(mut node: &mut Node<K, V>) {
		let Some(parent) = node.get_parent_mut() else {
			node.color = NodeColor::Black;
            return;
        };
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

				Self::insert_equilibrate(grandparent);
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
	}

	/// Inserts a key/value pair in the tree and returns a mutable reference to
	/// the value.
	///
	/// Arguments:
	/// - `key` is the key to insert.
	/// - `val` is the value to insert.
	/// - `cmp` is the comparison function.
	///
	/// If the key is already used, the previous key/value pair is dropped.
	pub fn insert(&mut self, key: K, val: V) -> Result<&mut V, Errno> {
		let n = match self.get_insert_node(&key) {
			Some(p) => {
				let order = key.cmp(&p.key);

				if order == Ordering::Equal {
					// Dropping old key/value pair and replacing with the new ones
					p.key = key;
					p.value = val;

					return Ok(&mut p.value);
				}

				let mut node = Node::new(key, val)?;
				let n = unsafe { node.as_mut() };

				match order {
					Ordering::Less => p.insert_left(n),
					Ordering::Greater => p.insert_right(n),

					_ => unreachable!(),
				}

				n
			}

			None => {
				debug_assert!(self.root.is_none());

				let mut node = Node::new(key, val)?;
				let n = unsafe { node.as_mut() };
				self.root = Some(node);

				n
			}
		};

		Self::insert_equilibrate(n);
		//#[cfg(config_debug_debug)]
		//self.check();
		self.update_root(n);

		Ok(&mut n.value)
	}

	/// Returns the leftmost node in the tree.
	fn get_leftmost_node(node: &'static mut Node<K, V>) -> &'static mut Node<K, V> {
		let mut n = node;
		while let Some(left) = n.get_left_mut() {
			n = left;
		}

		n
	}

	/// Fixes the tree after deletion in the case where the deleted node and its
	/// replacement are both black.
	///
	/// `node` is the node to fix.
	fn remove_fix_double_black(node: &mut Node<K, V>) {
		let Some(parent) = node.get_parent_mut() else {
            return;
		};
		let Some(sibling) = node.get_sibling_mut() else {
            Self::remove_fix_double_black(parent);
            return;
        };

		if sibling.is_red() {
			parent.color = NodeColor::Red;
			sibling.color = NodeColor::Black;

			if sibling.is_left_child() {
				parent.right_rotate();
			} else {
				parent.left_rotate();
			}

			Self::remove_fix_double_black(node);
			return;
		}

		// from here, `sibling` is black

		let s_left = sibling.get_left_mut();
		let s_right = sibling.get_right_mut();
		match (s_left, s_right) {
			(Some(s_left), _) if s_left.is_red() => {
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
			}

			(_, Some(s_right)) if s_right.is_red() => {
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
			}

			_ => {
				// `sibling` has two black children
				sibling.color = NodeColor::Red;

				if parent.is_black() {
					Self::remove_fix_double_black(parent);
				} else {
					parent.color = NodeColor::Black;
				}
			}
		}
	}

	/// Removes the given node `node` from the tree.
	///
	/// The function returns the value of the removed node.
	fn remove_node(&mut self, node: &mut Node<K, V>) -> V {
		let left = node.get_left_mut();
		let right = node.get_right_mut();
		let replacement = match (left, right) {
			// The node has two children
			// The leftmost node may have a child on the right
			(Some(_left), Some(right)) => Some(Self::get_leftmost_node(right)),
			// The node has only one child on the left
			(Some(left), _) => Some(left),
			// The node has only one child on the right
			(_, Some(right)) => Some(right),
			// The node has no children
			_ => None,
		};

		let both_black =
			node.is_black() && replacement.as_ref().map(|r| r.is_black()).unwrap_or(true);
		let parent = node.get_parent_mut();

		let Some(replacement) = replacement else {
            // The node has no children

            match parent {
                Some(_parent) => {
                    if both_black {
                        Self::remove_fix_double_black(node);
                        self.update_root(node);
                    } else if let Some(sibling) = node.get_sibling_mut() {
                        sibling.color = NodeColor::Red;
                    }
                }

				// The node is root
                None => {
                    unsafe {
                        debug_assert_eq!(
                            self.root.unwrap().as_mut() as *mut Node<K, V>,
                            node as *mut _
                        );
                    }

                    self.root = None;
                }
            }

            node.unlink();

			let (_, val) = unsafe { drop_node(node) };
			return val;
        };

		if node.get_left().is_some() && node.get_right().is_some() {
			mem::swap(&mut node.key, &mut replacement.key);
			mem::swap(&mut node.value, &mut replacement.value);

			return self.remove_node(replacement);
		}

		let Some(parent) = parent else {
            // The node is the root

            replacement.unlink();
            let (key, value) = unsafe { drop_node(replacement) };

            node.left = None;
            node.right = None;

            node.key = key;
            let mut val = value;
            mem::swap(&mut val, &mut node.value);

            return val;
        };

		replacement.parent = None;
		if node.is_left_child() {
			parent.left = None;
			parent.insert_left(replacement);
		} else {
			parent.right = None;
			parent.insert_right(replacement);
		}

		node.unlink();
		let (_, val) = unsafe { drop_node(node) };

		if both_black {
			Self::remove_fix_double_black(replacement);
			self.update_root(replacement);
		} else {
			replacement.color = NodeColor::Black;
		}

		val
	}

	/// Removes a value from the tree. If the value is present several times in
	/// the tree, only one node is removed.
	///
	/// `key` is the key to select the node to remove.
	///
	/// If the key exists, the function returns the value of the removed node.
	pub fn remove(&mut self, key: &K) -> Option<V> {
		let node = self.get_mut_node(key)?;
		let value = self.remove_node(node);

		//#[cfg(config_debug_debug)]
		//self.check();
		Some(value)
	}

	/// Calls the given closure for every nodes in the subtree with root `root`.
	///
	/// `traversal_order` defines the order in which the tree is traversed.
	fn foreach_nodes<F: FnMut(&Node<K, V>)>(
		root: &Node<K, V>,
		f: &mut F,
		traversal_order: TraveralOrder,
	) {
		let (first, second) = if traversal_order == TraveralOrder::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};

		if traversal_order == TraveralOrder::PreOrder {
			f(root);
		}

		if let Some(mut n) = first {
			Self::foreach_nodes(unsafe { n.as_mut() }, f, traversal_order);
		}

		if traversal_order == TraveralOrder::InOrder
			|| traversal_order == TraveralOrder::ReverseInOrder
		{
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes(unsafe { n.as_mut() }, f, traversal_order);
		}

		if traversal_order == TraveralOrder::PostOrder {
			f(root);
		}
	}

	/// Calls the given closure for every nodes in the subtree with root `root`.
	///
	/// `traversal_order` defines the order in which the tree is traversed.
	fn foreach_nodes_mut<F: FnMut(&mut Node<K, V>)>(
		root: &mut Node<K, V>,
		f: &mut F,
		traversal_order: TraveralOrder,
	) {
		let (first, second) = if traversal_order == TraveralOrder::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};

		if traversal_order == TraveralOrder::PreOrder {
			f(root);
		}

		if let Some(mut n) = first {
			Self::foreach_nodes_mut(unsafe { n.as_mut() }, f, traversal_order);
		}

		if traversal_order == TraveralOrder::InOrder
			|| traversal_order == TraveralOrder::ReverseInOrder
		{
			f(root);
		}

		if let Some(mut n) = second {
			Self::foreach_nodes_mut(unsafe { n.as_mut() }, f, traversal_order);
		}

		if traversal_order == TraveralOrder::PostOrder {
			f(root);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach<F: FnMut(&K, &V)>(&self, mut f: F, traversal_order: TraveralOrder) {
		if let Some(n) = self.root {
			Self::foreach_nodes(
				unsafe { n.as_ref() },
				&mut |n: &Node<K, V>| {
					f(&n.key, &n.value);
				},
				traversal_order,
			);
		}
	}

	/// Calls the given closure for every values.
	pub fn foreach_mut<F: FnMut(&K, &mut V)>(&mut self, mut f: F, traversal_order: TraveralOrder) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(
				unsafe { n.as_mut() },
				&mut |n: &mut Node<K, V>| {
					f(&n.key, &mut n.value);
				},
				traversal_order,
			);
		}
	}

	/// Checks the integrity of the tree.
	///
	/// If the tree is invalid, the function makes the kernel panic.
	///
	/// This function is available only in debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		if let Some(root) = self.root {
			let mut explored_nodes = Vec::<*const c_void>::new();

			Self::foreach_nodes(
				unsafe { root.as_ref() },
				&mut |n: &Node<K, V>| {
					assert!(n as *const _ as usize >= memory::PROCESS_END as usize);

					for e in explored_nodes.iter() {
						assert_ne!(*e, n as *const _ as *const c_void);
					}
					explored_nodes.push(n as *const _ as *const c_void).unwrap();

					if let Some(left) = n.get_left() {
						assert!(left as *const _ as usize >= memory::PROCESS_END as usize);
						assert!(ptr::eq(
							left.get_parent().unwrap() as *const _,
							n as *const _
						));
						assert!(left.key <= n.key);
					}

					if let Some(right) = n.get_right() {
						assert!(right as *const _ as usize >= memory::PROCESS_END as usize);
						assert!(ptr::eq(
							right.get_parent().unwrap() as *const _,
							n as *const _
						));
						assert!(right.key >= n.key);
					}
				},
				TraveralOrder::PreOrder,
			);
		}
	}

	/// Returns an iterator for the current binary tree.
	#[inline]
	pub fn iter(&self) -> MapIterator<K, V> {
		MapIterator::new(self)
	}

	/// Returns a mutable iterator for the current binary tree.
	#[inline]
	pub fn iter_mut(&mut self) -> MapMutIterator<K, V> {
		MapMutIterator::new(self)
	}

	/// Returns an immutable iterator on the given range of keys.
	#[inline]
	pub fn range<R: RangeBounds<K>>(&self, range: R) -> MapRange<'_, K, V, R> {
		MapRange::new(self, range)
	}

	/// Returns a mutable iterator on the given range of keys.
	#[inline]
	pub fn range_mut<R: RangeBounds<K>>(&mut self, range: R) -> MapMutRange<'_, K, V, R> {
		MapMutRange::new(self, range)
	}

	/// Retains only the elements specified by the predicate.
	pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut _f: F) {
		// TODO
		todo!();
	}
}

/// An iterator for the Map structure. This iterator traverses the tree in pre
/// order.
pub struct MapIterator<'a, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'a Map<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
}

impl<'a, K: Ord, V> MapIterator<'a, K, V> {
	/// Creates an iterator for the given reference.
	fn new(tree: &'a Map<K, V>) -> Self {
		MapIterator {
			tree,
			node: tree
				.root
				.map(|mut n| unsafe { NonNull::new(Map::get_leftmost_node(n.as_mut())).unwrap() }),
		}
	}
}

impl<'a, K: 'static + Ord, V> Iterator for MapIterator<'a, K, V> {
	type Item = (&'a K, &'a V);

	fn next(&mut self) -> Option<Self::Item> {
		let node = self.node;
		if let Some(n) = unwrap_pointer(&node) {
			if let Some(mut node) = n.get_right() {
				while let Some(n) = node.get_left() {
					node = n;
				}

				self.node = NonNull::new(node as *const _ as *mut _);
			} else {
				let mut tmp = n;
				let mut n = n.get_parent();
				while let Some(inner) = n {
					if !tmp.is_right_child() {
						break;
					}

					tmp = inner;
					n = tmp.get_parent();
				}

				self.node = n.map(|n| NonNull::new(n as *const _ as *mut _).unwrap());
			}
		}

		let node = unwrap_pointer(&node)?;
		Some((&node.key, &node.value))
	}
}

impl<'a, K: 'static + Ord, V> IntoIterator for &'a Map<K, V> {
	type IntoIter = MapIterator<'a, K, V>;
	type Item = (&'a K, &'a V);

	fn into_iter(self) -> Self::IntoIter {
		MapIterator::new(self)
	}
}

/// An iterator for the `Map` structure.
///
/// This iterator traverses the tree in pre order.
pub struct MapMutIterator<'a, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'a mut Map<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
}

impl<'a, K: Ord, V> MapMutIterator<'a, K, V> {
	/// Creates an iterator for the given reference.
	fn new(tree: &'a mut Map<K, V>) -> Self {
		let node = tree
			.root
			.map(|mut n| unsafe { NonNull::new(Map::get_leftmost_node(n.as_mut())).unwrap() });

		MapMutIterator {
			tree,
			node,
		}
	}
}

impl<'a, K: 'static + Ord, V> Iterator for MapMutIterator<'a, K, V> {
	type Item = (&'a K, &'a mut V);

	fn next(&mut self) -> Option<Self::Item> {
		let mut node = self.node;
		if let Some(n) = unwrap_pointer(&node) {
			if let Some(mut node) = n.get_right() {
				while let Some(n) = node.get_left() {
					node = n;
				}

				self.node = NonNull::new(node as *const _ as *mut _);
			} else {
				let mut tmp = n;
				let mut n = n.get_parent();
				while let Some(inner) = n {
					if !tmp.is_right_child() {
						break;
					}

					tmp = inner;
					n = tmp.get_parent();
				}

				self.node = n.map(|n| NonNull::new(n as *const _ as *mut _).unwrap());
			}
		}

		let node = unwrap_pointer_mut(&mut node)?;
		Some((&node.key, &mut node.value))
	}
}

impl<'a, K: 'static + Ord, V> IntoIterator for &'a mut Map<K, V> {
	type IntoIter = MapMutIterator<'a, K, V>;
	type Item = (&'a K, &'a mut V);

	fn into_iter(self) -> Self::IntoIter {
		MapMutIterator::new(self)
	}
}

/// Iterator over a range of keys in a map.
pub struct MapRange<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> {
	/// Inner iterator.
	iter: MapIterator<'m, K, V>,
	/// The range to iterate on.
	range: R,
}

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> MapRange<'m, K, V, R> {
	/// Creates an iterator for the given reference.
	fn new(tree: &'m Map<K, V>, range: R) -> Self {
		let node = tree.get_start_node(range.start_bound());

		let iter = MapIterator {
			tree,
			node,
		};

		Self {
			iter,
			range,
		}
	}
}

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> Iterator for MapRange<'m, K, V, R> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let (key, value) = self.iter.next()?;

		if self.range.contains(key) {
			Some((key, value))
		} else {
			None
		}
	}
}

/// Iterator over a range of keys in a map (mutably).
pub struct MapMutRange<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> {
	/// Inner iterator.
	iter: MapMutIterator<'m, K, V>,
	/// The range to iterate on.
	range: R,
}

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> MapMutRange<'m, K, V, R> {
	/// Creates an iterator for the given reference.
	fn new(tree: &'m mut Map<K, V>, range: R) -> Self {
		let node = tree.get_start_node(range.start_bound());

		let iter = MapMutIterator {
			tree,
			node,
		};

		Self {
			iter,
			range,
		}
	}
}

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> Iterator for MapMutRange<'m, K, V, R> {
	type Item = (&'m K, &'m mut V);

	fn next(&mut self) -> Option<Self::Item> {
		let (key, value) = self.iter.next()?;

		if self.range.contains(key) {
			Some((key, value))
		} else {
			None
		}
	}
}

impl<K: 'static + TryClone + Ord, V: TryClone> TryClone for Map<K, V> {
	fn try_clone(&self) -> Result<Self, Errno> {
		let mut new = Self::new();
		for (k, v) in self {
			let k_res: Result<_, Errno> = k.try_clone().map_err(Into::into);
			let v_res: Result<_, Errno> = v.try_clone().map_err(Into::into);

			new.insert(k_res?, v_res?)?;
		}
		Ok(new)
	}
}

impl<K: 'static + Ord + fmt::Debug, V> fmt::Debug for Map<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(mut n) = self.root {
			Self::foreach_nodes(
				unsafe { n.as_mut() },
				&mut |n| {
					for _ in 0..n.get_node_depth() {
						let _ = write!(f, "\t");
					}

					let color = if n.color == NodeColor::Red {
						"red"
					} else {
						"black"
					};
					let _ = writeln!(f, "{:?} ({:?})", n.key, color);
				},
				TraveralOrder::ReverseInOrder,
			);
			Ok(())
		} else {
			write!(f, "<Empty tree>")
		}
	}
}

impl<K: 'static + Ord, V> Drop for Map<K, V> {
	fn drop(&mut self) {
		if let Some(mut n) = self.root {
			Self::foreach_nodes_mut(
				unsafe { n.as_mut() },
				&mut |n| unsafe {
					drop_node(n);
				},
				TraveralOrder::PostOrder,
			);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn binary_tree0() {
		let b = Map::<i32, ()>::new();
		assert!(b.get(0).is_none());
	}

	#[test_case]
	fn binary_tree_insert0() {
		let mut b = Map::<i32, i32>::new();

		b.insert(0, 0).unwrap();
		assert_eq!(*b.get(0).unwrap(), 0);
	}

	#[test_case]
	fn binary_tree_insert1() {
		let mut b = Map::<i32, i32>::new();

		for i in 0..10 {
			b.insert(i, i).unwrap();
		}

		for i in 0..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert2() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert3() {
		let mut b = Map::<u32, u32>::new();

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
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			for i in i..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}

			b.remove(&i);

			assert!(b.get(i).is_none());
			for i in (i + 1)..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove1() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove2() {
		let mut b = Map::<i32, i32>::new();

		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}

		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove3() {
		let mut b = Map::<i32, i32>::new();

		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
			assert_eq!(b.remove(&i).unwrap(), i);
			assert!(b.get(i).is_none());
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove4() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.remove(&i).unwrap(), i);
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove5() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		for i in -9..10 {
			if i % 2 == 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				assert_eq!(b.remove(&i).unwrap(), i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(!b.is_empty());

		for i in -9..10 {
			if i % 2 != 0 {
				assert_eq!(*b.get(i).unwrap(), i);
				assert_eq!(b.remove(&i).unwrap(), i);
				assert!(b.get(i).is_none());
			}
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_foreach0() {
		let b = Map::<i32, i32>::new();
		b.foreach(
			|_, _| {
				assert!(false);
			},
			TraveralOrder::PreOrder,
		);
	}

	#[test_case]
	fn binary_tree_foreach1() {
		let mut b = Map::<i32, i32>::new();
		b.insert(0, 0).unwrap();

		let mut passed = false;
		b.foreach(
			|key, _| {
				assert!(!passed);
				assert_eq!(*key, 0);
				passed = true;
			},
			TraveralOrder::PreOrder,
		);
		assert!(passed);
	}

	// TODO test iterators (both exhaustive and range)
}
