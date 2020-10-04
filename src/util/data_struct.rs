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

	// TODO get_value (macro)

	// TODO insert_front
	// TODO insert_before
	// TODO insert_after

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
