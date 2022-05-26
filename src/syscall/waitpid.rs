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
/// The function is built such as iterating on `i` until the function returns None gives every
/// targets for the system call.
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
	} else if i == 0 && scheduler.get_by_pid(pid as _).is_some() {
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
/// `wstatus` is a reference to the wait status. If None, the wstatus is not written.
/// `rusage` is the pointer to the resource usage structure. If None, the rusage is not written.
fn wait_proc(proc: &mut Process, wstatus: Option<&mut i32>, rusage: Option<&mut RUsage>) {
	if let Some(wstatus) = wstatus {
		*wstatus = get_wstatus(&proc);
	}
	if let Some(rusage) = rusage {
		*rusage = proc.get_rusage().clone();
	}

	proc.clear_waitable();
}

/// Checks if at least one process corresponding to the given constraint is waitable. If yes, the
/// function clears its waitable state, sets the wstatus and returns the process's PID.
/// `proc` is the current process.
/// `pid` is the constraint given to the system call.
/// `wstatus` is a reference to the wait status. If None, the wstatus is not written.
/// `rusage` is the pointer to the resource usage structure.
fn check_waitable(proc: &mut Process, pid: i32, wstatus: Option<&mut i32>,
	rusage: Option<&mut RUsage>) -> Result<Option<Pid>, Errno> {
	let mut scheduler_guard = process::get_scheduler().lock();
	let scheduler = scheduler_guard.get_mut();

	// Iterating on every target processes, checking if they can be waited on
	let mut i = 0;
	while let Some(pid) = get_target(scheduler, proc, pid, i) {
		if pid == proc.get_pid() {
			// If waitable, return
			if proc.is_waitable() {
				wait_proc(proc, wstatus, rusage);
				return Ok(Some(pid));
			}
		} else if let Some(p) = scheduler.get_by_pid(pid) {
			let mut proc_guard = p.lock();
			let p = proc_guard.get_mut();
			let pid = p.get_pid();

			// If waitable, return
			if p.is_waitable() {
				wait_proc(p, wstatus, rusage);

				// If the process was a zombie, remove it
				if p.get_state() == process::State::Zombie {
					drop(proc_guard);
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
	// Sleeping until a target process is waitable
	loop {
		// Check if at least one target process is waitable
		{
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			// TODO Apply to every processes that cannot be waited on
			if pid == INIT_PID as i32 || pid == proc.get_pid() as i32 {
				return Err(errno!(ECANCELED));
			}

			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();

			let wstatus = wstatus.get_mut(&mem_space_guard)?;
			let rusage = match rusage {
				Some(ref rusage) => rusage.get_mut(&mem_space_guard)?,
				None => None,
			};

			// If waitable, return
			if let Some(p) = check_waitable(proc, pid, wstatus, rusage)? {
				return Ok(p as _);
			}

			// If the flag is set, do not wait
			if options & WNOHANG != 0 {
				return Ok(0);
			}

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
