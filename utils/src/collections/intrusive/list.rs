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

use crate::ptr::arc::Arc;
use core::{cell::UnsafeCell, marker::PhantomData, ptr::NonNull};

/// A non-concurrent, intrusive, doubly-linked list node.
///
/// Most operations on this structure are unsafe as they cannot fit Rust's borrowing rules.
#[derive(Default)]
pub struct ListNode {
	prev: UnsafeCell<Option<NonNull<ListNode>>>,
	next: UnsafeCell<Option<NonNull<ListNode>>>,
}

impl ListNode {
	/// Returns a reference to the previous element.
	#[inline]
	pub fn previous(&self) -> Option<&Self> {
		unsafe { (*self.prev.get()).map(|n| n.as_ref()) }
	}

	/// Returns a reference to the next element.
	#[inline]
	pub fn next(&self) -> Option<&Self> {
		unsafe { (*self.next.get()).map(|n| n.as_ref()) }
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

/// The base of a non-concurrent, intrusive, doubly linked list.
///
/// The elements inside the list have to reside in an [`Arc`]. This prevents ownership issues and
/// preserves soundness by disallowing mutability on the inner list node.
///
/// This structure uses mutability in order to enforce locking in concurrent contexts.
///
/// Inside, the linked list forms a cycle to allow iterating both ways.
///
/// An instance can be created with [`crate::list`].
#[derive(Default)]
pub struct List<T, const OFF: usize> {
	// We are using `prev` as the tail and `next` as the head
	head: ListNode,
	_phantom: PhantomData<T>,
}

/// Initialize a new list.
///
/// This macro can be used in a `const` context.
#[macro_export]
macro_rules! list {
	($ty:ty, $field:ident) => {{
		const OFF: usize = core::mem::offset_of!($ty, $field);
		List::<$ty, OFF>::_new()
	}};
}

impl<T, const OFF: usize> List<T, OFF> {
	/// Use [`crate::list`] instead!
	pub const fn _new() -> Self {
		Self {
			head: ListNode {
				prev: UnsafeCell::new(None),
				next: UnsafeCell::new(None),
			},
			_phantom: PhantomData,
		}
	}

	/// Inserts `val` at the first position of the list.
	pub fn insert_front(&mut self, val: Arc<T>) {
		let base_node = NonNull::from(&mut self.head);
		unsafe {
			let inner = NonNull::from(&*Arc::into_raw(val));
			let mut node = inner.byte_add(OFF).cast::<ListNode>();
			let node_ref = node.as_mut();
			// Insert node in the list
			if self.head.next().is_some() {
				// There is already an element in the list
				node_ref.insert_after(base_node);
			} else {
				// There is element in the list: insert the first element and make a cycle
				*node_ref.prev.get_mut() = Some(base_node);
				*node_ref.next.get_mut() = Some(base_node);
				*self.head.prev.get_mut() = Some(node);
				*self.head.next.get_mut() = Some(node);
			}
		}
	}

	/// Removes the first element of the list and returns it, if any.
	pub fn remove_front(&mut self) -> Option<Arc<T>> {
		// If the list is empty, return `None`
		let head = (*self.head.next.get_mut())?;
		unsafe {
			// Unlink
			head.as_ref().unlink();
			// Return the element
			let elem = head.byte_sub(OFF).cast().as_ptr();
			Some(Arc::from_raw(elem))
		}
	}
}
