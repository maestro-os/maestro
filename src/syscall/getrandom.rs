//! The `getrandom` system call allows to get random bytes.

use crate::errno::Errno;
use crate::process::regs::Regs;

pub fn getrandom(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
