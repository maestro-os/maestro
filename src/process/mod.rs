/// This module handles processes.
/// TODO

pub mod mem_space;
pub mod pid;
pub mod scheduler;
pub mod tss;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use crate::event::InterruptCallback;
use crate::event;
use crate::memory::vmem;
use crate::util::Regs;
use crate::util::ptr::SharedPtr;
use crate::util;
use mem_space::MemSpace;
use mem_space::{MAPPING_FLAG_WRITE, MAPPING_FLAG_USER, MAPPING_FLAG_NOLAZY};
use pid::PIDManager;
use pid::Pid;
use scheduler::Scheduler;

/// The size of the userspace stack of a process in number of pages.
const USER_STACK_SIZE: usize = 2048;
/// The flags for the userspace stack mapping.
const USER_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;
/// The size of the kernelspace stack of a process in number of pages.
const KERNEL_STACK_SIZE: usize = 8;
/// The flags for the kernelspace stack mapping.
const KERNEL_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_NOLAZY;

/// Type representing the type of a signal.
pub type SignalType = u8;

/// Process abort.
pub const SIGABRT: SignalType = 0;
/// Alarm clock.
pub const SIGALRM: SignalType = 1;
/// Access to an undefined portion of a memory object.
pub const SIGBUS: SignalType = 2;
/// Child process terminated.
pub const SIGCHLD: SignalType = 3;
/// Continue executing.
pub const SIGCONT: SignalType = 4;
/// Erroneous arithmetic operation.
pub const SIGFPE: SignalType = 5;
/// Hangup.
pub const SIGHUP: SignalType = 6;
/// Illigal instruction.
pub const SIGILL: SignalType = 7;
/// Terminal interrupt.
pub const SIGINT: SignalType = 8;
/// Kill.
pub const SIGKILL: SignalType = 9;
/// Write on a pipe with no one to read it.
pub const SIGPIPE: SignalType = 10;
/// Terminal quit.
pub const SIGQUIT: SignalType = 11;
/// Invalid memory reference.
pub const SIGSEGV: SignalType = 12;
/// Stop executing.
pub const SIGSTOP: SignalType = 13;
/// Termination.
pub const SIGTERM: SignalType = 14;
/// Terminal stop.
pub const SIGTSTP: SignalType = 15;
/// Background process attempting read.
pub const SIGTTIN: SignalType = 16;
/// Background process attempting write.
pub const SIGTTOU: SignalType = 17;
/// User-defined signal 1.
pub const SIGUSR1: SignalType = 18;
/// User-defined signal 2.
pub const SIGUSR2: SignalType = 19;
/// Pollable event.
pub const SIGPOLL: SignalType = 20;
/// Profiling timer expired.
pub const SIGPROF: SignalType = 21;
/// Bad system call.
pub const SIGSYS: SignalType = 22;
/// Trace/breakpoint trap.
pub const SIGTRAP: SignalType = 23;
/// High bandwidth data is available at a socket.
pub const SIGURG: SignalType = 24;
/// Virtual timer expired.
pub const SIGVTALRM: SignalType = 25;
/// CPU time limit exceeded.
pub const SIGXCPU: SignalType = 26;
/// File size limit exceeded.
pub const SIGXFSZ: SignalType = 27;
/// Window resize.
pub const SIGWINCH: SignalType = 28;

/// The opcode of the `hlt` instruction.
const HLT_INSTRUCTION: u8 = 0xf4;

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

	/// The priority of the process.
	priority: usize,
	/// The number of quantum run during the cycle.
	quantum_count: usize,

	/// A pointer to the parent process.
	parent: Option::<*mut Process>, // TODO Use a weak pointer
	// TODO Children list

	/// The last saved registers state
	regs: Regs,
	/// The virtual memory of the process containing every mappings.
	mem_space: MemSpace,

	/// A pointer to the userspace stack.
	user_stack: *const c_void,
	/// A pointer to the kernelspace stack.
	kernel_stack: *const c_void,

	// TODO File descriptors
	// TODO Signals list

	/// The exit status of the process after exiting.
	exit_status: u8,
}

// TODO Use mutexes. Take into account sharing with interrupts
/// The PID manager.
static mut PID_MANAGER: MaybeUninit::<PIDManager> = MaybeUninit::uninit();
/// The processes scheduler.
static mut SCHEDULER: MaybeUninit::<SharedPtr::<Scheduler>> = MaybeUninit::uninit();

/// Scheduler ticking callback.
pub struct ProcessFaultCallback {}

impl InterruptCallback for ProcessFaultCallback {
	fn is_enabled(&self) -> bool {
		true
	}

	fn call(&mut self, id: u32, code: u32, regs: &util::Regs) -> bool {
		let scheduler = unsafe { // Access to global variable
			SCHEDULER.assume_init_mut()
		};
		if let Some(curr_proc) = scheduler.get_current_process() {
			let signal = match id {
				0x0d => {
					// TODO Make sure the process's virtual memory is bound
					let inst_prefix = unsafe {
						*(regs.eip as *const u8)
					};
					if inst_prefix == HLT_INSTRUCTION {
						curr_proc.exit(regs.eax);
						// TODO Returning the function shall not result in resuming execution
						None
					} else {
						Some(SIGSEGV)
					}
				},
				0x0e => {
					let accessed_ptr = unsafe { // Call to ASM function
						vmem::x86::cr2_get()
					};
					if curr_proc.mem_space.handle_page_fault(accessed_ptr, code) {
						None
					} else {
						Some(SIGSEGV)
					}
				},
				_ => None,
			};

			if let Some(signal) = signal {
				curr_proc.kill(signal);
				// TODO The function must return to execute other event handlers, but the process
				// must not continue execution
				unsafe { // Call to ASM function
					crate::kernel_loop();
				}
			}

			true
		} else {
			false
		}
	}
}

/// Initializes processes system. This function must be called only once, at kernel initialization.
pub fn init() -> Result::<(), ()> {
	tss::init();
	tss::flush();

	unsafe { // Access to global variable
		PID_MANAGER.write(PIDManager::new()?);
		SCHEDULER.write(Scheduler::new()?);
	}

	// TODO Register for all errors that can be caused by a process
	// TODO Use only one instance?
	event::register_callback(0x0d, u32::MAX, ProcessFaultCallback {})?;
	event::register_callback(0x0e, u32::MAX, ProcessFaultCallback {})?;

	Ok(())
}

impl Process {
	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns None.
	pub fn get_by_pid(pid: Pid) -> Option::<SharedPtr::<Self>> {
		unsafe { // Access to global variable
			SCHEDULER.assume_init_mut()
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
			PID_MANAGER.assume_init_mut()
		}.get_unique_pid()?;
		let mut mem_space = MemSpace::new()?;
		let user_stack = mem_space.map_stack(None, USER_STACK_SIZE, USER_STACK_FLAGS)?;
		// TODO On fail, free user_stack (use RAII?)
		let kernel_stack = mem_space.map_stack(None, KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;

		let process = Self {
			pid: pid,
			state: State::Running,
			owner: owner,

			priority: 0,
			quantum_count: 0,

			parent: parent,

			regs: Regs {
				ebp: 0x0,
				esp: user_stack as _,
				eip: entry_point as _,
				eflags: 0x0,
				eax: 0x0,
				ebx: 0x0,
				ecx: 0x0,
				edx: 0x0,
				esi: 0x0,
				edi: 0x0,
			},
			mem_space: mem_space,

			user_stack: user_stack,
			kernel_stack: kernel_stack,

			exit_status: 0,
		};

		unsafe { // Access to global variable
			SCHEDULER.assume_init_mut()
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

	/// Returns the priority of the process. A greater number means a higher priority relative to
	/// other processes.
	pub fn get_priority(&self) -> usize {
		self.priority
	}

	/// Returns the process's parent if exists.
	pub fn get_parent(&self) -> Option::<*mut Process> {
		self.parent
	}

	/// Kills the process with the given signal type `type`. This function enqueues a new signal
	/// to be processed. If the process doesn't have a signal handler, the default action for the
	/// signal is executed.
	pub fn kill(&mut self, _type: SignalType) {
		// TODO
	}

	/// Exits the process with the given `status`. This function changes the process's status to
	/// `Zombie`.
	pub fn exit(&mut self, status: u32) {
		self.exit_status = (status & 0xff) as u8;
		self.state = State::Zombie;
	}
}
