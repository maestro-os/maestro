/// TODO doc

use crate::process::Process;
use crate::util::lock::mutex::MutexGuard;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(_regs: &util::Regs) -> u32 {
	let mut mutex = Process::get_current().unwrap();
	let mut guard = MutexGuard::new(&mut mutex);
	let curr_proc = guard.get_mut();
	let new_proc = curr_proc.fork();
	if let Err(new_proc) = new_proc {
		-new_proc as _
	} else {
		new_proc.unwrap().lock().get().get_pid() as _
	}
}
