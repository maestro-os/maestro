//! The `waitpid` system call allows to wait for an event from a child process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::State;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::pid::INIT_PID;
use crate::process::pid::Pid;
use crate::process::regs::Regs;
use crate::process::rusage::RUsage;
use crate::process;

/// Wait flag. Returns immediately if no child has exited.
const WNOHANG: i32 =    0b001;
/// Wait flag. Returns if a child has stopped.
const WUNTRACED: i32 =  0b010;
/// Wait flag. Returns if a stopped child has been resumed by delivery of SIGCONT.
const WCONTINUED: i32 = 0b100;

/// Returns the `i`th target process for the given constraint `pid`.
/// `pid` is the constraint given to the system call.
/// `i` is the index of the target process.
/// The function is built such as iterating on `i` until the function returns None gives every
/// targets for the system call.
fn get_target(pid: i32, i: usize) -> Option<Pid> {
	let curr_proc_mutex = Process::get_current().unwrap();
	let curr_proc_guard = curr_proc_mutex.lock();
	let curr_proc = curr_proc_guard.get();

	if pid < -1 {
		let group_processes = curr_proc.get_group_processes();

		if i < group_processes.len() {
			Some(group_processes[i])
		} else {
			None
		}
	} else if pid == -1 {
		let children = curr_proc.get_children();

		if i < children.len() {
			Some(children[i])
		} else {
			None
		}
	} else if pid == 0 {
		let group = curr_proc.get_group_processes();

		if i < group.len() {
			Some(group[i])
		} else {
			None
		}
	} else if i == 0 {
		Some(pid as _)
	} else {
		None
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

/// Waits on the given process.
/// `proc` is the current process.
/// `wstatus` is a reference to the wait status.
/// `rusage` is the pointer to the resource usage structure.
fn wait_proc(proc: &mut Process, wstatus: &mut i32, rusage: &mut RUsage) {
	*wstatus = get_wstatus(&proc);
	*rusage = proc.get_rusage().clone();

	proc.clear_waitable();
}

/// Checks if at least one process corresponding to the given constraint is waitable. If yes, the
/// function clears its waitable state, sets the wstatus and returns the process's PID.
/// `pid` is the constraint given to the system call.
/// `wstatus` is a reference to the wait status.
/// `rusage` is the pointer to the resource usage structure.
fn check_waitable(pid: i32, wstatus: &mut i32, rusage: &mut RUsage) -> Result<Option<Pid>, Errno> {
	// Iterating on every target processes, checking if they can be waited on
	let mut i = 0;
	while let Some(pid) = get_target(pid, i) {
		let mut scheduler_guard = process::get_scheduler().lock();
		let scheduler = scheduler_guard.get_mut();

		if let Some(p) = scheduler.get_by_pid(pid) {
			let mut p_guard = p.lock();
			let p = p_guard.get_mut();
			let pid = p.get_pid();

			// If waitable, return
			if p.is_waitable() {
				wait_proc(p, wstatus, rusage);

				// If the process was a zombie, remove it
				if p.get_state() == process::State::Zombie {
					drop(p_guard);
					scheduler.remove_process(pid);
				}

				return Ok(Some(pid));
			}
		}

		i += 1;
	}
	if i == 0 {
		// No target
		return Err(errno!(ECHILD));
	}

	Ok(None)
}

/// Executes the `waitpid` system call.
/// `pid` is the PID to wait for.
/// `wstatus` is the pointer on which to write the status.
/// `options` are flags passed with the syscall.
/// `rusage` is the pointer to the resource usage structure.
pub fn do_waitpid(pid: i32, wstatus: SyscallPtr<i32>, options: i32,
	rusage: Option<SyscallPtr<RUsage>>) -> Result<i32, Errno> {
	// Checking `pid`
	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		if pid == INIT_PID as i32 || pid == proc.get_pid() as i32 {
			return Err(errno!(ECANCELED));
		}
	}

	// Sleeping until a target process is waitable
	loop {
		let mut wstatus_val = Default::default();
		let mut rusage_val = Default::default();

		// Check if at least one target process is waitable
		let result = check_waitable(pid, &mut wstatus_val, &mut rusage_val)?;

		{
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			if let Some(wstatus) = wstatus.get_mut(&mem_space_guard)? {
				*wstatus = wstatus_val;
			}

			if let Some(ref rusage) = rusage {
				if let Some(rusage) = rusage.get_mut(&mem_space_guard)? {
					*rusage = rusage_val;
				}
			}
		}

		// On success, return
		if let Some(p) = result {
			return Ok(p as _);
		}

		// If the flag is set, do not wait
		if options & WNOHANG != 0 {
			return Ok(0);
		}

		{
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			// When a child process is paused or resumed by a signal or is terminated, it changes
			// the state of the current process to wake it up
			proc.set_state(process::State::Sleeping);
		}

		crate::wait();
	}
}

/// The implementation of the `waitpid` syscall.
pub fn waitpid(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let wstatus: SyscallPtr<i32> = (regs.ecx as usize).into();
	let options = regs.edx as i32;

	do_waitpid(pid, wstatus, options, None)
}
