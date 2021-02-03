/// This module handles processes.
/// TODO

pub mod pid;
pub mod scheduler;
pub mod tss;

use core::ffi::c_void;
use crate::process::pid::PIDManager;
use crate::process::pid::Pid;
use crate::process::scheduler::Scheduler;
use crate::util::Regs;
use crate::util::ptr::SharedPtr;

/// An enumeration containing possible states for a process.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum State {
	/// The process is running or waiting to run.
	Running,
	/// The process is waiting for an event.
	Sleeping,
	/// The process has been stopped by a signal or by tracing.
	Stopped,
	/// The process has been killed.
	Zombie,
}

/// Type representing a User ID.
type Uid = u16;

/// The Process Control Block (PCB).
/// TODO doc
pub struct Process {
	/// The ID of the process.
	pid: Pid,
	/// The current state of the process.
	state: State,
	/// The ID of the process's owner.
	owner: Uid,

	/// A pointer to the parent process.
	parent: Option::<*mut Process>, // TODO Use a weak pointer
	// TODO Children list

	/// The last saved registers state
	regs: Regs,

	// TODO Virtual memory

	/// A pointer to the userspace stack.
	user_stack: *mut c_void,
	/// A pointer to the kernelspace stack.
	kernel_stack: *mut c_void,

	// TODO File descriptors
	// TODO Signals list
}

// TODO Use MaybeUninit?
/// The PID manager.
static mut PID_MANAGER: Option::<PIDManager> = None; // TODO Wrap in mutex
/// The processes scheduler.
static mut SCHEDULER: Option::<Scheduler> = None; // TODO Wrap in mutex

/// Initializes processes system.
pub fn init() -> Result::<(), ()> {
	tss::init();
	tss::flush();

	unsafe { // Access to global variable
		PID_MANAGER = Some(PIDManager::new()?);
		SCHEDULER = Some(Scheduler::new()?);
	}

	Ok(())
}

impl Process {
	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns None.
	pub fn get_by_pid(pid: Pid) -> Option::<SharedPtr::<Self>> {
		unsafe { // Access to global variable
			SCHEDULER.as_mut().unwrap()
		}.get_by_pid(pid)
	}

	/// Creates a new process, assigns an unique PID to it and places it into the scheduler's
	/// queue. The process is set to state `Running` by default.
	/// `parent` is the parent of the process (optional).
	/// `owner` is the ID of the process's owner.
	pub fn new(parent: Option::<*mut Process>, owner: Uid, entry_point: *const c_void)
		-> Result::<SharedPtr::<Self>, ()> {

		// TODO Deadlock fix: requires both memory allocator and PID allocator
		let pid = unsafe { // Access to global variable
			PID_MANAGER.as_mut().unwrap()
		}.get_unique_pid()?;
		let user_stack = core::ptr::null_mut::<c_void>(); // TODO
		let kernel_stack = core::ptr::null_mut::<c_void>(); // TODO

		let process = Self {
			pid: pid,
			state: State::Running,
			owner: owner,

			parent: parent,

			regs: Regs {
				ebp: 0x0,
				esp: 0x0,
				eip: entry_point as _,
				eflags: 0x0,
				eax: 0x0,
				ebx: 0x0,
				ecx: 0x0,
				edx: 0x0,
				esi: 0x0,
				edi: 0x0,
			},

			user_stack: user_stack,
			kernel_stack: kernel_stack,
		};

		unsafe { // Access to global variable
			SCHEDULER.as_mut().unwrap()
		}.add_process(process)
	}

	/// Returns the process's PID.
	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	/// Returns the process's current state.
	pub fn get_current_state(&self) -> State {
		self.state
	}

	/// Returns the process's owner ID.
	pub fn get_owner(&self) -> Uid {
		self.owner
	}

	/// Returns the process's parent if exists.
	pub fn get_parent(&self) -> Option::<*mut Process> {
		self.parent
	}

	// TODO
}
