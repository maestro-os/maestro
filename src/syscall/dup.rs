//! The `dup` syscall allows to duplicate a file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `dup` syscall.
pub fn dup(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
    let oldfd = regs.ebx;

    Ok(proc.duplicate_fd(oldfd, None)?.get_id() as _)
}
