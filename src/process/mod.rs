/// This module handles processes.
/// TODO

pub mod mem_space;
pub mod pid;
pub mod scheduler;
pub mod semaphore;
pub mod signal;
pub mod tss;

use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use crate::errno::Errno;
use crate::errno;
use crate::event::{InterruptCallback, InterruptResult, InterruptResultAction};
use crate::event;
use crate::filesystem::File;
use crate::filesystem::Uid;
use crate::filesystem::file_descriptor::FileDescriptor;
use crate::filesystem::path::Path;
use crate::memory::vmem;
use crate::util::FailableClone;
use crate::util::Regs;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::*;
use crate::util::ptr::SharedPtr;
use crate::util;
use mem_space::MemSpace;
use mem_space::{MAPPING_FLAG_WRITE, MAPPING_FLAG_USER, MAPPING_FLAG_NOLAZY};
use pid::PIDManager;
use pid::Pid;
use scheduler::Scheduler;
use signal::Signal;
use signal::SignalHandler;
use signal::SignalType;

/// The size of the userspace stack of a process in number of pages.
const USER_STACK_SIZE: usize = 2048;
/// The flags for the userspace stack mapping.
const USER_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_USER;
/// The size of the kernelspace stack of a process in number of pages.
const KERNEL_STACK_SIZE: usize = 64;
/// The flags for the kernelspace stack mapping.
const KERNEL_STACK_FLAGS: u8 = MAPPING_FLAG_WRITE | MAPPING_FLAG_NOLAZY;

/// The default value of the eflags register.
const DEFAULT_EFLAGS: u32 = 0x1202;

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

/// Type representing an exit status.
type ExitStatus = u8;

/// The Process Control Block (PCB).
/// TODO doc
pub struct Process {
	/// The ID of the process.
	pid: Pid,
	/// The ID of the process group.
	pgid: Pid,

	/// The current state of the process.
	state: State,
	/// The ID of the process's owner.
	owner: Uid,

	/// The priority of the process.
	priority: usize,
	/// The number of quantum run during the cycle.
	quantum_count: usize,

	/// A pointer to the parent process.
	parent: Option::<NonNull::<Process>>, // TODO Use a weak pointer
	/// The list of children processes.
	children: Vec::<Pid>,
	/// The list of processes in the process group.
	process_group: Vec::<Pid>,

	/// The last saved registers state
	regs: Regs,
	/// Tells whether the process was syscalling or not.
	syscalling: bool,
	/// The virtual memory of the process containing every mappings.
	mem_space: MemSpace,

	/// A pointer to the userspace stack.
	user_stack: *const c_void,
	/// A pointer to the kernelspace stack.
	kernel_stack: *const c_void,

	/// The current working directory.
	cwd: Path,
	/// The list of open file descriptors.
	file_descriptors: Vec::<FileDescriptor>,

	/// The FIFO containing awaiting signals.
	signals_queue: Vec::<Signal>, // TODO Use a dedicated FIFO structure
	/// The list of signal handlers.
	signal_handlers: [Option<SignalHandler>; signal::SIGNALS_COUNT],

	/// The exit status of the process after exiting.
	exit_status: ExitStatus,
}

/// The PID manager.
static mut PID_MANAGER: MaybeUninit::<Mutex::<PIDManager>> = MaybeUninit::uninit();
/// The processes scheduler.
static mut SCHEDULER: MaybeUninit::<SharedPtr::<Mutex::<Scheduler>>> = MaybeUninit::uninit();

/// Scheduler ticking callback.
pub struct ProcessFaultCallback {}

impl InterruptCallback for ProcessFaultCallback {
	fn is_enabled(&self) -> bool {
		true
	}

	fn call(&mut self, id: u32, code: u32, regs: &util::Regs, ring: u32) -> InterruptResult {
		if ring < 3 {
			return InterruptResult::new(true, InterruptResultAction::Panic);
		}

		let mutex = unsafe {
			SCHEDULER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		let scheduler = guard.get_mut();

		if let Some(mut curr_proc) = scheduler.get_current_process() {
			let mut curr_proc_guard = MutexGuard::new(&mut curr_proc);
			let curr_proc = curr_proc_guard.get_mut();

			match id {
				0x0d => {
					let vmem = curr_proc.get_mem_space_mut().get_vmem();
					let mut inst_prefix = 0;
					vmem::vmem_switch(vmem.as_ref(), || {
						inst_prefix = unsafe {
							*(regs.eip as *const u8)
						};
					});

					if inst_prefix == HLT_INSTRUCTION {
						curr_proc.exit(regs.eax);
					} else {
						curr_proc.kill(signal::SIGSEGV).unwrap();
					}
				},
				0x0e => {
					let accessed_ptr = unsafe {
						vmem::x86::cr2_get()
					};
					if !curr_proc.mem_space.handle_page_fault(accessed_ptr, code) {
						curr_proc.kill(signal::SIGSEGV).unwrap();
					}
				},
				_ => {},
			}

			if curr_proc.get_state() == State::Running {
				InterruptResult::new(false, InterruptResultAction::Resume)
			} else {
				// TODO Avoid skipping other event handlers while ensuring the process won't
				// resume?
				InterruptResult::new(true, InterruptResultAction::Loop)
			}
		} else {
			InterruptResult::new(true, InterruptResultAction::Panic)
		}
	}
}

/// Initializes processes system. This function must be called only once, at kernel initialization.
pub fn init() -> Result<(), Errno> {
	tss::init();
	tss::flush();

	let cores_count = 1; // TODO
	unsafe {
		PID_MANAGER.write(Mutex::new(PIDManager::new()?));
		SCHEDULER.write(Scheduler::new(cores_count)?);
	}

	// TODO Register for all errors that can be caused by a process
	// TODO Use only one instance?
	event::register_callback(0x0d, u32::MAX, ProcessFaultCallback {})?;
	event::register_callback(0x0e, u32::MAX, ProcessFaultCallback {})?;

	Ok(())
}

impl Process {
	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_pid(pid: Pid) -> Option::<SharedPtr::<Mutex::<Self>>> {
		let mutex = unsafe {
			SCHEDULER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		guard.get_mut().get_by_pid(pid)
	}

	/// Returns the current running process. If no process is running, the function returns None.
	pub fn get_current() -> Option::<SharedPtr::<Mutex::<Self>>> {
		let mutex = unsafe {
			SCHEDULER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		guard.get_mut().get_current_process()
	}

	/// Creates a new process, assigns an unique PID to it and places it into the scheduler's
	/// queue. The process is set to state `Running` by default.
	/// `parent` is the parent of the process (optional).
	/// `owner` is the ID of the process's owner.
	/// `entry_point` is the pointer to the first instruction of the process.
	/// `cwd` the path to the process's working directory.
	pub fn new(parent: Option::<NonNull::<Process>>, owner: Uid, entry_point: *const c_void,
		cwd: Path) -> Result::<SharedPtr::<Mutex::<Self>>, Errno> {
		let pid = {
			let mutex = unsafe {
				PID_MANAGER.assume_init_mut()
			};
			let mut guard = MutexGuard::new(mutex);
			guard.get_mut().get_unique_pid()
		}?;

		// TODO Handle leaks on fail
		let mut mem_space = MemSpace::new()?;
		let user_stack = mem_space.map_stack(None, USER_STACK_SIZE, USER_STACK_FLAGS)?;
		let kernel_stack = mem_space.map_stack(None, KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;

		let process = Self {
			pid: pid,
			pgid: pid,

			state: State::Running,
			owner: owner,

			priority: 0,
			quantum_count: 0,

			parent: parent,
			children: Vec::new(),
			process_group: Vec::new(),

			regs: Regs {
				ebp: 0x0,
				esp: user_stack as _,
				eip: entry_point as _,
				eflags: DEFAULT_EFLAGS,
				eax: 0x0,
				ebx: 0x0,
				ecx: 0x0,
				edx: 0x0,
				esi: 0x0,
				edi: 0x0,
			},
			syscalling: false,
			mem_space: mem_space,

			user_stack: user_stack,
			kernel_stack: kernel_stack,

			cwd: cwd,
			file_descriptors: Vec::new(),

			signals_queue: Vec::new(),
			signal_handlers: [None; signal::SIGNALS_COUNT],

			exit_status: 0,
		};

		let mutex = unsafe {
			SCHEDULER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		guard.get_mut().add_process(process)
	}

	/// Returns the process's PID.
	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	/// Returns the process's group ID.
	pub fn get_pgid(&self) -> Pid {
		self.pgid
	}

	/// Tells whether the process is among a group and is not its owner.
	pub fn is_in_group(&self) -> bool {
		self.pgid != 0 && self.pgid != self.pid
	}

	/// Sets the process's group ID to the given value `pgid`.
	pub fn set_pgid(&mut self, pgid: Pid) -> Result<(), Errno> {
		if self.is_in_group() {
			let mut mutex = Process::get_by_pid(self.pgid).unwrap();
			let mut guard = MutexGuard::new(&mut mutex);
			let old_group_process = guard.get_mut();
			let i = old_group_process.process_group.binary_search(&self.pid).unwrap();
			old_group_process.process_group.remove(i);
		}

		self.pgid = {
			if pgid == 0 {
				self.pid
			} else {
				pgid
			}
		};

		if pgid != self.pid {
			if let Some(mut mutex) = Process::get_by_pid(pgid) {
				let mut guard = MutexGuard::new(&mut mutex);
				let new_group_process = guard.get_mut();
				let i = new_group_process.process_group.binary_search(&self.pid).unwrap_err();
				// TODO Beware of memory shortage
				new_group_process.process_group.insert(i, self.pid).unwrap();
				Ok(())
			} else {
				Err(errno::ESRCH)
			}
		} else {
			Ok(())
		}
	}

	/// Returns a reference to the list of PIDs of processes in the current process's group.
	pub fn get_group_processes(&self) -> &Vec<Pid> {
		&self.process_group
	}

	/// Returns the parent process's PID.
	pub fn get_parent_pid(&self) -> Pid {
		if let Some(mut parent) = self.parent {
			unsafe {
				parent.as_mut()
			}.get_pid()
		} else {
			self.get_pid()
		}
	}

	/// Returns the process's current state.
	pub fn get_state(&self) -> State {
		self.state
	}

	/// Sets the process's state to `new_state`.
	pub fn set_state(&mut self, new_state: State) {
		self.state = new_state;
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
	pub fn get_parent(&self) -> Option::<NonNull::<Process>> {
		self.parent
	}

	/// Returns a reference to the list of the process's children.
	pub fn get_children(&self) -> &Vec<Pid> {
		&self.children
	}

	/// Adds the process with the given PID `pid` as child to the process.
	pub fn add_child(&mut self, pid: Pid) -> Result<(), Errno> {
		let r = self.children.binary_search(&pid);
		let i = if let Ok(i) = r {
			i
		} else {
			r.unwrap_err()
		};
		self.children.insert(i, pid)
	}

	/// Removes the process with the given PID `pid` as child to the process.
	pub fn remove_child(&mut self, pid: Pid) {
		let r = self.children.binary_search(&pid);
		if let Ok(i) = r {
			self.children.remove(i);
		}
	}

	/// Returns a reference to the process's memory space.
	pub fn get_mem_space(&self) -> &MemSpace {
		&self.mem_space
	}

	/// Returns a mutable reference to the process's memory space.
	pub fn get_mem_space_mut(&mut self) -> &mut MemSpace {
		&mut self.mem_space
	}

	/// Returns a reference to the process's current working directory.
	pub fn get_cwd(&self) -> &Path {
		&self.cwd
	}

	/// Sets the process's current working directory.
	pub fn set_cwd(&mut self, path: Path) {
		self.cwd = path;
	}

	/// Returns the process's saved state registers.
	pub fn get_regs(&self) -> &util::Regs {
		&self.regs
	}

	/// Sets the process's saved state registers.
	pub fn set_regs(&mut self, regs: &util::Regs) {
		self.regs = *regs;
	}

	/// Tells whether the process was syscalling before being interrupted.
	pub fn is_syscalling(&self) -> bool {
		self.syscalling
	}

	/// Returns an available file descriptor ID. If no ID is available, the function returns an
	/// Err.
	fn get_available_fd(&mut self) -> Result::<u32, Errno> {
		// TODO
		Err(errno::EMFILE) // TODO If too many file descriptors are open systemwide, use ENFILE
	}

	/// Opens a file, creates a file descriptor and returns a mutable reference to it.
	/// `file` the file to open.
	/// If the file cannot be open, the function returns an Err.
	pub fn open_file(&mut self, file: &mut File) -> Result::<&mut FileDescriptor, Errno> {
		let id = self.get_available_fd()?;
		let index = self.file_descriptors.binary_search_by(| fd | {
			fd.get_id().cmp(&id)
		}).unwrap_err();
		let fd = FileDescriptor::new(id, file);
		if self.file_descriptors.insert(index, fd).is_ok() {
			Ok(&mut self.file_descriptors[index])
		} else {
			Err(errno::ENOMEM)
		}
	}

	/// Returns the file descriptor with ID `id`. If the file descriptor doesn't exist, the
	/// function returns None.
	pub fn get_fd(&mut self, id: u32) -> Option::<&mut FileDescriptor> {
		let result = self.file_descriptors.binary_search_by(| fd | {
			fd.get_id().cmp(&id)
		});
		if let Ok(index) = result {
			Some(&mut self.file_descriptors[index])
		} else {
			None
		}
	}

	/// Closes the file descriptor with the ID `id`. The function returns an Err if the file
	/// descriptor doesn't exist.
	pub fn close_fd(&mut self, id: u32) -> Result<(), Errno> {
		let result = self.file_descriptors.binary_search_by(| fd | {
			fd.get_id().cmp(&id)
		});
		if let Ok(index) = result {
			self.file_descriptors.remove(index);
			Ok(())
		} else {
			Err(errno::EBADF)
		}
	}

	/// Returns the exit code if the process has ended.
	pub fn get_exit_code(&self) -> Option<ExitStatus> {
		if self.state == State::Zombie {
			Some(self.exit_status)
		} else {
			None
		}
	}

	/// Forks the current process. Duplicating everything for it to be identical, except the PID,
	/// the parent process and children processes. On fail, the function returns an Err with the
	/// appropriate Errno.
	pub fn fork(&mut self) -> Result::<SharedPtr::<Mutex::<Self>>, Errno> {
		let pid = {
			let mutex = unsafe {
				PID_MANAGER.assume_init_mut()
			};
			let mut guard = MutexGuard::new(mutex);
			guard.get_mut().get_unique_pid()
		}?;

		let mut regs = self.regs;
		regs.eax = 0;

		let process = Self {
			pid: pid,
			pgid: self.pgid,

			state: State::Running,
			owner: self.owner,

			priority: self.priority,
			quantum_count: 0,

			parent: NonNull::new(self as _),
			children: Vec::new(),
			process_group: Vec::new(),

			regs: regs,
			syscalling: self.syscalling,
			mem_space: self.mem_space.fork()?,

			user_stack: self.user_stack,
			kernel_stack: self.kernel_stack,

			cwd: self.cwd.failable_clone()?,
			file_descriptors: self.file_descriptors.failable_clone()?,

			signals_queue: Vec::new(),
			signal_handlers: self.signal_handlers,

			exit_status: self.exit_status,
		};
		self.add_child(pid)?;

		let mutex = unsafe {
			SCHEDULER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		guard.get_mut().add_process(process)
	}

	/// Returns the signal handler for the signal type `type_`.
	pub fn get_signal_handler(&self, type_: SignalType) -> Option<SignalHandler> {
		self.signal_handlers[type_ as usize]
	}

	/// Kills the process with the given signal type `type`. This function enqueues a new signal
	/// to be processed. If the process doesn't have a signal handler, the default action for the
	/// signal is executed.
	pub fn kill(&mut self, type_: SignalType) -> Result<(), Errno> {
		// TODO Use preallocated memory for the signals queue?
		let signal = Signal::new(type_)?;
		if signal.can_catch() && self.get_signal_handler(type_).is_some() {
			self.signals_queue.push(signal)?;
		} else {
			signal.execute_action(self);
		}

		Ok(())
	}

	/// Exits the process with the given `status`. This function changes the process's status to
	/// `Zombie`.
	pub fn exit(&mut self, status: u32) {
		self.exit_status = (status & 0xff) as ExitStatus;
		self.state = State::Zombie;
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		if let Some(mut parent) = self.get_parent() {
			unsafe {
				parent.as_mut()
			}.remove_child(self.pid);
		}

		let mutex = unsafe {
			PID_MANAGER.assume_init_mut()
		};
		let mut guard = MutexGuard::new(mutex);
		guard.get_mut().release_pid(self.pid);
	}
}
