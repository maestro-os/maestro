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

pub mod exec;
pub mod mem_space;
pub mod pid;
pub mod rusage;
pub mod scheduler;
pub mod signal;
pub mod user_desc;

use crate::{
	arch::x86::{cli, gdt, idt, idt::IntFrame, tss, FxState},
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
	memory::{buddy, buddy::FrameOrder, oom, VirtAddr},
	process::{
		mem_space::{copy, copy::SyscallPtr},
		pid::{PidHandle, IDLE_PID, INIT_PID},
		rusage::Rusage,
		scheduler::{
			switch,
			switch::{idle_task, KThreadEntry},
			Scheduler, SCHEDULER,
		},
		signal::SigSet,
	},
	register_get,
	sync::mutex::{IntMutex, Mutex},
	syscall::FromSyscallArg,
	time::timer::TimerManager,
};
use core::{
	ffi::c_int,
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	mem,
	mem::ManuallyDrop,
	ptr::NonNull,
	sync::atomic::{
		AtomicBool, AtomicPtr, AtomicU32, AtomicU8,
		Ordering::{Acquire, Relaxed, Release, SeqCst},
	},
};
use mem_space::MemSpace;
use pid::Pid;
use signal::{Signal, SignalHandler};
use utils::{
	collections::{
		path::{Path, PathBuf},
		vec::Vec,
	},
	errno,
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
	unsafe_mut::UnsafeMut,
};

/// The opcode of the `hlt` instruction.
const HLT_INSTRUCTION: u8 = 0xf4;

/// The path to the TTY device file.
const TTY_DEVICE_PATH: &str = "/dev/tty";

/// The default file creation mask.
const DEFAULT_UMASK: file::Mode = 0o022;

/// The size of the userspace stack of a process in number of pages.
const USER_STACK_SIZE: usize = 2048;
/// The size of the kernelspace stack of a process in number of pages.
const KERNEL_STACK_ORDER: FrameOrder = 4;

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
#[repr(u8)]
#[derive(Clone, Copy, Eq, Debug, PartialEq)]
pub enum State {
	/// The process is running or waiting to run.
	Running = 0,
	/// The process is waiting for an event.
	Sleeping = 1,
	/// The process has been stopped by a signal or by tracing.
	Stopped = 2,
	/// The process has been killed.
	Zombie = 3,
}

impl State {
	/// Returns the state with the given ID.
	fn from_id(id: u8) -> Self {
		match id {
			0 => Self::Running,
			1 => Self::Sleeping,
			2 => Self::Stopped,
			3 => Self::Zombie,
			_ => unreachable!(),
		}
	}

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
}

/// Wrapper for the kernel stack, allowing to free it on drop.
struct KernelStack(NonNull<u8>);

impl KernelStack {
	/// Allocates a new stack.
	pub fn new() -> AllocResult<Self> {
		buddy::alloc_kernel(KERNEL_STACK_ORDER, 0).map(Self)
	}

	/// Returns a pointer to the top of the stack.
	#[inline]
	pub fn top(&self) -> NonNull<u8> {
		unsafe { self.0.add(buddy::get_frame_size(KERNEL_STACK_ORDER)) }
	}
}

impl Drop for KernelStack {
	fn drop(&mut self) {
		unsafe {
			buddy::free_kernel(self.0.as_ptr(), KERNEL_STACK_ORDER);
		}
	}
}

/// A process's links to other processes.
#[derive(Default)]
pub struct ProcessLinks {
	/// A pointer to the parent process.
	parent: Option<Arc<Process>>,
	/// The list of children processes.
	pub children: Vec<Pid>,
	/// The process's group leader. The PID of the group leader is the PGID of this process.
	///
	/// If `None`, the process is its own leader (to avoid self reference).
	group_leader: Option<Arc<Process>>,
	/// The list of processes in the process group.
	pub process_group: Vec<Pid>,
}

/// A process's filesystem access information.
pub struct ProcessFs {
	/// The process's access profile, containing user and group IDs.
	pub access_profile: AccessProfile,
	/// The process's current umask.
	pub umask: AtomicU32,
	/// Current working directory
	///
	/// The field contains both the path and the directory.
	pub cwd: Arc<vfs::Entry>,
	/// Current root path used by the process
	pub chroot: Arc<vfs::Entry>,
}

impl ProcessFs {
	/// Returns the current umask.
	pub fn umask(&self) -> file::Mode {
		self.umask.load(Acquire)
	}
}

impl Clone for ProcessFs {
	fn clone(&self) -> Self {
		Self {
			access_profile: self.access_profile,
			umask: AtomicU32::new(self.umask.load(Acquire)),
			cwd: self.cwd.clone(),
			chroot: self.chroot.clone(),
		}
	}
}

/// A process's signal management information.
pub struct ProcessSignal {
	/// The list of signal handlers.
	pub handlers: Arc<Mutex<[SignalHandler; signal::SIGNALS_COUNT]>>,
	/// A bitfield storing the set of blocked signals.
	pub sigmask: SigSet,
	/// A bitfield storing the set of pending signals.
	sigpending: SigSet,

	/// The exit status of the process after exiting.
	pub exit_status: ExitStatus,
	/// The terminating signal.
	pub termsig: u8,
}

impl ProcessSignal {
	/// Creates a new instance.
	pub fn new() -> AllocResult<Self> {
		Ok(ProcessSignal {
			handlers: Arc::new(Default::default())?,
			sigmask: Default::default(),
			sigpending: Default::default(),

			exit_status: 0,
			termsig: 0,
		})
	}

	/// Tells whether the given signal is blocked by the process.
	pub fn is_signal_blocked(&self, sig: Signal) -> bool {
		self.sigmask.is_set(sig as _)
	}

	/// Returns the ID of the next signal to be handled, clearing it from the pending signals mask.
	///
	/// If no signal is pending, the function returns `None`.
	pub fn next_signal(&mut self) -> Option<Signal> {
		if self.sigpending.is_empty() {
			return None;
		}
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
		if let Some(id) = sig {
			self.sigpending.clear(id as _);
		}
		sig
	}
}

/// The **Process Control Block** (PCB). This structure stores all the information
/// about a process.
pub struct Process {
	/// The ID of the process.
	pid: PidHandle,
	/// The thread ID of the process.
	pub tid: Pid,

	/// The current state of the process.
	state: AtomicU8,
	/// If `true`, the parent can resume after a `vfork`.
	pub vfork_done: AtomicBool,
	/// The links to other processes.
	pub links: Mutex<ProcessLinks>,

	/// A pointer to the kernelspace stack.
	kernel_stack: KernelStack,
	/// Kernel stack pointer of saved context.
	kernel_sp: AtomicPtr<u8>,
	/// The process's FPU state.
	fpu: Mutex<FxState>,
	/// TLS entries.
	pub tls: Mutex<[gdt::Entry; TLS_ENTRIES_COUNT]>, // TODO rwlock

	/// The virtual memory of the process.
	pub mem_space: UnsafeMut<Option<Arc<IntMutex<MemSpace>>>>,
	/// Filesystem access information.
	pub fs: Mutex<ProcessFs>, // TODO rwlock
	/// The list of open file descriptors with their respective ID.
	pub file_descriptors: UnsafeMut<Option<Arc<Mutex<FileDescriptorTable>>>>,
	/// Process's timers, shared between all threads of the same process.
	pub timer_manager: Arc<Mutex<TimerManager>>,
	/// The process's signal management structure.
	pub signal: Mutex<ProcessSignal>, // TODO rwlock

	/// The process's resources usage.
	pub rusage: Mutex<Rusage>,
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
		let proc = Process::current();
		if unlikely(proc.is_idle_task()) {
			return CallbackResult::Panic;
		}
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
				let ptr = SyscallPtr::<u8>::from_ptr(frame.get_program_counter());
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
		let proc = Process::current();
		let Some(mem_space) = proc.mem_space.as_ref() else {
			return CallbackResult::Panic;
		};
		// Check access
		let sig = mem_space.lock().handle_page_fault(accessed_addr, code);
		match sig {
			Ok(true) => {}
			Ok(false) => {
				if ring < 3 {
					// Check if the fault was caused by a user <-> kernel copy
					if (copy::raw_copy as usize..copy::copy_fault as usize).contains(&pc) {
						// Jump to `copy_fault`
						frame.set_program_counter(copy::copy_fault as usize);
					} else {
						return CallbackResult::Panic;
					}
				} else {
					proc.kill(Signal::SIGSEGV);
				}
			}
			Err(_) => proc.kill(Signal::SIGBUS),
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
	pub fn get_by_pid(pid: Pid) -> Option<Arc<Self>> {
		SCHEDULER.lock().get_by_pid(pid)
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(tid: Pid) -> Option<Arc<Self>> {
		SCHEDULER.lock().get_by_tid(tid)
	}

	/// Returns the current running process.
	pub fn current() -> Arc<Self> {
		SCHEDULER.lock().get_current_process()
	}

	/// Creates a kernel thread.
	///
	/// Arguments:
	/// - `pid` is the PID to use. If `None`, an available PID is allocated
	/// - `queue` tells whether the thread shall be added to the scheduler's queue
	/// - `entry` is entry point of the newly created thread
	pub fn new_kthread(
		pid: Option<Pid>,
		entry: KThreadEntry,
		queue: bool,
	) -> AllocResult<Arc<Self>> {
		let pid = match pid {
			Some(pid) => PidHandle::mark_used(pid)?,
			None => PidHandle::unique()?,
		};
		let tid = *pid;
		let kernel_stack = KernelStack::new()?;
		let kernel_sp = unsafe { switch::init_kthread(kernel_stack.top(), entry) };
		let thread = Arc::new(Self {
			pid,
			tid,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Default::default(),

			kernel_stack,
			kernel_sp: AtomicPtr::new(kernel_sp),
			fpu: Mutex::new(FxState([0; 512])),
			tls: Default::default(),

			// TODO this is not needed. find a way to avoid init
			mem_space: Default::default(),
			fs: Mutex::new(ProcessFs {
				access_profile: AccessProfile::KERNEL,
				umask: Default::default(),
				cwd: vfs::ROOT.clone(),
				chroot: vfs::ROOT.clone(),
			}),
			file_descriptors: Default::default(),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(0)?))?,
			signal: Mutex::new(ProcessSignal::new()?),

			rusage: Default::default(),
		})?;
		if queue {
			SCHEDULER.lock().add_process(thread.clone())?;
		}
		Ok(thread)
	}

	/// Creates an idle task.
	///
	/// The idle task is a special process, running in kernelspace, used by the scheduler when no
	/// task is ready to run.
	#[inline]
	pub(crate) fn idle_task() -> AllocResult<Arc<Self>> {
		Self::new_kthread(Some(0), || unsafe { idle_task() }, false)
	}

	/// Creates the init process and places it into the scheduler's queue.
	///
	/// The process is set to state [`State::Running`] by default and has user root.
	pub fn init() -> EResult<Arc<Self>> {
		let rs = ResolutionSettings::kernel_follow();
		// Create the default file descriptors table
		let file_descriptors = {
			let mut fds_table = FileDescriptorTable::default();
			let tty_path = PathBuf::try_from(TTY_DEVICE_PATH.as_bytes())?;
			let tty_ent = vfs::get_file_from_path(&tty_path, &rs)?;
			let tty_file = File::open_entry(tty_ent, O_RDWR)?;
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
		let proc = Arc::new(Self {
			pid: PidHandle::mark_used(INIT_PID)?,
			tid: INIT_PID,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Mutex::new(ProcessLinks::default()),

			kernel_stack: KernelStack::new()?,
			kernel_sp: AtomicPtr::default(),
			fpu: Mutex::new(FxState([0; 512])),
			tls: Default::default(),

			mem_space: UnsafeMut::new(None),
			fs: Mutex::new(ProcessFs {
				access_profile: rs.access_profile,
				umask: AtomicU32::new(DEFAULT_UMASK),
				cwd: root_dir.clone(),
				chroot: root_dir,
			}),
			file_descriptors: UnsafeMut::new(Some(Arc::new(Mutex::new(file_descriptors))?)),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(INIT_PID)?))?,
			signal: Mutex::new(ProcessSignal {
				handlers: Arc::new(Default::default())?,
				sigmask: Default::default(),
				sigpending: Default::default(),

				exit_status: 0,
				termsig: 0,
			}),

			rusage: Default::default(),
		})?;
		SCHEDULER.lock().add_process(proc.clone())?;
		Ok(proc)
	}

	/// Returns the process's ID.
	#[inline]
	pub fn get_pid(&self) -> Pid {
		*self.pid
	}

	/// Tells whether the process is an idle task.
	pub fn is_idle_task(&self) -> bool {
		*self.pid == IDLE_PID
	}

	/// Tells whether the process is the init process.
	#[inline(always)]
	pub fn is_init(&self) -> bool {
		*self.pid == INIT_PID
	}

	/// Returns the process group ID.
	pub fn get_pgid(&self) -> Pid {
		self.links
			.lock()
			.group_leader
			.as_ref()
			.map(|p| p.get_pid())
			.unwrap_or(self.get_pid())
	}

	/// Sets the process's group ID to the given value `pgid`, updating the associated group.
	pub fn set_pgid(&self, pgid: Pid) -> EResult<()> {
		let pid = self.get_pid();
		let new_leader = (pgid != 0 && pgid != pid)
			.then(|| Process::get_by_pid(pgid).ok_or_else(|| errno!(ESRCH)))
			.transpose()?;
		let mut links = self.links.lock();
		let old_leader = mem::replace(&mut links.group_leader, new_leader.clone());
		// Remove process from the old group's list
		if let Some(leader) = old_leader {
			let mut links = leader.links.lock();
			if let Ok(i) = links.process_group.binary_search(&pid) {
				links.process_group.remove(i);
			}
		}
		// Add process to the new group's list
		if let Some(leader) = new_leader {
			let mut links = leader.links.lock();
			if let Err(i) = links.process_group.binary_search(&pid) {
				oom::wrap(|| links.process_group.insert(i, pid));
			}
		}
		Ok(())
	}

	/// The function tells whether the process is in an orphaned process group.
	pub fn is_in_orphan_process_group(&self) -> bool {
		self.links
			.lock()
			.group_leader
			.as_ref()
			.map(|group_leader| group_leader.get_state() == State::Zombie)
			.unwrap_or(false)
	}

	/// Returns the parent process's PID.
	pub fn get_parent_pid(&self) -> Pid {
		self.links
			.lock()
			.parent
			.as_ref()
			.map(|parent| parent.get_pid())
			.unwrap_or(self.get_pid())
	}

	/// Adds the process with the given PID `pid` as child to the process.
	pub fn add_child(&self, pid: Pid) -> AllocResult<()> {
		let mut links = self.links.lock();
		let i = links.children.binary_search(&pid).unwrap_or_else(|i| i);
		links.children.insert(i, pid)
	}

	/// Unlinks the process from its parent and group.
	pub fn unlink(&self) {
		let (parent, group_leader) = {
			let mut links = self.links.lock();
			(links.parent.take(), links.group_leader.take())
		};
		if let Some(parent) = parent {
			let mut links = parent.links.lock();
			if let Ok(i) = links.children.binary_search(&self.get_pid()) {
				links.children.remove(i);
			}
		}
		if let Some(group_leader) = group_leader {
			let mut links = group_leader.links.lock();
			if let Ok(i) = links.process_group.binary_search(&self.get_pid()) {
				links.process_group.remove(i);
			}
		}
	}

	/// Returns the process's current state.
	///
	/// **Note**: since the process cannot be locked, this function may cause data races. Use with
	/// caution
	#[inline(always)]
	pub fn get_state(&self) -> State {
		let id = self.state.load(Relaxed);
		State::from_id(id)
	}

	/// Sets the process's state to `new_state`.
	///
	/// If the transition from the previous state to `new_state` is invalid, the function does
	/// nothing.
	pub fn set_state(&self, new_state: State) {
		// Disable interruptions to ensure the function can finish before the scheduler switches
		// context (and thus never resume if the new state is `Zombie`)
		idt::wrap_disable_interrupts(|| {
			let Ok(old_state) = self.state.fetch_update(Release, Acquire, |old_state| {
				let old_state = State::from_id(old_state);
				let valid = matches!(
					(old_state, new_state),
					(State::Running | State::Sleeping, _) | (State::Stopped, State::Running)
				);
				valid.then_some(new_state as u8)
			}) else {
				// Invalid transition, do nothing
				return;
			};
			let old_state = State::from_id(old_state);
			if new_state == old_state {
				return;
			}
			#[cfg(feature = "strace")]
			println!(
				"[strace {pid}] changed state: {old_state:?} -> {new_state:?}",
				pid = self.get_pid()
			);
			// Update the number of running processes
			if new_state == State::Running {
				SCHEDULER.lock().increment_running();
			} else if old_state == State::Running {
				SCHEDULER.lock().decrement_running();
			}
			if new_state == State::Zombie {
				if self.is_init() {
					panic!("Terminated init process!");
				}
				// Remove the memory space and file descriptors table to reclaim memory
				unsafe {
					//self.mem_space = None; // TODO Handle the case where the memory space is
					// bound
					*self.file_descriptors.get_mut() = None;
				}
				// Attach every child to the init process
				let init_proc = Process::get_by_pid(INIT_PID).unwrap();
				let children = mem::take(&mut self.links.lock().children);
				for child_pid in children {
					// Check just in case
					if child_pid == *self.pid {
						continue;
					}
					// TODO do the same for process group members
					if let Some(child) = Process::get_by_pid(child_pid) {
						child.links.lock().parent = Some(init_proc.clone());
						oom::wrap(|| init_proc.add_child(child_pid));
					}
				}
				// Set vfork as done just in case
				self.vfork_wake();
			}
			// Send SIGCHLD
			if matches!(new_state, State::Running | State::Stopped | State::Zombie) {
				let links = self.links.lock();
				if let Some(parent) = &links.parent {
					parent.kill(Signal::SIGCHLD);
				}
			}
		});
	}

	/// Tells whether there is a pending signal on the process.
	pub fn has_pending_signal(&self) -> bool {
		let signal = self.signal.lock();
		signal.sigpending.0 & !signal.sigmask.0 != 0
	}

	/// Wakes up the process if in [`State::Sleeping`] state.
	pub fn wake(&self) {
		// TODO make sure the ordering is right
		let res = self.state.fetch_update(SeqCst, SeqCst, |old_state| {
			(old_state == State::Sleeping as _).then_some(State::Running as _)
		});
		#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] changed state: {old_state:?} -> {new_state:?}",
			old_state = State::Sleeping,
			new_state = State::Running,
			pid = self.get_pid()
		);
		// Update the number of running processes
		if res.is_ok() {
			SCHEDULER.lock().increment_running();
		}
	}

	/// Signals the parent that the `vfork` operation has completed.
	pub fn vfork_wake(&self) {
		self.vfork_done.store(true, Release);
		let links = self.links.lock();
		if let Some(parent) = &links.parent {
			parent.set_state(State::Running);
		}
	}

	/// Tells whether the vfork operation has completed.
	#[inline]
	pub fn is_vfork_done(&self) -> bool {
		self.vfork_done.load(Relaxed)
	}

	/// Reads the last known userspace registers state.
	///
	/// This information is stored at the beginning of the process's interrupt stack.
	#[inline]
	pub fn user_regs(&self) -> IntFrame {
		// (x86) The frame will always be complete since entering the stack from the beginning can
		// only be done from userspace, thus the stack pointer and segment are present for `iret`
		unsafe {
			self.kernel_stack
				.top()
				.cast::<IntFrame>()
				.sub(1)
				.read_volatile()
		}
	}

	/// Forks the current process.
	///
	/// Arguments:
	/// - `this` is the parent process.
	/// - `fork_options` are the options for the fork operation.
	///
	/// On fail, the function returns an error.
	///
	/// If the `this` is not running, the behaviour is undefined.
	pub fn fork(this: Arc<Self>, fork_options: ForkOptions) -> EResult<Arc<Self>> {
		debug_assert!(matches!(this.get_state(), State::Running));
		let pid = PidHandle::unique()?;
		let pid_int = *pid;
		// Clone memory space
		let mem_space = {
			let curr_mem_space = this.mem_space.as_ref().unwrap();
			if fork_options.share_memory {
				curr_mem_space.clone()
			} else {
				Arc::new(IntMutex::new(curr_mem_space.lock().fork()?))?
			}
		};
		// Clone file descriptors
		let file_descriptors = if fork_options.share_fd {
			this.file_descriptors.get().clone()
		} else {
			this.file_descriptors
				.as_ref()
				.map(|fds| -> EResult<_> {
					let fds = fds.lock();
					let new_fds = fds.duplicate(false)?;
					Ok(Arc::new(Mutex::new(new_fds))?)
				})
				.transpose()?
		};
		// Clone signal handlers
		let signal_handlers = {
			let signal_manager = this.signal.lock();
			if fork_options.share_sighand {
				signal_manager.handlers.clone()
			} else {
				let handlers = signal_manager.handlers.lock().clone();
				Arc::new(Mutex::new(handlers))?
			}
		};
		let group_leader = this
			.links
			.lock()
			.group_leader
			.clone()
			.unwrap_or_else(|| this.clone());
		let proc = Arc::new(Self {
			pid,
			tid: pid_int,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Mutex::new(ProcessLinks {
				parent: Some(this.clone()),
				group_leader: Some(group_leader.clone()),
				..Default::default()
			}),

			kernel_stack: KernelStack::new()?,
			kernel_sp: AtomicPtr::default(),
			fpu: Mutex::new(this.fpu.lock().clone()),
			tls: Mutex::new(*this.tls.lock()),

			mem_space: UnsafeMut::new(Some(mem_space)),
			fs: Mutex::new(this.fs.lock().clone()),
			file_descriptors: UnsafeMut::new(file_descriptors),
			// TODO if creating a thread: timer_manager: this.timer_manager.clone(),
			timer_manager: Arc::new(Mutex::new(TimerManager::new(pid_int)?))?,
			signal: Mutex::new(ProcessSignal {
				handlers: signal_handlers,
				sigmask: this.signal.lock().sigmask,
				sigpending: Default::default(),

				exit_status: 0,
				termsig: 0,
			}),

			rusage: Default::default(),
		})?;
		// TODO on failure, must undo
		this.add_child(pid_int)?;
		{
			let mut links = group_leader.links.lock();
			if let Err(i) = links.process_group.binary_search(&pid_int) {
				links.process_group.insert(i, pid_int)?;
			}
		}
		SCHEDULER.lock().add_process(proc.clone())?;
		Ok(proc)
	}

	/// Kills the process with the given signal `sig`.
	///
	/// If the process doesn't have a signal handler, the default action for the signal is
	/// executed.
	pub fn kill(&self, sig: Signal) {
		let mut signal_manager = self.signal.lock();
		// Ignore blocked signals
		if sig.can_catch() && signal_manager.sigmask.is_set(sig as _) {
			return;
		}
		// Statistics
		self.rusage.lock().ru_nsignals += 1;
		#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] received signal `{sig}`",
			pid = self.get_pid(),
			sig = sig as c_int
		);
		signal_manager.sigpending.set(sig as _);
	}

	/// Kills every process in the process group.
	pub fn kill_group(&self, sig: Signal) {
		self.links
			.lock()
			.process_group
			.iter()
			.filter_map(|pid| Process::get_by_pid(*pid))
			.for_each(|proc| {
				proc.kill(sig);
			});
		self.kill(sig);
	}

	/// Exits the process with the given `status`.
	///
	/// This function changes the process's status to `Zombie`.
	pub fn exit(&self, status: u32) {
		#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] exited with status `{status}`",
			pid = *self.pid
		);
		self.signal.lock().exit_status = status as ExitStatus;
		self.set_state(State::Zombie);
	}
}

impl fmt::Debug for Process {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Process").field("pid", &self.pid).finish()
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
		let fs = proc.fs.lock();
		self.uid == fs.access_profile.uid
			|| self.uid == fs.access_profile.suid
			|| self.euid == fs.access_profile.uid
			|| self.euid == fs.access_profile.suid
	}
}

impl Drop for Process {
	fn drop(&mut self) {
		if self.is_init() {
			panic!("Terminated init process!");
		}
	}
}

/// Returns `true` if the execution shall continue. Else, the execution shall be paused.
fn yield_current_impl(frame: &mut IntFrame) -> bool {
	// Disable interruptions to prevent execution from being stopped before the reference to
	// `Process` is dropped
	cli();
	// If the process is not running anymore, stop execution
	let proc = Process::current();
	if proc.get_state() != State::Running {
		return false;
	}
	// Get signal handler to execute, if any
	let (sig, handler) = {
		let mut signal_manager = proc.signal.lock();
		let Some(sig) = signal_manager.next_signal() else {
			return true;
		};
		let handler = signal_manager.handlers.lock()[sig as usize].clone();
		(sig, handler)
	};
	// Prepare for execution of signal handler
	handler.exec(sig, &proc, frame);
	// If the process is still running, continue execution
	proc.get_state() == State::Running
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
/// This function disables interruptions.
///
/// This function never returns in case the process state turns to [`State::Zombie`].
pub fn yield_current(ring: u8, frame: &mut IntFrame) {
	// If returning to kernelspace, do nothing
	if ring < 3 {
		return;
	}
	// Use a separate function to drop everything, since `Scheduler::tick` may never return
	let cont = yield_current_impl(frame);
	if !cont {
		Scheduler::tick();
	}
}
