//! A process is a task running on the kernel. A multitasking system allows several processes to
//! run at the same time by sharing the CPU resources using a scheduler.

// TODO Maintain the open file descriptors count

pub mod exec;
pub mod iovec;
pub mod mem_space;
pub mod oom;
pub mod pid;
pub mod regs;
pub mod rusage;
pub mod scheduler;
pub mod semaphore;
pub mod signal;
pub mod tss;
pub mod user_desc;

use core::cmp::max;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ptr::NonNull;
use crate::cpu;
use crate::errno::Errno;
use crate::errno;
use crate::event::{InterruptResult, InterruptResultAction};
use crate::event;
use crate::file::Gid;
use crate::file::ROOT_UID;
use crate::file::Uid;
use crate::file::fcache;
use crate::file::fd::FD_CLOEXEC;
use crate::file::fd::FileDescriptor;
use crate::file::fd::NewFDConstraint;
use crate::file::open_file::FDTarget;
use crate::file::open_file::OpenFile;
use crate::file::open_file;
use crate::file::path::Path;
use crate::file;
use crate::gdt::ldt::LDT;
use crate::gdt;
use crate::limits;
use crate::process::open_file::O_CLOEXEC;
use crate::tty::TTYHandle;
use crate::tty;
use crate::util::FailableClone;
use crate::util::container::bitfield::Bitfield;
use crate::util::container::vec::Vec;
use crate::util::lock::*;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::IntWeakPtr;
use crate::util::ptr::SharedPtr;
use crate::vec;
use mem_space::MemSpace;
use pid::PIDManager;
use pid::Pid;
use regs::Regs;
use rusage::RUsage;
use scheduler::Scheduler;
use signal::Signal;
use signal::SignalAction;
use signal::SignalHandler;
use signal::SignalType;

/// The opcode of the `hlt` instruction.
const HLT_INSTRUCTION: u8 = 0xf4;

/// The path to the TTY device file.
const TTY_DEVICE_PATH: &str = "/dev/tty";

/// The default file creation mask.
const DEFAULT_UMASK: file::Mode = 0o022;

/// The size of the userspace stack of a process in number of pages.
const USER_STACK_SIZE: usize = 2048;
/// The flags for the userspace stack mapping.
const USER_STACK_FLAGS: u8 = mem_space::MAPPING_FLAG_WRITE | mem_space::MAPPING_FLAG_USER;
/// The size of the kernelspace stack of a process in number of pages.
const KERNEL_STACK_SIZE: usize = 64;
/// The flags for the kernelspace stack mapping.
const KERNEL_STACK_FLAGS: u8 = mem_space::MAPPING_FLAG_WRITE | mem_space::MAPPING_FLAG_NOLAZY;

/// The file descriptor number of the standard input stream.
const STDIN_FILENO: u32 = 0;
/// The file descriptor number of the standard output stream.
const STDOUT_FILENO: u32 = 1;
/// The file descriptor number of the standard error stream.
const STDERR_FILENO: u32 = 2;

/// The number of TLS entries per process.
pub const TLS_ENTRIES_COUNT: usize = 3;

/// An enumeration containing possible states for a process.
#[derive(Clone, Copy, Debug, PartialEq)]
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

/// Structure representing options to be passed to the fork function.
pub struct ForkOptions {
	/// If true, the parent and child processes both share the same address space.
	pub share_memory: bool,
	/// If true, the parent and child processes both share the same file descriptors table.
	pub share_fd: bool,
	/// If true, the parent and child processes both share the same signal handlers table.
	pub share_sighand: bool,

	/// If true, the parent is stopped until the child process exits or executes a program.
	pub vfork: bool,
}

impl Default for ForkOptions {
	fn default() -> Self {
		Self {
			share_memory: false,
			share_fd: false,
			share_sighand: false,

			vfork: false,
		}
	}
}

/// The vfork operation is similar to the fork operation except the parent process isn't executed
/// until the child process exits or executes a program.
/// The reason for this is to prevent useless copies of memory pages when the child process is
/// created only to execute a program.
/// It implies that the child process shares the same memory space as the parent.
#[derive(Clone, Copy, Debug, PartialEq)]
enum VForkState {
	/// The process is not in vfork state.
	None,

	/// The process is the parent waiting for the child to terminate.
	Waiting,
	/// The process is the child the parent waits for.
	Executing,
}

/// The Process Control Block (PCB). This structure stores all the informations about a process.
pub struct Process {
	/// The ID of the process.
	pid: Pid,
	/// The ID of the process group.
	pgid: Pid,
	/// The thread ID of the process.
	tid: Pid,

	/// The process's current TTY.
	tty: TTYHandle,

	/// The real ID of the process's user owner.
	uid: Uid,
	/// The real ID of the process's group owner.
	gid: Gid,

	/// The effective ID of the process's user owner.
	euid: Uid,
	/// The effective ID of the process's group owner.
	egid: Gid,

	/// The process's current umask.
	umask: file::Mode,

	/// The current state of the process.
	state: State,
	/// The current vfork state of the process (see documentation of `VForkState`).
	vfork_state: VForkState,

	/// The priority of the process.
	priority: usize,
	/// The number of quantum run during the cycle.
	quantum_count: usize,

	/// A pointer to the parent process.
	parent: Option<IntWeakPtr<Process>>,
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
	mem_space: Option<IntSharedPtr<MemSpace>>,
	/// A pointer to the userspace stack.
	user_stack: Option<*const c_void>,
	/// A pointer to the kernelspace stack.
	kernel_stack: Option<*const c_void>,

	/// The current working directory.
	cwd: Path,
	/// The list of open file descriptors with their respective ID.
	file_descriptors: Option<SharedPtr<Vec<FileDescriptor>>>,

	/// A bitfield storing the set of blocked signals.
	sigmask: Bitfield,
	/// A bitfield storing the set of pending signals.
	sigpending: Bitfield,
	/// The list of signal handlers.
	signal_handlers: SharedPtr<[SignalHandler; signal::SIGNALS_COUNT]>,

	/// TLS entries.
	tls_entries: [gdt::Entry; TLS_ENTRIES_COUNT],
	/// The process's local descriptor table.
	ldt: Option<LDT>,

	/// TODO doc
	set_child_tid: Option<NonNull<i32>>,
	/// TODO doc
	clear_child_tid: Option<NonNull<i32>>,

	/// The process's resources usage.
	rusage: RUsage,

	/// The exit status of the process after exiting.
	exit_status: ExitStatus,
	/// The terminating signal.
	termsig: u8,
}

/// The PID manager.
static mut PID_MANAGER: MaybeUninit<Mutex<PIDManager>> = MaybeUninit::uninit();
/// The processes scheduler.
static mut SCHEDULER: MaybeUninit<IntSharedPtr<Scheduler>> = MaybeUninit::uninit();

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
		}.lock();
		let scheduler = guard.get_mut();

		if let Some(curr_proc) = scheduler.get_current_process() {
			let mut curr_proc_guard = curr_proc.lock();
			let curr_proc = curr_proc_guard.get_mut();

			match id {
				// Divide-by-zero
				// x87 Floating-Point Exception
				// SIMD Floating-Point Exception
				0x00 | 0x10 | 0x13 => {
					curr_proc.kill(Signal::new(signal::SIGFPE).unwrap(), true);
					curr_proc.signal_next();
				},

				// Breakpoint
				0x03 => {
					curr_proc.kill(Signal::new(signal::SIGTRAP).unwrap(), true);
					curr_proc.signal_next();
				},

				// Invalid Opcode
				0x06 => {
					curr_proc.kill(Signal::new(signal::SIGILL).unwrap(), true);
					curr_proc.signal_next();
				},

				// General Protection Fault
				0x0d => {
					let inst_prefix = unsafe {
						*(regs.eip as *const u8)
					};

					if inst_prefix == HLT_INSTRUCTION {
						curr_proc.exit(regs.eax);
					} else {
						curr_proc.kill(Signal::new(signal::SIGSEGV).unwrap(), true);
						curr_proc.signal_next();
					}
				},

				// Alignment Check
				0x11 => {
					curr_proc.kill(Signal::new(signal::SIGBUS).unwrap(), true);
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
		}.lock();
		let scheduler = guard.get_mut();

		if let Some(curr_proc) = scheduler.get_current_process() {
			let mut curr_proc_guard = curr_proc.lock();
			let curr_proc = curr_proc_guard.get_mut();

			let accessed_ptr = unsafe {
				cpu::cr2_get()
			};

			// Handling page fault
			let success = {
				let mem_space = curr_proc.get_mem_space().unwrap();
				let mut mem_space_guard = mem_space.lock();
				let mem_space = mem_space_guard.get_mut();

				mem_space.handle_page_fault(accessed_ptr, code)
			};

			if !success {
				if ring < 3 {
					return InterruptResult::new(true, InterruptResultAction::Panic);
				} else {
					curr_proc.kill(Signal::new(signal::SIGSEGV).unwrap(), true);
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
pub fn get_scheduler() -> &'static mut IntMutex<Scheduler> {
	unsafe { // Safe because using Mutex
		SCHEDULER.assume_init_mut()
	}
}

impl Process {
	/// Returns the process with PID `pid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_pid(pid: Pid) -> Option<IntSharedPtr<Self>> {
		let guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock();

		guard.get().get_by_pid(pid)
	}

	/// Returns the process with TID `tid`. If the process doesn't exist, the function returns
	/// None.
	pub fn get_by_tid(tid: Pid) -> Option<IntSharedPtr<Self>> {
		let guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock();

		guard.get().get_by_tid(tid)
	}

	/// Returns the current running process. If no process is running, the function returns None.
	pub fn get_current() -> Option<IntSharedPtr<Self>> {
		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock();

		guard.get_mut().get_current_process()
	}

	/// Creates the init process and places it into the scheduler's queue. The process is set to
	/// state `Running` by default.
	/// The created process has user root.
	pub fn new() -> Result<IntSharedPtr<Self>, Errno> {
		// TODO Prevent calling twice

		let mut process = Self {
			pid: pid::INIT_PID,
			pgid: pid::INIT_PID,
			tid: pid::INIT_PID,

			tty: tty::get(None).unwrap(), // Initialization with the init TTY

			uid: 0,
			gid: 0,

			euid: 0,
			egid: 0,

			umask: DEFAULT_UMASK,

			state: State::Running,
			vfork_state: VForkState::None,

			priority: 0,
			quantum_count: 0,

			parent: None,
			children: Vec::new(),
			process_group: Vec::new(),

			regs: Regs::default(),
			syscalling: false,

			handled_signal: None,
			saved_regs: Regs::default(),
			waitable: false,

			mem_space: None,
			user_stack: None,
			kernel_stack: None,

			cwd: Path::root(),
			file_descriptors: Some(SharedPtr::new(Vec::new())?),

			sigmask: Bitfield::new(signal::SIGNALS_COUNT)?,
			sigpending: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: SharedPtr::new([SignalHandler::Default; signal::SIGNALS_COUNT])?,

			tls_entries: [gdt::Entry::default(); TLS_ENTRIES_COUNT],
			ldt: None,

			set_child_tid: None,
			clear_child_tid: None,

			rusage: RUsage::default(),

			exit_status: 0,
			termsig: 0,
		};

		// Creating STDIN, STDOUT and STDERR
		{
			let mutex = fcache::get();
			let mut guard = mutex.lock();
			let files_cache = guard.get_mut();

			let tty_path = Path::from_str(TTY_DEVICE_PATH.as_bytes(), false).unwrap();
			let tty_file = files_cache.as_mut().unwrap()
				.get_file_from_path(&tty_path, process.uid, process.gid, true)?;
			let stdin_fd = process.create_fd(open_file::O_RDWR, FDTarget::File(tty_file))?;
			assert_eq!(stdin_fd.get_id(), STDIN_FILENO);

			process.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDOUT_FILENO), false)?;
			process.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDERR_FILENO), false)?;
		}

		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock();
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

	/// Returns the process's thread ID.
	#[inline(always)]
	pub fn get_tid(&self) -> Pid {
		self.tid
	}

	/// Tells whether the process is among a group and is not its owner.
	#[inline(always)]
	pub fn is_in_group(&self) -> bool {
		self.pgid != 0 && self.pgid != self.pid
	}

	/// Sets the process's group ID to the given value `pgid`.
	pub fn set_pgid(&mut self, pgid: Pid) -> Result<(), Errno> {
		// Removing the process from its old group
		if self.is_in_group() {
			let mutex = Process::get_by_pid(self.pgid).unwrap();
			let mut guard = mutex.lock();
			let old_group_process = guard.get_mut();

			if let Ok(i) = old_group_process.process_group.binary_search(&self.pid) {
				old_group_process.process_group.remove(i);
			}
		}

		self.pgid = if pgid == 0 {
			self.pid
		} else {
			pgid
		};

		if pgid == self.pid {
			return Ok(());
		}

		// Adding the process to the new group
		if let Some(mutex) = Process::get_by_pid(pgid) {
			let mut guard = mutex.lock();
			let new_group_process = guard.get_mut();

			let i = new_group_process.process_group.binary_search(&self.pid).unwrap_err();
			new_group_process.process_group.insert(i, self.pid)
		} else {
			Err(errno!(ESRCH))
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
			let guard = parent.lock();
			guard.get().get_pid()
		} else {
			self.get_pid()
		}
	}

	/// Returns the TTY associated with the process.
	pub fn get_tty(&self) -> TTYHandle {
		self.tty.clone()
	}

	/// Returns the process's real user owner ID.
	#[inline(always)]
	pub fn get_uid(&self) -> Uid {
		self.uid
	}

	/// Sets the process's real user owner ID.
	#[inline(always)]
	pub fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	/// Returns the process's real group owner ID.
	#[inline(always)]
	pub fn get_gid(&self) -> Gid {
		self.gid
	}

	/// Sets the process's real group owner ID.
	#[inline(always)]
	pub fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	/// Returns the process's effective user owner ID.
	#[inline(always)]
	pub fn get_euid(&self) -> Uid {
		self.euid
	}

	/// Returns the process's effective group owner ID.
	#[inline(always)]
	pub fn get_egid(&self) -> Gid {
		self.egid
	}

	/// Returns the file creation mask.
	#[inline(always)]
	pub fn get_umask(&self) -> file::Mode {
		self.umask
	}

	/// Sets the file creation mask.
	#[inline(always)]
	pub fn set_umask(&mut self, umask: file::Mode) {
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

	/// Tells whether the scheduler can run the process.
	pub fn can_run(&self) -> bool {
		self.get_state() == State::Running
			&& self.vfork_state != VForkState::Waiting
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
		self.kill(signal::Signal::new(signal::SIGCHLD).unwrap(), false);

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
	pub fn get_parent(&self) -> Option<&IntWeakPtr<Process>> {
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
	pub fn get_mem_space(&self) -> Option<IntSharedPtr<MemSpace>> {
		self.mem_space.clone()
	}

	/// Sets the new memory space for the process, dropping the previous if any.
	#[inline(always)]
	pub fn set_mem_space(&mut self, mem_space: Option<IntSharedPtr<MemSpace>>) {
		// TODO Handle multicore
		// If the process is currently running, switch the memory space
		if self.state == State::Running {
			if let Some(mem_space) = &mem_space {
				mem_space.lock().get().bind();
			} else {
				kernel_panic!("Dropping the memory space of a running process!");
			}
		}

		self.mem_space = mem_space;
	}

	/// Returns a reference to the process's current working directory.
	#[inline(always)]
	pub fn get_cwd(&self) -> &Path {
		&self.cwd
	}

	/// Sets the process's current working directory.
	/// If the given path is relative, it is made absolute by concatenated with `/`.
	#[inline(always)]
	pub fn set_cwd(&mut self, path: Path) -> Result<(), Errno> {
		if !path.is_absolute() {
			self.cwd = Path::root().concat(&path)?;
			self.cwd.reduce()
		} else {
			self.cwd = path;
			Ok(())
		}
	}

	/// Returns the process's saved state registers.
	#[inline(always)]
	pub fn get_regs(&self) -> &Regs {
		&self.regs
	}

	/// Sets the process's saved state registers.
	#[inline(always)]
	pub fn set_regs(&mut self, regs: Regs) {
		self.regs = regs;
	}

	/// Updates the TSS on the current core for the process.
	pub fn update_tss(&self) {
		// Filling the TSS
		let tss = tss::get();
		tss.ss0 = gdt::KERNEL_DS as _;
		tss.ss = gdt::USER_DS as _;
		// Setting the kernel stack pointer
		tss.esp0 = self.kernel_stack.unwrap() as _;
	}

	/// Prepares for context switching to the process.
	/// A call to this function MUST be followed by a context switch to the process.
	pub fn prepare_switch(&mut self) {
		debug_assert_eq!(self.get_state(), State::Running);

		// Incrementing the number of ticks the process had
		self.quantum_count += 1;

		// Updates teh TSS for the process
		self.update_tss();

		// Binding the memory space
		self.get_mem_space().unwrap().lock().get().bind();

		// Updating TLS entries in the GDT
		for i in 0..TLS_ENTRIES_COUNT {
			self.update_tls(i);
		}

		// Updating LDT if present
		if let Some(ldt) = &self.ldt {
			ldt.load();
		}

		// If a signal is pending on the process, execute it
		self.signal_next();
	}

	/// Initializes the process to run without a program.
	/// `pc` is the initial program counter.
	pub fn init_dummy(&mut self, pc: *const c_void) -> Result<(), Errno> {
		// Creating the memory space and the stacks
		let mut mem_space = MemSpace::new()?;
		let kernel_stack = mem_space.map_stack(KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;
		let user_stack = mem_space.map_stack(USER_STACK_SIZE, USER_STACK_FLAGS)?;

		self.mem_space = Some(IntSharedPtr::new(mem_space)?);
		self.kernel_stack = Some(kernel_stack);
		self.user_stack = Some(user_stack);

		// Setting the registers' initial state
		let regs = Regs {
			esp: user_stack as _,
			eip: pc as _,
			..Default::default()
		};
		self.regs = regs;

		Ok(())
	}

	/// Tells whether the process was syscalling before being interrupted.
	#[inline(always)]
	pub fn is_syscalling(&self) -> bool {
		self.syscalling && !self.is_handling_signal()
	}

	/// Sets the process's syscalling state.
	#[inline(always)]
	pub fn set_syscalling(&mut self, syscalling: bool) {
		self.syscalling = syscalling;
	}

	/// Returns the available file descriptor with the lowest ID. If no ID is available, the
	/// function returns an error.
	/// `file_descriptors` is the file descriptors table.
	/// `min` is the minimum value for the file descriptor to be returned.
	fn get_available_fd(file_descriptors: &Vec<FileDescriptor>, min: Option<u32>)
		-> Result<u32, Errno> {
		if file_descriptors.len() >= limits::OPEN_MAX {
			return Err(errno!(EMFILE));
		}

		if file_descriptors.is_empty() {
			return Ok(0);
		}

		// TODO Use binary search?
		for (i, fd) in file_descriptors.iter().enumerate() {
			if let Some(min) = min {
				if fd.get_id() < min {
					continue;
				}
			}

			if (i as u32) < fd.get_id() {
				return Ok(i as u32);
			}
		}

		let id = if let Some(min) = min {
			max(min, file_descriptors.len() as u32)
		} else {
			file_descriptors.len() as u32
		};
		Ok(id)
	}

	/// Creates a file descriptor and returns a pointer to it with its ID.
	/// `flags` are the file descriptor's flags.
	/// `target` is the target of the newly created file descriptor.
	/// If the target is a file and cannot be open, the function returns an Err.
	pub fn create_fd(&mut self, flags: i32, target: FDTarget) -> Result<FileDescriptor, Errno> {
		let mut file_descriptors_guard = self.file_descriptors.as_ref().unwrap().lock();
		let file_descriptors = file_descriptors_guard.get_mut();

		let id = Self::get_available_fd(file_descriptors, None)?;
		let open_file = OpenFile::new(flags, target)?;
		let i = file_descriptors.binary_search_by(| fd | fd.get_id().cmp(&id)).unwrap_err();

		// Flags for the fd
		let flags = if flags & O_CLOEXEC != 0 {
			FD_CLOEXEC
		} else {
			0
		};

		file_descriptors.insert(i, FileDescriptor::new(id, flags, SharedPtr::new(open_file)?))?;
		Ok(file_descriptors[i].clone())
	}

	/// Duplicates the file descriptor with id `id`.
	/// The new file descriptor ID follows the constraint given be `constraint`.
	/// `cloexec` tells whether the new file descriptor has the O_CLOEXEC flag enabled.
	/// The function returns a pointer to the file descriptor with its ID.
	pub fn duplicate_fd(&mut self, id: u32, constraint: NewFDConstraint, cloexec: bool)
		-> Result<FileDescriptor, Errno> {
		let mut file_descriptors_guard = self.file_descriptors.as_ref().unwrap().lock();
		let file_descriptors = file_descriptors_guard.get_mut();

		// The ID of the new FD
		let new_id = match constraint {
			NewFDConstraint::None => Self::get_available_fd(file_descriptors, None)?,
			NewFDConstraint::Fixed(id) => id,
			NewFDConstraint::Min(min) => Self::get_available_fd(file_descriptors, Some(min))?,
		};

		// The flags of the new FD
		let flags = if cloexec {
			FD_CLOEXEC
		} else {
			0
		};

		// The open file for the new FD
		let open_file = Self::get_fd_(file_descriptors, id).ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		// Creating the FD
		let fd = FileDescriptor::new(new_id, flags, open_file);

		// Inserting the FD
		let index = file_descriptors.binary_search_by(| fd | fd.get_id().cmp(&new_id));
		let index = {
			if let Ok(i) = index {
				file_descriptors[i] = fd;
				i
			} else {
				let i = index.unwrap_err();
				file_descriptors.insert(i, fd)?;
				i
			}
		};

		Ok(file_descriptors[index].clone())
	}

	/// Duplicates file descriptors to make the process have its own copy. This function doesn't
	/// duplicate open file descriptions.
	/// This function is meant to be executed on program execution, meaning that file descriptors
	/// with the flag FD_CLOEXEC are discarded.
	pub fn duplicate_fds(&mut self) -> Result<(), Errno> {
		let mut new_fds = vec![];

		{
			let fds_guard = self.file_descriptors.as_ref().unwrap().lock();
			let fds = fds_guard.get();

			for fd in fds {
				if fd.get_flags() & FD_CLOEXEC == 0 {
					new_fds.push(fd.clone())?;
				}
			}
		}

		self.file_descriptors = Some(SharedPtr::new(new_fds)?);
		Ok(())
	}

	/// Returns the file descriptor with ID `id`.
	/// `file_descriptors` is the file descriptors table.
	/// If the file descriptor doesn't exist, the function returns None.
	fn get_fd_(file_descriptors: &Vec<FileDescriptor>, id: u32) -> Option<FileDescriptor> {
		let result = file_descriptors.binary_search_by(| fd | fd.get_id().cmp(&id));
		result.ok().map(| index | file_descriptors[index].clone())
	}

	/// Returns the file descriptor with ID `id`.
	/// If the file descriptor doesn't exist, the function returns None.
	pub fn get_fd(&self, id: u32) -> Option<FileDescriptor> {
		let file_descriptors_guard = self.file_descriptors.as_ref().unwrap().lock();
		let file_descriptors = file_descriptors_guard.get();

		Self::get_fd_(file_descriptors, id)
	}

	/// Closes the file descriptor with the ID `id`. The function returns an Err if the file
	/// descriptor doesn't exist.
	pub fn close_fd(&mut self, id: u32) -> Result<(), Errno> {
		let mut file_descriptors_guard = self.file_descriptors.as_ref().unwrap().lock();
		let file_descriptors = file_descriptors_guard.get_mut();

		let result = file_descriptors.binary_search_by(| fd | fd.get_id().cmp(&id));

		if let Ok(index) = result {
			file_descriptors.remove(index);
			Ok(())
		} else {
			Err(errno!(EBADF))
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
	/// `fork_options` are the options for the fork operation.
	/// On fail, the function returns an Err with the appropriate Errno.
	/// If the process is not running, the behaviour is undefined.
	pub fn fork(&mut self, parent: IntWeakPtr<Self>, fork_options: ForkOptions)
		-> Result<IntSharedPtr<Self>, Errno> {
		debug_assert_eq!(self.get_state(), State::Running);

		// FIXME PID is leaked if the following code fails
		let pid = {
			let mutex = unsafe {
				PID_MANAGER.assume_init_mut()
			};
			let mut guard = mutex.lock();
			guard.get_mut().get_unique_pid()
		}?;

		// Handling vfork
		let vfork_state = if fork_options.vfork {
			self.vfork_state = VForkState::Waiting; // TODO Cancel if the following code fails
			VForkState::Executing
		} else {
			VForkState::None
		};

		// Cloning memory space
		let (mem_space, kernel_stack) = {
			let curr_mem_space = self.get_mem_space().unwrap();

			if fork_options.share_memory || fork_options.vfork {
				// Allocating a kernel stack for the new process
				let new_kernel_stack = curr_mem_space.lock().get_mut()
					.map_stack(KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;

				(curr_mem_space.clone(), Some(new_kernel_stack as _))
			} else {
				(IntSharedPtr::new(curr_mem_space.lock().get_mut().fork()?)?, self.kernel_stack)
			}
		};

		// Cloning file descriptors table
		let file_descriptors = if fork_options.share_fd {
			self.file_descriptors.clone()
		} else {
			let curr_fds_guard = self.file_descriptors.as_ref().unwrap().lock();
			let curr_fds = curr_fds_guard.get();

			let mut fds = Vec::with_capacity(curr_fds.len())?;
			for fd in curr_fds.iter() {
				fds.push(fd.clone())?;
			}

			Some(SharedPtr::new(fds)?)
		};

		// Cloning signal handlers
		let signal_handlers = if fork_options.share_sighand {
			self.signal_handlers.clone()
		} else {
			SharedPtr::new(self.signal_handlers.lock().get().clone())?
		};

		let process = Self {
			pid,
			pgid: self.pgid,
			tid: self.pid,

			tty: self.tty.clone(),

			uid: self.uid,
			gid: self.gid,

			euid: self.euid,
			egid: self.egid,

			umask: self.umask,

			state: State::Running,
			vfork_state,

			priority: self.priority,
			quantum_count: 0,

			parent: Some(parent),
			children: Vec::new(),
			process_group: Vec::new(),

			regs: self.regs,
			syscalling: false,

			handled_signal: self.handled_signal,
			saved_regs: self.saved_regs,
			waitable: false,

			mem_space: Some(mem_space),

			user_stack: self.user_stack,
			kernel_stack,

			cwd: self.cwd.failable_clone()?,
			file_descriptors: file_descriptors,

			sigmask: self.sigmask.failable_clone()?,
			sigpending: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: signal_handlers,

			tls_entries: self.tls_entries,
			ldt: {
				if let Some(ldt) = &self.ldt {
					Some(ldt.failable_clone()?)
				} else {
					None
				}
			},

			set_child_tid: self.set_child_tid,
			clear_child_tid: self.clear_child_tid,

			rusage: RUsage::default(),

			exit_status: self.exit_status,
			termsig: 0,
		};
		self.add_child(pid)?;

		let mut guard = unsafe {
			SCHEDULER.assume_init_mut()
		}.lock();
		guard.get_mut().add_process(process)
	}

	/// Returns the signal handler for the signal type `type_`.
	#[inline(always)]
	pub fn get_signal_handler(&self, type_: SignalType) -> SignalHandler {
		debug_assert!((type_ as usize) < signal::SIGNALS_COUNT);
		self.signal_handlers.lock().get()[type_ as usize]
	}

	/// Sets the signal handler `handler` for the signal type `type_`.
	#[inline(always)]
	pub fn set_signal_handler(&mut self, type_: SignalType, handler: SignalHandler) {
		debug_assert!((type_ as usize) < signal::SIGNALS_COUNT);
		self.signal_handlers.lock().get_mut()[type_ as usize] = handler;
	}

	/// Tells whether the process is handling a signal.
	#[inline(always)]
	pub fn is_handling_signal(&self) -> bool {
		self.handled_signal.is_some()
	}

	/// Kills the process with the given signal `sig`. If the process doesn't have a signal
	/// handler, the default action for the signal is executed.
	/// If `no_handler` is true and if the process is already handling a signal, the function
	/// executes the default action of the signal regardless the user-specified action.
	pub fn kill(&mut self, sig: Signal, no_handler: bool) {
		if self.get_state() == State::Stopped
			&& sig.get_default_action() == SignalAction::Continue {
			self.set_state(State::Running);
		}

		let no_handler = self.is_handling_signal() && no_handler;
		if !sig.can_catch() || no_handler {
			sig.execute_action(self, no_handler);
		} else {
			self.sigpending.set(sig.get_type() as _);
		}
	}

	/// Kills every processes in the process group.
	/// Arguments are the same as `kill`.
	pub fn kill_group(&mut self, sig: Signal, no_handler: bool) {
		for pid in self.process_group.iter() {
			if *pid != self.pid {
				if let Some(proc_mutex) = Process::get_by_tid(*pid) {
					let mut proc_guard = proc_mutex.lock();
					let proc = proc_guard.get_mut();

					proc.kill(sig.clone(), no_handler);
				}
			}
		}

		self.kill(sig, no_handler);
	}

	/// Returns an immutable reference to the process's blocked signals mask.
	#[inline(always)]
	pub fn get_sigmask(&self) -> &[u8] {
		self.sigmask.as_slice()
	}

	/// Returns a mutable reference to the process's blocked signals mask.
	#[inline(always)]
	pub fn get_sigmask_mut(&mut self) -> &mut [u8] {
		self.sigmask.as_mut_slice()
	}

	/// Returns an immutable reference to the process's pending signals mask.
	#[inline(always)]
	pub fn get_pending_signals(&self) -> &[u8] {
		self.sigpending.as_slice()
	}

	/// Returns a mutable reference to the process's pending signals mask.
	#[inline(always)]
	pub fn get_pending_signals_mut(&mut self) -> &mut [u8] {
		self.sigpending.as_mut_slice()
	}

	/// Tells whether the process has a signal pending.
	#[inline(always)]
	pub fn has_signal_pending(&self) -> bool {
		self.sigpending.find_set().is_some()
	}

	/// Makes the process handle the next signal. If the process is already handling a signal or if
	/// no signal is queued, the function does nothing.
	pub fn signal_next(&mut self) {
		// Looking for a pending signal with respect to the signal mask
		let mut sig = None;
		self.sigpending.for_each(| i, b | {
			if let Ok(s) = Signal::new(i as _) {
				if b && !(s.can_catch() && self.sigmask.is_set(i)) {
					sig = Some(s);
					return false;
				}
			}

			true
		});

		// If a signal is to be executed, execute it
		if let Some(sig) = sig {
			sig.execute_action(self, false);
		}
	}

	/// Clear the signal from the list of pending signals.
	/// If the signal is already cleared, the function does nothing.
	pub fn signal_clear(&mut self, sig: SignalType) {
		self.sigpending.clear(sig as _);
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
		if self.handled_signal.is_some() {
			self.handled_signal = None;
			self.regs = self.saved_regs;
		}
	}

	/// Returns the list of TLS entries for the process.
	pub fn get_tls_entries(&mut self) -> &mut [gdt::Entry] {
		&mut self.tls_entries
	}

	/// Returns a mutable reference to the process's LDT.
	/// If the LDT doesn't exist, the function creates one.
	pub fn get_ldt_mut(&mut self) -> Result<&mut LDT, Errno> {
		if self.ldt.is_none() {
			self.ldt = Some(LDT::new()?);
		}

		Ok(self.ldt.as_mut().unwrap())
	}

	/// Updates the `n`th TLS entry in the GDT.
	/// If `n` is out of bounds, the function does nothing.
	pub fn update_tls(&self, n: usize) {
		if n < TLS_ENTRIES_COUNT {
			unsafe { // Safe because the offset is checked by the condition
				self.tls_entries[n].update_gdt(gdt::TLS_OFFSET + n * size_of::<gdt::Entry>());
			}
		}
	}

	/// Sets the `clear_child_tid` attribute of the process.
	pub fn set_clear_child_tid(&mut self, ptr: Option<NonNull<i32>>) {
		self.clear_child_tid = ptr;
	}

	/// Returns an immutable reference to the process's resource usage structure.
	pub fn get_rusage(&self) -> &RUsage {
		&self.rusage
	}

	/// If the process is a vfork child, resets its state and its parent's state.
	pub fn reset_vfork(&mut self) {
		if self.vfork_state != VForkState::Executing {
			return;
		}

		self.vfork_state = VForkState::None;

		// Resetting the parent's vfork state if needed
		if let Some(parent) = self.get_parent() {
			let parent = parent.get_mut().unwrap();
			let mut guard = parent.lock();
			guard.get_mut().vfork_state = VForkState::None;
		}
	}

	/// Exits the process with the given `status`. This function changes the process's status to
	/// `Zombie`.
	pub fn exit(&mut self, status: u32) {
		self.exit_status = (status & 0xff) as ExitStatus;
		self.set_state(State::Zombie);

		// TODO Remove memory space?
		self.file_descriptors = None;
		// TODO Remove signal handlers

		self.reset_vfork();

		if let Some(parent) = self.get_parent() {
			// Wake the parent
			let parent = parent.get_mut().unwrap();
			let mut guard = parent.lock();
			guard.get_mut().wakeup();
		}
	}

	/// Returns the number of physical memory pages used by the process.
	pub fn get_memory_usage(&self) -> u32 {
		// TODO
		todo!();
	}

	/// Tells whether the given user ID has the permission to kill the current process.
	pub fn can_kill(&self, uid: Uid) -> bool {
		uid == ROOT_UID || uid == self.uid // TODO Also check saved user ID
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
		if self.is_init() {
			kernel_panic!("Terminated init process!");
		}

		// Freeing the kernel stack. This is required because the process might share the same
		// memory space with several other processes. And since, each process has its own kernel
		// stack, not freeing it could result in a memory leak
		oom::wrap(|| {
			if let Some(kernel_stack) = self.kernel_stack {
				if let Some(mutex) = &self.mem_space {
					mutex.lock().get_mut().unmap_stack(kernel_stack, KERNEL_STACK_SIZE)?;
				}
			}

			Ok(())
		});

		// Removing the child from the parent process
		if let Some(parent) = self.get_parent() {
			let parent = parent.get_mut().unwrap();
			let mut guard = parent.lock();
			guard.get_mut().remove_child(self.pid);
		}

		// Freeing the PID
		let mutex = unsafe {
			PID_MANAGER.assume_init_mut()
		};
		let mut guard = mutex.lock();
		guard.get_mut().release_pid(self.pid);
	}
}
