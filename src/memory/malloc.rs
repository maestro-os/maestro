/*
 * This file handles allocations of chunks of kernel memory.
 */

use core::ffi::c_void;

/*
 * Allocates `n` bytes of kernel memory and returns a pointer to the beginning of the allocated
 * chunk. If the allocation fails, the function shall return None.
 */
pub fn alloc(_n: usize) -> Option<*const c_void> {
	// TODO
	None
}

/*
 * Changes the size of the memory previously allocated with `alloc`. `ptr` is the pointer to the
 * chunk of memory. `n` is the new size of the chunk of memory. If the reallocation fails, the
 * chunk is left untouched.
 */
pub fn realloc(_ptr: *const c_void, _n: usize) -> Option<*const c_void> {
	// TODO
	None
}

/*
 * Frees the memory at the pointer `ptr` previously allocated with `alloc`. Subsequent uses of the
 * associated memory are undefined.
 */
pub fn free(_ptr: *const c_void) {
	// TODO
}
