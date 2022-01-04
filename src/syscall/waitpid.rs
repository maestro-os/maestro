//! The `waitpid` system call allows to wait for an event from a child process.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::scheduler::Scheduler;
use crate::process;

/// Wait flag. Returns immediately if no child has exited.
const WNOHANG: i32 =    0b001;
/// Wait flag. Returns if a child has stopped.
const WUNTRACED: i32 =  0b010;
/// Wait flag. Returns if a stopped child has been resumed by delivery of SIGCONT.
const WCONTINUED: i32 = 0b100;

/// Returns the `i`th target process for the given constraint `pid`.
/// `scheduler` is a reference to the process scheduler.
/// `proc` is the current process.
/// `pid` is the constraint given to the system call.
/// `i` is the index of the target process.
/// The function built such as iterating on `i` until the function returns None gives every targets
/// for the system call.
fn get_target(scheduler: &mut Scheduler, proc: &Process, pid: i32, i: usize) -> Option<Pid> {
	if pid < -1 {
		let _group_leader = scheduler.get_by_pid(-pid as _)?;
		let group_processes = proc.get_group_processes();
		if i < group_processes.len() {
			let p = group_processes[i];

			scheduler.get_by_pid(p)?;
			Some(p)
		} else {
			None
		}
	} else if pid == -1 {
		let children = proc.get_children();
		if i < children.len() {
			let p = children[i];

			scheduler.get_by_pid(p)?;
			Some(p)
		} else {
			None
		}
	} else if pid == 0 {
		let group = proc.get_group_processes();
		if i < group.len() {
			Some(group[i])
		} else {
			None
		}
	} else {
		if i == 0 && scheduler.get_by_pid(pid as _).is_some() {
			Some(pid as _)
		} else {
			None
		}
	}
}

/// Returns the wait status for the given process.
fn get_wstatus(proc: &Process) -> i32 {
	let status = proc.get_exit_status().unwrap_or(0);
	let termsig = proc.get_termsig();
	let stopped = proc.get_state() == State::Stopped;

	let mut wstatus = ((status as i32 & 0xff) << 8) | (termsig as i32 & 0x7f);
	if !stopped {
		wstatus |= 1 << 7;
	}

	wstatus
}

/// Checks if at least one process corresponding to the given constraint is waitable. If yes, the
/// function clears its waitable state, sets the wstatus and returns the process's PID.
/// `proc` is the current process.
/// `pid` is the constraint given to the system call.
/// `wstatus` is a reference to the wait status. If None, the wstatus is not written.
fn check_waitable(proc: &Process, pid: i32, wstatus: &mut Option<&mut i32>)
	-> Result<Option<Pid>, Errno> {
	let mut scheduler_guard = process::get_scheduler().lock();
	let scheduler = scheduler_guard.get_mut();

	// Iterating on every target processes, checking if they can be waited on
	let mut i = 0;
	while let Some(pid) = get_target(scheduler, proc, pid, i) {
		if let Some(p) = scheduler.get_by_pid(pid) {
			let mut proc_guard = p.lock();
			let p = proc_guard.get_mut();

			// If waitable, return
			if p.is_waitable() {
				if let Some(wstatus) = wstatus {
					**wstatus = get_wstatus(&p);
				}

				p.clear_waitable();

				if p.get_state() == process::State::Zombie {
					let pid = p.get_pid();
					drop(proc_guard);
					scheduler.remove_process(pid);
				}

				return Ok(Some(pid as _));
			}
		}

		i += 1;
	}
	if i == 0 {
		// No target
		return Err(errno::ECHILD);
	}

	Ok(None)
}

/// Executes the `waitpid` system call.
/// `pid` is the PID to wait for.
/// `wstatus` is the pointer on which to write the status.
/// `options` are flags passed with the syscall.
pub fn do_waitpid(pid: i32, wstatus: *mut i32, options: i32) -> Result<i32, Errno> {
	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let len = size_of::<i32>();
		if !proc.get_mem_space().unwrap().can_access(wstatus as _, len, true, true) {
			return Err(errno::EINVAL);
		}
	}

	let mut wstatus = {
		if wstatus as usize != 0x0 {
			Some(unsafe { // Safe because the pointer is checked before
				&mut *wstatus
			})
		} else {
			None
		}
	};

	// Sleeping until a target process is waitable
	loop {
		// Check if at least one target process is waitable
		{
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			// If waitable, return
			if let Some(p) = check_waitable(proc, pid, &mut wstatus)? {
				return Ok(p as _);
			}
		}

		// If the flag is set, do not wait
		if options & WNOHANG != 0 {
			return Ok(0);
		}

		// When a child process is paused or resumed by a signal or is terminated, it changes the
		// state of the current process to wake it up
		{
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			proc.set_state(process::State::Sleeping);
		}

		crate::wait();
	}
}

/// The implementation of the `waitpid` syscall.
pub fn waitpid(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let wstatus = regs.ecx as *mut i32;
	let options = regs.edx as i32;

	do_waitpid(pid, wstatus, options)
}
