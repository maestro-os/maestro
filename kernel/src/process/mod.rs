/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! A process is a task running on the kernel.
//!
//! A multitasking system allows
//! several processes to run at the same time by sharing the CPU resources using
//! a scheduler.

// TODO Do not reallocate a PID of used as a pgid
// TODO When a process receives a signal or exits, log it if the `strace` feature is enabled

pub mod exec;
pub mod iovec;
pub mod mem_space;
pub mod oom;
pub mod pid;
pub mod regs;
pub mod rusage;
pub mod scheduler;
pub mod signal;
#[cfg(target_arch = "x86")]
pub mod tss;
pub mod user_desc;

use crate::{
	event,
	event::CallbackResult,
	file,
	file::{
		fd::{FileDescriptorTable, NewFDConstraint},
		fs::procfs::ProcFS,
		mountpoint, open_file,
		path::{Path, PathBuf},
		perm::{AccessProfile, ROOT_UID},
		vfs,
		vfs::ResolutionSettings,
		FileLocation,
	},
	gdt,
	memory::{buddy, buddy::FrameOrder},
	process::{mountpoint::MountSource, open_file::OpenFile},
	register_get,
	time::timer::TimerManager,
	tty,
	tty::TTYHandle,
};
use core::{
	any::Any,
	ffi::c_void,
	mem::{size_of, ManuallyDrop},
	ptr::NonNull,
};
use mem_space::MemSpace;
use pid::{PIDManager, Pid};
use regs::Regs;
use rusage::RUsage;
use scheduler::Scheduler;
use signal::{Signal, SignalAction, SignalHandler};
#[cfg(target_arch = "x86")]
use tss::TSS;
use utils::{
	collections::{bitfield::Bitfield, string::String, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	lock::{once::OnceInit, IntMutex, Mutex},
	ptr::arc::{Arc, Weak},
	TryClone,
};

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
const KERNEL_STACK_ORDER: FrameOrder = 2;

/// The file descriptor number of the standard input stream.
const STDIN_FILENO: u32 = 0;
/// The file descriptor number of the standard output stream.
const STDOUT_FILENO: u32 = 1;
/// The file descriptor number of the standard error stream.
const STDERR_FILENO: u32 = 2;

/// The number of TLS entries per process.
pub const TLS_ENTRIES_COUNT: usize = 3;

/// The size of the redzone in userspace, in bytes.
///
/// The redzone, defined by the System V ABI, is a zone of memory located right after the top of
/// the stack which can be used by the process.
///
/// When handling a signal, the kernel must make sure not to clobber this zone, thus an offset is
/// added.
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

/// Process forking parameters.
#[derive(Debug, Default)]
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

	/// If `true`, the parent is paused until the child process exits or executes
	/// a program.
	///
	/// Underneath, this option makes the parent and child use the same memory space.
	///
	/// This is useful in order to avoid an unnecessary clone of the memory space in case the
	/// child process executes a program or exits quickly.
	pub vfork: bool,
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

/// The **Process Control Block** (PCB). This structure stores all the information
/// about a process.
pub struct Process {
	/// The ID of the process.
	pub pid: Pid,
	/// The ID of the process group.
	pub pgid: Pid,
	/// The thread ID of the process.
	pub tid: Pid,

	/// The argv of the process.
	pub argv: Arc<Vec<String>>,
	/// The path to the process's executable.
	pub exec_path: Arc<PathBuf>,

	/// The process's current TTY.
	tty: TTYHandle,

	/// The process's access profile, containing user and group IDs.
	pub access_profile: AccessProfile,
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
	/// Tells whether the process was executing a system call.
	pub syscalling: bool,

	/// Tells whether the process is handling a signal.
	handled_signal: Option<Signal>,
	/// Registers state to be restored by `sigreturn`.
	sigreturn_regs: Regs,
	/// Tells whether the process has information that can be retrieved by
	/// wait/waitpid.
	waitable: bool,

	/// Structure managing the process's timers. This manager is shared between all threads of the
	/// same process.
	timer_manager: Arc<Mutex<TimerManager>>,

	/// The virtual memory of the process containing every mappings.
	mem_space: Option<Arc<IntMutex<MemSpace>>>,
	/// A pointer to the userspace stack.
	user_stack: Option<*mut c_void>,
	/// A pointer to the kernelspace stack.
	kernel_stack: NonNull<c_void>,

	/// Current working directory
	///
	/// The field contains both the path and location of the directory.
	pub cwd: Arc<(PathBuf, FileLocation)>,
	/// Current root path used by the process
	pub chroot: FileLocation,
	/// The list of open file descriptors with their respective ID.
	pub file_descriptors: Option<Arc<Mutex<FileDescriptorTable>>>,

	/// A bitfield storing the set of blocked signals.
	pub sigmask: Bitfield,
	/// A bitfield storing the set of pending signals.
	sigpending: Bitfield,
	/// The list of signal handlers.
	pub signal_handlers: Arc<Mutex<[SignalHandler; signal::SIGNALS_COUNT]>>,

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
static PID_MANAGER: OnceInit<Mutex<PIDManager>> = unsafe { OnceInit::new() };
/// The processes scheduler.
static SCHEDULER: OnceInit<IntMutex<Scheduler>> = unsafe { OnceInit::new() };

/// Initializes processes system. This function must be called only once, at
/// kernel initialization.
pub(crate) fn init() -> EResult<()> {
	TSS::init();
	// Init schedulers
	let cores_count = 1; // TODO
	unsafe {
		PID_MANAGER.init(Mutex::new(PIDManager::new()?));
		SCHEDULER.init(Mutex::new(Scheduler::new(cores_count)?));
	}
	// Register interruption callbacks
	let callback = |id: u32, _code: u32, regs: &Regs, ring: u32| {
		if ring < 3 {
			return CallbackResult::Panic;
		}
		// Get process
		let curr_proc = {
			let mut sched = SCHEDULER.get().lock();
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
			0x00 | 0x10 | 0x13 => curr_proc.kill_now(&Signal::SIGFPE),
			// Breakpoint
			0x03 => curr_proc.kill_now(&Signal::SIGTRAP),
			// Invalid Opcode
			0x06 => curr_proc.kill_now(&Signal::SIGILL),
			// General Protection Fault
			0x0d => {
				let inst_prefix = unsafe { *(regs.eip as *const u8) };
				if inst_prefix == HLT_INSTRUCTION {
					curr_proc.exit(regs.eax, false);
				} else {
					curr_proc.kill_now(&Signal::SIGSEGV);
				}
			}
			// Alignment Check
			0x11 => curr_proc.kill_now(&Signal::SIGBUS),
			_ => {}
		}
		if matches!(curr_proc.get_state(), State::Running) {
			CallbackResult::Continue
		} else {
			CallbackResult::Idle
		}
	};
	let page_fault_callback = |_id: u32, code: u32, _regs: &Regs, ring: u32| {
		let accessed_ptr = unsafe { register_get!("cr2") } as *const c_void;
		// Get process
		let curr_proc = Process::current();
		let Some(curr_proc) = curr_proc else {
			return CallbackResult::Panic;
		};
		let mut curr_proc = curr_proc.lock();
		// Handle page fault
		let success = {
			let Some(mem_space_mutex) = curr_proc.get_mem_space() else {
				return CallbackResult::Panic;
			};
			let mut mem_space = mem_space_mutex.lock();
			mem_space.handle_page_fault(accessed_ptr, code)
		};
		if !success {
			if ring < 3 {
				return CallbackResult::Panic;
			} else {
				curr_proc.kill_now(&Signal::SIGSEGV);
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
#[inline]
pub fn get_scheduler() -> &'static IntMutex<Scheduler> {
	SCHEDULER.get()
}

impl Process {
	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(pid: Pid) -> Option<Arc<IntMutex<Self>>> {
		get_scheduler().lock().get_by_pid(pid)
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(tid: Pid) -> Option<Arc<IntMutex<Self>>> {
		get_scheduler().lock().get_by_tid(tid)
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function returns `None`.
	pub fn current() -> Option<Arc<IntMutex<Self>>> {
		get_scheduler().lock().get_current_process()
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function makes the kernel panic.
	pub fn current_assert() -> Arc<IntMutex<Self>> {
		Self::current().expect("no running process")
	}

	/// Registers the current process to the procfs.
	fn register_procfs(&self) -> EResult<()> {
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
	fn unregister_procfs(&self) -> AllocResult<()> {
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
	/// The process is set to state [`State::Running`] by default and has user root.
	pub fn new() -> EResult<Arc<IntMutex<Self>>> {
		let rs = ResolutionSettings::kernel_follow();
		// Create the default file descriptors table
		let file_descriptors = {
			let mut fds_table = FileDescriptorTable::default();
			let tty_path = Path::new(TTY_DEVICE_PATH.as_bytes())?;
			let tty_file_mutex = vfs::get_file_from_path(tty_path, &rs)?;
			let tty_file = tty_file_mutex.lock();
			let loc = tty_file.get_location();
			let file = vfs::get_file_from_location(loc)?;
			let open_file = OpenFile::new(file, open_file::O_RDWR)?;
			let stdin_fd = fds_table.create_fd(0, open_file)?;
			assert_eq!(stdin_fd.get_id(), STDIN_FILENO);
			fds_table.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDOUT_FILENO), false)?;
			fds_table.duplicate_fd(STDIN_FILENO, NewFDConstraint::Fixed(STDERR_FILENO), false)?;
			fds_table
		};
		let root_loc = mountpoint::root_location();
		let process = Self {
			pid: pid::INIT_PID,
			pgid: pid::INIT_PID,
			tid: pid::INIT_PID,

			argv: Arc::new(Vec::new())?,
			exec_path: Arc::new(PathBuf::root())?,

			tty: tty::get(None).unwrap(), // Initialization with the init TTY

			access_profile: rs.access_profile,
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
			sigreturn_regs: Regs::default(),
			waitable: false,

			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid::INIT_PID)?))?,

			mem_space: None,
			user_stack: None,
			kernel_stack: buddy::alloc_kernel(KERNEL_STACK_ORDER)?,

			cwd: Arc::new((PathBuf::root(), root_loc.clone()))?,
			chroot: root_loc,
			file_descriptors: Some(Arc::new(Mutex::new(file_descriptors))?),

			sigmask: Bitfield::new(signal::SIGNALS_COUNT)?,
			sigpending: Bitfield::new(signal::SIGNALS_COUNT)?,
			signal_handlers: Arc::new(Mutex::new(Default::default()))?,

			tls_entries: [gdt::Entry::default(); TLS_ENTRIES_COUNT],

			set_child_tid: None,
			clear_child_tid: None,

			rusage: RUsage::default(),

			exit_status: 0,
			termsig: 0,
		};
		process.register_procfs()?;
		Ok(get_scheduler().lock().add_process(process)?)
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

	/// Sets the process's group ID to the given value `pgid`, updating the associated group.
	pub fn set_pgid(&mut self, pgid: Pid) -> EResult<()> {
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

	/// Returns an immutable slice to the PIDs of the process in the group of the current process.
	#[inline(always)]
	pub fn get_group_processes(&self) -> &[Pid] {
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
			.and_then(Weak::upgrade)
			.map(|parent| parent.lock().pid)
			.unwrap_or(self.pid)
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
				panic!("Terminated init process!");
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

	/// Wakes up the process if in [`State::Sleeping`] state.
	pub fn wake(&mut self) {
		if self.state == State::Sleeping {
			self.set_state(State::Running);
		}
	}

	/// Tells whether the current process has information to be retrieved by
	/// the `waitpid` system call.
	pub fn is_waitable(&self) -> bool {
		self.waitable
	}

	/// Sets the process waitable with the given signal type.
	pub fn set_waitable(&mut self, sig_type: u8) {
		self.waitable = true;
		self.termsig = sig_type;
		// Wake the parent
		let parent = self.get_parent().as_ref().and_then(Weak::upgrade);
		if let Some(parent) = parent {
			let mut parent = parent.lock();
			parent.kill(&Signal::SIGCHLD);
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

	/// Returns the process's parent.
	///
	/// If the process is the init process, the function returns `None`.
	#[inline(always)]
	pub fn get_parent(&self) -> Option<Weak<IntMutex<Process>>> {
		self.parent.clone()
	}

	/// Returns an immutable slice of the PIDs of the process's children.
	#[inline(always)]
	pub fn get_children(&self) -> &[Pid] {
		&self.children
	}

	/// Adds the process with the given PID `pid` as child to the process.
	pub fn add_child(&mut self, pid: Pid) -> AllocResult<()> {
		let i = self.children.binary_search(&pid).unwrap_or_else(|i| i);
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
	pub fn get_mem_space(&self) -> Option<&Arc<IntMutex<MemSpace>>> {
		self.mem_space.as_ref()
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
				panic!("Dropping the memory space of a running process!");
			}
		}

		self.mem_space = mem_space;
	}

	/// Updates the TSS on the current kernel for the process.
	pub fn update_tss(&self) {
		let kernel_stack_begin =
			self.kernel_stack.as_ptr() as usize + buddy::get_frame_size(KERNEL_STACK_ORDER);
		// Fill the TSS
		unsafe {
			TSS.0.esp0 = kernel_stack_begin as _;
			TSS.0.ss0 = gdt::KERNEL_DS as _;
			TSS.0.ss = gdt::USER_DS as _;
		}
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
			if let Some(sig) = self.get_next_signal() {
				// Prepare signal for execution
				let signal_handlers = self.signal_handlers.clone();
				let signal_handlers = signal_handlers.lock();
				let sig_handler = &signal_handlers[sig.get_id() as usize];
				sig_handler.prepare_execution(&mut *self, &sig, false);
				// If the process has been killed by the signal, abort switching
				if !matches!(self.state, State::Running) {
					return;
				}
			}
		}
		// Update the TSS for the process
		self.update_tss();
		// Update TLS entries in the GDT
		for i in 0..TLS_ENTRIES_COUNT {
			self.update_tls(i);
		}
		gdt::flush();
		// Bind the memory space
		self.get_mem_space().unwrap().lock().bind();
		// Increment the number of ticks the process had
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
	) -> EResult<Arc<IntMutex<Self>>> {
		debug_assert!(matches!(self.get_state(), State::Running));
		// Handle vfork
		let vfork_state = if fork_options.vfork {
			self.vfork_state = VForkState::Waiting; // TODO Cancel if the following code fails
			VForkState::Executing
		} else {
			VForkState::None
		};
		// Clone memory space
		let mem_space = {
			let curr_mem_space = self.get_mem_space().unwrap();
			if fork_options.share_memory || fork_options.vfork {
				curr_mem_space.clone()
			} else {
				Arc::new(IntMutex::new(curr_mem_space.lock().fork()?))?
			}
		};
		// Clone file descriptors
		let file_descriptors = if fork_options.share_fd {
			self.file_descriptors.clone()
		} else {
			self.file_descriptors
				.as_ref()
				.map(|fds| -> EResult<_> {
					let fds = fds.lock();
					let new_fds = fds.duplicate(false)?;
					Ok(Arc::new(Mutex::new(new_fds))?)
				})
				.transpose()?
		};
		// Clone signal handlers
		let signal_handlers = if fork_options.share_sighand {
			self.signal_handlers.clone()
		} else {
			Arc::new(Mutex::new(self.signal_handlers.lock().clone()))?
		};
		// FIXME PID is leaked if the following code fails
		let pid = PID_MANAGER.get().lock().get_unique_pid()?;
		let process = Self {
			pid,
			pgid: self.pgid,
			tid: pid,

			argv: self.argv.clone(),
			exec_path: self.exec_path.clone(),

			tty: self.tty.clone(),

			access_profile: self.access_profile,
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
			sigreturn_regs: self.sigreturn_regs.clone(),
			waitable: false,

			// TODO if creating a thread: timer_manager: self.timer_manager.clone(),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid)?))?,

			mem_space: Some(mem_space),
			user_stack: self.user_stack,
			kernel_stack: buddy::alloc_kernel(KERNEL_STACK_ORDER)?,

			cwd: self.cwd.clone(),
			chroot: self.chroot.clone(),
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
		Ok(get_scheduler().lock().add_process(process)?)
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
	pub fn kill(&mut self, sig: &Signal) {
		// Ignore blocked signals
		if sig.can_catch() && self.sigmask.is_set(sig.get_id() as _) {
			return;
		}
		// Statistics
		self.rusage.ru_nsignals = self.rusage.ru_nsignals.saturating_add(1);
		if matches!(self.get_state(), State::Stopped)
			&& sig.get_default_action() == SignalAction::Continue
		{
			self.set_state(State::Running);
		}
		// Set the signal as pending
		self.sigpending.set(sig.get_id() as _);
	}

	/// Same as [`Self::kill`], except the signal prepared for execution directly.
	///
	/// This is useful for cases where the execution of the program **MUST NOT** resume before
	/// handling the signal (such as hardware faults).
	pub fn kill_now(&mut self, sig: &Signal) {
		self.kill(sig);
		let signal_handlers = self.signal_handlers.clone();
		let signal_handlers = signal_handlers.lock();
		signal_handlers[sig.get_id() as usize].prepare_execution(self, sig, false);
	}

	/// Kills every process in the process group.
	pub fn kill_group(&mut self, sig: Signal) {
		for pid in self.process_group.iter() {
			if *pid != self.pid {
				if let Some(proc_mutex) = Process::get_by_pid(*pid) {
					let mut proc = proc_mutex.lock();
					proc.kill(&sig);
				}
			}
		}
		self.kill(&sig);
	}

	/// Tells whether the given signal is blocked by the process.
	pub fn is_signal_blocked(&self, sig: &Signal) -> bool {
		self.sigmask.is_set(sig.get_id() as _)
	}

	/// Returns the ID of the next signal to be handled.
	///
	/// If no signal is pending, the function returns `None`.
	pub fn get_next_signal(&self) -> Option<Signal> {
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

	/// Returns the pointer to use as a stack when executing a signal handler.
	pub fn get_signal_stack(&self) -> *const c_void {
		// TODO Handle the case where an alternate stack is specified (sigaltstack + flag
		// SA_ONSTACK)
		(self.regs.esp as usize - REDZONE_SIZE) as _
	}

	/// Saves the process's state to handle a signal.
	///
	/// `sig` is the signal.
	///
	/// If the process is already handling a signal, the behaviour is undefined.
	pub fn signal_save(&mut self, sig: Signal, sigreturn_regs: Regs) {
		debug_assert!(!self.is_handling_signal());
		self.handled_signal = Some(sig);
		self.sigreturn_regs = sigreturn_regs;
	}

	/// Restores the process's state after handling a signal.
	pub fn signal_restore(&mut self) {
		if let Some(sig) = self.handled_signal.take() {
			self.regs = self.sigreturn_regs.clone();
			self.sigpending.clear(sig.get_id() as _);
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
	///
	/// This function doesn't flush the GDT's cache. Thus it is the caller's responsibility.
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
		let sig = if signaled {
			self.exit_status = 0;
			self.termsig = status as ExitStatus;
			self.termsig
		} else {
			self.exit_status = status as ExitStatus;
			self.termsig = 0;
			0
		};

		self.set_state(State::Zombie);
		self.reset_vfork();
		self.set_waitable(sig);
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

	/// Returns the OOM score, used by the OOM killer to determine the process
	/// to kill in case the system runs out of memory.
	///
	/// A higher score means a higher probability of getting killed.
	pub fn get_oom_score(&self) -> u16 {
		let mut score = 0;

		// If the process is owned by the superuser, give it a bonus
		if self.access_profile.is_privileged() {
			score -= 100;
		}

		// TODO Compute the score using physical memory usage
		// TODO Take into account userspace-set values (oom may be disabled for this
		// process, an absolute score or a bonus might be given, etc...)

		score
	}
}

impl AccessProfile {
	/// Tells whether the agent can kill the process.
	pub fn can_kill(&self, proc: &Process) -> bool {
		let uid = self.get_uid();
		let euid = self.get_euid();
		// if privileged
		if uid == ROOT_UID || euid == ROOT_UID {
			return true;
		}

		// if sender's `uid` or `euid` equals receiver's `uid` or `suid`
		uid == proc.access_profile.get_uid()
			|| uid == proc.access_profile.get_suid()
			|| euid == proc.access_profile.get_uid()
			|| euid == proc.access_profile.get_suid()
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		if self.is_init() {
			panic!("Terminated init process!");
		}
		// Unregister the process from the procfs
		oom::wrap(|| self.unregister_procfs());
		// Free kernel stack
		unsafe {
			buddy::free_kernel(self.kernel_stack.as_ptr(), KERNEL_STACK_ORDER);
		}
		// Free the PID
		PID_MANAGER.get().lock().release_pid(self.pid);
	}
}
