/// This module handles system calls.
///
/// TODO doc

use crate::util;

/// This function is called whenever a system call is triggered.
#[no_mangle]
pub extern "C" fn syscall_handler(_regs: &util::Regs) {
	// TODO
}
