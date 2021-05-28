//! The `dup2` syscall allows to duplicate a file descriptor, specifying the id of the newly
//! created file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `dup2` syscall.
pub fn dup2(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
    let oldfd = regs.ebx;
    let newfd = regs.ecx;

    Ok(proc.duplicate_fd(oldfd, Some(newfd))?.get_id() as _)
}
