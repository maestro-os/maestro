/*
 * This files handles data structures used into the kernel.
 */

/*
 * Converts an `Option<*mut Self>` into a `Option<*const Self>`.
 */
#[inline(always)]
fn option_mut_to_const<T>(option: Option<*mut T>) -> Option<*const T> {
	if let Some(ptr) = option {
		Some(ptr as *const T)
	} else {
		None
	}
}

/*
 * Structure representing a node in a doubly-linked list.
 *
 * TODO Explain difference between floating and non-floating lists
 */
pub struct LinkedList {
	/* Pointer to the previous element in the list */
	prev: Option<*mut LinkedList>,
	/* Pointer to the next element in the list */
	next: Option<*mut LinkedList>,
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
			prev: None,
			next: None,
		}
	}

	/*
	 * Returns the previous element if it exsits, or None.
	 */
	pub fn get_prev(&self) -> Option<&'static mut LinkedList> {
		if self.prev.is_some() {
			Some(unsafe { &mut *self.prev.unwrap() })
		} else {
			None
		}
	}

	/*
	 * Returns the next element if it exsits, or None.
	 */
	pub fn get_next(&self) -> Option<&'static mut LinkedList> {
		if self.next.is_some() {
			Some(unsafe { &mut *self.next.unwrap() })
		} else {
			None
		}
	}

	/*
	 * Returns the size of the linked list, counting previous elements.
	 */
	pub fn left_size(&self) -> usize {
		let mut i = 0;
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			i += 1;
			curr = option_mut_to_const(unsafe { (*curr.unwrap()).prev });
		}
		i
	}

	/*
	 * Returns the size of the linked list, counting next elements.
	 */
	pub fn right_size(&self) -> usize {
		let mut i = 0;
		let mut curr: Option<*const Self> = Some(self);

		while curr.is_some() {
			i += 1;
			curr = option_mut_to_const(unsafe { (*curr.unwrap()).next });
		}
		i
	}

	/*
	 * Executes the given closure `f` for each nodes after the given node `node`, including the
	 * given one. The nodes are not mutable.
	 */
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

	/*
	 * Same as `foreach` except the nodes are mutable.
	 */
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

	/*
	 * Links back adjacent nodes to the current node.
	 */
	unsafe fn link_back(&mut self) {
		if self.next.is_some() {
			(*self.next.unwrap()).prev = Some(self);
		}
		if self.prev.is_some() {
			(*self.prev.unwrap()).next = Some(self);
		}
	}

	/*
	 * Inserts the node at the beginning of the given linked list `front`.
	 */
	pub fn insert_front(&mut self, front: &mut Option<*mut LinkedList>) {
		self.prev = None;
		self.next = *front as _;
		*front = Some(self);
		unsafe {
			self.link_back();
		}
	}

	/*
	 * Inserts the node before node `node` in the given linked list `front`.
	 */
	pub fn insert_before(&mut self, front: &mut Option<*mut LinkedList>, node: &mut LinkedList) {
		if front.is_some() && front.unwrap() == node {
			*front = Some(self);
		}

		self.insert_before_floating(node);
	}

	/*
	 * Inserts the node before node `node` in a floating linked list.
	 */
	pub fn insert_before_floating(&mut self, node: &mut LinkedList) {
		unsafe {
			self.prev = (*node).prev;
			self.next = Some(node);
			self.link_back();
		}
	}

	/*
	 * Inserts the node after node `node` in the given linked list `front`.
	 */
	pub fn insert_after(&mut self, node: &mut LinkedList) {
		unsafe {
			self.prev = Some(node);
			self.next = (*node).next;
			self.link_back();
		}
	}

	pub fn unlink(&mut self, front: &mut Option<*mut LinkedList>) {
		if front.is_some() && front.unwrap() == self {
			*front = self.next;
		}

		self.unlink_floating();
	}

	/*
	 * Unlinks the current node from the floating linked list.
	 */
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

// TODO Binary tree
