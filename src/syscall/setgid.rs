//! The `setgid` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `setgid` syscall.
pub fn setgid(_proc: &mut Process, _regs: &util::Regs) -> Result<i32, Errno> {
    // TODO

    Ok(0)
}
