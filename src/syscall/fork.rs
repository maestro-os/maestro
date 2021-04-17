/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(_regs: &util::Regs) -> u32 {
	let curr_proc = Process::get_current().unwrap().lock().get();
	let new_proc = curr_proc.fork();
	if let Err(new_proc) = new_proc {
		-new_proc as _
	} else {
		new_proc.unwrap().lock().get().get_pid() as _
	}
}
