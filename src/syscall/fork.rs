/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(_regs: &util::Regs) -> u32 {
	let new_proc = Process::get_current().unwrap().fork();
	if let Err(new_proc) = new_proc {
		-new_proc as _
	} else {
		new_proc.unwrap().get_pid() as _
	}
}
