/*
 * This files handles data structures used into the kernel.
 */

use crate::memory::NULL;

/*
 * Structure representing a node in a doubly-linked list.
 */
pub struct LinkedList {
	/* Pointer to the previous element in the list */
	prev: *mut LinkedList,
	/* Pointer to the next element in the list */
	next: *mut LinkedList,
}

/*
 * Returns a reference to the element of type `type` for the given linked list node `node` stored
 * in field `field`.
 */
#[macro_export]
macro_rules! linked_list_get {
	($node:expr, $type:ty, $field:ident) => {
		crate::container_of!($node, $type, $field)
	}
}

impl LinkedList {
	/*
	 * Creates a single node.
	 */
	pub fn new_single() -> Self {
		Self {
			prev: NULL as _,
			next: NULL as _,
		}
	}

	/*
	 * Returns the previous element if it exsits, or None.
	 */
	pub fn get_prev(&self) -> Option<&'static mut LinkedList> {
		if self.prev != NULL as _ {
			Some(unsafe { &mut *self.prev })
		} else {
			None
		}
	}

	/*
	 * Returns the next element if it exsits, or None.
	 */
	pub fn get_next(&self) -> Option<&'static mut LinkedList> {
		if self.next != NULL as _ {
			Some(unsafe { &mut *self.next })
		} else {
			None
		}
	}

	/*
	 * Returns the size of the linked list, counting previous elements.
	 */
	pub fn left_size(&self) -> usize {
		let mut i = 0;
		let mut curr = self as *const LinkedList;

		while curr != NULL as _ {
			i += 1;
			curr = unsafe { (*curr).prev };
		}
		i
	}

	/*
	 * Returns the size of the linked list, counting next elements.
	 */
	pub fn right_size(&self) -> usize {
		let mut i = 0;
		let mut curr = self as *const LinkedList;

		while curr != NULL as _ {
			i += 1;
			curr = unsafe { (*curr).next };
		}
		i
	}

	/*
	 * Executes the given closure `f` for each nodes after the given node `node`, including the
	 * given one. The nodes are not mutable.
	 */
	pub fn foreach<T>(&self, f: T) where T: Fn(&LinkedList) {
		let mut curr = self as *const LinkedList;

		while curr != NULL as _ {
			unsafe {
				f(&*curr);
				curr = (*curr).next;
			}
		}
	}

	/*
	 * Same as `foreach` except the nodes are mutable.
	 */
	pub fn foreach_mut<T>(&mut self, f: T) where T: Fn(&mut LinkedList) {
		let mut curr = self as *mut LinkedList;

		while curr != NULL as _ {
			unsafe {
				f(&mut *curr);
				curr = (*curr).prev;
			}
		}
	}

	/*
	 * Links back adjacent nodes to the current node.
	 */
	unsafe fn link_back(&mut self) {
		if self.next != NULL as _ {
			(*self.next).prev = self;
		}
		if self.prev != NULL as _ {
			(*self.prev).next = self;
		}
	}

	/*
	 * Inserts the node at the beginning of the given linked list `front`.
	 */
	pub fn insert_front(&mut self, front: &mut *mut LinkedList) {
		self.prev = NULL as _;
		self.next = *front as _;
		*front = self as _;
		unsafe {
			self.link_back();
		}
	}

	/*
	 * Inserts the node before node `node` in the given linked list `front`.
	 */
	pub fn insert_before(&mut self, front: &mut *mut LinkedList, node: *mut LinkedList) {
		debug_assert!(node != NULL as _);

		if *front == node {
			*front = self;
		}

		unsafe {
			self.prev = (*node).prev;
			self.next = node;
			self.link_back();
		}
	}

	/*
	 * Inserts the node after node `node` in the given linked list `front`.
	 */
	pub fn insert_after(&mut self, front: &mut *mut LinkedList, node: *mut LinkedList) {
		debug_assert!(node != NULL as _);

		if *front == NULL as _ {
			*front = self;
		}

		unsafe {
			self.prev = node;
			self.next = (*node).next;
			self.link_back();
		}
	}

	/*
	 * Unlinks the current node from the linked list.
	 */
	pub fn unlink(&mut self) {
		if self.prev != NULL as _ {
			unsafe {
				(*self.prev).next = self.next;
			}
		}
		if self.next != NULL as _ {
			unsafe {
				(*self.next).prev = self.prev;
			}
		}
		self.prev = NULL as _;
		self.next = NULL as _;
	}
}

// TODO Binary tree
