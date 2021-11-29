//! This module implements the `user_desc` structure, which is used in userspace to specify the
//! value for a descriptor, either a local or global descriptor.

use core::ffi::c_void;

/// The size of the user_desc structure in bytes.
const USER_DESC_SIZE: usize = 13;

/// The `user_desc` structure.
#[repr(transparent)]
pub struct UserDesc {
    val: &'static mut [i8; USER_DESC_SIZE],
}

impl UserDesc {
    /// Creates a new instance from the given pointer.
    pub unsafe fn from_ptr(ptr: *mut c_void) -> Self {
        Self {
            val: &mut *(ptr as *mut [i8; USER_DESC_SIZE]),
        }
    }

    /// Returns the entry number.
    #[inline(always)]
    pub fn get_entry_number(&self) -> i32 {
        unsafe { // Safe because the structure is large enough
		    *(&self.val[0] as *const _ as *const i32)
	    }
    }

    /// Sets the entry number.
    pub fn set_entry_number(&mut self, number: i32) {
        unsafe { // Safe because the structure is large enough
		    *(&mut self.val[0] as *mut _ as *mut i32) = number;
	    }
    }

    /// Returns the base address.
    #[inline(always)]
    pub fn get_base_addr(&self) -> i32 {
	    unsafe { // Safe because the structure is large enough
		    *(&self.val[4] as *const _ as *const i32)
	    }
    }

    /// Returns the limit.
    #[inline(always)]
    pub fn get_limit(&self) -> i32 {
	    unsafe { // Safe because the structure is large enough
		    *(&self.val[8] as *const _ as *const i32)
	    }
    }

    // TODO
}
