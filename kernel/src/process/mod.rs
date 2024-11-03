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
pub mod rusage;
pub mod scheduler;
pub mod signal;
pub mod user_desc;

use crate::{
	arch::x86::{gdt, idt::IntFrame, tss, tss::TSS},
	event,
	event::CallbackResult,
	file,
	file::{
		fd::{FileDescriptorTable, NewFDConstraint},
		perm::AccessProfile,
		vfs,
		vfs::ResolutionSettings,
		File, O_RDWR,
	},
	memory::{buddy, buddy::FrameOrder, VirtAddr},
	process::{
		mem_space::{copy, copy::SyscallPtr},
		pid::PidHandle,
		scheduler::{Scheduler, SCHEDULER},
		signal::SigSet,
	},
	register_get,
	syscall::FromSyscallArg,
	time::timer::TimerManager,
};
use core::{
	ffi::{c_int, c_void},
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	mem,
	mem::{size_of, ManuallyDrop},
	ptr::NonNull,
	sync::atomic::AtomicPtr,
};
use mem_space::MemSpace;
use pid::Pid;
use rusage::RUsage;
use signal::{Signal, SignalAction, SignalHandler};
use utils::{
	collections::{
		path::{Path, PathBuf},
		string::String,
		vec::Vec,
	},
	errno,
	errno::{AllocResult, EResult},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
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
#[derive(Clone, Eq, Debug, PartialEq)]
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
	pub fn as_char(&self) -> char {
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

	/// The stack address the child process begins with.
	pub stack: Option<NonNull<c_void>>,
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
	pid: PidHandle,
	/// The ID of the process group.
	pub pgid: Pid,
	/// The thread ID of the process.
	pub tid: Pid,

	/// The argv of the process.
	pub argv: Arc<Vec<String>>,
	/// The environment variables of the process, separated by `\0`.
	pub envp: Arc<String>,
	/// The path to the process's executable.
	pub exec_path: Arc<PathBuf>,

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

	/// A pointer to the parent process.
	parent: Option<Arc<IntMutex<Process>>>,
	/// The list of children processes.
	children: Vec<Pid>,
	/// The list of processes in the process group.
	process_group: Vec<Pid>,

	/// Tells whether the process has information that can be retrieved by
	/// wait/waitpid.
	waitable: bool,

	/// Structure managing the process's timers. This manager is shared between all threads of the
	/// same process.
	timer_manager: Arc<Mutex<TimerManager>>,

	/// Kernel stack pointer of saved context.
	kernel_sp: AtomicPtr<u8>,

	/// The virtual memory of the process.
	mem_space: Option<Arc<IntMutex<MemSpace>>>,
	/// A pointer to the kernelspace stack.
	kernel_stack: NonNull<u8>,

	/// Current working directory
	///
	/// The field contains both the path and the directory.
	pub cwd: Arc<vfs::Entry>,
	/// Current root path used by the process
	pub chroot: Arc<vfs::Entry>,
	/// The list of open file descriptors with their respective ID.
	pub file_descriptors: Option<Arc<Mutex<FileDescriptorTable>>>,

	/// A bitfield storing the set of blocked signals.
	pub sigmask: SigSet,
	/// A bitfield storing the set of pending signals.
	sigpending: SigSet,
	/// The list of signal handlers.
	pub signal_handlers: Arc<Mutex<[SignalHandler; signal::SIGNALS_COUNT]>>,

	/// TLS entries.
	pub tls_entries: [gdt::Entry; TLS_ENTRIES_COUNT],

	/// The process's resources usage.
	rusage: RUsage,

	/// The exit status of the process after exiting.
	exit_status: ExitStatus,
	/// The terminating signal.
	termsig: u8,
}

/// Initializes processes system. This function must be called only once, at
/// kernel initialization.
pub(crate) fn init() -> EResult<()> {
	tss::init();
	scheduler::init()?;
	// Register interruption callbacks
	let callback = |id: u32, _code: u32, frame: &mut IntFrame, ring: u8| {
		if ring < 3 {
			return CallbackResult::Panic;
		}
		// Get process
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		match id {
			// Divide-by-zero
			// x87 Floating-Point Exception
			// SIMD Floating-Point Exception
			0x00 | 0x10 | 0x13 => proc.kill(Signal::SIGFPE),
			// Breakpoint
			0x03 => proc.kill(Signal::SIGTRAP),
			// Invalid Opcode
			0x06 => proc.kill(Signal::SIGILL),
			// General Protection Fault
			0x0d => {
				// Get the instruction opcode
				let ptr = SyscallPtr::<u8>::from_syscall_arg(frame.get_program_counter());
				let opcode = ptr.copy_from_user();
				// If the instruction is `hlt`, exit
				if opcode == Ok(Some(HLT_INSTRUCTION)) {
					proc.exit(frame.get_syscall_id() as _);
				} else {
					proc.kill(Signal::SIGSEGV);
				}
			}
			// Alignment Check
			0x11 => proc.kill(Signal::SIGBUS),
			_ => {}
		}
		CallbackResult::Continue
	};
	let page_fault_callback = |_id: u32, code: u32, frame: &mut IntFrame, ring: u8| {
		let accessed_addr = VirtAddr(register_get!("cr2"));
		let pc = frame.get_program_counter();
		// Get current process
		let Some(curr_proc) = Process::current_opt() else {
			return CallbackResult::Panic;
		};
		let mut curr_proc = curr_proc.lock();
		// Check access
		let success = {
			let Some(mem_space_mutex) = curr_proc.get_mem_space() else {
				return CallbackResult::Panic;
			};
			let mut mem_space = mem_space_mutex.lock();
			mem_space.handle_page_fault(accessed_addr, code)
		};
		if !success {
			if ring < 3 {
				// Check if the fault was caused by a user <-> kernel copy
				if (copy::raw_copy as usize..copy::copy_fault as usize).contains(&pc) {
					// Jump to `copy_fault`
					frame.set_program_counter(copy::copy_fault as usize);
				} else {
					return CallbackResult::Panic;
				}
			} else {
				curr_proc.kill(Signal::SIGSEGV);
			}
		}
		CallbackResult::Continue
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

impl Process {
	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(pid: Pid) -> Option<Arc<IntMutex<Self>>> {
		SCHEDULER.get().lock().get_by_pid(pid)
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(tid: Pid) -> Option<Arc<IntMutex<Self>>> {
		SCHEDULER.get().lock().get_by_tid(tid)
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function returns `None`.
	pub fn current_opt() -> Option<Arc<IntMutex<Self>>> {
		SCHEDULER.get().lock().get_current_process()
	}

	/// Returns the current running process.
	///
	/// If no process is running, the function makes the kernel panic.
	pub fn current() -> Arc<IntMutex<Self>> {
		Self::current_opt().expect("no running process")
	}

	/// Creates the init process and places it into the scheduler's queue.
	///
	/// The process is set to state [`State::Running`] by default and has user root.
	pub fn init() -> EResult<Arc<IntMutex<Self>>> {
		let rs = ResolutionSettings::kernel_follow();
		// Create the default file descriptors table
		let file_descriptors = {
			let mut fds_table = FileDescriptorTable::default();
			let tty_path = PathBuf::try_from(TTY_DEVICE_PATH.as_bytes())?;
			let tty_file = vfs::get_file_from_path(&tty_path, &rs)?;
			let tty_file = File::open_entry(tty_file, O_RDWR)?;
			let (stdin_fd_id, _) = fds_table.create_fd(0, tty_file)?;
			assert_eq!(stdin_fd_id, STDIN_FILENO);
			fds_table.duplicate_fd(
				STDIN_FILENO as _,
				NewFDConstraint::Fixed(STDOUT_FILENO as _),
				false,
			)?;
			fds_table.duplicate_fd(
				STDIN_FILENO as _,
				NewFDConstraint::Fixed(STDERR_FILENO as _),
				false,
			)?;
			fds_table
		};
		let root_dir = vfs::get_file_from_path(Path::root(), &rs)?;
		let pid = PidHandle::init()?;
		let process = Self {
			pid,
			pgid: pid::INIT_PID,
			tid: pid::INIT_PID,

			argv: Arc::new(Vec::new())?,
			envp: Arc::new(String::new())?,
			exec_path: Arc::new(PathBuf::root()?)?,

			access_profile: rs.access_profile,
			umask: DEFAULT_UMASK,

			state: State::Running,
			vfork_state: VForkState::None,

			priority: 0,
			nice: 0,

			parent: None,
			children: Vec::new(),
			process_group: Vec::new(),

			waitable: false,

			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid::INIT_PID)?))?,

			kernel_sp: AtomicPtr::default(),

			mem_space: None,
			kernel_stack: buddy::alloc_kernel(KERNEL_STACK_ORDER)?,

			cwd: root_dir.clone(),
			chroot: root_dir,
			file_descriptors: Some(Arc::new(Mutex::new(file_descriptors))?),

			sigmask: Default::default(),
			sigpending: Default::default(),
			signal_handlers: Arc::new(Mutex::new(Default::default()))?,

			tls_entries: [gdt::Entry::default(); TLS_ENTRIES_COUNT],

			rusage: RUsage::default(),

			exit_status: 0,
			termsig: 0,
		};
		Ok(SCHEDULER.get().lock().add_process(process)?)
	}

	/// Returns the process's ID.
	pub fn get_pid(&self) -> u16 {
		self.pid.get()
	}

	/// Tells whether the process is the init process.
	#[inline(always)]
	pub fn is_init(&self) -> bool {
		self.pid.get() == pid::INIT_PID
	}

	/// Tells whether the process is among a group and is not its owner.
	#[inline(always)]
	pub fn is_in_group(&self) -> bool {
		self.pgid != 0 && self.pgid != self.pid.get()
	}

	/// Sets the process's group ID to the given value `pgid`, updating the associated group.
	pub fn set_pgid(&mut self, pgid: Pid) -> EResult<()> {
		let old_pgid = self.pgid;
		let new_pgid = if pgid == 0 { self.pid.get() } else { pgid };
		if old_pgid == new_pgid {
			return Ok(());
		}
		if new_pgid != self.pid.get() {
			// Add the process to the new group
			let Some(proc_mutex) = Process::get_by_pid(new_pgid) else {
				return Err(errno!(ESRCH));
			};
			let mut new_group_process = proc_mutex.lock();
			let i = new_group_process
				.process_group
				.binary_search(&self.pid.get())
				.unwrap_err();
			new_group_process.process_group.insert(i, self.pid.get())?;
		}
		// Remove the process from its former group
		if self.is_in_group() {
			if let Some(proc_mutex) = Process::get_by_pid(old_pgid) {
				let mut old_group_process = proc_mutex.lock();
				if let Ok(i) = old_group_process
					.process_group
					.binary_search(&self.pid.get())
				{
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
			.map(|parent| parent.lock().pid.get())
			.unwrap_or(self.pid.get())
	}

	/// Returns the process's current state.
	#[inline(always)]
	pub fn get_state(&self) -> State {
		self.state.clone()
	}

	/// Sets the process's state to `new_state`.
	pub fn set_state(&mut self, new_state: State) {
		if self.state == new_state || self.state == State::Zombie {
			return;
		}
		// Update the number of running processes
		if self.state != State::Running && new_state == State::Running {
			SCHEDULER.get().lock().increment_running();
		} else if self.state == State::Running {
			SCHEDULER.get().lock().decrement_running();
		}
		self.state = new_state;
		if self.state == State::Zombie {
			if self.is_init() {
				panic!("Terminated init process!");
			}
			// Remove the memory space and file descriptors table to save memory
			//self.mem_space = None; // TODO Handle the case where the memory space is bound
			self.file_descriptors = None;
			// Attach every child to the init process
			let init_proc_mutex = Process::get_by_pid(pid::INIT_PID).unwrap();
			let mut init_proc = init_proc_mutex.lock();
			let children = mem::take(&mut self.children);
			for child_pid in children {
				// Check just in case
				if child_pid == self.pid.get() {
					continue;
				}
				if let Some(child_mutex) = Process::get_by_pid(child_pid) {
					child_mutex.lock().parent = Some(init_proc_mutex.clone());
					oom::wrap(|| init_proc.add_child(child_pid));
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
		if let Some(parent) = &self.parent {
			let mut parent = parent.lock();
			parent.kill(Signal::SIGCHLD);
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
	pub fn get_parent(&self) -> Option<Arc<IntMutex<Process>>> {
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

	/// Returns the last known userspace registers state.
	///
	/// This information is stored at the beginning of the process's interrupt stack.
	pub fn user_regs(&self) -> IntFrame {
		todo!()
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
		// Set kernel stack pointer
		unsafe {
			let kernel_stack_begin = self
				.kernel_stack
				.as_ptr()
				.add(buddy::get_frame_size(KERNEL_STACK_ORDER));
			TSS.set_kernel_stack(kernel_stack_begin);
		}
	}

	/// Returns the exit status if the process has ended.
	#[inline(always)]
	pub fn get_exit_status(&self) -> Option<ExitStatus> {
		matches!(self.state, State::Zombie).then_some(self.exit_status)
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
	/// `fork_options` are the options for the fork operation.
	///
	/// On fail, the function returns an `Err` with the appropriate Errno.
	///
	/// If the process is not running, the behaviour is undefined.
	pub fn fork(
		this: Arc<IntMutex<Self>>,
		fork_options: ForkOptions,
	) -> EResult<Arc<IntMutex<Self>>> {
		let mut proc = this.lock();
		debug_assert!(matches!(proc.get_state(), State::Running));
		// Handle vfork
		let vfork_state = if fork_options.vfork {
			proc.vfork_state = VForkState::Waiting; // TODO Cancel if the following code fails
			VForkState::Executing
		} else {
			VForkState::None
		};
		// Clone memory space
		let mem_space = {
			let curr_mem_space = proc.get_mem_space().unwrap();
			if fork_options.share_memory || fork_options.vfork {
				curr_mem_space.clone()
			} else {
				Arc::new(IntMutex::new(curr_mem_space.lock().fork()?))?
			}
		};
		// Clone file descriptors
		let file_descriptors = if fork_options.share_fd {
			proc.file_descriptors.clone()
		} else {
			proc.file_descriptors
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
			proc.signal_handlers.clone()
		} else {
			Arc::new(Mutex::new(proc.signal_handlers.lock().clone()))?
		};
		let pid = PidHandle::unique()?;
		let pid_int = pid.get();
		let process = Self {
			pid,
			pgid: proc.pgid,
			tid: pid_int,

			argv: proc.argv.clone(),
			envp: proc.envp.clone(),
			exec_path: proc.exec_path.clone(),

			access_profile: proc.access_profile,
			umask: proc.umask,

			state: State::Running,
			vfork_state,

			priority: proc.priority,
			nice: proc.nice,

			parent: Some(this.clone()),
			children: Vec::new(),
			process_group: Vec::new(),

			waitable: false,

			// TODO if creating a thread: timer_manager: proc.timer_manager.clone(),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid_int)?))?,

			kernel_sp,

			mem_space: Some(mem_space),
			kernel_stack: buddy::alloc_kernel(KERNEL_STACK_ORDER)?,

			cwd: proc.cwd.clone(),
			chroot: proc.chroot.clone(),
			file_descriptors,

			sigmask: proc.sigmask,
			sigpending: Default::default(),
			signal_handlers,

			tls_entries: proc.tls_entries,

			rusage: RUsage::default(),

			exit_status: proc.exit_status,
			termsig: 0,
		};
		proc.add_child(pid_int)?;
		Ok(SCHEDULER.get().lock().add_process(process)?)
	}

	/// Kills the process with the given signal `sig`.
	///
	/// If the process doesn't have a signal handler, the default action for the signal is
	/// executed.
	pub fn kill(&mut self, sig: Signal) {
		// Cannot kill a zombie process
		if unlikely(self.state == State::Zombie) {
			return;
		}
		// Ignore blocked signals
		if sig.can_catch() && self.sigmask.is_set(sig.get_id() as _) {
			return;
		}
		// Statistics
		self.rusage.ru_nsignals = self.rusage.ru_nsignals.saturating_add(1);
		// If the signal's action can be executed now, do it
		{
			let handlers = self.signal_handlers.clone();
			let handlers = handlers.lock();
			let handler = &handlers[sig.get_id() as usize];
			match handler {
				SignalHandler::Ignore => return,
				SignalHandler::Default
					if self.state != State::Stopped
						|| sig.get_default_action() == SignalAction::Continue =>
				{
					sig.get_default_action().exec(sig, self);
					return;
				}
				_ => {}
			}
		}
		// Resume execution if necessary
		if matches!(self.state, State::Sleeping)
			|| (matches!(self.state, State::Stopped)
				&& sig.get_default_action() == SignalAction::Continue)
		{
			self.set_state(State::Running);
		}
		// Set the signal as pending
		self.sigpending.set(sig.get_id() as _);
	}

	/// Kills every process in the process group.
	pub fn kill_group(&mut self, sig: Signal) {
		self.process_group
			.iter()
			// Avoid deadlock
			.filter(|pid| **pid != self.pid.get())
			.filter_map(|pid| Process::get_by_pid(*pid))
			.for_each(|proc_mutex| {
				let mut proc = proc_mutex.lock();
				proc.kill(sig);
			});
		self.kill(sig);
	}

	/// Tells whether the given signal is blocked by the process.
	pub fn is_signal_blocked(&self, sig: Signal) -> bool {
		self.sigmask.is_set(sig.get_id() as _)
	}

	/// Returns the ID of the next signal to be handled.
	///
	/// If `peek` is `false`, the signal is cleared from the bitfield.
	///
	/// If no signal is pending, the function returns `None`.
	pub fn next_signal(&mut self, peek: bool) -> Option<Signal> {
		let sig = self
			.sigpending
			.iter()
			.enumerate()
			.filter(|(_, b)| *b)
			.filter_map(|(i, _)| {
				let s = Signal::try_from(i as c_int).ok()?;
				(!s.can_catch() || !self.sigmask.is_set(i)).then_some(s)
			})
			.next();
		if !peek {
			if let Some(id) = sig {
				self.sigpending.clear(id.get_id() as _);
			}
		}
		sig
	}

	/// Updates the `n`th TLS entry in the GDT.
	///
	/// If `n` is out of bounds, the function does nothing.
	///
	/// This function does not flush the GDT's cache. Thus, it is the caller's responsibility.
	pub fn update_tls(&self, n: usize) {
		if let Some(ent) = self.tls_entries.get(n) {
			unsafe {
				ent.update_gdt(gdt::TLS_OFFSET + n * size_of::<gdt::Entry>());
			}
		}
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
		// Reset the parent's vfork state if needed
		if let Some(parent) = &self.parent {
			let mut parent = parent.lock();
			parent.vfork_state = VForkState::None;
		}
	}

	/// Exits the process with the given `status`.
	///
	/// This function changes the process's status to `Zombie`.
	pub fn exit(&mut self, status: u32) {
		#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] exited with status `{status}`",
			pid = self.pid.get()
		);
		self.exit_status = status as ExitStatus;
		self.set_state(State::Zombie);
		self.reset_vfork();
		self.set_waitable(0);
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
		let mut score: u16 = 0;
		// TODO Compute the score using physical memory usage
		// TODO Take into account userspace-set values (oom may be disabled for this
		// process, an absolute score or a bonus might be given, etc...)
		// If the process is owned by the superuser, give it a bonus
		if self.access_profile.is_privileged() {
			score = score.saturating_sub(100);
		}
		score
	}
}

impl fmt::Debug for Process {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Process")
			.field("pid", &self.pid.get())
			.finish()
	}
}

impl AccessProfile {
	/// Tells whether the agent can kill the process.
	pub fn can_kill(&self, proc: &Process) -> bool {
		// if privileged
		if self.is_privileged() {
			return true;
		}
		// if sender's `uid` or `euid` equals receiver's `uid` or `suid`
		self.uid == proc.access_profile.uid
			|| self.uid == proc.access_profile.suid
			|| self.euid == proc.access_profile.uid
			|| self.euid == proc.access_profile.suid
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		if self.is_init() {
			panic!("Terminated init process!");
		}
		// Free kernel stack
		unsafe {
			buddy::free_kernel(self.kernel_stack.as_ptr(), KERNEL_STACK_ORDER);
		}
	}
}

/// Before returning to userspace from the current context, this function checks the state of the
/// current process to potentially alter the execution flow.
///
/// Arguments:
/// - `ring` is the ring the current context is returning to.
/// - `frame` is the interrupt frame.
///
/// The execution flow can be altered by:
/// - The process is no longer in [`State::Running`] state
/// - A signal handler has to be executed
///
/// This function never returns in case the process state turns to [`State::Zombie`].
pub fn yield_current(ring: u8, frame: &mut IntFrame) {
	// If returning to kernelspace, do nothing
	if ring < 3 {
		return;
	}
	let proc_mutex = Process::current();
	let mut proc = proc_mutex.lock();
	// If the process is not running anymore, stop execution
	if proc.state != State::Running {
		Scheduler::tick();
	}
	// If no signal is pending, return
	let Some(sig) = proc.next_signal(false) else {
		return;
	};
	// Prepare signal for execution
	let handlers = proc.signal_handlers.clone();
	let handlers = handlers.lock();
	handlers[sig.get_id() as usize].exec(sig, &mut proc, frame);
	// Alter the execution flow of the current context according to the new state of the
	// process
	match proc.state {
		State::Running => {}
		State::Sleeping | State::Stopped | State::Zombie => Scheduler::tick(),
	}
}
