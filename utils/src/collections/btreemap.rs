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

//! Implementation of the [`BTreeMap`] object.
//!
//! See [Rust's documentation](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html) for details.

use crate::{
	errno::{AllocResult, CollectResult},
	AllocError, TryClone,
};
use alloc::alloc::Global;
use core::{
	alloc::{Allocator, Layout},
	borrow::Borrow,
	cell::UnsafeCell,
	cmp::Ordering,
	fmt,
	intrinsics::likely,
	iter::{FusedIterator, TrustedLen},
	mem,
	ops::{Bound, RangeBounds},
	ptr,
	ptr::NonNull,
};

// TODO refactor to use an actual B-tree instead of a simple Red-Black tree with a single element
// per node

// TODO implement DoubleEndedIterator for all iterators

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

/// Drops the node at the given pointer, except the key and value fields which
/// are returned.
///
/// # Safety
///
/// The caller must ensure the pointer points to a valid node and must not use it after calling
/// this function since it will be dropped.
#[inline]
unsafe fn drop_node<K, V>(ptr: NonNull<Node<K, V>>) -> (K, V) {
	let node = ptr.read();
	Global.deallocate(ptr.cast(), Layout::new::<Node<K, V>>());
	let Node::<K, V> {
		key,
		value,
		..
	} = node;
	(key, value)
}

/// Unwraps the given pointer option into a reference option.
#[inline]
fn unwrap_pointer<'a, K, V>(ptr: Option<NonNull<Node<K, V>>>) -> Option<&'a mut Node<K, V>> {
	ptr.map(|mut p| unsafe { p.as_mut() })
}

impl<K, V> Node<K, V> {
	/// Creates a new node with the given `value`.
	///
	/// The node is colored [`NodeColor::Red`] by default.
	fn new(key: K, value: V) -> AllocResult<NonNull<Self>> {
		let s = Self {
			parent: None,
			left: None,
			right: None,
			color: NodeColor::Red,

			key,
			value,
		};
		let ptr = Global.allocate(Layout::new::<Self>())?.cast();
		unsafe {
			ptr.write(s);
		}
		Ok(ptr)
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
	fn get_parent<'a>(&self) -> Option<&'a mut Self> {
		unwrap_pointer(self.parent)
	}

	/// Returns a mutable reference to the parent child node.
	#[inline]
	fn get_left<'a>(&self) -> Option<&'a mut Self> {
		unwrap_pointer(self.left)
	}

	/// Returns a reference to the left child node.
	#[inline]
	fn get_right<'a>(&self) -> Option<&'a mut Self> {
		unwrap_pointer(self.right)
	}

	/// Tells whether the node is a left child.
	#[inline]
	fn is_left_child(&self) -> bool {
		self.get_parent()
			.and_then(|parent| parent.get_left())
			.map(|cur| ptr::eq(cur as *const Self, self as *const Self))
			.unwrap_or(false)
	}

	/// Tells whether the node is a right child.
	#[inline]
	fn is_right_child(&self) -> bool {
		self.get_parent()
			.and_then(|parent| parent.get_right())
			.map(|cur| ptr::eq(cur as *const Self, self as *const Self))
			.unwrap_or(false)
	}

	/// Returns a reference to the sibling node.
	#[inline]
	fn get_sibling<'a>(&self) -> Option<&'a mut Self> {
		let parent = self.get_parent()?;
		if self.is_left_child() {
			parent.get_right()
		} else {
			parent.get_left()
		}
	}

	/// Returns a reference to the uncle node.
	#[inline]
	fn get_uncle<'a>(&self) -> Option<&'a mut Self> {
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
	/// If the node already has a left child, the old node is leaked.
	#[inline]
	fn insert_left(&mut self, node: &mut Node<K, V>) {
		self.left = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/// Inserts the given node `node` to right of the current node.
	///
	/// If the node already has a right child, the old node is leaked.
	#[inline]
	fn insert_right(&mut self, node: &mut Node<K, V>) {
		self.right = NonNull::new(node);
		node.parent = NonNull::new(self);
	}

	/*
	/// Returns the depth of the node in the tree.
	///
	/// This function has `O(log n)` complexity.
	fn get_node_depth(&self) -> usize {
		self.get_parent().map_or(0, |n| n.get_node_depth() + 1)
	}*/

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

/// Searches for a node in the tree with `node` as root and returns a
/// reference to it.
///
/// `cmp` is the comparison function to use for the search.
fn get_node<K: Ord, V>(
	mut node: &mut Node<K, V>,
	cmp: impl Fn(&K) -> Ordering,
) -> Result<&mut Node<K, V>, &mut Node<K, V>> {
	loop {
		let next = match cmp(&node.key) {
			Ordering::Less => node.get_left(),
			Ordering::Greater => node.get_right(),
			Ordering::Equal => break Ok(node),
		};
		let Some(next) = next else {
			// The node cannot be found, return the node on which it would be inserted
			break Err(node);
		};
		node = next;
	}
}

/// Returns a reference to the leftmost node in the tree.
fn get_leftmost_node<K, V>(node: &mut Node<K, V>) -> &mut Node<K, V> {
	let mut n = node;
	while let Some(left) = n.get_left() {
		n = left;
	}
	n
}

/// Returns the start node for a range iterator starting at `start`.
fn get_start_node<K: Ord, V>(
	mut node: &mut Node<K, V>,
	start: Bound<&K>,
) -> Option<NonNull<Node<K, V>>> {
	let (key, exclude) = match start {
		Bound::Unbounded => return NonNull::new(get_leftmost_node(node)),
		Bound::Included(key) => (key, false),
		Bound::Excluded(key) => (key, true),
	};
	// The last in-bound element encountered.
	let mut last = None;
	loop {
		let in_bound = match node.key.cmp(key) {
			Ordering::Less => false,
			Ordering::Greater => true,
			Ordering::Equal => !exclude,
		};
		let next = if in_bound {
			let next = node.get_left();
			last = Some(node);
			next
		} else {
			node.get_right()
		};
		let Some(next) = next else {
			break;
		};
		node = next;
	}
	last.map(NonNull::from)
}

/// Balances the tree after insertion of node `node`.
fn insert_balance<K, V>(mut node: &mut Node<K, V>) {
	let Some(parent) = node.get_parent() else {
		node.color = NodeColor::Black;
		return;
	};
	if parent.is_black() {
		return;
	}
	if parent.get_parent().is_none() {
		// The parent is the root and is red
		parent.color = NodeColor::Black;
		return;
	}
	// The node's parent exists and is red
	if let Some(uncle) = node.get_uncle() {
		if uncle.is_red() {
			let grandparent = parent.get_parent().unwrap();
			parent.color = NodeColor::Black;
			uncle.color = NodeColor::Black;
			grandparent.color = NodeColor::Red;
			insert_balance(grandparent);
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

/// An entry with a used key.
pub struct OccupiedEntry<'t, K: Ord, V> {
	/// The entry's node.
	node: &'t mut Node<K, V>,
}

impl<'t, K: Ord, V> OccupiedEntry<'t, K, V> {
	/// Returns an immutable reference to the key.
	pub fn key(&self) -> &K {
		&self.node.key
	}

	/// Returns an immutable reference to the value.
	pub fn get(&self) -> &V {
		&self.node.value
	}

	/// Returns a mutable reference to the value.
	pub fn get_mut(&mut self) -> &mut V {
		&mut self.node.value
	}

	/// Converts the [`OccupiedEntry`] into a mutable reference to the value in the entry with a
	/// lifetime bound to the map itself.
	pub fn into_mut(self) -> &'t mut V {
		&mut self.node.value
	}

	/// Sets the value to `value` and returns the previous value.
	pub fn insert(&mut self, value: V) -> V {
		mem::replace(self.get_mut(), value)
	}
}

/// An entry with an unused key.
pub struct VacantEntry<'t, K: Ord, V> {
	/// The tree in which the entry is located.
	tree: &'t mut BTreeMap<K, V>,
	/// The key to use for insertion.
	key: K,
	/// The parent of the node to be created for insertion.
	///
	/// If `None`, the node is inserted at the root of the tree.
	parent: Option<&'t mut Node<K, V>>,
}

impl<'t, K: Ord, V> VacantEntry<'t, K, V> {
	/// Inserts the given `value` and returns a reference to it.
	pub fn insert(self, value: V) -> AllocResult<&'t mut V> {
		let mut node = Node::new(self.key, value)?;
		let n = unsafe { node.as_mut() };
		match self.parent {
			Some(parent) => {
				match n.key.cmp(&parent.key) {
					Ordering::Less => parent.insert_left(n),
					Ordering::Greater => parent.insert_right(n),
					// If equal, the key is already used and `self` should not exist
					_ => unreachable!(),
				}
				insert_balance(n);
				self.tree.update_root(n);
			}
			// The tree is empty. Insert as root
			None => *self.tree.root.get_mut() = Some(node),
		}
		self.tree.len += 1;
		#[cfg(config_debug_debug)]
		self.tree.check();
		Ok(&mut n.value)
	}
}

/// An entry in a [`BTreeMap`].
pub enum Entry<'t, K: Ord, V> {
	Occupied(OccupiedEntry<'t, K, V>),
	Vacant(VacantEntry<'t, K, V>),
}

/// Specifies the order in which the tree is to be traversed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraversalOrder {
	/// Accesses the data, then left child, then right child
	PreOrder,
	/// Accesses left child, then the data, then right child
	InOrder,
	/// Accesses right child, then the data, then left child
	ReverseInOrder,
	/// Accesses left child, then right child, then the data
	PostOrder,
}

/// The implementation of the B-tree map.
pub struct BTreeMap<K: Ord, V> {
	/// The root node of the binary tree.
	root: UnsafeCell<Option<NonNull<Node<K, V>>>>,
	/// The current number of elements in the tree.
	len: usize,
}

impl<K: Ord, V> Default for BTreeMap<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: Ord, V> BTreeMap<K, V> {
	/// Creates a new empty binary tree.
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
	fn get_root<'a>(&self) -> Option<&'a mut Node<K, V>> {
		unsafe { Some((*self.root.get()).as_mut()?.as_mut()) }
	}

	/// Returns the first key/value pair in tree. The returned key is the minimum present in
	/// the tree.
	///
	/// If the tree is empty, the function returns `None`.
	pub fn first_key_value(&self) -> Option<(&K, &V)> {
		let node = get_leftmost_node(self.get_root()?);
		Some((&node.key, &node.value))
	}

	/// Removes and returns the first key/value pair in tree. The returned key is the minimum
	/// present in the tree.
	///
	/// If the tree is empty, the function returns `None`.
	pub fn pop_first(&mut self) -> Option<(K, V)> {
		let node = get_leftmost_node(self.get_root()?);
		let (key, value) = self.remove_node(node);
		Some((key, value))
	}

	/// Returns the entry corresponding to the given key.
	pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
		let Some(root) = self.get_root() else {
			return Entry::Vacant(VacantEntry {
				tree: self,
				key,
				parent: None,
			});
		};
		match get_node(root, |k| key.cmp(k.borrow())) {
			Ok(node) => Entry::Occupied(OccupiedEntry {
				node,
			}),
			Err(parent) => Entry::Vacant(VacantEntry {
				tree: self,
				key,
				parent: Some(parent),
			}),
		}
	}

	/// Searches for the given key in the tree and returns a reference.
	///
	/// `key` is the key to find.
	#[inline]
	pub fn get<Q>(&self, key: &Q) -> Option<&V>
	where
		K: Borrow<Q> + Ord,
		Q: Ord + ?Sized,
	{
		let root = self.get_root()?;
		let node = get_node(root, |k| key.cmp(k.borrow())).ok()?;
		Some(&node.value)
	}

	/// Searches for the given key in the tree and returns a mutable reference.
	///
	/// `key` is the key to find.
	#[inline]
	pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
	where
		K: Borrow<Q> + Ord,
		Q: Ord + ?Sized,
	{
		let root = self.get_root()?;
		let node = get_node(root, |k| key.cmp(k.borrow())).ok()?;
		Some(&mut node.value)
	}

	/// Tells whether the collection contains an entry with the given key.
	#[inline]
	pub fn contains_key(&self, k: &K) -> bool {
		self.get(k).is_some()
	}

	/// Searches for a node in the tree using the given comparison function
	/// `cmp` instead of the [`Ord`] trait.
	pub fn cmp_get<F: Fn(&K, &V) -> Ordering>(&self, cmp: F) -> Option<&V> {
		let mut node = self.get_root()?;
		loop {
			let ord = cmp(&node.key, &node.value);
			match ord {
				Ordering::Less => node = node.get_left()?,
				Ordering::Greater => node = node.get_right()?,
				Ordering::Equal => break Some(&node.value),
			}
		}
	}

	/// Searches for a node in the tree using the given comparison function
	/// `cmp` instead of the [`Ord`] trait and returns a mutable reference.
	pub fn cmp_get_mut<F: Fn(&K, &V) -> Ordering>(&mut self, cmp: F) -> Option<&mut V> {
		let mut node = self.get_root()?;
		loop {
			let ord = cmp(&node.key, &node.value);
			match ord {
				Ordering::Less => node = node.get_left()?,
				Ordering::Greater => node = node.get_right()?,
				Ordering::Equal => break Some(&mut node.value),
			}
		}
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

	/// Inserts a key/value pair in the tree.
	///
	/// If the key is already used, the function returns the previous value.
	pub fn insert(&mut self, key: K, val: V) -> AllocResult<Option<V>> {
		match self.entry(key) {
			Entry::Occupied(mut e) => Ok(Some(e.insert(val))),
			Entry::Vacant(e) => {
				e.insert(val)?;
				Ok(None)
			}
		}
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
	/// The function returns the key and value of the removed node.
	fn remove_node(&mut self, node: &mut Node<K, V>) -> (K, V) {
		let left = node.get_left();
		let right = node.get_right();
		let replacement = match (left, right) {
			// The node has two children
			// The leftmost node may have a child on the right
			(Some(_left), Some(right)) => Some(get_leftmost_node(right)),
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
					debug_assert_eq!(self.get_root().unwrap() as *mut Node<K, V>, node as *mut _);
					*self.root.get_mut() = None;
				}
			}
			node.unlink();
			self.len -= 1;
			return unsafe { drop_node(node.into()) };
		};
		if node.get_left().is_some() && node.get_right().is_some() {
			mem::swap(&mut node.key, &mut replacement.key);
			mem::swap(&mut node.value, &mut replacement.value);
			return self.remove_node(replacement);
		}
		let Some(parent) = parent else {
			// The node is the root
			replacement.unlink();
			let (mut key, value) = unsafe { drop_node(replacement.into()) };
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
		let (key, val) = unsafe { drop_node(node.into()) };
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
	pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
	where
		K: Borrow<Q>,
		Q: Ord + ?Sized,
	{
		let root = self.get_root()?;
		let node = get_node(root, |k| key.cmp(k.borrow())).ok()?;
		let (_, value) = self.remove_node(node);
		#[cfg(config_debug_debug)]
		self.check();
		Some(value)
	}

	/*
	/// Calls the given closure for every node in the subtree with root `root`.
	///
	/// `traversal_order` defines the order in which the tree is traversed.
	fn foreach_node<F: FnMut(&Node<K, V>)>(
		root: &Node<K, V>,
		f: &mut F,
		traversal_order: TraversalOrder,
	) {
		let (first, second) = if traversal_order == TraversalOrder::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};
		if traversal_order == TraversalOrder::PreOrder {
			(*f)(root);
		}
		if let Some(mut n) = first {
			Self::foreach_node(unsafe { n.as_mut() }, f, traversal_order);
		}
		if traversal_order == TraversalOrder::InOrder
			|| traversal_order == TraversalOrder::ReverseInOrder
		{
			(*f)(root);
		}
		if let Some(mut n) = second {
			Self::foreach_node(unsafe { n.as_mut() }, f, traversal_order);
		}
		if traversal_order == TraversalOrder::PostOrder {
			(*f)(root);
		}
	}*/

	/// Calls the given closure for every node in the subtree with root `root`.
	///
	/// `traversal_order` defines the order in which the tree is traversed.
	fn foreach_node_mut<F: FnMut(&mut Node<K, V>)>(
		root: &mut Node<K, V>,
		f: &mut F,
		traversal_order: TraversalOrder,
	) {
		let (first, second) = if traversal_order == TraversalOrder::ReverseInOrder {
			(root.right, root.left)
		} else {
			(root.left, root.right)
		};
		if traversal_order == TraversalOrder::PreOrder {
			f(root);
		}
		if let Some(mut n) = first {
			Self::foreach_node_mut(unsafe { n.as_mut() }, f, traversal_order);
		}
		if traversal_order == TraversalOrder::InOrder
			|| traversal_order == TraversalOrder::ReverseInOrder
		{
			f(root);
		}
		if let Some(mut n) = second {
			Self::foreach_node_mut(unsafe { n.as_mut() }, f, traversal_order);
		}
		if traversal_order == TraversalOrder::PostOrder {
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
		Self::foreach_node(
			root,
			&mut |n: &Node<K, V>| {
				assert!(n as *const _ as usize >= crate::memory::PROCESS_END as usize);
				for e in explored_nodes.iter() {
					assert_ne!(*e, n as *const _ as *const c_void);
				}
				explored_nodes.push(n as *const _ as *const c_void).unwrap();
				if let Some(left) = n.get_left() {
					assert!(left as *const _ as usize >= crate::memory::PROCESS_END as usize);
					assert!(ptr::eq(
						left.get_parent().unwrap() as *const _,
						n as *const _
					));
					assert!(left.key <= n.key);
				}
				if let Some(right) = n.get_right() {
					assert!(right as *const _ as usize >= crate::memory::PROCESS_END as usize);
					assert!(ptr::eq(
						right.get_parent().unwrap() as *const _,
						n as *const _
					));
					assert!(right.key >= n.key);
				}
			},
			TraversalOrder::PreOrder,
		);
	}

	/// Returns an immutable iterator for the current binary tree.
	///
	/// Iterator traversal has complexity `O(n)` in time and `O(1)` in space.
	#[inline]
	pub fn iter(&self) -> MapIterator<K, V> {
		let node = self.get_root().map(|n| NonNull::from(get_leftmost_node(n)));
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
		let node = self.get_root().map(|n| NonNull::from(get_leftmost_node(n)));
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
		let node = self
			.get_root()
			.and_then(|root| get_start_node(root, range.start_bound()));
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
		let node = self
			.get_root()
			.and_then(|root| get_start_node(root, range.start_bound()));
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
		let node = self.get_root().map(|n| NonNull::from(get_leftmost_node(n)));
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

	/// Removes all elements.
	pub fn clear(&mut self) {
		let Some(root) = self.get_root() else {
			return;
		};
		Self::foreach_node_mut(
			root,
			&mut |n| unsafe {
				drop_node(n.into());
			},
			TraversalOrder::PostOrder,
		);
		*self.root.get_mut() = None;
		self.len = 0;
	}
}

impl<K: TryClone<Error = E> + Ord, V: TryClone<Error = E>, E: From<AllocError>> TryClone
	for BTreeMap<K, V>
{
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		Ok(self
			.iter()
			.map(|(key, value)| Ok((key.try_clone()?, value.try_clone()?)))
			.collect::<Result<CollectResult<Self>, Self::Error>>()?
			.0?)
	}
}

// TODO make a separate structure which borrows the tree for this implementation?
/*impl<K: Ord + fmt::Debug, V> fmt::Debug for BTreeMap<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let Some(root) = self.get_root() else {
			return write!(f, "<Empty tree>");
		};
		Self::foreach_node(
			root,
			&mut |n| {
				// Tabs
				let _ = write!(f, "{:\t^1$}", "", n.get_node_depth());
				let color = match n.color {
					NodeColor::Red => "red",
					NodeColor::Black => "black",
				};
				let _ = writeln!(f, "{key:?} ({color})", key = n.key);
			},
			TraversalOrder::ReverseInOrder,
		);
		Ok(())
	}
}*/

impl<K: Ord + fmt::Debug, V: fmt::Debug> fmt::Debug for BTreeMap<K, V> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "BTreeMap ")?;
		f.debug_map().entries(self.iter()).finish()?;
		Ok(())
	}
}

impl<K: Ord, V> Drop for BTreeMap<K, V> {
	fn drop(&mut self) {
		self.clear();
	}
}

/// Returns the next node in an iterator for the given node.
///
/// This is an inner function for node iterators.
fn next_node<'a, K, V>(node: &Node<K, V>) -> Option<&'a mut Node<K, V>> {
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

impl<K: Ord, V> FromIterator<(K, V)> for CollectResult<BTreeMap<K, V>> {
	fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
		let iter = iter.into_iter();
		let res = (|| {
			let mut map = BTreeMap::new();
			for (k, v) in iter {
				map.insert(k, v)?;
			}
			Ok(map)
		})();
		Self(res)
	}
}

impl<K: Ord, V> IntoIterator for BTreeMap<K, V> {
	type IntoIter = MapIntoIter<K, V>;
	type Item = (K, V);

	fn into_iter(self) -> Self::IntoIter {
		// Get start node
		let node = self.get_root().map(|n| NonNull::from(get_leftmost_node(n)));
		let remaining = self.len;
		// Avoid dropping twice
		mem::forget(self);
		MapIntoIter {
			node,
			remaining,
		}
	}
}

/// Consuming iterator over a [`BTreeMap`].
pub struct MapIntoIter<K: Ord, V> {
	/// The current node.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of remaining elements.
	remaining: usize,
}

impl<K: Ord, V> Iterator for MapIntoIter<K, V> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		let mut node = self.node?;
		let n = unsafe { node.as_mut() };
		if let Some(parent) = n.get_parent() {
			// The current node is not root.
			// The left side can be ignored since it has been handled in previous iterations
			if let Some(right) = n.get_right() {
				parent.insert_left(right);
				self.node = NonNull::new(get_leftmost_node(right));
			} else {
				self.node = n.parent;
			}
		} else {
			// The current node is root. Go to the lowest element of the right subtree
			self.node = n.get_right().map(|n| NonNull::from(get_leftmost_node(n)));
		}
		n.unlink();
		self.remaining -= 1;
		Some(unsafe { drop_node(node) })
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		(self.remaining, Some(self.remaining))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}

impl<K: Ord, V> ExactSizeIterator for MapIntoIter<K, V> {}

impl<K: Ord, V> FusedIterator for MapIntoIter<K, V> {}

unsafe impl<K: Ord, V> TrustedLen for MapIntoIter<K, V> {}

impl<K: Ord, V> Drop for MapIntoIter<K, V> {
	fn drop(&mut self) {
		// Drop remaining elements
		for _ in self.by_ref() {}
	}
}

/// Immutable reference iterator for [`BTreeMap`]. This iterator traverses the tree in pre-order.
pub struct MapIterator<'m, K: Ord, V> {
	/// The tree to iterate on.
	tree: &'m BTreeMap<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,
}

impl<'m, K: Ord, V> Iterator for MapIterator<'m, K, V> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let node = unwrap_pointer(self.node)?;
		self.node = next_node(node).map(NonNull::from);
		self.i += 1;
		Some((&node.key, &node.value))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.tree.len() - self.i;
		(len, Some(len))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}

impl<'m, K: Ord, V> IntoIterator for &'m BTreeMap<K, V> {
	type IntoIter = MapIterator<'m, K, V>;
	type Item = (&'m K, &'m V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter()
	}
}

impl<'m, K: Ord, V> ExactSizeIterator for MapIterator<'m, K, V> {}

impl<'m, K: Ord, V> FusedIterator for MapIterator<'m, K, V> {}

unsafe impl<'m, K: Ord, V> TrustedLen for MapIterator<'m, K, V> {}

/// Mutable reference iterator for [`BTreeMap`]. This iterator traverses the tree in pre-order.
pub struct MapMutIterator<'m, K: Ord, V> {
	/// The tree to iterate on.
	tree: &'m mut BTreeMap<K, V>,
	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,
}

impl<'m, K: Ord, V> Iterator for MapMutIterator<'m, K, V> {
	type Item = (&'m K, &'m mut V);

	fn next(&mut self) -> Option<Self::Item> {
		let node = unwrap_pointer(self.node)?;
		self.node = next_node(node).map(NonNull::from);
		self.i += 1;
		Some((&node.key, &mut node.value))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.tree.len() - self.i;
		(len, Some(len))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}

impl<'m, K: Ord, V> IntoIterator for &'m mut BTreeMap<K, V> {
	type IntoIter = MapMutIterator<'m, K, V>;
	type Item = (&'m K, &'m mut V);

	fn into_iter(self) -> Self::IntoIter {
		self.iter_mut()
	}
}

impl<'m, K: Ord, V> ExactSizeIterator for MapMutIterator<'m, K, V> {}

impl<'m, K: Ord, V> FusedIterator for MapMutIterator<'m, K, V> {}

unsafe impl<'m, K: Ord, V> TrustedLen for MapMutIterator<'m, K, V> {}

/// Same as [`MapIterator`], but restrained to a predefined range.
pub struct MapRange<'m, K: Ord, V, R: RangeBounds<K>> {
	/// Inner iterator.
	iter: MapIterator<'m, K, V>,
	/// The range to iterate on.
	range: R,
}

impl<'m, K: Ord, V, R: RangeBounds<K>> Iterator for MapRange<'m, K, V, R> {
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

/// Same as [`MapMutIterator`], but restrained to a predefined range.
pub struct MapMutRange<'m, K: Ord, V, R: RangeBounds<K>> {
	/// Inner iterator.
	iter: MapMutIterator<'m, K, V>,
	/// The range to iterate on.
	range: R,
}

impl<'m, K: Ord, V, R: RangeBounds<K>> Iterator for MapMutRange<'m, K, V, R> {
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

/// An iterator that traverses a [`BTreeMap`] in ascending order and removes, then yields elements
/// that match the associated predicate.
pub struct DrainFilter<'m, K: Ord, V, F>
where
	F: FnMut(&K, &mut V) -> bool,
{
	/// The tree to iterate on.
	tree: &'m mut BTreeMap<K, V>,

	/// The current node of the iterator.
	node: Option<NonNull<Node<K, V>>>,
	/// The number of nodes travelled so far.
	i: usize,

	/// The predicate to check whether an element must be drained.
	pred: F,
}

impl<'m, K: Ord, V, F: FnMut(&K, &mut V) -> bool> Iterator for DrainFilter<'m, K, V, F> {
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
		//let next = next_node(node).map(NonNull::new);
		let next = self
			.tree
			.get_root()
			.map(|n| NonNull::from(get_leftmost_node(n)));
		// remove the current node
		let (k, v) = self.tree.remove_node(node);
		// place cursor on next node
		self.node = next;
		self.i += 1;
		Some((k, v))
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::{collections::vec::Vec, math::pseudo_rand};

	#[test]
	fn binary_tree0() {
		let b = BTreeMap::<i32, ()>::new();
		assert!(b.get(&0).is_none());
		assert_eq!(b.len(), 0);
	}

	#[test]
	fn binary_tree_insert0() {
		let mut b = BTreeMap::<i32, i32>::new();
		b.insert(0, 0).unwrap();
		assert_eq!(*b.get(&0).unwrap(), 0);
		assert_eq!(b.len(), 1);
	}

	#[test]
	fn binary_tree_insert1() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in 0..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 1) as usize);
		}
		for i in 0..10 {
			assert_eq!(*b.get(&i).unwrap(), i);
		}
	}

	#[test]
	fn binary_tree_insert2() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}
		for i in -9..10 {
			assert_eq!(*b.get(&i).unwrap(), i);
		}
	}

	#[test]
	fn binary_tree_insert3() {
		let mut b = BTreeMap::<u32, u32>::new();
		let mut val = 0;
		for i in 0..100 {
			val = pseudo_rand(val, 1664525, 1013904223, 0x100);
			b.insert(val, val).unwrap();
			assert_eq!(b.len(), (i + 1) as usize);
		}
		val = 0;
		for _ in 0..100 {
			val = pseudo_rand(val, 1664525, 1013904223, 0x100);
			assert_eq!(*b.get(&val).unwrap(), val);
		}
	}

	#[test]
	fn binary_tree_remove0() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}
		let mut count = b.len();
		for i in -9..10 {
			for i in i..10 {
				assert_eq!(*b.get(&i).unwrap(), i);
			}
			b.remove(&i);
			assert!(b.get(&i).is_none());
			for i in (i + 1)..10 {
				assert_eq!(*b.get(&i).unwrap(), i);
			}
			count -= 1;
			assert_eq!(b.len(), count);
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_remove1() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.len(), (i + 10) as usize);
		}
		let mut count = b.len();
		for i in (-9..10).rev() {
			assert_eq!(*b.get(&i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(&i).is_none());

			count -= 1;
			assert_eq!(b.len(), count);
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_remove2() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}
		let mut count = b.len();
		for i in (-9..10).rev() {
			assert_eq!(*b.get(&i).unwrap(), i);
			b.remove(&i);
			assert!(b.get(&i).is_none());
			count -= 1;
			assert_eq!(b.len(), count);
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_remove3() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in (-9..10).rev() {
			b.insert(i, i).unwrap();
		}
		for i in -9..10 {
			assert_eq!(*b.get(&i).unwrap(), i);
			assert_eq!(b.remove(&i).unwrap(), i);
			assert!(b.get(&i).is_none());
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_remove4() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in -9..10 {
			b.insert(i, i).unwrap();
			assert_eq!(b.remove(&i).unwrap(), i);
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_remove5() {
		let mut b = BTreeMap::<i32, i32>::new();
		for i in -9..10 {
			b.insert(i, i).unwrap();
		}
		for i in -9..10 {
			if i % 2 == 0 {
				assert_eq!(*b.get(&i).unwrap(), i);
				assert_eq!(b.remove(&i).unwrap(), i);
				assert!(b.get(&i).is_none());
			}
		}
		assert!(!b.is_empty());
		for i in -9..10 {
			if i % 2 != 0 {
				assert_eq!(*b.get(&i).unwrap(), i);
				assert_eq!(b.remove(&i).unwrap(), i);
				assert!(b.get(&i).is_none());
			}
		}
		assert!(b.is_empty());
	}

	#[test]
	fn binary_tree_intoiter() {
		let b = (0..1000)
			.map(|i| (i, i))
			.collect::<CollectResult<BTreeMap<_, _>>>()
			.0
			.unwrap();
		assert_eq!(b.len(), 1000);
		let mut count = 0;
		for (a, (b, c)) in b.into_iter().enumerate() {
			assert_eq!(a, b);
			assert_eq!(b, c);
			count += 1;
		}
		assert_eq!(count, 1000);
	}

	#[test]
	fn binary_tree_iter0() {
		let b = BTreeMap::<i32, i32>::new();
		assert_eq!(b.iter().count(), 0);
	}

	#[test]
	fn binary_tree_iter1() {
		let b = (-9..10)
			.map(|i| (i, i))
			.collect::<CollectResult<BTreeMap<_, _>>>()
			.0
			.unwrap();
		assert_eq!(b.iter().count(), b.len());
		assert!(b.iter().is_sorted());
	}

	#[test]
	fn binary_tree_range0() {
		let b = BTreeMap::<i32, i32>::new();
		assert_eq!(b.range(..).count(), 0);
		assert_eq!(b.range(0..).count(), 0);
		assert_eq!(b.range(1..).count(), 0);
		assert_eq!(b.range(1..100).count(), 0);
		assert_eq!(b.range(..100).count(), 0);
	}

	#[test]
	fn binary_tree_range1() {
		let b = (-9..10)
			.map(|i| (i, i))
			.collect::<CollectResult<BTreeMap<_, _>>>()
			.0
			.unwrap();
		assert_eq!(b.range(..).count(), b.len());
		assert!(b.range(..).is_sorted());
		assert_eq!(b.range(0..10).count(), 10);
		assert!(b.range(0..10).is_sorted());
		assert_eq!(b.range(..10).count(), b.len());
		assert!(b.range(..10).is_sorted());
		assert_eq!(b.range(0..).count(), 10);
		assert!(b.range(0..).is_sorted());
	}

	#[test]
	fn binary_tree_range2() {
		let b = [0, 8, 3, 17, 4]
			.into_iter()
			.map(|i| (i, i))
			.collect::<CollectResult<BTreeMap<_, _>>>()
			.0
			.unwrap();
		assert_eq!(b.range(..).count(), b.len());
		assert!(b.range(..).is_sorted());
		let foo = b
			.range(1..=4)
			.map(|(i, _)| *i)
			.collect::<CollectResult<Vec<_>>>()
			.0
			.unwrap();
		assert_eq!(foo.as_slice(), [3, 4]);
		let foo = b
			.range(-3..=4)
			.map(|(i, _)| *i)
			.collect::<CollectResult<Vec<_>>>()
			.0
			.unwrap();
		assert_eq!(foo.as_slice(), [0, 3, 4]);
		let foo = b
			.range(-3..)
			.map(|(i, _)| *i)
			.collect::<CollectResult<Vec<_>>>()
			.0
			.unwrap();
		assert_eq!(foo.as_slice(), [0, 3, 4, 8, 17]);
	}

	#[test]
	fn binary_tree_drain0() {
		let mut b = (-9..10)
			.map(|i| (i, i))
			.collect::<CollectResult<BTreeMap<_, _>>>()
			.0
			.unwrap();
		let len = b.len();
		assert!(b
			.drain_filter(|k, v| k == v && k % 2 == 0)
			.all(|(k, v)| k == v && k % 2 == 0));
		assert_eq!(b.len(), len / 2 + 1);
		assert!(b.into_iter().all(|(k, v)| k == v && k % 2 != 0));
	}
}
