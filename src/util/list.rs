//! This module implements the LinkedList utility.
//!
//! What's called a "floating linked list" is a linked list which doesn't have a beginning, it may
//! be accessed only in its middle, through its elements.

use core::marker::PhantomData;
use core::ptr::NonNull;

/// A list of elements working with a double linked list. It's important to note that the elements
/// stored in this container are NOT owned by it, meaning that when the container is destroyed,
/// the list still exists.
/// This structure is not totally safe. If the first object is removed while considered into a
/// floating linked list, then the associated List won't be aware and the overall might result in
/// a dangling pointer. It especially has to be taken into account when auto-dropping a node.
pub struct List<T> {
	/// The front of the list.
	front: Option<NonNull<ListNode>>,
	/// The offset of the node in the element stored by the list.
	inner_offset: usize,

	/// Phantom data to be able to keep the type `T`
	_phantom: PhantomData<T>,
}

impl<T> List<T> {
	/// Creates a new List with the given inner offset. This function should not be called directly
	/// but only through the dedicated macro `list_new`.
	pub const fn new(inner_offset: usize) -> Self {
		List::<T> {
			front: None,
			inner_offset,
			_phantom: core::marker::PhantomData::<T>,
		}
	}

	/// Tells whether the list is empty.
	pub fn is_empty(&self) -> bool {
		self.front.is_none()
	}

	/// Returns the number of elements in the list.
	pub fn size(&self) -> usize {
		if let Some(front) = self.front {
			unsafe {
				front.as_ref()
			}.right_size()
		} else {
			0
		}
	}

	/// Returns the offset of the node in the element stored bt the list.
	pub fn get_inner_offset(&self) -> usize {
		self.inner_offset
	}

	/// Returns a mutable reference to the front node if the list is not empty.
	pub fn get_front(&mut self) -> Option::<&'static mut ListNode> {
		if let Some(front) = self.front {
			Some(unsafe {
				&mut *front.as_ptr()
			})
		} else {
			None
		}
	}

	/// Inserts the given element at the front of list.
	pub fn insert_front(&mut self, node: &mut ListNode) {
		node.prev = None;
		node.next = self.front;
		self.front = NonNull::new(node as _);
		unsafe {
			node.link_back();
		}
	}

	/// Unlinks the first element at the front of the list.
	pub fn unlink_front(&mut self) {
		if let Some(mut front) = self.front {
			let f = unsafe {
				front.as_mut()
			};

			unsafe {
				f.unlink_floating();
			}
			self.front = f.next;
		}
	}

	/// Executes the given closure `f` for each nodes in the list.
	pub fn foreach<F>(&self, f: F) where F: Fn(&ListNode) {
		if let Some(front) = self.front {
			unsafe {
				front.as_ref()
			}.foreach(f);
		}
	}

	/// Same as `foreach` except the nodes are mutable.
	pub fn foreach_mut<F>(&mut self, f: F) where F: Fn(&mut ListNode) {
		if let Some(mut front) = self.front {
			unsafe {
				front.as_mut()
			}.foreach_mut(f);
		}
	}
}

impl<T> Clone for List<T> {
	fn clone(&self) -> Self {
		Self {
			front: self.front,
			inner_offset: self.inner_offset,
			_phantom: core::marker::PhantomData::<T>,
		}
	}
}

impl<T> Copy for List<T> {}

/// Creates a new List object for the given type and field.
/// If the parameter `field` is not the name of a field of type ListNode, the behaviour is
/// undefined.
#[macro_export]
macro_rules! list_new {
	($type:ty, $field:ident) => {
		crate::util::list::List::<$type>::new(crate::offset_of!($type, $field))
	}
}

// TODO Make immovable
/// A node of a List. This structure is meant to be used inside of the structure to be stored in
/// the list.
#[derive(Debug)]
pub struct ListNode {
	/// Pointer to the previous element in the list
	prev: Option<NonNull<ListNode>>,
	/// Pointer to the next element in the list
	next: Option<NonNull<ListNode>>,
}

/// Converts the given option to a const pointer option for iterations.
fn option_to_const(from: Option<NonNull<ListNode>>) -> Option<*const ListNode> {
	if let Some(inner) = from {
		Some(inner.as_ptr() as _)
	} else {
		None
	}
}

/// Converts the given option to a mut pointer option for iterations.
fn option_to_mut(from: Option<NonNull<ListNode>>) -> Option<*mut ListNode> {
	if let Some(inner) = from {
		Some(inner.as_ptr() as _)
	} else {
		None
	}
}

impl ListNode {
	/// Creates a single node.
	pub fn new_single() -> Self {
		Self {
			prev: None,
			next: None,
		}
	}

	/// Returns a reference to the structure storing the node.
	/// `offset` is the offset of the field of the node in the structure.
	pub fn get<T>(&self, offset: usize) -> &'static T {
		unsafe {
			&*(((self as *const _ as usize) - offset) as *const T)
		}
	}

	/// Returns a mutable reference to the structure storing the node.
	/// `offset` is the offset of the field of the node in the structure.
	pub fn get_mut<T>(&mut self, offset: usize) -> &'static mut T {
		unsafe {
			&mut *(((self as *mut _ as usize) - offset) as *mut T)
		}
	}

	/// Tells whether the node is single in the list.
	pub fn is_single(&self) -> bool {
		self.prev.is_none() && self.next.is_none()
	}

	/// Returns the previous element if it exsits, or None.
	pub fn get_prev(&self) -> Option<&'static mut ListNode> {
		if let Some(ptr) = self.prev {
			Some(unsafe {
				&mut *ptr.as_ptr()
			})
		} else {
			None
		}
	}

	/// Returns the next element if it exsits, or None.
	pub fn get_next(&self) -> Option<&'static mut ListNode> {
		if let Some(ptr) = self.next {
			Some(unsafe {
				&mut *ptr.as_ptr()
			})
		} else {
			None
		}
	}

	/// Returns the size of the linked list, counting previous elements.
	pub fn left_size(&self) -> usize {
		let mut i = 0;
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			i += 1;
			curr = option_to_const(unsafe {
				(*curr.unwrap()).prev
			});
		}
		i
	}

	/// Returns the size of the linked list, counting next elements.
	pub fn right_size(&self) -> usize {
		let mut i = 0;
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			i += 1;
			curr = option_to_const(unsafe {
				(*curr.unwrap()).next
			});
		}
		i
	}

	/// Executes the given closure `f` for each nodes after the current one, included. The nodes
	/// are not mutable.
	pub fn foreach<F>(&self, f: F) where F: Fn(&ListNode) {
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			let c = curr.unwrap();
			unsafe {
				f(&*c);
				curr = option_to_const((*c).next);
			}
		}
	}

	/// Same as `foreach` except the nodes are mutable.
	pub fn foreach_mut<F>(&mut self, f: F) where F: Fn(&mut ListNode) {
		let mut curr: Option<*mut Self> = Some(self);

		while curr.is_some() {
			let c = curr.unwrap();
			unsafe {
				f(&mut *c);
				curr = option_to_mut((*c).next);
			}
		}
	}

	/// Links back adjacent nodes to the current node.
	unsafe fn link_back(&mut self) {
		let curr_node = NonNull::new(self);

		if self.next.is_some() {
			self.next.unwrap().as_mut().prev = curr_node;
		}
		if self.prev.is_some() {
			self.prev.unwrap().as_mut().next = curr_node;
		}
	}

	/// Inserts the node before node `node` in the given linked list `front`.
	/// If the current node is not single, the behaviour is undefined.
	pub fn insert_before(&mut self, front: &mut Option<*mut ListNode>, node: &mut ListNode) {
		if front.is_some() && front.unwrap() == node {
			*front = Some(self);
		}

		self.insert_before_floating(node);
	}

	/// Inserts the node before node `node` in a floating linked list.
	/// If the current node is not single, the behaviour is undefined.
	pub fn insert_before_floating(&mut self, node: &mut ListNode) {
		debug_assert!(self.is_single());

		unsafe {
			self.prev = (*node).prev;
			self.next = NonNull::new(node);
			self.link_back();
		}
	}

	/// Inserts the node after node `node` in the given linked list `front`.
	/// If the current node is not single, the behaviour is undefined.
	pub fn insert_after(&mut self, node: &mut ListNode) {
		debug_assert!(self.is_single());

		unsafe {
			self.prev = NonNull::new(node);
			self.next = (*node).next;
			self.link_back();
		}
	}

	/// Unlinks the current node from the floating linked list.
	/// The function is unsafe because if it is called to unlink a node that is owned by a List as
	/// if it was in a floating-list, the operation might create a dangling pointer on that List.
	pub unsafe fn unlink_floating(&mut self) {
		if self.prev.is_some() {
			self.prev.unwrap().as_mut().next = self.next;
		}
		if self.next.is_some() {
			self.next.unwrap().as_mut().prev = self.prev;
		}
		self.prev = None;
		self.next = None;
	}

	/// Unlinks the current node from the given list.
	pub fn unlink_from<T>(&mut self, list: &mut List::<T>) {
		if let Some(front) = list.front {
			if front.as_ptr() == self {
				list.unlink_front();
			}
		}

		unsafe {
			self.unlink_floating();
		}
	}
}

impl Drop for ListNode {
	fn drop(&mut self) {
		unsafe {
			self.unlink_floating();
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn linked_list_insert_before0() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();
		let mut front: Option<*mut ListNode> = None;

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_none());
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_before1() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();
		let mut front: Option<*mut ListNode> = Some(&mut l1 as _);

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_some() && front.unwrap() == &mut l0 as _);
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_before2() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();
		let mut front: Option<*mut ListNode> = Some(&mut l0 as _);

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_some() && front.unwrap() == &mut l0 as _);
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_before_floating0() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();

		l0.insert_before_floating(&mut l1);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_before_floating1() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();
		let mut l2 = ListNode::new_single();

		l0.insert_before_floating(&mut l2);
		l1.insert_before_floating(&mut l2);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_some());
		assert!(l1.next.unwrap() == NonNull::new(&mut l2 as _).unwrap());

		assert!(l2.prev.is_some());
		assert!(l2.prev.unwrap() == NonNull::new(&mut l1 as _).unwrap());
		assert!(l2.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_after0() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();

		l1.insert_after(&mut l0);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_none());
	}

	#[test_case]
	fn linked_list_insert_after1() {
		let mut l0 = ListNode::new_single();
		let mut l1 = ListNode::new_single();
		let mut l2 = ListNode::new_single();

		l2.insert_after(&mut l0);
		l1.insert_after(&mut l0);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == NonNull::new(&mut l1 as _).unwrap());

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == NonNull::new(&mut l0 as _).unwrap());
		assert!(l1.next.is_some());
		assert!(l1.next.unwrap() == NonNull::new(&mut l2 as _).unwrap());

		assert!(l2.prev.is_some());
		assert!(l2.prev.unwrap() == NonNull::new(&mut l1 as _).unwrap());
		assert!(l2.next.is_none());
	}

	// TODO More tests
}
