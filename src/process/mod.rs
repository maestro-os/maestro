//! A process is a task running on the kernel. A multitasking system allows several processes to
//! run at the same time by sharing the CPU resources using a scheduler.

pub mod mem_space;
pub mod oom;
pub mod pid;
pub mod scheduler;
pub mod semaphore;
pub mod signal;
pub mod tss;

use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;
use crate::errno::Errno;
use crate::errno;
use crate::event::{InterruptResult, InterruptResultAction};
use crate::event;
use crate::file::Gid;
use crate::file::Uid;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor::FileDescriptor;
use crate::file::file_descriptor;
use crate::file::path::Path;
use crate::file;
use crate::limits;
use crate::memory::vmem;
use crate::util::FailableClone;
use crate::util::Regs;
use crate::util::container::bitfield::Bitfield;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::*;
use crate::util::ptr::SharedPtr;
use crate::util::ptr::WeakPtr;
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

/// The path to the TTY device file.
const TTY_DEVICE_PATH: &str = "/dev/tty";

/// The default file creation mask.
const DEFAULT_UMASK: u16 = 0o022;

/// The file descriptor number of the standard input stream.
const STDIN_FILENO: u32 = 0;
/// The file descriptor number of the standard output stream.
const STDOUT_FILENO: u32 = 1;
/// The file descriptor number of the standard error stream.
const STDERR_FILENO: u32 = 2;

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

/// The Process Control Block (PCB). This structure stores all the informations about a process.
pub struct Process {
	/// The ID of the process.
	pid: Pid,
	/// The ID of the process group.
	pgid: Pid,

	/// The ID of the process's user owner.
	uid: Uid,
	/// The ID of the process's group owner.
	gid: Gid,

	// TODO euid and egid

	/// File creation mask.
	umask: u16,

	/// The current state of the process.
	state: State,
	/// The priority of the process.
	priority: usize,
	/// The number of quantum run during the cycle.
	quantum_count: usize,

	/// A pointer to the parent process.
	parent: Option<WeakPtr<Process>>,
	/// The list of children processes.
	children: Vec<Pid>,
	/// The list of processes in the process group.
	process_group: Vec<Pid>,

	/// The last saved registers state.
	regs: Regs,
	/// Tells whether the process was syscalling or not.
	syscalling: bool,

	/// Tells whether the process is handling a signal.
	handled_signal: Option<SignalType>,
	/// The saved state of registers, used when handling a signal.
	saved_regs: Regs,
	/// Tells whether the process has information that can be retrieved by wait/waitpid.
	waitable: bool,

	/// The virtual memory of the process containing every mappings.
	mem_space: Option<MemSpace>,

	/// A pointer to the userspace stack.
	user_stack: *const c_void,
	/// A pointer to the kernelspace stack.
	kernel_stack: *const c_void,

	/// The current working directory.
	cwd: Path,
	/// The list of open file descriptors.
	file_descriptors: Vec<FileDescriptor>,

	/// A bitfield storing signals that have been received and are not handled yet.
	signals_bitfield: Bitfield,
	/// The list of signal handlers.
	signal_handlers: [SignalHandler; signal::SIGNALS_COUNT],

	/// The exit status of the process after exiting.
	exit_status: ExitStatus,
	/// The terminating signal.
	termsig: u8,
}

/// The PID manager.
static mut PID_MANAGER: MaybeUninit<Mutex<PIDManager>> = MaybeUninit::uninit();
/// The processes scheduler.
static mut SCHEDULER: MaybeUninit<SharedPtr<Scheduler>>
	= MaybeUninit::uninit();

/// Initializes processes system. This function must be called only once, at kernel initialization.
pub fn init() -> Result<(), Errno> {
	tss::init();
	tss::flush();

	let cores_count = 1; // TODO
	unsafe {
		PID_MANAGER.write(Mutex::new(PIDManager::new()?));
		SCHEDULER.write(Scheduler::new(cores_count)?);
	}

	let callback = | id: u32, _code: u32, regs: &Regs, ring: u32 | {
		if ring < 3 {
			return InterruptResult::new(true, InterruptResultAction::Panic);
		}

		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		let scheduler = guard.get_mut();

		if let Some(mut curr_proc) = scheduler.get_current_process() {
			let mut curr_proc_guard = curr_proc.lock(false);
			let curr_proc = curr_proc_guard.get_mut();

			match id {
				// Divide-by-zero
				// x87 Floating-Point Exception
				// SIMD Floating-Point Exception
				0x00 | 0x10 | 0x13 => {
					curr_proc.kill(Signal::new(signal::SIGFPE).unwrap());
					curr_proc.signal_next();
				},

				// Breakpoint
				0x03 => {
					curr_proc.kill(Signal::new(signal::SIGTRAP).unwrap());
					curr_proc.signal_next();
				},

				// Invalid Opcode
				0x06 => {
					curr_proc.kill(Signal::new(signal::SIGILL).unwrap());
					curr_proc.signal_next();
				},

				// General Protection Fault
				0x0d => {
					let vmem = curr_proc.get_mem_space_mut().unwrap().get_vmem();
					let mut inst_prefix = 0;
					vmem::switch(vmem.as_ref(), || {
						inst_prefix = unsafe {
							*(regs.eip as *const u8)
						};
					});

					if inst_prefix == HLT_INSTRUCTION {
						curr_proc.exit(regs.eax);
					} else {
						curr_proc.kill(Signal::new(signal::SIGSEGV).unwrap());
						curr_proc.signal_next();
					}
				},

				// Alignment Check
				0x11 => {
					curr_proc.kill(Signal::new(signal::SIGBUS).unwrap());
					curr_proc.signal_next();
				},

				_ => {},
			}

			if curr_proc.get_state() == State::Running {
				InterruptResult::new(false, InterruptResultAction::Resume)
			} else {
				InterruptResult::new(true, InterruptResultAction::Loop)
			}
		} else {
			InterruptResult::new(true, InterruptResultAction::Panic)
		}
	};
	let page_fault_callback = | _id: u32, code: u32, _regs: &Regs, ring: u32 | {
		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		let scheduler = guard.get_mut();

		if let Some(mut curr_proc) = scheduler.get_current_process() {
			let mut curr_proc_guard = curr_proc.lock(false);
			let curr_proc = curr_proc_guard.get_mut();

			let accessed_ptr = unsafe {
				vmem::x86::cr2_get()
			};

			if !curr_proc.get_mem_space_mut().unwrap().handle_page_fault(accessed_ptr, code) {
				if ring < 3 {
					return InterruptResult::new(true, InterruptResultAction::Panic);
				} else {
					curr_proc.kill(Signal::new(signal::SIGSEGV).unwrap());
					curr_proc.signal_next();
				}
			}

			if curr_proc.get_state() == State::Running {
				InterruptResult::new(false, InterruptResultAction::Resume)
			} else {
				InterruptResult::new(true, InterruptResultAction::Loop)
			}
		} else {
			InterruptResult::new(true, InterruptResultAction::Panic)
		}
	};

	let _ = ManuallyDrop::new(event::register_callback(0x00, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x03, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x06, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x0d, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x0e, u32::MAX, page_fault_callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x10, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x11, u32::MAX, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x13, u32::MAX, callback)?);

	Ok(())
}

/// Returns a mutable reference to the scheduler's Mutex.
pub fn get_scheduler() -> &'static mut Mutex<Scheduler> {
	unsafe { // Safe because using Mutex
		SCHEDULER.assume_init_mut()
	}
}

impl Process {
	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_pid(pid: Pid) -> Option<SharedPtr<Self>> {
		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		guard.get_mut().get_by_pid(pid)
	}

	/// Returns the current running process. If no process is running, the function returns None.
	pub fn get_current() -> Option<SharedPtr<Self>> {
		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		guard.get_mut().get_current_process()
	}

	/// Creates the init process and places it into the scheduler's queue. The process is set to
	/// state `Running` by default.
	/// `entry_point` is the pointer to the first instruction of the process.
	pub fn new_init(entry_point: *const c_void) -> Result<SharedPtr<Self>, Errno> {
		let mut mem_space = MemSpace::new()?;
		let user_stack = mem_space.map_stack(None, USER_STACK_SIZE, USER_STACK_FLAGS)?;
		let kernel_stack = mem_space.map_stack(None, KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;

		let mut process = Self {
			pid: pid::INIT_PID,
			pgid: pid::INIT_PID,

			uid: 0,
			gid: 0,

			umask: DEFAULT_UMASK,

			state: State::Running,
			priority: 0,
			quantum_count: 0,

			parent: None,
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

			handled_signal: None,
			saved_regs: Regs {
				ebp: 0x0,
				esp: 0x0,
				eip: 0x0,
				eflags: 0x0,
				eax: 0x0,
				ebx: 0x0,
				ecx: 0x0,
				edx: 0x0,
				esi: 0x0,
				edi: 0x0,
			},
			waitable: false,

			mem_space: Some(mem_space),

			user_stack,
			kernel_stack,

			cwd: Path::root(),
			file_descriptors: Vec::new(),

			signals_bitfield: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: [SignalHandler::Default; signal::SIGNALS_COUNT],

			exit_status: 0,
			termsig: 0,
		};

		// Creating STDIN, STDOUT and STDERR
		{
			let mutex = file::get_files_cache();
			let mut guard = mutex.lock(true);
			let files_cache = guard.get_mut();

			let tty_path = Path::from_string(TTY_DEVICE_PATH, false).unwrap();
			let tty_file = files_cache.get_file_from_path(&tty_path).unwrap();
			let stdin_fd = process.create_fd(file_descriptor::O_RDWR, FDTarget::File(tty_file))
				.unwrap();
			assert_eq!(stdin_fd.get_id(), STDIN_FILENO);

			process.duplicate_fd(STDIN_FILENO, Some(STDOUT_FILENO)).unwrap();
			process.duplicate_fd(STDIN_FILENO, Some(STDERR_FILENO)).unwrap();
		}

		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		guard.get_mut().add_process(process)
	}

	/// Tells whether the process is the init process.
	#[inline(always)]
	pub fn is_init(&self) -> bool {
		self.get_pid() == pid::INIT_PID
	}

	/// Returns the process's PID.
	#[inline(always)]
	pub fn get_pid(&self) -> Pid {
		self.pid
	}

	/// Returns the process's group ID.
	#[inline(always)]
	pub fn get_pgid(&self) -> Pid {
		self.pgid
	}

	/// Tells whether the process is among a group and is not its owner.
	#[inline(always)]
	pub fn is_in_group(&self) -> bool {
		self.pgid != 0 && self.pgid != self.pid
	}

	/// Sets the process's group ID to the given value `pgid`.
	pub fn set_pgid(&mut self, pgid: Pid) -> Result<(), Errno> {
		if self.is_in_group() {
			let mut mutex = Process::get_by_pid(self.pgid).unwrap();
			let mut guard = mutex.lock(false);
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
				let mut guard = mutex.lock(false);
				let new_group_process = guard.get_mut();
				let i = new_group_process.process_group.binary_search(&self.pid).unwrap_err();
				new_group_process.process_group.insert(i, self.pid)
			} else {
				Err(errno::ESRCH)
			}
		} else {
			Ok(())
		}
	}

	/// Returns a reference to the list of PIDs of processes in the current process's group.
	#[inline(always)]
	pub fn get_group_processes(&self) -> &Vec<Pid> {
		&self.process_group
	}

	/// Returns the parent process's PID.
	pub fn get_parent_pid(&self) -> Pid {
		if let Some(parent) = &self.parent {
			let parent = parent.get_mut().unwrap();
			let guard = parent.lock(false);
			guard.get().get_pid()
		} else {
			self.get_pid()
		}
	}

	/// Returns the process's user owner ID.
	#[inline(always)]
	pub fn get_uid(&self) -> Uid {
		self.uid
	}

	/// Sets the process's user owner ID.
	#[inline(always)]
	pub fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	/// Returns the process's group owner ID.
	#[inline(always)]
	pub fn get_gid(&self) -> Gid {
		self.gid
	}

	/// Sets the process's group owner ID.
	#[inline(always)]
	pub fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	/// Returns the file creation mask.
	#[inline(always)]
	pub fn get_umask(&self) -> u16 {
		self.umask
	}

	/// Sets the file creation mask.
	#[inline(always)]
	pub fn set_umask(&mut self, umask: u16) {
		self.umask = umask;
	}

	/// Returns the process's current state.
	#[inline(always)]
	pub fn get_state(&self) -> State {
		self.state
	}

	/// Sets the process's state to `new_state`.
	pub fn set_state(&mut self, new_state: State) {
		if self.state != State::Zombie {
			self.state = new_state;
		}

		if self.state == State::Zombie {
			if self.is_init() {
				kernel_panic!("Terminated init process!");
			}

			// TODO Attach every child to the init process

			// Removing the memory space to save memory
			// TODO Handle the case where the memory space is bound
			// TODO self.mem_space = None;

			self.waitable = true;
		}
	}

	/// Tells whether the current process has informations to be retrieved by the `waitpid` system
	/// call.
	#[inline(always)]
	pub fn is_waitable(&self) -> bool {
		self.waitable
	}

	/// Sets the process waitable with the given signal type `type_`.
	#[inline(always)]
	pub fn set_waitable(&mut self, type_: u8) {
		self.waitable = true;
		self.termsig = type_;
	}

	/// Clears the waitable flag.
	#[inline(always)]
	pub fn clear_waitable(&mut self) {
		self.waitable = false;
	}

	/// Wakes up the process. The function sends a signal SIGCHLD to the process and, if it was in
	/// Sleeping state, changes it to Running.
	pub fn wakeup(&mut self) {
		self.kill(signal::Signal::new(signal::SIGCHLD).unwrap());

		if self.state == State::Sleeping {
			self.state = State::Running;
		}
	}

	/// Returns the priority of the process. A greater number means a higher priority relative to
	/// other processes.
	#[inline(always)]
	pub fn get_priority(&self) -> usize {
		self.priority
	}

	/// Returns the process's parent if exists.
	#[inline(always)]
	pub fn get_parent(&self) -> Option<&WeakPtr<Process>> {
		self.parent.as_ref()
	}

	/// Returns a reference to the list of the process's children.
	#[inline(always)]
	pub fn get_children(&self) -> &Vec<Pid> {
		&self.children
	}

	/// Tells whether the process has a child with the given pid.
	#[inline(always)]
	pub fn has_child(&self, pid: Pid) -> bool {
		self.children.binary_search(&pid).is_ok()
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
		if let Ok(i) = self.children.binary_search(&pid) {
			self.children.remove(i);
		}
	}

	/// Returns a reference to the process's memory space.
	/// If the process is terminated, the function returns None.
	#[inline(always)]
	pub fn get_mem_space(&self) -> Option<&MemSpace> {
		self.mem_space.as_ref()
	}

	/// Returns a mutable reference to the process's memory space.
	/// If the process is terminated, the function returns None.
	#[inline(always)]
	pub fn get_mem_space_mut(&mut self) -> Option<&mut MemSpace> {
		self.mem_space.as_mut()
	}

	/// Returns a reference to the process's current working directory.
	#[inline(always)]
	pub fn get_cwd(&self) -> &Path {
		&self.cwd
	}

	/// Sets the process's current working directory.
	#[inline(always)]
	pub fn set_cwd(&mut self, path: Path) {
		self.cwd = path;
	}

	/// Returns the process's saved state registers.
	#[inline(always)]
	pub fn get_regs(&self) -> &Regs {
		&self.regs
	}

	/// Sets the process's saved state registers.
	#[inline(always)]
	pub fn set_regs(&mut self, regs: &Regs) {
		self.regs = *regs;
	}

	/// Tells whether the process was syscalling before being interrupted.
	#[inline(always)]
	pub fn is_syscalling(&self) -> bool {
		self.syscalling
	}

	/// Sets the process's syscalling state.
	#[inline(always)]
	pub fn set_syscalling(&mut self, syscalling: bool) {
		self.syscalling = syscalling;
	}

	/// Returns the available file descriptor with the lowest ID. If no ID is available, the
	/// function returns an error.
	fn get_available_fd(&mut self) -> Result<u32, Errno> {
		if self.file_descriptors.is_empty() {
			return Ok(0);
		}

		for (i, fd) in self.file_descriptors.iter().enumerate() {
			if (i as u32) < fd.get_id() {
				return Ok(i as u32);
			}
		}

		let id = self.file_descriptors.len();
		if id < limits::OPEN_MAX {
			Ok(id as u32)
		} else {
			Err(errno::EMFILE)
		}
	}

	/// Creates a file descriptor and returns a mutable reference to it.
	/// `flags` are the file descriptor's flags.
	/// `target` is the target of the newly created file descriptor.
	/// If the target is a file and cannot be open, the function returns an Err.
	pub fn create_fd(&mut self, flags: i32, target: FDTarget)
		-> Result<&mut FileDescriptor, Errno> {
		let id = self.get_available_fd()?;
		let fd = FileDescriptor::new(id, flags, target)?;
		let index = self.file_descriptors.binary_search_by(| fd | {
			fd.get_id().cmp(&id)
		}).unwrap_err();

		self.file_descriptors.insert(index, fd)?;
		Ok(&mut self.file_descriptors[index])
	}

	/// Duplicates the file descriptor with id `id`.
	/// `new_id` if specified, the id of the new file descriptor. If already used, the previous
	/// file descriptor shall be closed.
	pub fn duplicate_fd(&mut self, id: u32, new_id: Option<u32>)
		-> Result<&mut FileDescriptor, Errno> {
		let new_id = {
			if let Some(new_id) = new_id {
				new_id
			} else {
				self.get_available_fd()?
			}
		};

		let curr_fd = self.get_fd(id).ok_or(errno::EBADF)?;
		let new_fd = FileDescriptor::new(new_id, curr_fd.get_flags(),
			curr_fd.get_target().clone())?;

		let index = self.file_descriptors.binary_search_by(| fd | {
			fd.get_id().cmp(&new_id)
		});
		let index = {
			if let Ok(i) = index {
				self.file_descriptors[i] = new_fd;
				i
			} else {
				let i = index.unwrap_err();
				self.file_descriptors.insert(i, new_fd)?;
				i
			}
		};

		Ok(&mut self.file_descriptors[index])
	}

	/// Returns the file descriptor with ID `id`. If the file descriptor doesn't exist, the
	/// function returns None.
	pub fn get_fd(&mut self, id: u32) -> Option<&mut FileDescriptor> {
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

	/// Returns the exit status if the process has ended.
	#[inline(always)]
	pub fn get_exit_status(&self) -> Option<ExitStatus> {
		if self.state == State::Zombie {
			Some(self.exit_status)
		} else {
			None
		}
	}

	/// Returns the signal that killed the process.
	#[inline(always)]
	pub fn get_termsig(&self) -> u8 {
		self.termsig
	}

	/// Forks the current process. The internal state of the process (registers and memory) are
	/// copied.
	/// `parent` is the parent of the new process.
	/// On fail, the function returns an Err with the appropriate Errno.
	/// If the process is not running, the behaviour is undefined.
	pub fn fork(&mut self, parent: WeakPtr<Self>) -> Result<SharedPtr<Self>, Errno> {
		debug_assert_eq!(self.get_state(), State::Running);

		let pid = {
			let mutex = unsafe {
				PID_MANAGER.assume_init_mut()
			};
			let mut guard = mutex.lock(false);
			guard.get_mut().get_unique_pid()
		}?;

		let mut regs = self.regs;
		regs.eax = 0;

		let process = Self {
			pid,
			pgid: self.pgid,

			uid: self.uid,
			gid: self.gid,

			umask: self.umask,

			state: State::Running,
			priority: self.priority,
			quantum_count: 0,

			parent: Some(parent),
			children: Vec::new(),
			process_group: Vec::new(),

			regs,
			syscalling: false,

			handled_signal: self.handled_signal,
			saved_regs: self.saved_regs,
			waitable: false,

			mem_space: Some(self.get_mem_space_mut().unwrap().fork()?),

			user_stack: self.user_stack,
			kernel_stack: self.kernel_stack,

			cwd: self.cwd.failable_clone()?,
			file_descriptors: self.file_descriptors.failable_clone()?,

			signals_bitfield: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: self.signal_handlers,

			exit_status: self.exit_status,
			termsig: 0,
		};
		self.add_child(pid)?;

		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock(false);
		guard.get_mut().add_process(process)
	}

	/// Returns the signal handler for the signal type `type_`.
	#[inline(always)]
	pub fn get_signal_handler(&self, type_: SignalType) -> SignalHandler {
		debug_assert!((type_ as usize) < self.signal_handlers.len());
		self.signal_handlers[type_ as usize]
	}

	/// Sets the signal handler `handler` for the signal type `type_`.
	#[inline(always)]
	pub fn set_signal_handler(&mut self, type_: SignalType, handler: SignalHandler) {
		debug_assert!((type_ as usize) < self.signal_handlers.len());
		self.signal_handlers[type_ as usize] = handler;
	}

	/// Tells whether the process is handling a signal.
	#[inline(always)]
	pub fn is_handling_signal(&self) -> bool {
		self.handled_signal.is_some()
	}

	/// Kills the process with the given signal `sig`. If the process doesn't have a signal
	/// handler, the default action for the signal is executed.
	/// `no_handle` tells whether the signal handler must be ignored.
	pub fn kill(&mut self, sig: Signal) {
		if self.get_state() == State::Stopped && sig.is_continuation() {
			self.set_state(State::Running);
		}

		if sig.can_catch() {
			self.signals_bitfield.set(sig.get_type() as _);
		} else {
			sig.execute_action(self);
		}
	}

	/// Tells whether the process has a signal pending.
	#[inline(always)]
	pub fn has_signal_pending(&self) -> bool {
		self.signals_bitfield.find_set().is_some()
	}

	/// Makes the process handle the next signal. If the process is already handling a signal or if
	/// not signal is queued, the function does nothing.
	pub fn signal_next(&mut self) {
		if self.is_handling_signal() {
			return;
		}

		if let Some(signum) = self.signals_bitfield.find_set() {
			let sig = Signal::new(signum as _).unwrap();
			sig.execute_action(self);
		}
	}

	/// Saves the process's state to handle a signal.
	/// `sig` is the signal number.
	/// If the process is already handling a signal, the behaviour is undefined.
	pub fn signal_save(&mut self, sig: SignalType) {
		debug_assert!(!self.is_handling_signal());

		self.saved_regs = self.regs;
		self.handled_signal = Some(sig);
	}

	/// Restores the process's state after handling a signal.
	pub fn signal_restore(&mut self) {
		if let Some(sig) = self.handled_signal {
			self.signals_bitfield.clear(sig as _);

			self.handled_signal = None;
			self.regs = self.saved_regs;

			// Ensuring the process doesn't resume executing userspace code in kernelspace
			self.syscalling = false;
		}
	}

	/// Exits the process with the given `status`. This function changes the process's status to
	/// `Zombie`.
	pub fn exit(&mut self, status: u32) {
		self.exit_status = (status & 0xff) as ExitStatus;
		self.set_state(State::Zombie);

		if let Some(parent) = self.get_parent() {
			let parent = parent.get_mut().unwrap();
			let mut guard = parent.lock(false);
			guard.get_mut().wakeup();
		}
	}

	/// Returns the number of physical memory pages used by the process.
	pub fn get_memory_usage(&self) -> u32 {
		// TODO
		todo!();
	}

	/// Returns the OOM score, used by the OOM killer to determine the process to kill in case the
	/// system runs out of memory. A higher score means a higher probability of getting killed.
	pub fn get_oom_score(&self) -> u16 {
		let mut score = 0;

		// If the process is owned by the superuser, give it a bonus
		if self.uid == 0 {
			score -= 100;
		}

		// TODO Compute the score using physical memory usage
		// TODO Take into account userspace-set values (oom may be disabled for this process,
		// an absolute score or a bonus might be given, etc...)

		score
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		debug_assert!(!self.is_init());
		// When terminated, a process gives all its children to the init process
		debug_assert!(self.get_children().is_empty());

		// Checking the process is a zombie
		debug_assert_eq!(self.get_state(), State::Zombie);
		debug_assert!(self.mem_space.is_none());

		if let Some(parent) = self.get_parent() {
			let parent = parent.get_mut().unwrap();
			let mut guard = parent.lock(false);
			guard.get_mut().remove_child(self.pid);
		}

		let mutex = unsafe {
			PID_MANAGER.assume_init_mut()
		};
		let mut guard = mutex.lock(false);
		guard.get_mut().release_pid(self.pid);
	}
}
