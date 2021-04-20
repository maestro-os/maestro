/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(proc: &mut Process, _regs: &util::Regs) -> u32 {
	let new_proc = proc.fork();
	if let Err(new_proc) = new_proc {
		-new_proc as _
	} else {
		new_proc.unwrap().lock().get().get_pid() as _
	}
}
