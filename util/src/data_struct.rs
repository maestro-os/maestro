/// This files implements data structures.
/// Data structures present in this files are guaranteed to not require any memory allocations.

/// Converts an `Option<*mut Self>` into a `Option<*const Self>`.
#[inline(always)]
fn option_mut_to_const<T>(option: Option<*mut T>) -> Option<*const T> {
	if let Some(ptr) = option {
		Some(ptr as *const T)
	} else {
		None
	}
}

/// Structure representing a node in a doubly-linked list.
/// 
/// TODO Explain difference between floating and non-floating lists
#[derive(Debug)]
pub struct LinkedList {
	/// Pointer to the previous element in the list 
	prev: Option<*mut LinkedList>,
	/// Pointer to the next element in the list 
	next: Option<*mut LinkedList>,
}

/// Returns a reference to the element of type `type` for the given linked list node `node` stored
/// in field `field`.
#[macro_export]
macro_rules! linked_list_get {
	($node:expr, $type:ty, $field:ident) => {
		crate::container_of!($node, $type, $field)
	}
}

impl LinkedList {
	/// Creates a single node.
	pub fn new_single() -> Self {
		Self {
			prev: None,
			next: None,
		}
	}

	/// Tells whether the node is single in the list.
	pub fn is_single(&self) -> bool {
		self.prev.is_none() && self.next.is_none()
	}

	/// Returns the previous element if it exsits, or None.
	pub fn get_prev(&self) -> Option<&mut LinkedList> {
		if self.prev.is_some() {
			Some(unsafe { &mut *self.prev.unwrap() })
		} else {
			None
		}
	}

	/// Returns the next element if it exsits, or None.
	pub fn get_next(&self) -> Option<&mut LinkedList> {
		if self.next.is_some() {
			Some(unsafe { &mut *self.next.unwrap() })
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
			curr = option_mut_to_const(unsafe { (*curr.unwrap()).prev });
		}
		i
	}

	/// Returns the size of the linked list, counting next elements.
	pub fn right_size(&self) -> usize {
		let mut i = 0;
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			i += 1;
			curr = option_mut_to_const(unsafe { (*curr.unwrap()).next });
		}
		i
	}

	/// Executes the given closure `f` for each nodes after the given node `node`, including the
	/// given one. The nodes are not mutable.
	pub fn foreach<T>(&self, f: T) where T: Fn(&LinkedList) {
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			let c = curr.unwrap();
			unsafe {
				f(&*c);
				curr = option_mut_to_const((*c).next);
			}
		}
	}

	/// Same as `foreach` except the nodes are mutable.
	pub fn foreach_mut<T>(&mut self, f: T) where T: Fn(&mut LinkedList) {
		let mut curr: Option<*mut Self> = Some(self);

		while curr.is_some() {
			let c = curr.unwrap();
			unsafe {
				f(&mut *c);
				curr = (*c).next;
			}
		}
	}

	/// Links back adjacent nodes to the current node.
	unsafe fn link_back(&mut self) {
		if self.next.is_some() {
			(*self.next.unwrap()).prev = Some(self);
		}
		if self.prev.is_some() {
			(*self.prev.unwrap()).next = Some(self);
		}
	}

	/// Inserts the node at the beginning of the given linked list `front`.
	pub fn insert_front(&mut self, front: &mut Option<*mut LinkedList>) {
		self.prev = None;
		self.next = *front;
		*front = Some(self);
		unsafe {
			self.link_back();
		}
	}

	/// Inserts the node before node `node` in the given linked list `front`.
	/// If the node is not single, the behaviour is undefined.
	pub fn insert_before(&mut self, front: &mut Option<*mut LinkedList>, node: &mut LinkedList) {
		if front.is_some() && front.unwrap() == node {
			*front = Some(self);
		}

		self.insert_before_floating(node);
	}

	/// Inserts the node before node `node` in a floating linked list.
	/// If the node is not single, the behaviour is undefined.
	pub fn insert_before_floating(&mut self, node: &mut LinkedList) {
		debug_assert!(self.is_single());

		unsafe {
			self.prev = (*node).prev;
			self.next = Some(node);
			self.link_back();
		}
	}

	/// Inserts the node after node `node` in the given linked list `front`.
	/// If the node is not single, the behaviour is undefined.
	pub fn insert_after(&mut self, node: &mut LinkedList) {
		debug_assert!(self.is_single());

		unsafe {
			self.prev = Some(node);
			self.next = (*node).next;
			self.link_back();
		}
	}

	/// Unlinks the current node from the linked list with front `front`.
	pub fn unlink(&mut self, front: &mut Option<*mut LinkedList>) {
		if front.is_some() && front.unwrap() == self {
			*front = self.next;
		}

		self.unlink_floating();
	}

	/// Unlinks the current node from the floating linked list.
	pub fn unlink_floating(&mut self) {
		if self.prev.is_some() {
			unsafe {
				(*self.prev.unwrap()).next = self.next;
			}
		}
		if self.next.is_some() {
			unsafe {
				(*self.next.unwrap()).prev = self.prev;
			}
		}
		self.prev = None;
		self.next = None;
	}
}

/*impl Drop for LinkedList {
	fn drop(&mut self) {
		self.unlink_floating();
	}
}*/

// TODO Binary tree

#[cfg(test)]
mod test {
	use super::*;

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_before0() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();
		let mut front: Option<*mut LinkedList> = None;

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_none());
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_before1() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();
		let mut front: Option<*mut LinkedList> = Some(&mut l1 as _);

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_some() && front.unwrap() == &mut l0 as _);
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_before2() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();
		let mut front: Option<*mut LinkedList> = Some(&mut l0 as _);

		l0.insert_before(&mut front, &mut l1);

		assert!(front.is_some() && front.unwrap() == &mut l0 as _);
		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_before_floating0() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();

		l0.insert_before_floating(&mut l1);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_before_floating1() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();
		let mut l2 = LinkedList::new_single();

		l0.insert_before_floating(&mut l2);
		l1.insert_before_floating(&mut l2);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_some());
		assert!(l1.next.unwrap() == &mut l2 as _);

		assert!(l2.prev.is_some());
		assert!(l2.prev.unwrap() == &mut l1 as _);
		assert!(l2.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_after0() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();

		l1.insert_after(&mut l0);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_none());
	}

	#[cfg_attr(userspace, test)]
	#[cfg_attr(not(userspace), test_case)]
	fn linked_list_insert_after1() {
		let mut l0 = LinkedList::new_single();
		let mut l1 = LinkedList::new_single();
		let mut l2 = LinkedList::new_single();

		l2.insert_after(&mut l0);
		l1.insert_after(&mut l0);

		assert!(l0.prev.is_none());
		assert!(l0.next.is_some());
		assert!(l0.next.unwrap() == &mut l1 as _);

		assert!(l1.prev.is_some());
		assert!(l1.prev.unwrap() == &mut l0 as _);
		assert!(l1.next.is_some());
		assert!(l1.next.unwrap() == &mut l2 as _);

		assert!(l2.prev.is_some());
		assert!(l2.prev.unwrap() == &mut l1 as _);
		assert!(l2.next.is_none());
	}

	// TODO
}
