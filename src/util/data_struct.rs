/*
 * This files handles data structures used into the kernel.
 */

use crate::memory::NULL;
use crate::memory::Void;

/*
 * Structure representing a node in a doubly-linked list.
 */
struct LinkedList {
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
		::container_of!($node, $type, $field)
	}
}

impl LinkedList {
	/*
	 * Returns the size of the linked list, counting previous elements.
	 */
	pub fn left_size(&self) -> usize {
		let mut i = 0;
		let mut curr = self as *const LinkedList;

		while curr as *const Void != NULL {
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

		while curr as *const Void != NULL {
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

		while curr as *const Void != NULL {
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

		while curr as *const Void != NULL {
			unsafe {
				f(&mut *curr);
				curr = (*curr).prev;
			}
		}
	}

	/*
	 * TODO
	 */
	//pub fn insert_front() {
		// TODO
	//}

	/*
	 * TODO
	 */
	//pub fn insert_before() {
		// TODO
	//}

	/*
	 * TODO
	 */
	//pub fn insert_after() {
		// TODO
	//}

	/*
	 * Unlinks the current node from the linked list.
	 */
	pub fn unlink(&mut self) {
		if self.prev as *const Void != NULL {
			unsafe {
				(*self.prev).next = self.next;
			}
		}
		if self.next as *const Void != NULL {
			unsafe {
				(*self.next).prev = self.prev;
			}
		}
		self.prev = NULL as _;
		self.next = NULL as _;
	}
}

// TODO Binary tree
