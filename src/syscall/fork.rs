/// TODO doc

use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(_regs: &util::Regs) -> u32 {
	let result = Process::get_current().fork();
	if let Ok(new_proc) = result {
		new_proc.get_pid()
	} else {
		-result.unwrap_err() as _
	}
}
