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

//! A non-concurrent, intrusive, doubly-linked list implementation.
//!
//! Intrusive linked-lists are useful in context where memory allocations should be avoided.
//! Otherwise, prefer using other collections.

use crate::ptr::arc::Arc;
use core::{
	cell::UnsafeCell, fmt, fmt::Formatter, hint::unlikely, marker::PhantomData, mem, ptr,
	ptr::NonNull,
};

/// A non-concurrent, intrusive, doubly-linked list node.
///
/// Most operations on this structure are unsafe as they cannot fit Rust's borrowing rules.
#[derive(Default)]
pub struct ListNode {
	prev: UnsafeCell<Option<NonNull<ListNode>>>,
	next: UnsafeCell<Option<NonNull<ListNode>>>,
}

impl ListNode {
	/// Returns the container of `self`.
	///
	/// `inner_off` is the offset of `self` inside of `T`.
	///
	/// # Safety
	///
	/// If `self` is not a field inside of `T`, or if `inner_off` is invalid, the behaviour is
	/// undefined.
	#[inline]
	pub unsafe fn container<T>(&self, inner_off: usize) -> &T {
		&*(self as *const Self).byte_sub(inner_off).cast::<T>()
	}

	/// Returns a reference to the previous element.
	#[inline]
	pub fn prev(&self) -> Option<&Self> {
		unsafe { (*self.prev.get()).map(|n| n.as_ref()) }
	}

	/// Returns a reference to the next element.
	#[inline]
	pub fn next(&self) -> Option<&Self> {
		unsafe { (*self.next.get()).map(|n| n.as_ref()) }
	}

	/// Inserts `self` before `node` in the list.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure concurrency and consistency are handled
	/// correctly.
	pub unsafe fn insert_before(&self, mut node: NonNull<ListNode>) {
		// Insert in the new list
		*self.next.get() = Some(node);
		*self.prev.get() = *node.as_ref().prev.get();
		// Link back
		*node.as_mut().prev.get() = Some(NonNull::from(self));
		if let Some(prev) = self.prev() {
			*prev.next.get() = Some(NonNull::from(self));
		}
	}

	/// Inserts `self` after `node` in the list.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure concurrency and consistency are handled
	/// correctly.
	pub unsafe fn insert_after(&self, mut node: NonNull<ListNode>) {
		// Insert in the new list
		*self.prev.get() = Some(node);
		*self.next.get() = *node.as_ref().next.get();
		// Link back
		*node.as_mut().next.get() = Some(NonNull::from(self));
		if let Some(next) = self.next() {
			*next.prev.get() = Some(NonNull::from(self));
		}
	}

	/// Unlinks `self` from its list. If not in a list, the function does nothing
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure concurrency and consistency are handled
	/// correctly.
	pub unsafe fn unlink(&self) {
		let prev = (*self.prev.get()).take();
		let next = (*self.next.get()).take();
		if let Some(mut prev) = prev {
			*prev.as_mut().next.get() = next;
		}
		if let Some(mut next) = next {
			*next.as_mut().prev.get() = prev;
		}
	}
}

impl fmt::Debug for ListNode {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("ListNode").finish()
	}
}

/// The base of a non-concurrent, intrusive, doubly linked list.
///
/// The elements inside the list have to reside in an [`Arc`]. This prevents ownership issues and
/// preserves soundness by disallowing mutability on the inner list node.
///
/// This structure uses mutability in order to enforce locking in concurrent contexts.
///
/// Inside, the linked list forms a cycle to allow iterating both ways.
///
/// A list of `Foo` structures can be created this way:
/// ```no_run
/// use utils::{collections::list::ListNode, list};
///
/// struct Foo {
/// 	foo: i32,
/// 	node: ListNode,
/// }
///
/// let _list = list!(Foo, node);
/// ```
///
/// Pinning the list is required to avoid dangling pointers.
///
/// When dropped, if the list is not empty, the remaining nodes are all unlinked.
pub struct List<T, const OFF: usize> {
	// This is the head element. `prev` points to the tail
	head: Option<NonNull<ListNode>>,
	_data: PhantomData<T>,
}

/// Initialize a new list.
///
/// This macro can be used in a `const` context.
#[macro_export]
macro_rules! list {
	($ty:ty, $field:ident) => {
		<$crate::list_type!($ty, $field)>::_new()
	};
}

/// The type signature for a list.
///
/// This macro is necessary to avoid having to specify the `OFF` generic manually.
#[macro_export]
macro_rules! list_type {
	($ty:ty, $field:ident) => {
		$crate::collections::list::List::<$ty, { core::mem::offset_of!($ty, $field) }>
	};
}

impl<T, const OFF: usize> List<T, OFF> {
	/// Use [`crate::list`] instead!
	pub const fn _new() -> Self {
		Self {
			head: None,
			_data: PhantomData,
		}
	}

	fn get_node(val: &T) -> NonNull<ListNode> {
		unsafe { NonNull::from(val).byte_add(OFF).cast::<ListNode>() }
	}

	#[inline]
	fn head_node(&self) -> Option<&ListNode> {
		unsafe { self.head.map(|n| n.as_ref()) }
	}

	#[inline]
	fn tail_node(&self) -> Option<&ListNode> {
		self.head_node()?.prev()
	}

	/// Returns a reference to the first element of the list.
	#[inline]
	pub fn front(&self) -> Option<Arc<T>> {
		let cursor = Cursor {
			list: NonNull::from(self),
			node: self.head_node()?,
		};
		Some(cursor.arc())
	}

	/// Returns a reference to the last element of the list.
	#[inline]
	pub fn back(&self) -> Option<Arc<T>> {
		let cursor = Cursor {
			list: NonNull::from(self),
			node: self.tail_node()?,
		};
		Some(cursor.arc())
	}

	/// Returns an iterator over the list.
	pub fn iter(&mut self) -> Iter<'_, T, OFF> {
		Iter {
			list: NonNull::from(&mut *self),
			range: self.head_node().map(|head| (head, head.prev().unwrap())),
			fuse: false,
		}
	}

	/// Inserts `val` at the first position of the list.
	pub fn insert_front(&mut self, val: Arc<T>) {
		let node = Self::get_node(&val);
		// Keep reference
		mem::forget(val);
		if let Some(head) = self.head {
			// There is already an element in the list
			unsafe {
				node.as_ref().insert_before(head);
			}
		} else {
			// The list is empty: make a cycle
			unsafe {
				*node.as_ref().prev.get() = Some(node);
				*node.as_ref().next.get() = Some(node);
			}
		}
		// Update head
		self.head = Some(node);
	}

	/// Inserts `val` at the last position of the list.
	pub fn insert_back(&mut self, val: Arc<T>) {
		let node = Self::get_node(&val);
		// Keep reference
		mem::forget(val);
		if let Some(head) = self.head {
			// There is already an element in the list
			unsafe {
				node.as_ref().insert_before(head);
			}
		} else {
			// The list is empty: make a cycle
			unsafe {
				*node.as_ref().prev.get() = Some(node);
				*node.as_ref().next.get() = Some(node);
			}
			// Set as head
			self.head = Some(node);
		}
	}

	/// Rotates the circular list, making the second element the new head, and the old head the new
	/// tail.
	pub fn rotate_left(&mut self) {
		self.head = self.head.and_then(|h| unsafe { *h.as_ref().next.get() });
	}

	/// Rotates the circular list, making the tail the new head, and the old head the second
	/// element.
	pub fn rotate_right(&mut self) {
		self.head = self.head.and_then(|h| unsafe { *h.as_ref().prev.get() });
	}

	/// Removes the first element of the list and returns it, if any.
	pub fn remove_front(&mut self) -> Option<Arc<T>> {
		let cursor = Cursor {
			list: NonNull::from(&mut *self),
			node: self.head_node()?,
		};
		Some(cursor.remove())
	}

	/// Removes the last element of the list and returns it, if any.
	pub fn remove_back(&mut self) -> Option<Arc<T>> {
		let cursor = Cursor {
			list: NonNull::from(&mut *self),
			node: self.tail_node()?,
		};
		Some(cursor.remove())
	}

	/// Removes a value from the list.
	///
	/// # Safety
	///
	/// The function is marked as unsafe because it cannot ensure `val` actually is inserted in
	/// `self`. This is the caller's responsibility.
	pub unsafe fn remove(&mut self, val: &Arc<T>) {
		let cursor = Cursor {
			list: NonNull::from(&mut *self),
			node: Self::get_node(val).as_ref(),
		};
		cursor.remove();
	}

	/// Moves the node to the beginning of the list.
	///
	/// # Safety
	///
	/// The function is marked as unsafe because it cannot ensure `val` actually is inserted in
	/// `self`. This is the caller's responsibility.
	pub unsafe fn lru_promote(&mut self, val: &Arc<T>) {
		let mut cursor = Cursor {
			list: NonNull::from(&mut *self),
			node: Self::get_node(val).as_ref(),
		};
		cursor.lru_promote();
	}

	/// Unlinks all the elements from the list.
	pub fn clear(&mut self) {
		for node in self.iter() {
			node.remove();
		}
	}
}

impl<T, const OFF: usize> Drop for List<T, OFF> {
	fn drop(&mut self) {
		self.clear();
	}
}

/// Cursor over an element in a [`List`].
pub struct Cursor<'l, T: 'l, const OFF: usize> {
	list: NonNull<List<T, OFF>>,
	node: &'l ListNode,
}

impl<'l, T: 'l, const OFF: usize> Cursor<'l, T, OFF> {
	/// Returns the cursor's node.
	#[inline]
	pub fn node(&self) -> &ListNode {
		self.node
	}

	/// Returns a reference to the node's value.
	#[inline]
	pub fn value(&self) -> &T {
		unsafe { self.node.container(OFF) }
	}

	/// Returns an [`Arc`] with the value in it.
	#[inline]
	pub fn arc(&self) -> Arc<T> {
		let arc = unsafe { Arc::from_raw(self.value()) };
		// Increment reference count
		mem::forget(arc.clone());
		arc
	}

	/// Removes the element from the list, returning the value as an [`Arc`].
	pub fn remove(mut self) -> Arc<T> {
		unsafe {
			let list = self.list.as_mut();
			// Cannot fail since `self` is in the list
			let head = list.head_node().unwrap();
			// If the node to remove is the head, change it
			if ptr::eq(self.node, head) {
				list.head = head
					.next()
					// If nothing else than the head remain, the list shall become empty
					.filter(|next| !ptr::eq(*next, head))
					.map(NonNull::from);
			}
			self.node.unlink();
			Arc::from_raw(self.value())
		}
	}

	/// Moves the node to the beginning of the list.
	///
	/// This is useful when the list is used as an LRU.
	pub fn lru_promote(&mut self) {
		unsafe {
			let list = self.list.as_mut();
			// Cannot fail since `self` is in the list
			let head = list.head.unwrap();
			// If the node is already the head, do nothing
			if ptr::eq(self.node, head.as_ptr()) {
				return;
			}
			// Move the node in the list
			self.node.unlink();
			self.node.insert_before(head);
			// Update head
			list.head.replace(NonNull::from(self.node));
		}
	}
}

/// Double-ended iterator over a [`List`], returning a [`Cursor`] for each element.
pub struct Iter<'l, T: 'l, const OFF: usize> {
	list: NonNull<List<T, OFF>>,
	range: Option<(&'l ListNode, &'l ListNode)>,
	fuse: bool,
}

impl<'l, T: 'l, const OFF: usize> Iterator for Iter<'l, T, OFF> {
	type Item = Cursor<'l, T, OFF>;

	fn next(&mut self) -> Option<Self::Item> {
		let (start, end) = self.range.as_mut()?;
		if unlikely(self.fuse) {
			return None;
		}
		if unlikely(ptr::eq(*start, *end)) {
			self.fuse = true;
		}
		let node = *start;
		// Cannot fail since the list is a cycle
		*start = start.next().unwrap();
		Some(Cursor {
			list: self.list,
			node,
		})
	}
}

impl<'l, T: 'l, const OFF: usize> DoubleEndedIterator for Iter<'l, T, OFF> {
	fn next_back(&mut self) -> Option<Self::Item> {
		let (start, end) = self.range.as_mut()?;
		if unlikely(self.fuse) {
			return None;
		}
		if unlikely(ptr::eq(*start, *end)) {
			self.fuse = true;
		}
		let node = *end;
		// Cannot fail since the list is a cycle
		*end = end.prev().unwrap();
		Some(Cursor {
			list: self.list,
			node,
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use core::pin::pin;

	struct Foo {
		foo: usize,
		node: ListNode,
	}

	fn init(list: &mut list_type!(Foo, node)) {
		list.insert_front(
			Arc::new(Foo {
				foo: 0,
				node: ListNode::default(),
			})
			.unwrap(),
		);
		list.insert_front(
			Arc::new(Foo {
				foo: 1,
				node: ListNode::default(),
			})
			.unwrap(),
		);
		list.insert_front(
			Arc::new(Foo {
				foo: 2,
				node: ListNode::default(),
			})
			.unwrap(),
		);
	}

	#[test]
	fn list_basic() {
		let mut list = pin!(list!(Foo, node));
		init(&mut list);

		let mut iter = list.iter();
		let mut cnt = 0;
		for (i, j) in iter.by_ref().rev().enumerate() {
			assert_eq!(i, j.value().foo);
			cnt += 1;
		}
		assert!(iter.next().is_none());
		assert_eq!(cnt, 3);
	}

	#[test]
	fn list_remove() {
		let mut list = pin!(list!(Foo, node));
		init(&mut list);

		let removed = list
			.as_mut()
			.iter()
			.find(|c| c.value().foo == 1)
			.map(|c| c.remove());

		let mut iter = list.iter().rev();
		assert_eq!(iter.next().map(|n| n.value().foo), Some(0));
		assert_eq!(iter.next().map(|n| n.value().foo), Some(2));
		assert!(iter.next().is_none());

		assert_eq!(removed.map(|n| n.foo), Some(1));
	}

	#[test]
	fn list_lru_promote() {
		let mut list = pin!(list!(Foo, node));
		init(&mut list);

		let mut promoted = list.iter().find(|c| c.value().foo == 1).unwrap();
		promoted.lru_promote();

		let mut iter = list.iter().rev();
		assert_eq!(iter.next().map(|n| n.value().foo), Some(0));
		assert_eq!(iter.next().map(|n| n.value().foo), Some(2));
		assert_eq!(iter.next().map(|n| n.value().foo), Some(1));
		assert!(iter.next().is_none());
	}
}
