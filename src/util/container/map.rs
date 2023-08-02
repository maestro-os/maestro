//! This module implements a binary tree container.

use crate::errno::Errno;
use crate::memory;
use crate::memory::malloc;
use crate::util::TryClone;
use core::cell::UnsafeCell;
use core::cmp::Ordering;
use core::fmt;
use core::intrinsics::likely;
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
fn unwrap_pointer<K, V>(ptr: Option<NonNull<Node<K, V>>>) -> Option<&'static mut Node<K, V>> {
	ptr.map(|mut p| unsafe {
		debug_assert!(p.as_ptr() as usize >= memory::PROCESS_END as usize);
		p.as_mut()
	})
}

impl<K: 'static + Ord, V: 'static> Node<K, V> {
	/// Creates a new node with the given `value`.
	///
	/// The node is colored `Red` by default.
	fn new(key: K, value: V) -> Result<NonNull<Self>, Errno> {
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
	fn is_red(&self) -> bool {
		self.color == NodeColor::Red
	}

	/// Tells whether the node is black.
	#[inline]
	fn is_black(&self) -> bool {
		self.color == NodeColor::Black
	}

	/// Returns a reference to the parent child node.
	#[inline]
	fn get_parent(&self) -> Option<&'static mut Self> {
		unwrap_pointer(self.parent)
	}

	/// Returns a reference to the grandparent node.
	#[inline]
	fn get_grandparent(&self) -> Option<&'static mut Self> {
		self.get_parent()?.get_parent()
	}

	/// Returns a mutable reference to the parent child node.
	#[inline]
	fn get_left(&self) -> Option<&'static mut Self> {
		unwrap_pointer(self.left)
	}

	/// Returns a reference to the left child node.
	#[inline]
	fn get_right(&self) -> Option<&'static mut Self> {
		unwrap_pointer(self.right)
	}

	/// Tells whether the node is a left child.
	#[inline]
	fn is_left_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_left() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Tells whether the node is a right child.
	#[inline]
	fn is_right_child(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			if let Some(n) = parent.get_right() {
				return ptr::eq(n as *const _, self as *const _);
			}
		}

		false
	}

	/// Returns a reference to the sibling node.
	#[inline]
	fn get_sibling(&self) -> Option<&'static mut Self> {
		let parent = self.get_parent()?;

		if self.is_left_child() {
			parent.get_right()
		} else {
			parent.get_left()
		}
	}

	/// Returns a reference to the uncle node.
	#[inline]
	fn get_uncle(&self) -> Option<&'static mut Self> {
		self.get_parent()?.get_sibling()
	}

	/// Tells whether the node and its parent and grandparent form a triangle.
	#[inline]
	fn is_triangle(&self) -> bool {
		if let Some(parent) = self.get_parent() {
			return self.is_left_child() != parent.is_left_child();
		}

		false
	}

	/// Tells whether the node has at least one red child.
	#[inline]
	fn has_red_child(&self) -> bool {
		self.get_left().map_or(false, |n| n.is_red())
			|| self.get_right().map_or(false, |n| n.is_red())
	}

	/// Applies a left tree rotation with the current node as root.
	///
	/// If the current node doesn't have a right child, the function does
	/// nothing.
	fn left_rotate(&mut self) {
		let Some(pivot) = self.get_right() else {
            return;
		};

		if let Some(parent) = self.get_parent() {
			if self.is_right_child() {
				parent.right = NonNull::new(pivot);
			} else {
				parent.left = NonNull::new(pivot);
			}

			pivot.parent = NonNull::new(parent);
		} else {
			pivot.parent = None;
		}

		let left = pivot.get_left();
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
	fn right_rotate(&mut self) {
		let Some(pivot) = self.get_left() else {
            return;
		};

		if let Some(parent) = self.get_parent() {
			if self.is_left_child() {
				parent.left = NonNull::new(pivot);
			} else {
				parent.right = NonNull::new(pivot);
			}

			pivot.parent = NonNull::new(parent);
		} else {
			pivot.parent = None;
		}

		let right = pivot.get_right();
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
	fn insert_left(&mut self, node: &mut Node<K, V>) {
		debug_assert!(self.left.is_none());
		debug_assert!(node.parent.is_none());

		self.left = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Inserts the given node `node` to right of the current node.
	///
	/// If the node already has a right child, the behaviour is undefined.
	#[inline]
	fn insert_right(&mut self, node: &mut Node<K, V>) {
		debug_assert!(self.right.is_none());
		debug_assert!(node.parent.is_none());

		self.right = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Returns the depth of the node in the tree.
	///
	/// This function has `O(log n)` complexity.
	fn get_node_depth(&self) -> usize {
		self.get_parent().map_or(0, |n| n.get_node_depth() + 1)
	}

	/// Unlinks the node from its tree.
	fn unlink(&mut self) {
		if let Some(parent) = self.get_parent() {
			if self.is_left_child() {
				parent.left = None;
			} else if self.is_right_child() {
				parent.right = None;
			}

			self.parent = None;
		}

		if let Some(left) = self.get_left() {
			if let Some(p) = left.parent {
				if ptr::eq(p.as_ptr(), self as _) {
					left.parent = None;
				}
			}

			self.left = None;
		}

		if let Some(right) = self.get_right() {
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
	root: UnsafeCell<Option<NonNull<Node<K, V>>>>,
	/// The current number of elements in the tree.
	len: usize,
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
			root: UnsafeCell::new(None),
			len: 0,
		}
	}

	/// Returns the number of elements in the tree.
	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Tells whether the tree is empty.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns a reference to the root node.
	#[inline]
	fn get_root(&self) -> Option<&'static mut Node<K, V>> {
		unsafe { Some((&mut *self.root.get()).as_mut()?.as_mut()) }
	}

	/// Returns an reference to the leftmost node in the tree.
	fn get_leftmost_node(node: &'static mut Node<K, V>) -> &'static mut Node<K, V> {
		let mut n = node;
		while let Some(left) = n.get_left() {
			n = left;
		}

		n
	}

	/// Searches for a node with the given key in the tree and returns a
	/// reference.
	///
	/// `key` is the key to find.
	fn get_node(&self, key: &K) -> Option<&'static mut Node<K, V>> {
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

	/// Returns the start node for a range iterator starting at `start`.
	fn get_start_node(&self, start: Bound<&K>) -> Option<NonNull<Node<K, V>>> {
		let mut node = self.get_root();

		let (key, exclude) = match start {
			Bound::Unbounded => return NonNull::new(Self::get_leftmost_node(node?)),

			Bound::Included(key) => (key, false),
			Bound::Excluded(key) => (key, true),
		};

		// The last in-bound element encountered.
		let mut last = None;

		while let Some(n) = node {
			let in_bound = match n.key.cmp(&key) {
				Ordering::Less => false,
				Ordering::Greater => true,
				Ordering::Equal => !exclude,
			};
			if in_bound {
				node = n.get_left();
				last = Some(n);
			} else {
				node = n.get_right();
			}
		}

		last.and_then(|n| NonNull::new(n))
	}

	/// Returns the first key/value pair of the tree. The returned key is the minimum present in
	/// the tree.
	///
	/// If the tree is empty, the function returns `None`.
	pub fn first_key_value(&self) -> Option<(&K, &V)> {
		let node = Self::get_leftmost_node(self.get_root()?);
		Some((&node.key, &node.value))
	}

	/// Removes and returns the first key/value pair of the tree. The returned key is the minimum
	/// present in the tree.
	///
	/// If the tree is empty, the function returns `None`.
	pub fn pop_first(&mut self) -> Option<(K, V)> {
		let node = Self::get_leftmost_node(self.get_root()?);
		let (key, value) = self.remove_node(node);
		Some((key, value))
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
		let node = self.get_node(&key)?;
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
		let mut node = self.get_root();

		while let Some(n) = node {
			let ord = cmp(&n.key, &n.value);

			match ord {
				Ordering::Less => node = n.get_left(),
				Ordering::Greater => node = n.get_right(),
				Ordering::Equal => return Some(&mut n.value),
			}
		}

		None
	}

	/// Updates the root of the tree.
	///
	/// `node` is a node inserted in the tree.
	fn update_root(&mut self, mut node: &mut Node<K, V>) {
		while let Some(n) = node.get_parent() {
			node = n;
		}

		*self.root.get_mut() = NonNull::new(node);
	}

	/// For node insertion, returns the parent node on which it will be
	/// inserted.
	fn get_insert_node(&mut self, key: &K) -> Option<&'static mut Node<K, V>> {
		let mut node = self.get_root();

		while let Some(n) = node {
			let ord = key.cmp(&n.key);

			let next = match ord {
				Ordering::Less => n.get_left(),
				Ordering::Greater => n.get_right(),
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
		let Some(parent) = node.get_parent() else {
			node.color = NodeColor::Black;
            return;
        };
		if parent.is_black() {
			return;
		}

		// The node's parent exists and is red
		if let Some(uncle) = node.get_uncle() {
			if uncle.is_red() {
				let grandparent = parent.get_parent().unwrap();
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

		let parent = node.get_parent().unwrap();
		let grandparent = parent.get_parent().unwrap();

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
				debug_assert!(self.get_root().is_none());

				let mut node = Node::new(key, val)?;
				unsafe { node.as_mut() }
			}
		};

		Self::insert_equilibrate(n);
		//#[cfg(config_debug_debug)]
		//self.check();
		self.update_root(n);

		self.len += 1;
		Ok(&mut n.value)
	}

	/// Fixes the tree after deletion in the case where the deleted node and its
	/// replacement are both black.
	///
	/// `node` is the node to fix.
	fn remove_fix_double_black(node: &mut Node<K, V>) {
		let Some(parent) = node.get_parent() else {
            return;
		};
		let Some(sibling) = node.get_sibling() else {
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

		let s_left = sibling.get_left();
		let s_right = sibling.get_right();
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
	fn remove_node(&mut self, node: &mut Node<K, V>) -> (K, V) {
		let left = node.get_left();
		let right = node.get_right();
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

		let both_black = node.is_black() && replacement.as_ref().map_or(true, |r| r.is_black());
		let parent = node.get_parent();

		let Some(replacement) = replacement else {
            // The node has no children

            match parent {
                Some(_parent) => {
                    if both_black {
                        Self::remove_fix_double_black(node);
                        self.update_root(node);
                    } else if let Some(sibling) = node.get_sibling() {
                        sibling.color = NodeColor::Red;
                    }
                }

				// The node is root
                None => {
                    debug_assert_eq!(
                        self.get_root().unwrap() as *mut Node<K, V>,
                        node as *mut _
                    );

                    *self.root.get_mut() = None;
                }
            }

            node.unlink();

            self.len -= 1;
			return unsafe { drop_node(node) };
        };

		if node.get_left().is_some() && node.get_right().is_some() {
			mem::swap(&mut node.key, &mut replacement.key);
			mem::swap(&mut node.value, &mut replacement.value);

			return self.remove_node(replacement);
		}

		let Some(parent) = parent else {
            // The node is the root

            replacement.unlink();
            let (mut key, value) = unsafe { drop_node(replacement) };

            node.left = None;
            node.right = None;

            mem::swap(&mut key, &mut node.key);
            let mut val = value;
            mem::swap(&mut val, &mut node.value);

            self.len -= 1;
            return (key, val);
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
		let (key, val) = unsafe { drop_node(node) };

		if both_black {
			Self::remove_fix_double_black(replacement);
			self.update_root(replacement);
		} else {
			replacement.color = NodeColor::Black;
		}

		self.len -= 1;
		(key, val)
	}

	/// Removes a value from the tree. If the value is present several times in
	/// the tree, only one node is removed.
	///
	/// `key` is the key to select the node to remove.
	///
	/// If the key exists, the function returns the value of the removed node.
	pub fn remove(&mut self, key: &K) -> Option<V> {
		let node = self.get_node(key)?;
		let (_, value) = self.remove_node(node);

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

	/// Checks the integrity of the tree.
	///
	/// If the tree is invalid, the function makes the kernel panic.
	///
	/// This function is available only in debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		let Some(root) = self.get_root() else {
            return;
		};

		let mut explored_nodes = Vec::<*const c_void>::new();

		Self::foreach_nodes(
			root,
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

	/// Returns an immutable iterator for the current binary tree.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	#[inline]
	pub fn iter(&self) -> MapIterator<K, V> {
		let node = self
			.get_root()
			.map(|n| NonNull::new(Map::get_leftmost_node(n)).unwrap());

		MapIterator {
			tree: self,

			node,
			i: 0,
		}
	}

	/// Returns a mutable iterator for the current binary tree.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	#[inline]
	pub fn iter_mut(&mut self) -> MapMutIterator<K, V> {
		let node = self
			.get_root()
			.map(|n| NonNull::new(Map::get_leftmost_node(n)).unwrap());

		MapMutIterator {
			tree: self,

			node,
			i: 0,
		}
	}

	/// Returns an immutable iterator on the given range of keys.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	#[inline]
	pub fn range<R: RangeBounds<K>>(&self, range: R) -> MapRange<'_, K, V, R> {
		let node = self.get_start_node(range.start_bound());

		MapRange {
			iter: MapIterator {
				tree: self,

				node,
				i: 0,
			},
			range,
		}
	}

	/// Returns a mutable iterator on the given range of keys.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	#[inline]
	pub fn range_mut<R: RangeBounds<K>>(&mut self, range: R) -> MapMutRange<'_, K, V, R> {
		let node = self.get_start_node(range.start_bound());

		MapMutRange {
			iter: MapMutIterator {
				tree: self,

				node,
				i: 0,
			},
			range,
		}
	}

	/// Drains elements than match the given predicate and returns an iterator to drained elements.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	pub fn drain_filter<F>(&mut self, pred: F) -> DrainFilter<'_, K, V, F>
	where
		F: FnMut(&K, &mut V) -> bool,
	{
		let node = self
			.get_root()
			.map(|n| NonNull::new(Map::get_leftmost_node(n)).unwrap());

		DrainFilter {
			tree: self,

			node,
			i: 0,

			pred,
		}
	}

	/// Retains only the elements matching the given predicate.
	///
	/// This function has complexity `O(n)` in time and `O(1)` in space.
	pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut pred: F) {
		self.drain_filter(|k, v| !pred(k, v));
	}
}

/// Returns the next node in an iterator for the given node.
///
/// This is an inner function for node iterators.
fn next_node<K: Ord + 'static, V: 'static>(
	node: &mut Node<K, V>,
) -> Option<&'static mut Node<K, V>> {
	if let Some(mut node) = node.get_right() {
		while let Some(n) = node.get_left() {
			node = n;
		}

		Some(node)
	} else {
		let mut node = node;
		let mut parent = node.get_parent();
		while let Some(p) = parent {
			if !node.is_right_child() {
				return Some(p);
			}

			node = p;
			parent = node.get_parent();
		}

		None
	}
}

/// An iterator for the Map structure. This iterator traverses the tree in pre
/// order.
pub struct MapIterator<'m, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'m Map<K, V>,

	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,
}

impl<'m, K: 'static + Ord, V> Iterator for MapIterator<'m, K, V> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let node = unwrap_pointer(self.node)?;

		self.node = next_node(node).and_then(|n| NonNull::new(n));
		self.i += 1;

		Some((&node.key, &node.value))
	}

	fn count(self) -> usize {
		self.tree.len()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.tree.len() - self.i;
		(len, Some(len))
	}
}

impl<'m, K: 'static + Ord, V> IntoIterator for &'m Map<K, V> {
	type IntoIter = MapIterator<'m, K, V>;
	type Item = (&'m K, &'m V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

/// An iterator for the `Map` structure.
///
/// This iterator traverses the tree in pre order.
pub struct MapMutIterator<'m, K: 'static + Ord, V: 'static> {
	/// The binary tree to iterate into.
	tree: &'m mut Map<K, V>,

	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,
}

impl<'m, K: 'static + Ord, V> Iterator for MapMutIterator<'m, K, V> {
	type Item = (&'m K, &'m mut V);

	fn next(&mut self) -> Option<Self::Item> {
		let node = unwrap_pointer(self.node)?;

		self.node = next_node(node).and_then(|n| NonNull::new(n));
		self.i += 1;

		Some((&node.key, &mut node.value))
	}

	fn count(self) -> usize {
		self.tree.len()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.tree.len() - self.i;
		(len, Some(len))
	}
}

impl<'m, K: 'static + Ord, V> IntoIterator for &'m mut Map<K, V> {
	type IntoIter = MapMutIterator<'m, K, V>;
	type Item = (&'m K, &'m mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

/// Iterator over a range of keys in a map.
pub struct MapRange<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> {
	/// Inner iterator.
	iter: MapIterator<'m, K, V>,
	/// The range to iterate on.
	range: R,
}

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> Iterator for MapRange<'m, K, V, R> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let (key, value) = self.iter.next()?;

		if likely(self.range.contains(key)) {
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

impl<'m, K: 'static + Ord, V: 'static, R: RangeBounds<K>> Iterator for MapMutRange<'m, K, V, R> {
	type Item = (&'m K, &'m mut V);

	fn next(&mut self) -> Option<Self::Item> {
		let (key, value) = self.iter.next()?;

		if likely(self.range.contains(key)) {
			Some((key, value))
		} else {
			None
		}
	}
}

/// An iterator that traverses the tree in ascending order and removes, then yields elements that
/// match the associated predicate.
pub struct DrainFilter<'m, K, V, F>
where
	K: Ord + 'static,
	V: 'static,
	F: FnMut(&K, &mut V) -> bool,
{
	/// The tree to iterate on.
	tree: &'m mut Map<K, V>,

	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,

	/// The predicate to check whether an element must be drained.
	pred: F,
}

impl<'m, K: Ord + 'static, V: 'static, F: FnMut(&K, &mut V) -> bool> Iterator
	for DrainFilter<'m, K, V, F>
{
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		// get next matching node
		let mut node = unwrap_pointer(self.node)?;
		while !(self.pred)(&node.key, &mut node.value) {
			node = next_node(node)?;
		}

		// FIXME: `remove_node` swaps values between nodes, so the node returned by `next_node`
		// becomes invalid
		// get next node
		//let next = next_node(node).and_then(|n| NonNull::new(n));
		let next = self
			.tree
			.get_root()
			.map(|n| NonNull::new(Map::get_leftmost_node(n)).unwrap());

		// remove the current node
		let (k, v) = self.tree.remove_node(node);

		// place cursor on next node
		self.node = next;
		self.i += 1;

		Some((k, v))
	}
}

impl<K: 'static + TryClone + Ord, V: TryClone> TryClone for Map<K, V> {
	fn try_clone(&self) -> Result<Self, Errno> {
		let mut new = Self::new();
		for (k, v) in self {
			new.insert(k.try_clone()?, v.try_clone()?)?;
		}
		Ok(new)
	}
}

impl<K: 'static + Ord + fmt::Debug, V> fmt::Debug for Map<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if let Some(root) = self.get_root() {
			Self::foreach_nodes(
				root,
				&mut |n| {
					for _ in 0..n.get_node_depth() {
						let _ = write!(f, "\t");
					}

					let color = if n.color == NodeColor::Red {
						"red"
					} else {
						"black"
					};
					let _ = writeln!(f, "{:?} ({})", n.key, color);
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
		let Some(root) = self.get_root() else {
            return;
		};

		Self::foreach_nodes_mut(
			root,
			&mut |n| unsafe {
				drop_node(n);
			},
			TraveralOrder::PostOrder,
		);
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn binary_tree0() {
		let b = Map::<i32, ()>::new();
		assert!(b.get(0).is_none());
		assert_eq!(b.len(), 0);
	}

	#[test_case]
	fn binary_tree_insert0() {
		let mut b = Map::<i32, i32>::new();

		b.insert(0, 0).unwrap();
		assert_eq!(*b.get(0).unwrap(), 0);
		assert_eq!(b.len(), 1);
	}

	#[test_case]
	fn binary_tree_insert1() {
		let mut b = Map::<i32, i32>::new();

		for i in 0..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 1) as usize);
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
			assert_eq!(b.len(), (i + 10) as usize);
		}

		for i in -9..10 {
			assert_eq!(*b.get(i).unwrap(), i);
		}
	}

	#[test_case]
	fn binary_tree_insert3() {
		let mut b = Map::<u32, u32>::new();

		let mut val = 0;
		for i in 0..100 {
			val = crate::util::math::pseudo_rand(val, 1664525, 1013904223, 0x100);
			b.insert(val, val).unwrap();
			assert_eq!(b.len(), (i + 1) as usize);
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
			assert_eq!(b.len(), (i + 10) as usize);
		}

		let mut count = b.len();
		for i in -9..10 {
			for i in i..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}

			b.remove(&i);

			assert!(b.get(i).is_none());
			for i in (i + 1)..10 {
				assert_eq!(*b.get(i).unwrap(), i);
			}

			count -= 1;
			assert_eq!(b.len(), count);
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove1() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}

		let mut count = b.len();
		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(i).is_none());

			count -= 1;
			assert_eq!(b.len(), count);
		}

		assert!(b.is_empty());
	}

	#[test_case]
	fn binary_tree_remove2() {
		let mut b = Map::<i32, i32>::new();

		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}

		let mut count = b.len();
		for i in (-9..10).rev() {
			assert_eq!(*b.get(i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(i).is_none());

			count -= 1;
			assert_eq!(b.len(), count);
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
	fn binary_tree_iter0() {
		let b = Map::<i32, i32>::new();
		assert_eq!(b.iter().count(), 0);
	}

	#[test_case]
	fn binary_tree_iter1() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}

		assert_eq!(b.iter().count(), b.len());
		assert!(b.iter().is_sorted());
	}

	#[test_case]
	fn binary_tree_range0() {
		let b = Map::<i32, i32>::new();
		assert_eq!(b.range(..).count(), 0);
		assert_eq!(b.range(0..).count(), 0);
		assert_eq!(b.range(1..).count(), 0);
		assert_eq!(b.range(1..100).count(), 0);
		assert_eq!(b.range(..100).count(), 0);
	}

	#[test_case]
	fn binary_tree_range1() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}

		assert_eq!(b.range(..).count(), b.len());
		assert!(b.range(..).is_sorted());

		assert_eq!(b.range(0..10).count(), 10);
		assert!(b.range(0..10).is_sorted());

		assert_eq!(b.range(..10).count(), b.len());
		assert!(b.range(..10).is_sorted());

		assert_eq!(b.range(0..).count(), 10);
		assert!(b.range(0..).is_sorted());
	}

	#[test_case]
	fn binary_tree_drain0() {
		let mut b = Map::<i32, i32>::new();

		for i in -9..10 {
			b.insert(i, i).unwrap();
		}

		let len = b.len();
		assert!(b
			.drain_filter(|k, v| k == v && k % 2 == 0)
			.all(|(k, v)| k == v && k % 2 == 0));
		assert_eq!(b.len(), len / 2 + 1);
		assert!(b.into_iter().all(|(k, v)| k == v && k % 2 != 0));
	}
}
