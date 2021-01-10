#![cfg_attr(not(userspace), no_std)]

#![feature(maybe_uninit_ref)]

extern crate util;

mod buddy;
mod r#const;
mod malloc;

use core::ffi::c_void;
use crate::r#const::*;

/// Converts a kernel physical address to a virtual address.
pub fn kern_to_virt(ptr: *const c_void) -> *const c_void {
	debug_assert!(ptr < PROCESS_END);
	((ptr as usize) + (PROCESS_END as usize)) as *const _
}

/// Converts a kernel virtual address to a physical address.
pub fn kern_to_phys(ptr: *const c_void) -> *const c_void {
	debug_assert!(ptr >= PROCESS_END);
	((ptr as usize) - (PROCESS_END as usize)) as *const _
}
