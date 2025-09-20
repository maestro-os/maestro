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
	arch::x86::{FxState, gdt, idt::IntFrame, timer},
	file,
	file::{
		File, O_RDWR,
		fd::{FileDescriptorTable, NewFDConstraint},
		perm::AccessProfile,
		vfs,
	},
	int,
	int::CallbackResult,
	memory::{VirtAddr, buddy, buddy::FrameOrder, oom, user, user::UserPtr},
	process::{
		pid::{IDLE_PID, INIT_PID, PidHandle},
		rusage::Rusage,
		scheduler::{
			critical, dequeue, enqueue, switch,
			switch::{KThreadEntry, idle_task, save_segments},
		},
		signal::{AltStack, SIGNALS_COUNT, SigSet},
	},
	register_get,
	sync::{atomic::AtomicU64, rwlock::IntRwLock, spin::Spin},
	syscall::FromSyscallArg,
	time::timer::TimerManager,
};
use core::{
	cmp::Ordering,
	ffi::{c_int, c_void},
	fmt,
	fmt::Formatter,
	hint,
	hint::unlikely,
	mem,
	ops::Deref,
	ptr::NonNull,
	sync::atomic::{
		AtomicBool, AtomicI8, AtomicPtr, AtomicU8, AtomicU16, AtomicU32,
		Ordering::{Acquire, Relaxed, Release},
	},
};
use mem_space::MemSpace;
use pid::Pid;
use scheduler::cpu::{PerCpu, per_cpu};
use signal::{Signal, SignalHandler};
use utils::{
	collections::{
		btreemap::BTreeMap,
		list::ListNode,
		path::{Path, PathBuf},
		vec::Vec,
	},
	errno,
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
	unsafe_mut::UnsafeMut,
};

/// Atomic lock for a process's `state`
const STATE_LOCK: u8 = 1 << 7;

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
	/// The CPU the process is currently running on
	cur_cpu: Option<&'static PerCpu>,
	/// The last CPU the process ran on
	last_cpu: Option<&'static PerCpu>,

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
	/// Current working directory
	///
	/// The field contains both the path and the directory.
	pub cwd: Arc<vfs::Entry>,
	/// Current root path used by the process
	pub chroot: Arc<vfs::Entry>,
}

impl Clone for ProcessFs {
	fn clone(&self) -> Self {
		Self {
			access_profile: self.access_profile,
			cwd: self.cwd.clone(),
			chroot: self.chroot.clone(),
		}
	}
}

/// A process's signal management information.
pub struct ProcessSignal {
	/// The list of signal handlers
	pub handlers: Arc<Spin<[SignalHandler; SIGNALS_COUNT]>>,
	/// The alternative signal stack
	pub altstack: AltStack,
	/// A bitfield storing the set of blocked signals
	pub sigmask: SigSet,
	/// A bitfield storing the set of pending signals
	sigpending: SigSet,

	/// The exit status of the process after exiting
	pub exit_status: ExitStatus,
	/// The terminating signal
	pub termsig: u8,
}

impl ProcessSignal {
	/// Creates a new instance.
	pub fn new() -> AllocResult<Self> {
		Ok(ProcessSignal {
			handlers: Arc::new(Default::default())?,
			altstack: AltStack::default(),
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

	/// The current state of the process
	///
	/// [`STATE_LOCK`] write-locks the state, while allowing it to be read
	state: AtomicU8,
	/// If `true`, the parent can resume after a `vfork`.
	pub vfork_done: AtomicBool,
	/// The links to other processes.
	pub links: Spin<ProcessLinks>,

	/// The node in the scheduler's run queue.
	sched_node: ListNode,
	/// Process's niceness (`-20..=19`). Defines its scheduling priority (lower = higher priority)
	pub nice: AtomicI8,

	/// A pointer to the kernelspace stack.
	kernel_stack: KernelStack,
	/// Kernel stack pointer of saved context.
	kernel_sp: AtomicPtr<u8>,
	/// The process's FPU state.
	fpu: Spin<FxState>,

	/// FS segment selector
	fs_selector: AtomicU16,
	/// GS segment selector
	gs_selector: AtomicU16,
	/// FS segment hidden base
	fs_base: AtomicU64,
	/// GS segment hidden base
	gs_base: AtomicU64,
	/// TLS entries.
	pub tls: Spin<[gdt::Entry; TLS_ENTRIES_COUNT]>, // TODO rwlock

	/// The virtual memory of the process.
	mem_space: UnsafeMut<Option<Arc<MemSpace>>>,
	/// Filesystem access information.
	pub fs: Option<Spin<ProcessFs>>, // TODO rwlock
	/// The process's current umask.
	pub umask: AtomicU32,
	/// The list of open file descriptors with their respective ID.
	fd_table: UnsafeMut<Option<Arc<Spin<FileDescriptorTable>>>>,
	/// Process's timers, shared between all threads of the same process.
	pub timer_manager: Arc<Spin<TimerManager>>,
	/// The process's signal management structure.
	pub signal: Spin<ProcessSignal>, // TODO rwlock
	/// Events to be notified to the parent process upon `wait`.
	pub parent_event: AtomicU8,

	/// The process's resources usage.
	pub rusage: Spin<Rusage>,
}

/// The list of all processes on the system.
pub static PROCESSES: IntRwLock<BTreeMap<Pid, Arc<Process>>> = IntRwLock::new(BTreeMap::new());

/// Initializes processes management.
///
/// This function must be called only once, at kernel initialization.
pub(crate) fn init() -> EResult<()> {
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
				let ptr = UserPtr::<u8>::from_ptr(frame.get_program_counter());
				let opcode = ptr.copy_from_user();
				// If the instruction is `hlt`, exit
				if opcode == Ok(Some(HLT_INSTRUCTION)) {
					Process::exit(&proc, frame.get_syscall_id() as _);
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
	mem::forget(int::register_callback(0x00, callback)?);
	mem::forget(int::register_callback(0x03, callback)?);
	mem::forget(int::register_callback(0x06, callback)?);
	mem::forget(int::register_callback(0x0d, callback)?);
	mem::forget(int::register_callback(0x10, callback)?);
	mem::forget(int::register_callback(0x11, callback)?);
	mem::forget(int::register_callback(0x13, callback)?);
	mem::forget(int::register_callback(
		0x0e,
		|_id: u32, code: u32, frame: &mut IntFrame, ring: u8| {
			let accessed_addr = VirtAddr(register_get!("cr2"));
			let pc = frame.get_program_counter();
			let Some(mem_space) = per_cpu().mem_space.get() else {
				return CallbackResult::Panic;
			};
			// Check access
			let sig = mem_space.handle_page_fault(accessed_addr, code);
			match sig {
				Ok(true) => {}
				Ok(false) => {
					if ring < 3 {
						// Check if the fault was caused by a user <-> kernel copy
						if (user::raw_copy as usize..user::copy_fault as usize).contains(&pc) {
							// Jump to `copy_fault`
							frame.set_program_counter(user::copy_fault as usize);
						} else {
							return CallbackResult::Panic;
						}
					} else {
						Process::current().kill(Signal::SIGSEGV);
					}
				}
				Err(_) => Process::current().kill(Signal::SIGBUS),
			}
			CallbackResult::Continue
		},
	)?);
	mem::forget(int::register_callback(0x20, |_, _, _, _| {
		per_cpu().preempt_counter.fetch_and(!(1 << 31), Relaxed);
		CallbackResult::Continue
	})?);
	// Re-enable timer since it has been disabled by delay functions
	timer::apic::periodic(100_000_000);
	Ok(())
}

impl Process {
	/// Returns the process with PID `pid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_pid(pid: Pid) -> Option<Arc<Self>> {
		PROCESSES.read().get(&pid).cloned()
	}

	/// Returns the process with TID `tid`.
	///
	/// If the process doesn't exist, the function returns `None`.
	pub fn get_by_tid(_tid: Pid) -> Option<Arc<Self>> {
		todo!()
	}

	/// Returns the running process on the current core.
	#[inline]
	pub fn current() -> Arc<Self> {
		per_cpu().sched.get_current_process()
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
		let nice = if *pid == 0 {
			// The idle task has a lower priority than everyone else
			100
		} else {
			-20
		};
		let kernel_stack = KernelStack::new()?;
		let kernel_sp = unsafe { switch::init_kthread(kernel_stack.top(), entry) };
		let thread = Arc::new(Self {
			pid,
			tid,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Default::default(),

			sched_node: ListNode::default(),
			nice: AtomicI8::new(nice),

			kernel_stack,
			kernel_sp: AtomicPtr::new(kernel_sp),
			fpu: Spin::new(FxState([0; 512])),

			fs_selector: Default::default(),
			gs_selector: Default::default(),
			fs_base: Default::default(),
			gs_base: Default::default(),
			tls: Default::default(),

			// Not needed for kernel threads
			mem_space: Default::default(),
			fs: None,
			umask: Default::default(),
			fd_table: Default::default(),
			timer_manager: Arc::new(Spin::new(TimerManager::new(0)?))?,
			signal: Spin::new(ProcessSignal::new()?),
			parent_event: Default::default(),

			rusage: Default::default(),
		})?;
		if queue {
			PROCESSES.write().insert(*thread.pid, thread.clone())?;
			enqueue(&thread);
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
		// Create the default file descriptors table
		let mut fd_table = FileDescriptorTable::default();
		let tty_path = PathBuf::try_from(TTY_DEVICE_PATH.as_bytes())?;
		let tty_ent = vfs::get_file_from_path(&tty_path, true)?;
		let tty_file = File::open_entry(tty_ent, O_RDWR)?;
		let (stdin_fd_id, _) = fd_table.create_fd(0, tty_file)?;
		assert_eq!(stdin_fd_id, STDIN_FILENO);
		fd_table.duplicate_fd(
			STDIN_FILENO as _,
			NewFDConstraint::Fixed(STDOUT_FILENO as _),
			false,
		)?;
		fd_table.duplicate_fd(
			STDIN_FILENO as _,
			NewFDConstraint::Fixed(STDERR_FILENO as _),
			false,
		)?;
		let root_dir = vfs::get_file_from_path(Path::root(), false)?;
		let proc = Arc::new(Self {
			pid: PidHandle::mark_used(INIT_PID)?,
			tid: INIT_PID,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Spin::new(ProcessLinks::default()),

			sched_node: ListNode::default(),
			nice: AtomicI8::new(0),

			kernel_stack: KernelStack::new()?,
			kernel_sp: AtomicPtr::default(),
			fpu: Spin::new(FxState([0; 512])),

			fs_selector: Default::default(),
			gs_selector: Default::default(),
			fs_base: Default::default(),
			gs_base: Default::default(),
			tls: Default::default(),

			mem_space: UnsafeMut::new(None),
			fs: Some(Spin::new(ProcessFs {
				access_profile: AccessProfile::KERNEL,
				cwd: root_dir.clone(),
				chroot: root_dir,
			})),
			umask: AtomicU32::new(DEFAULT_UMASK),
			fd_table: UnsafeMut::new(Some(Arc::new(Spin::new(fd_table))?)),
			timer_manager: Arc::new(Spin::new(TimerManager::new(INIT_PID)?))?,
			signal: Spin::new(ProcessSignal {
				handlers: Arc::new(Default::default())?,
				altstack: Default::default(),
				sigmask: Default::default(),
				sigpending: Default::default(),

				exit_status: 0,
				termsig: 0,
			}),
			parent_event: Default::default(),

			rusage: Default::default(),
		})?;
		PROCESSES.write().insert(INIT_PID, proc.clone())?;
		enqueue(&proc);
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

	/// Returns the process's current state.
	///
	/// **Note**: since the process cannot be locked, this function may cause data races. Use with
	/// caution
	#[inline(always)]
	pub fn get_state(&self) -> State {
		let id = self.state.load(Relaxed) & !STATE_LOCK;
		State::from_id(id)
	}

	/// In a critical section, write-locks the process's state while executing `f`
	fn lock_state<F: FnOnce(State)>(&self, f: F) {
		critical(|| {
			let mut val;
			loop {
				val = self.state.fetch_or(STATE_LOCK, Release);
				if val & STATE_LOCK == 0 {
					break;
				}
				hint::spin_loop();
			}
			let state = State::from_id(val);
			f(state);
			self.state.fetch_and(!STATE_LOCK, Release);
		});
	}

	/// Sets the process's state to `new_state`.
	///
	/// If the transition from the previous state to `new_state` is invalid, the function does
	/// nothing.
	pub fn set_state(this: &Arc<Self>, new_state: State) {
		this.lock_state(|old_state| {
			let valid = matches!(
				(old_state, new_state),
				(State::Running | State::Sleeping, _) | (State::Stopped, State::Running)
			);
			if !valid {
				return;
			}
			if new_state == old_state {
				return;
			}
			// Update state
			this.state.store(STATE_LOCK | new_state as u8, Relaxed);
			#[cfg(feature = "strace")]
			println!(
				"[strace {pid}] changed state: {old_state:?} -> {new_state:?}",
				pid = this.get_pid()
			);
			// Enqueue or dequeue the process
			if new_state == State::Running {
				enqueue(this);
			} else if old_state == State::Running {
				dequeue(this);
			}
			if new_state == State::Zombie {
				if this.is_init() {
					panic!("Terminated init process!");
				}
				// Remove the memory space and file descriptors table to reclaim memory
				unsafe {
					//this.mem_space = None; // TODO Handle the case where the memory space is
					// bound
					*this.fd_table.get_mut() = None;
				}
				// Attach every child to the init process
				let init_proc = Process::get_by_pid(INIT_PID).unwrap();
				let children = mem::take(&mut this.links.lock().children);
				for child_pid in children {
					// Check just in case
					if child_pid == *this.pid {
						continue;
					}
					// TODO do the same for process group members
					if let Some(child) = Process::get_by_pid(child_pid) {
						child.links.lock().parent = Some(init_proc.clone());
						oom::wrap(|| init_proc.add_child(child_pid));
					}
				}
				// Set vfork as done just in case
				this.vfork_wake();
			}
			// Send SIGCHLD
			if matches!(new_state, State::Running | State::Stopped | State::Zombie) {
				let links = this.links.lock();
				if let Some(parent) = &links.parent {
					parent.kill(Signal::SIGCHLD);
				}
			}
		});
	}

	/// Wakes up the process if in [`Sleeping`] state.
	///
	/// Contrary to [`Self::set_state`], this function does not send a `SIGCHLD` signal
	pub fn wake(this: &Arc<Self>) {
		this.lock_state(|old_state| {
			if unlikely(old_state != State::Sleeping) {
				return;
			}
			this.state.store(STATE_LOCK | State::Running as u8, Relaxed);
			#[cfg(feature = "strace")]
			println!(
				"[strace {pid}] changed state: Sleeping -> Running",
				pid = this.get_pid()
			);
			enqueue(this);
		});
	}

	/// Signals the parent that the `vfork` operation has completed.
	pub fn vfork_wake(&self) {
		self.vfork_done.store(true, Release);
		let links = self.links.lock();
		if let Some(parent) = &links.parent {
			Process::set_state(parent, State::Running);
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
	/// - `frame` is the process's userspace frame
	/// - `stack` is the userspace stack to use. If null, the stack is left untouched
	/// - `fork_options` are the options for the fork operation
	pub fn fork(
		frame: &IntFrame,
		stack: *mut c_void,
		fork_options: ForkOptions,
	) -> EResult<Arc<Self>> {
		let parent = Process::current();
		let pid = PidHandle::unique()?;
		let pid_int = *pid;
		// Clone memory space
		let mem_space = {
			let curr_mem_space = parent.mem_space.as_ref().unwrap();
			if fork_options.share_memory {
				curr_mem_space.clone()
			} else {
				Arc::new(curr_mem_space.fork()?)?
			}
		};
		// Clone file descriptors
		let file_descriptors = if fork_options.share_fd {
			parent.fd_table.get().clone()
		} else {
			parent
				.fd_table
				.as_ref()
				.map(|fds| -> EResult<_> {
					let fds = fds.lock();
					let new_fds = fds.duplicate(false)?;
					Ok(Arc::new(Spin::new(new_fds))?)
				})
				.transpose()?
		};
		// Clone signal handlers
		let signal_handlers = {
			let signal_manager = parent.signal.lock();
			if fork_options.share_sighand {
				signal_manager.handlers.clone()
			} else {
				let handlers = signal_manager.handlers.lock().clone();
				Arc::new(Spin::new(handlers))?
			}
		};
		let group_leader = parent
			.links
			.lock()
			.group_leader
			.clone()
			.unwrap_or_else(|| parent.clone());
		// Init stack
		let kernel_stack = KernelStack::new()?;
		let mut frame = frame.clone();
		frame.rax = 0; // Return value
		if !stack.is_null() {
			frame.rsp = stack as _;
		}
		let kernel_sp = unsafe { switch::init_fork(kernel_stack.top(), frame) };
		let proc = Arc::new(Self {
			pid,
			tid: pid_int,

			state: AtomicU8::new(State::Running as _),
			vfork_done: AtomicBool::new(false),
			links: Spin::new(ProcessLinks {
				parent: Some(parent.clone()),
				group_leader: Some(group_leader.clone()),
				..Default::default()
			}),

			sched_node: ListNode::default(),
			nice: AtomicI8::new(0),

			kernel_stack,
			kernel_sp: AtomicPtr::new(kernel_sp),
			fpu: Spin::new(parent.fpu.lock().clone()),

			fs_selector: Default::default(),
			gs_selector: Default::default(),
			fs_base: Default::default(),
			gs_base: Default::default(),
			tls: Spin::new(*parent.tls.lock()),

			mem_space: UnsafeMut::new(Some(mem_space)),
			fs: Some(Spin::new(parent.fs().lock().clone())),
			umask: AtomicU32::new(parent.umask.load(Relaxed)),
			fd_table: UnsafeMut::new(file_descriptors),
			// TODO if creating a thread: timer_manager: parent.timer_manager.clone(),
			timer_manager: Arc::new(Spin::new(TimerManager::new(pid_int)?))?,
			signal: Spin::new(ProcessSignal {
				handlers: signal_handlers,
				altstack: Default::default(),
				sigmask: parent.signal.lock().sigmask,
				sigpending: Default::default(),

				exit_status: 0,
				termsig: 0,
			}),
			parent_event: Default::default(),

			rusage: Default::default(),
		})?;
		// Set FS and GS
		save_segments(&proc);
		// TODO on failure, must undo
		parent.add_child(pid_int)?;
		{
			let mut links = group_leader.links.lock();
			if let Err(i) = links.process_group.binary_search(&pid_int) {
				links.process_group.insert(i, pid_int)?;
			}
		}
		PROCESSES.write().insert(*proc.pid, proc.clone())?;
		enqueue(&proc);
		Ok(proc)
	}

	/// Returns the process's memory space.
	///
	/// If the process is a kernel thread, the function panics.
	#[inline]
	pub fn mem_space(&self) -> &Arc<MemSpace> {
		self.mem_space
			.get()
			.as_ref()
			.expect("kernel threads don't have a memory space")
	}

	/// Returns the process's memory space if any.
	#[inline]
	pub fn mem_space_opt(&self) -> &Option<Arc<MemSpace>> {
		self.mem_space.deref()
	}

	/// Returns the process's [`ProcessFs`].
	///
	/// If the process is a kernel thread, the function panics.
	#[inline]
	pub fn fs(&self) -> &Spin<ProcessFs> {
		self.fs
			.as_ref()
			.expect("kernel threads don't have ProcessFS structures")
	}

	/// Returns the umask
	#[inline]
	pub fn umask(&self) -> u32 {
		self.umask.load(Acquire)
	}

	/// Returns a reference to the file descriptors table
	#[inline]
	pub fn file_descriptors(&self) -> Arc<Spin<FileDescriptorTable>> {
		self.fd_table
			.get()
			.clone()
			.expect("kernel threads don't have a file descriptor table")
	}

	/// Tells whether there is a pending signal on the process.
	pub fn has_pending_signal(&self) -> bool {
		let signal = self.signal.lock();
		signal.sigpending.0 & !signal.sigmask.0 != 0
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
		/*#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] received signal `{sig}`",
			pid = self.get_pid(),
			sig = sig as c_int
		);*/
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

	/// Compares process priorities
	pub fn cmp_priority(&self, other: &Self) -> Ordering {
		let nice0 = self.nice.load(Acquire);
		let nice1 = other.nice.load(Acquire);
		nice0.cmp(&nice1).reverse() // niceness and priority are opposites
	}

	/// Exits the process with the given `status`.
	///
	/// This function changes the process's status to `Zombie`.
	pub fn exit(this: &Arc<Self>, status: u32) {
		#[cfg(feature = "strace")]
		println!(
			"[strace {pid}] exited with status `{status}`",
			pid = *this.pid
		);
		this.signal.lock().exit_status = status as ExitStatus;
		Process::set_state(this, State::Zombie);
	}

	/// Removes all references to the process in order to free the structure.
	///
	/// The process is unlinked from:
	/// - Its parent
	/// - Its group
	/// - Its scheduler
	/// - The processes list
	pub fn remove(this: Arc<Self>) {
		let (parent, group_leader) = {
			let mut links = this.links.lock();
			(links.parent.take(), links.group_leader.take())
		};
		if let Some(parent) = parent {
			let mut links = parent.links.lock();
			if let Ok(i) = links.children.binary_search(&this.get_pid()) {
				links.children.remove(i);
			}
		}
		if let Some(group_leader) = group_leader {
			let mut links = group_leader.links.lock();
			if let Ok(i) = links.process_group.binary_search(&this.get_pid()) {
				links.process_group.remove(i);
			}
		}
		dequeue(&this);
		PROCESSES.write().remove(&*this.pid);
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
		let fs = proc.fs().lock();
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
