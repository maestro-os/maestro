//! A process is a task running on the kernel.
//!
//! A multitasking system allows
//! several processes to run at the same time by sharing the CPU resources using
//! a scheduler.

// TODO Do not reallocate a PID of used as a pgid
// TODO When a process receives a signal, log it if the `strace` feature is enabled

pub mod exec;
pub mod iovec;
pub mod mem_space;
pub mod oom;
pub mod pid;
pub mod regs;
pub mod rusage;
pub mod scheduler;
pub mod signal;
pub mod tss;
pub mod user_desc;

use crate::cpu;
use crate::errno;
use crate::errno::Errno;
use crate::event;
use crate::event::CallbackResult;
use crate::file;
use crate::file::fd::FileDescriptorTable;
use crate::file::fd::NewFDConstraint;
use crate::file::fs::procfs::ProcFS;
use crate::file::mountpoint;
use crate::file::open_file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::Gid;
use crate::file::Uid;
use crate::file::ROOT_UID;
use crate::gdt;
use crate::memory;
use crate::process::mountpoint::MountSource;
use crate::time::timer::TimerManager;
use crate::tty;
use crate::tty::TTYHandle;
use crate::util::container::bitfield::Bitfield;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::*;
use crate::util::ptr::arc::Arc;
use crate::util::ptr::arc::Weak;
use crate::util::TryClone;
use core::any::Any;
use core::ffi::c_void;
use core::mem::size_of;
use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use mem_space::MemSpace;
use pid::PIDManager;
use pid::Pid;
use regs::Regs;
use rusage::RUsage;
use scheduler::Scheduler;
use signal::Signal;
use signal::SignalAction;
use signal::SignalHandler;

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

/// The size of the redzone in userspace, in bytes.
const REDZONE_SIZE: usize = 128;

/// An enumeration containing possible states for a process.
#[derive(Eq, Debug, PartialEq)]
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

impl State {
	/// Returns the character associated with the state.
	pub fn get_char(&self) -> char {
		match self {
			Self::Running => 'R',
			Self::Sleeping => 'S',
			Self::Stopped => 'T',
			Self::Zombie => 'Z',
		}
	}

	/// Returns the name of the state as string.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Running => "running",
			Self::Sleeping => "sleeping",
			Self::Stopped => "stopped",
			Self::Zombie => "zombie",
		}
	}
}

/// Type representing an exit status.
type ExitStatus = u8;

/// Structure representing options to be passed to the fork function.
#[derive(Debug)]
pub struct ForkOptions {
	/// If `true`, the parent and child processes both share the same address
	/// space.
	pub share_memory: bool,
	/// If `true`, the parent and child processes both share the same file
	/// descriptors table.
	pub share_fd: bool,
	/// If `true`, the parent and child processes both share the same signal
	/// handlers table.
	pub share_sighand: bool,

	/// If `true`, the parent is stopped until the child process exits or executes
	/// a program.
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

/// The vfork operation is similar to the fork operation except the parent
/// process isn't executed until the child process exits or executes a program.
///
/// The reason for this is to prevent useless copies of memory pages when the
/// child process is created only to execute a program.
///
/// It implies that the child process shares the same memory space as the
/// parent.
#[derive(Clone, Copy, Debug, PartialEq)]
enum VForkState {
	/// The process is not in vfork state.
	None,

	/// The process is the parent waiting for the child to terminate.
	Waiting,
	/// The process is the child the parent waits for.
	Executing,
}

/// The Process Control Block (PCB). This structure stores all the informations
/// about a process.
pub struct Process {
	/// The ID of the process.
	pub pid: Pid,
	/// The ID of the process group.
	pub pgid: Pid,
	/// The thread ID of the process.
	pub tid: Pid,

	/// The argv of the process.
	pub argv: Vec<String>,
	/// The path to the process's executable.
	exec_path: Path,

	/// The process's current TTY.
	tty: TTYHandle,

	/// The real ID of the process's user owner.
	pub uid: Uid,
	/// The real ID of the process's group owner.
	pub gid: Gid,

	/// The effective ID of the process's user owner.
	pub euid: Uid,
	/// The effective ID of the process's group owner.
	pub egid: Gid,

	/// The saved user ID of the process's owner.
	pub suid: Uid,
	/// The saved group ID of the process's owner.
	pub sgid: Gid,

	/// The process's current umask.
	pub umask: file::Mode,

	/// The current state of the process.
	state: State,
	/// The current vfork state of the process (see documentation of
	/// `VForkState`).
	vfork_state: VForkState,

	/// The priority of the process.
	pub priority: usize,
	/// The nice value of the process.
	pub nice: usize,
	/// The number of quantum run during the cycle.
	quantum_count: usize,

	/// A pointer to the parent process.
	parent: Option<Weak<IntMutex<Process>>>,
	/// The list of children processes.
	children: Vec<Pid>,
	/// The list of processes in the process group.
	process_group: Vec<Pid>,

	/// The last saved registers state.
	pub regs: Regs,
	/// Tells whether the process was syscalling or not.
	pub syscalling: bool,

	/// Tells whether the process is handling a signal.
	handled_signal: Option<Signal>,
	/// The saved state of registers, used when handling a signal.
	saved_regs: Regs,
	/// Tells whether the process has information that can be retrieved by
	/// wait/waitpid.
	waitable: bool,

	/// Structure managing the process's timers. This manager is shared between all threads of the
	/// same process.
	timer_manager: Arc<Mutex<TimerManager>>,

	/// The virtual memory of the process containing every mappings.
	mem_space: Option<Arc<IntMutex<MemSpace>>>,
	/// A pointer to the userspace stack.
	user_stack: Option<*const c_void>,
	/// A pointer to the kernelspace stack.
	kernel_stack: Option<*const c_void>,

	/// The current working directory.
	cwd: Path,
	/// The current chroot path.
	pub chroot: Path,
	/// The list of open file descriptors with their respective ID.
	file_descriptors: Option<Arc<Mutex<FileDescriptorTable>>>,

	/// A bitfield storing the set of blocked signals.
	pub sigmask: Bitfield,
	/// A bitfield storing the set of pending signals.
	sigpending: Bitfield,
	/// The list of signal handlers.
	signal_handlers: Arc<Mutex<[SignalHandler; signal::SIGNALS_COUNT]>>,

	/// TLS entries.
	tls_entries: [gdt::Entry; TLS_ENTRIES_COUNT],

	/// If a thread is started using `clone` with the `CLONE_CHILD_SETTID` flag, set_child_tid is
	/// set to the value passed in the ctid argument of that system call.
	set_child_tid: Option<NonNull<i32>>,
	/// If a thread is started using `clone` with the `CLONE_CHILD_CLEARTID` flag, clear_child_tid
	/// is set to the value passed in the ctid argument of that system call.
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
static mut SCHEDULER: MaybeUninit<Arc<IntMutex<Scheduler>>> = MaybeUninit::uninit();

/// Initializes processes system. This function must be called only once, at
/// kernel initialization.
pub fn init() -> Result<(), Errno> {
	tss::init();
	tss::flush();

	let cores_count = 1; // TODO
	unsafe {
		PID_MANAGER.write(Mutex::new(PIDManager::new()?));
		SCHEDULER.write(Scheduler::new(cores_count)?);
	}

	let callback = |id: u32, _code: u32, regs: &Regs, ring: u32| {
		if ring < 3 {
			return CallbackResult::Panic;
		}

		// Get process
		let curr_proc = {
			let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
			let mut sched = sched_mutex.lock();

			sched.get_current_process()
		};
		let Some(curr_proc) = curr_proc else {
			return CallbackResult::Panic;
		};
		let mut curr_proc = curr_proc.lock();

		match id {
			// Divide-by-zero
			// x87 Floating-Point Exception
			// SIMD Floating-Point Exception
			0x00 | 0x10 | 0x13 => {
				curr_proc.kill(&Signal::SIGFPE, true);
				curr_proc.signal_next();
			}

			// Breakpoint
			0x03 => {
				curr_proc.kill(&Signal::SIGTRAP, true);
				curr_proc.signal_next();
			}

			// Invalid Opcode
			0x06 => {
				curr_proc.kill(&Signal::SIGILL, true);
				curr_proc.signal_next();
			}

			// General Protection Fault
			0x0d => {
				let inst_prefix = unsafe { *(regs.eip as *const u8) };

				if inst_prefix == HLT_INSTRUCTION {
					curr_proc.exit(regs.eax, false);
				} else {
					curr_proc.kill(&Signal::SIGSEGV, true);
					curr_proc.signal_next();
				}
			}

			// Alignment Check
			0x11 => {
				curr_proc.kill(&Signal::SIGBUS, true);
				curr_proc.signal_next();
			}

			_ => {}
		}

		if matches!(curr_proc.get_state(), State::Running) {
			CallbackResult::Continue
		} else {
			CallbackResult::Idle
		}
	};
	let page_fault_callback = |_id: u32, code: u32, _regs: &Regs, ring: u32| {
		let accessed_ptr = unsafe { cpu::cr2_get() };

		// Get process
		let curr_proc = {
			let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
			let mut sched = sched_mutex.lock();

			sched.get_current_process()
		};
		let Some(curr_proc) = curr_proc else {
			return CallbackResult::Panic;
		};
		let mut curr_proc = curr_proc.lock();

		// Handle page fault
		let success = {
			let mem_space_mutex = curr_proc.get_mem_space().unwrap();
			let mut mem_space = mem_space_mutex.lock();

			mem_space.handle_page_fault(accessed_ptr, code)
		};

		if !success {
			if ring < 3 {
				return CallbackResult::Panic;
			} else {
				curr_proc.kill(&Signal::SIGSEGV, true);
				curr_proc.signal_next();
			}
		}

		if matches!(curr_proc.get_state(), State::Running) {
			CallbackResult::Continue
		} else {
			CallbackResult::Idle
		}
	};

	let _ = ManuallyDrop::new(event::register_callback(0x00, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x03, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x06, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x0d, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x0e, page_fault_callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x10, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x11, callback)?);
	let _ = ManuallyDrop::new(event::register_callback(0x13, callback)?);

	Ok(())
}

/// Returns a mutable reference to the scheduler's `Mutex`.
pub fn get_scheduler() -> &'static IntMutex<Scheduler> {
	unsafe {
		// Safe because using Mutex
		SCHEDULER.assume_init_ref()
	}
}

impl Process {
	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(pid: Pid) -> Option<Arc<IntMutex<Self>>> {
		let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
		sched_mutex.lock().get_by_pid(pid)
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(tid: Pid) -> Option<Arc<IntMutex<Self>>> {
		let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
		sched_mutex.lock().get_by_tid(tid)
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function returns `None`.
	pub fn current() -> Option<Arc<IntMutex<Self>>> {
		let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
		sched_mutex.lock().get_current_process()
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function makes the kernel panic.
	pub fn current_assert() -> Arc<IntMutex<Self>> {
		Self::current().expect("no running process")
	}

	/// Registers the current process to the procfs.
	fn register_procfs(&self) -> Result<(), Errno> {
		let procfs_source = MountSource::NoDev(b"procfs".try_into()?);
		let Some(fs) = mountpoint::get_fs(&procfs_source) else {
			return Ok(());
		};
		let mut fs_guard = fs.lock();
		let fs = &mut *fs_guard as &mut dyn Any;

		let procfs = fs.downcast_mut::<ProcFS>().unwrap();
		procfs.add_process(self.pid)?;

		Ok(())
	}

	/// Unregisters the current process from the procfs.
	fn unregister_procfs(&self) -> Result<(), Errno> {
		let procfs_source = MountSource::NoDev(b"procfs".try_into()?);
		let Some(fs) = mountpoint::get_fs(&procfs_source) else {
			return Ok(());
		};
		let mut fs_guard = fs.lock();
		let fs = &mut *fs_guard as &mut dyn Any;

		let procfs = fs.downcast_mut::<ProcFS>().unwrap();
		procfs.remove_process(self.pid)?;

		Ok(())
	}

	/// Creates the init process and places it into the scheduler's queue.
	///
	/// The process is set to state `Running` by default and has user root.
	pub fn new() -> Result<Arc<IntMutex<Self>>, Errno> {
		let uid = 0;
		let gid = 0;

		// Creating the default file descriptors table
		let file_descriptors = {
			let mut fds_table = FileDescriptorTable::default();

			let vfs_mutex = vfs::get();
			let mut vfs = vfs_mutex.lock();
			let vfs = vfs.as_mut().unwrap();

			let tty_path = Path::from_str(TTY_DEVICE_PATH.as_bytes(), false).unwrap();
			let tty_file_mutex = vfs.get_file_from_path(&tty_path, uid, gid, true)?;
			let tty_file = tty_file_mutex.lock();

			let loc = tty_file.get_location().clone();

			open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

			let stdin_fd = fds_table.create_fd(loc, 0, true, true)?;
			assert_eq!(stdin_fd.get_id(), STDIN_FILENO);

			fds_table.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDOUT_FILENO), false)?;
			fds_table.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDERR_FILENO), false)?;

			fds_table
		};

		let process = Self {
			pid: pid::INIT_PID,
			pgid: pid::INIT_PID,
			tid: pid::INIT_PID,

			argv: Vec::new(),
			exec_path: Path::root(),

			tty: tty::get(None).unwrap(), // Initialization with the init TTY

			uid,
			gid,

			euid: uid,
			egid: gid,

			suid: uid,
			sgid: gid,

			umask: DEFAULT_UMASK,

			state: State::Running,
			vfork_state: VForkState::None,

			priority: 0,
			nice: 0,
			quantum_count: 0,

			parent: None,
			children: Vec::new(),
			process_group: Vec::new(),

			regs: Regs::default(),
			syscalling: false,

			handled_signal: None,
			saved_regs: Regs::default(),
			waitable: false,

			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid::INIT_PID)?))?,

			mem_space: None,
			user_stack: None,
			kernel_stack: None,

			cwd: Path::root(),
			chroot: Path::root(),
			file_descriptors: Some(Arc::new(Mutex::new(file_descriptors))?),

			sigmask: Bitfield::new(signal::SIGNALS_COUNT)?,
			sigpending: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: Arc::new(Mutex::new(
				[SignalHandler::Default; signal::SIGNALS_COUNT],
			))?,

			tls_entries: [gdt::Entry::default(); TLS_ENTRIES_COUNT],

			set_child_tid: None,
			clear_child_tid: None,

			rusage: RUsage::default(),

			exit_status: 0,
			termsig: 0,
		};

		process.register_procfs()?;

		let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
		sched_mutex.lock().add_process(process)
	}

	/// Tells whether the process is the init process.
	#[inline(always)]
	pub fn is_init(&self) -> bool {
		self.pid == pid::INIT_PID
	}

	/// Tells whether the process is among a group and is not its owner.
	#[inline(always)]
	pub fn is_in_group(&self) -> bool {
		self.pgid != 0 && self.pgid != self.pid
	}

	/// Sets the process's group ID to the given value `pgid`.
	pub fn set_pgid(&mut self, pgid: Pid) -> Result<(), Errno> {
		let old_pgid = self.pgid;
		let new_pgid = if pgid == 0 { self.pid } else { pgid };

		if old_pgid == new_pgid {
			return Ok(());
		}

		if new_pgid != self.pid {
			// Adding the process to the new group
			if let Some(proc_mutex) = Process::get_by_pid(new_pgid) {
				let mut new_group_process = proc_mutex.lock();

				let i = new_group_process
					.process_group
					.binary_search(&self.pid)
					.unwrap_err();
				new_group_process.process_group.insert(i, self.pid)?;
			} else {
				return Err(errno!(ESRCH));
			}
		}

		// Removing the process from its old group
		if self.is_in_group() {
			if let Some(proc_mutex) = Process::get_by_pid(old_pgid) {
				let mut old_group_process = proc_mutex.lock();

				if let Ok(i) = old_group_process.process_group.binary_search(&self.pid) {
					old_group_process.process_group.remove(i);
				}
			}
		}

		self.pgid = new_pgid;
		Ok(())
	}

	/// Returns a reference to the list of PIDs of processes in the current
	/// process's group.
	#[inline(always)]
	pub fn get_group_processes(&self) -> &Vec<Pid> {
		&self.process_group
	}

	/// The function tells whether the process is in an orphaned process group.
	pub fn is_in_orphan_process_group(&self) -> bool {
		if !self.is_in_group() {
			return false;
		}

		Process::get_by_pid(self.pgid).is_none()
	}

	/// Returns the parent process's PID.
	pub fn get_parent_pid(&self) -> Pid {
		self.parent
			.as_ref()
			.and_then(|parent| parent.upgrade())
			.map(|parent| parent.lock().pid)
			.unwrap_or(self.pid)
	}

	/// Returns the path to the executable file of the process.
	pub fn get_exec_path(&self) -> &Path {
		&self.exec_path
	}

	/// Returns the TTY associated with the process.
	pub fn get_tty(&self) -> TTYHandle {
		self.tty.clone()
	}

	/// Returns the process's current state.
	#[inline(always)]
	pub fn get_state(&self) -> &State {
		&self.state
	}

	/// Sets the process's state to `new_state`.
	pub fn set_state(&mut self, new_state: State) {
		if self.state == new_state || self.state == State::Zombie {
			return;
		}

		// Update the number of running processes
		if self.state != State::Running && new_state == State::Running {
			get_scheduler().lock().increment_running();
		} else if self.state == State::Running {
			get_scheduler().lock().decrement_running();
		}

		self.state = new_state;

		if self.state == State::Zombie {
			if self.is_init() {
				kernel_panic!("Terminated init process!");
			}

			// Removing the memory space and file descriptors table to save memory
			//self.mem_space = None; // TODO Handle the case where the memory space is bound
			self.file_descriptors = None;

			// Attaching every child to the init process
			let init_proc_mutex = Process::get_by_pid(pid::INIT_PID).unwrap();
			let mut init_proc = init_proc_mutex.lock();
			for child_pid in self.children.iter() {
				// Check just in case
				if *child_pid == self.pid {
					continue;
				}

				if let Some(child_mutex) = Process::get_by_pid(*child_pid) {
					child_mutex.lock().parent = Some(Arc::downgrade(&init_proc_mutex));
					oom::wrap(|| init_proc.add_child(*child_pid));
				}
			}

			self.waitable = true;
		}
	}

	/// Tells whether the scheduler can run the process.
	pub fn can_run(&self) -> bool {
		matches!(self.get_state(), State::Running) && self.vfork_state != VForkState::Waiting
	}

	/// Wakes the process if sleeping.
	pub fn wake(&mut self) {
		if self.state == State::Sleeping {
			self.set_state(State::Running);
		}
	}

	/// Tells whether the current process has informations to be retrieved by
	/// the `waitpid` system call.
	pub fn is_waitable(&self) -> bool {
		self.waitable
	}

	/// Sets the process waitable with the given signal type `type_`.
	pub fn set_waitable(&mut self, sig_type: u8) {
		self.waitable = true;
		self.termsig = sig_type;

		// Wake the parent
		let parent = self.get_parent().and_then(|parent| parent.upgrade());
		if let Some(parent) = parent {
			let mut parent = parent.lock();
			parent.kill(&Signal::SIGCHLD, false);
			parent.wake();
		}
	}

	/// Clears the waitable flag.
	pub fn clear_waitable(&mut self) {
		self.waitable = false;
	}

	/// Returns the process's timer manager.
	pub fn timer_manager(&self) -> Arc<Mutex<TimerManager>> {
		self.timer_manager.clone()
	}

	/// Returns the process's parent if exists.
	#[inline(always)]
	pub fn get_parent(&self) -> Option<Weak<IntMutex<Process>>> {
		self.parent.clone()
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
		let i = match self.children.binary_search(&pid) {
			Ok(i) => i,
			Err(i) => i,
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
	///
	/// If the process is terminated, the function returns `None`.
	#[inline(always)]
	pub fn get_mem_space(&self) -> Option<Arc<IntMutex<MemSpace>>> {
		self.mem_space.clone()
	}

	/// Sets the new memory space for the process, dropping the previous if any.
	#[inline(always)]
	pub fn set_mem_space(&mut self, mem_space: Option<Arc<IntMutex<MemSpace>>>) {
		// TODO Handle multicore
		// If the process is currently running, switch the memory space
		if matches!(self.state, State::Running) {
			if let Some(mem_space) = &mem_space {
				mem_space.lock().bind();
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
	///
	/// If the given path is relative, it is made absolute by concatenated with
	/// `/`.
	#[inline(always)]
	pub fn set_cwd(&mut self, path: Path) -> Result<(), Errno> {
		self.cwd = Path::root().concat(&path)?;
		Ok(())
	}

	/// Returns the file descriptor table associated with the process.
	pub fn get_fds(&self) -> Option<Arc<Mutex<FileDescriptorTable>>> {
		self.file_descriptors.clone()
	}

	/// Sets the file descriptor table of the process.
	pub fn set_fds(&mut self, fds: Option<Arc<Mutex<FileDescriptorTable>>>) {
		self.file_descriptors = fds;
	}

	/// Updates the TSS on the current core for the process.
	pub fn update_tss(&self) {
		// Filling the TSS
		let tss = tss::get();
		tss.ss0 = gdt::KERNEL_DS as _;
		tss.ss = gdt::USER_DS as _;

		// Setting the kernel stack pointer
		let mut kernel_stack_ptr = self.kernel_stack.unwrap() as usize;
		if self.is_handling_signal() {
			// Preventing overlapping of stacks
			kernel_stack_ptr -= (KERNEL_STACK_SIZE / 2) * memory::PAGE_SIZE;
		}
		tss.esp0 = kernel_stack_ptr as _;
	}

	/// Prepares for context switching to the process.
	///
	/// The function may update the state of the process. Thus, the caller must
	/// check the state to ensure the process can actually be run.
	pub fn prepare_switch(&mut self) {
		if !matches!(self.state, State::Running) {
			return;
		}

		// If the process is not in a syscall and a signal is pending on the process,
		// execute it
		if !self.syscalling {
			self.signal_next();

			if !matches!(self.state, State::Running) {
				return;
			}
		}

		// Updates the TSS for the process
		self.update_tss();

		// Binding the memory space
		self.get_mem_space().unwrap().lock().bind();

		// Updating TLS entries in the GDT
		for i in 0..TLS_ENTRIES_COUNT {
			self.update_tls(i);
		}

		// Incrementing the number of ticks the process had
		self.quantum_count += 1;
	}

	/// Returns the exit status if the process has ended.
	#[inline(always)]
	pub fn get_exit_status(&self) -> Option<ExitStatus> {
		if matches!(self.state, State::Zombie) {
			Some(self.exit_status)
		} else {
			None
		}
	}

	/// Returns the signal number that killed the process.
	#[inline(always)]
	pub fn get_termsig(&self) -> u8 {
		self.termsig
	}

	/// Forks the current process.
	///
	/// The internal state of the process (registers and memory) are always copied.
	/// Other data may be copied according to provided fork options.
	///
	/// Arguments:
	/// - `parent` is the parent of the new process.
	/// - `fork_options` are the options for the fork operation.
	///
	/// On fail, the function returns an `Err` with the appropriate Errno.
	///
	/// If the process is not running, the behaviour is undefined.
	pub fn fork(
		&mut self,
		parent: Weak<IntMutex<Self>>,
		fork_options: ForkOptions,
	) -> Result<Arc<IntMutex<Self>>, Errno> {
		debug_assert!(matches!(self.get_state(), State::Running));

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
				let new_kernel_stack = curr_mem_space
					.lock()
					.map_stack(KERNEL_STACK_SIZE, KERNEL_STACK_FLAGS)?;

				(curr_mem_space, Some(new_kernel_stack as _))
			} else {
				(
					Arc::new(IntMutex::new(curr_mem_space.lock().fork()?))?,
					self.kernel_stack,
				)
			}
		};

		// Cloning file descriptors
		let file_descriptors = if fork_options.share_fd {
			self.file_descriptors.clone()
		} else {
			self.file_descriptors
				.as_ref()
				.map(|fds| {
					let fds = fds.lock();
					let new_fds = fds.duplicate(false)?;

					Arc::new(Mutex::new(new_fds))
				})
				.transpose()?
		};

		// Cloning signal handlers
		let signal_handlers = if fork_options.share_sighand {
			self.signal_handlers.clone()
		} else {
			Arc::new(Mutex::new(self.signal_handlers.lock().clone()))?
		};

		// FIXME PID is leaked if the following code fails
		let pid = {
			let mutex = unsafe { PID_MANAGER.assume_init_mut() };
			mutex.lock().get_unique_pid()
		}?;

		let process = Self {
			pid,
			pgid: self.pgid,
			tid: pid,

			argv: self.argv.try_clone()?,
			exec_path: self.exec_path.try_clone()?,

			tty: self.tty.clone(),

			uid: self.uid,
			gid: self.gid,

			euid: self.euid,
			egid: self.egid,

			suid: self.suid,
			sgid: self.sgid,

			umask: self.umask,

			state: State::Running,
			vfork_state,

			priority: self.priority,
			nice: self.nice,
			quantum_count: 0,

			parent: Some(parent),
			children: Vec::new(),
			process_group: Vec::new(),

			regs: self.regs.clone(),
			syscalling: false,

			handled_signal: self.handled_signal.clone(),
			saved_regs: self.saved_regs.clone(),
			waitable: false,

			// TODO if creating a thread: timer_manager: self.timer_manager.clone(),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid)?))?,

			mem_space: Some(mem_space),
			user_stack: self.user_stack,
			kernel_stack,

			cwd: self.cwd.try_clone()?,
			chroot: self.chroot.try_clone()?,
			file_descriptors,

			sigmask: self.sigmask.try_clone()?,
			sigpending: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers,

			tls_entries: self.tls_entries,

			set_child_tid: self.set_child_tid,
			clear_child_tid: self.clear_child_tid,

			rusage: RUsage::default(),

			exit_status: self.exit_status,
			termsig: 0,
		};

		process.register_procfs()?;

		self.add_child(pid)?;

		let sched_mutex = unsafe { SCHEDULER.assume_init_mut() };
		sched_mutex.lock().add_process(process)
	}

	/// Returns the signal handler for the signal `sig`.
	#[inline(always)]
	pub fn get_signal_handler(&self, sig: &Signal) -> SignalHandler {
		self.signal_handlers.lock()[sig.get_id() as usize]
	}

	/// Sets the signal handler `handler` for the signal `sig`.
	#[inline(always)]
	pub fn set_signal_handler(&mut self, sig: &Signal, handler: SignalHandler) {
		self.signal_handlers.lock()[sig.get_id() as usize] = handler;
	}

	/// Tells whether the process is handling a signal.
	#[inline(always)]
	pub fn is_handling_signal(&self) -> bool {
		self.handled_signal.is_some()
	}

	/// Kills the process with the given signal `sig`.
	///
	/// If the process doesn't have a signal handler, the default action for the signal is
	/// executed.
	///
	/// If `no_handler` is `true` and if the process is already handling a signal,
	/// the function executes the default action of the signal regardless the
	/// user-specified action.
	pub fn kill(&mut self, sig: &Signal, no_handler: bool) {
		if sig.can_catch() && self.sigmask.is_set(sig.get_id() as _) {
			return;
		}

		self.rusage.ru_nsignals += 1;

		if matches!(self.get_state(), State::Stopped)
			&& sig.get_default_action() == SignalAction::Continue
		{
			self.set_state(State::Running);
		}

		let no_handler = self.is_handling_signal() && no_handler;
		if !sig.can_catch() || no_handler {
			sig.execute_action(self, no_handler);
		} else {
			self.sigpending.set(sig.get_id() as _);
		}
	}

	/// Kills every processes in the process group.
	///
	/// Arguments are the same as `kill`.
	pub fn kill_group(&mut self, sig: Signal, no_handler: bool) {
		for pid in self.process_group.iter() {
			if *pid != self.pid {
				if let Some(proc_mutex) = Process::get_by_pid(*pid) {
					let mut proc = proc_mutex.lock();
					proc.kill(&sig, no_handler);
				}
			}
		}

		self.kill(&sig, no_handler);
	}

	/// Tells whether the given signal is blocked by the process.
	pub fn is_signal_blocked(&self, sig: &Signal) -> bool {
		self.sigmask.is_set(sig.get_id() as _)
	}

	/// Returns an immutable reference to the process's pending signals mask.
	#[inline(always)]
	pub fn get_pending_signals(&self) -> &[u8] {
		self.sigpending.as_slice()
	}

	/// Tells whether the process has a signal pending.
	#[inline(always)]
	pub fn has_signal_pending(&self) -> bool {
		self.sigpending.find_set().is_some()
	}

	/// Returns the ID of the next signal to be executed.
	///
	/// If no signal is pending or is the process is already handling a signal,
	/// the function returns `None`.
	pub fn get_next_signal(&self) -> Option<Signal> {
		if self.is_handling_signal() {
			return None;
		}

		self.sigpending
			.iter()
			.enumerate()
			.filter_map(|(i, b)| {
				if !b {
					return None;
				}

				let s = Signal::try_from(i as u32).ok()?;
				if !s.can_catch() || !self.sigmask.is_set(i) {
					Some(s)
				} else {
					None
				}
			})
			.next()
	}

	/// Makes the process handle the next signal.
	///
	/// If no signal is pending or is the process is already handling a signal,
	/// the function does nothing.
	pub fn signal_next(&mut self) {
		if let Some(sig) = self.get_next_signal() {
			sig.execute_action(self, false);
		}
	}

	/// Returns the pointer to use as a stack when executing a signal handler.
	pub fn get_signal_stack(&self) -> *const c_void {
		// TODO Handle alternate stacks
		(self.regs.esp as usize - REDZONE_SIZE) as _
	}

	/// Clear the signal from the list of pending signals.
	///
	/// If the signal is already cleared, the function does nothing.
	pub fn signal_clear(&mut self, sig: Signal) {
		self.sigpending.clear(sig.get_id() as _);
	}

	/// Saves the process's state to handle a signal.
	///
	/// `sig` is the signal.
	///
	/// If the process is already handling a signal, the behaviour is undefined.
	pub fn signal_save(&mut self, sig: Signal) {
		debug_assert!(!self.is_handling_signal());

		self.saved_regs = self.regs.clone();
		self.handled_signal = Some(sig);
	}

	/// Restores the process's state after handling a signal.
	pub fn signal_restore(&mut self) {
		if self.handled_signal.is_some() {
			self.handled_signal = None;
			self.regs = self.saved_regs.clone();
		}
	}

	/// Returns the list of TLS entries for the process.
	pub fn get_tls_entries(&mut self) -> &mut [gdt::Entry] {
		&mut self.tls_entries
	}

	/// Clears the process's TLS entries.
	pub fn clear_tls_entries(&mut self) {
		for e in &mut self.tls_entries {
			*e = Default::default();
		}
	}

	/// Updates the `n`th TLS entry in the GDT.
	///
	/// If `n` is out of bounds, the function does nothing.
	pub fn update_tls(&self, n: usize) {
		if n < TLS_ENTRIES_COUNT {
			unsafe {
				// Safe because the offset is checked by the condition
				self.tls_entries[n].update_gdt(gdt::TLS_OFFSET + n * size_of::<gdt::Entry>());
			}
		}
	}

	/// Sets the `clear_child_tid` attribute of the process.
	pub fn set_clear_child_tid(&mut self, ptr: Option<NonNull<i32>>) {
		self.clear_child_tid = ptr;
	}

	/// Returns an immutable reference to the process's resource usage
	/// structure.
	pub fn get_rusage(&self) -> &RUsage {
		&self.rusage
	}

	/// If the process is a vfork child, resets its state and its parent's
	/// state.
	pub fn reset_vfork(&mut self) {
		if self.vfork_state != VForkState::Executing {
			return;
		}

		self.vfork_state = VForkState::None;

		// Resetting the parent's vfork state if needed
		let parent = self.get_parent().and_then(|parent| parent.upgrade());
		if let Some(parent) = parent {
			let mut parent = parent.lock();
			parent.vfork_state = VForkState::None;
		}
	}

	/// Exits the process with the given `status`.
	///
	/// This function changes the process's status to `Zombie`.
	///
	/// `signaled` tells whether the process has been terminated by a signal. If
	/// `true`, `status` is interpreted as the signal number.
	pub fn exit(&mut self, status: u32, signaled: bool) {
		if signaled {
			self.exit_status = 0;
			self.termsig = (status & 0xff) as ExitStatus;
		} else {
			self.exit_status = (status & 0xff) as ExitStatus;
			self.termsig = 0;
		}

		self.set_state(State::Zombie);
		self.reset_vfork();
		self.set_waitable(0); // TODO Check parameter
	}

	/// Returns the number of virtual memory pages used by the process.
	pub fn get_vmem_usage(&self) -> usize {
		if let Some(mem_space_mutex) = &self.mem_space {
			let mem_space = mem_space_mutex.lock();
			mem_space.get_vmem_usage()
		} else {
			0
		}
	}

	/// Tells whether the given user ID has the permission to kill the current
	/// process.
	pub fn can_kill(&self, uid: Uid) -> bool {
		uid == ROOT_UID || uid == self.uid // TODO Also check saved user ID
	}

	/// Returns the OOM score, used by the OOM killer to determine the process
	/// to kill in case the system runs out of memory.
	///
	/// A higher score means a higher probability of getting killed.
	pub fn get_oom_score(&self) -> u16 {
		let mut score = 0;

		// If the process is owned by the superuser, give it a bonus
		if self.uid == ROOT_UID {
			score -= 100;
		}

		// TODO Compute the score using physical memory usage
		// TODO Take into account userspace-set values (oom may be disabled for this
		// process, an absolute score or a bonus might be given, etc...)

		score
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		if self.is_init() {
			kernel_panic!("Terminated init process!");
		}

		// Unregister the process from the procfs
		oom::wrap(|| self.unregister_procfs());

		// Freeing the kernel stack. This is required because the process might share
		// the same memory space with several other processes. And since, each process
		// has its own kernel stack, not freeing it could result in a memory leak
		oom::wrap(|| {
			if let Some(kernel_stack) = self.kernel_stack {
				if let Some(mutex) = &self.mem_space {
					mutex.lock().unmap_stack(kernel_stack, KERNEL_STACK_SIZE)?;
				}
			}

			Ok(())
		});

		// Freeing the PID
		let mut pid_manager = unsafe { PID_MANAGER.assume_init_mut() }.lock();
		pid_manager.release_pid(self.pid);
	}
}
