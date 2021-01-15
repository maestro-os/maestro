#![no_std]

#![feature(custom_test_frameworks)]
#![feature(maybe_uninit_ref)]

#![test_runner(util::selftest::runner)]
#![reexport_test_harness_main = "test_main"]

extern crate util;

pub mod buddy;
pub mod r#const;
pub mod malloc;

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
