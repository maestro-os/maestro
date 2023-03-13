//! When a resource is blocking, a process trying to use it must be put in `Sleeping` state until
//! the resource is available.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::process;
use crate::util::container::hashmap::HashMap;
use crate::util::io;

/// Handler allowing to make a process sleep when waiting on a resource, then resume its execution
/// when the resource is available.
#[derive(Debug, Default)]
pub struct BlockHandler {
	/// The list of processes waiting on the resource, along with the mask of events to wait for.
	waiting_procs: HashMap<Pid, u32>,
}

impl BlockHandler {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			waiting_procs: HashMap::new(),
		}
	}

	/// Adds the given process to the list of processes waiting on the resource.
	///
	/// The function sets the state of the process to `Sleeping`.
	/// When the event occurs, the process will be woken up.
	///
	/// `mask` is the mask of poll event to wait for.
	///
	pub fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> Result<(), Errno> {
		self.waiting_procs.insert(proc.get_pid(), mask)?;
		proc.set_state(process::State::Sleeping);

		Ok(())
	}

	/// Wakes processes for the events in the given mask.
	pub fn wake_processes(&mut self, mask: u32) {
		self.waiting_procs.retain(|pid, m| {
			let Some(proc_mutex) = Process::get_by_pid(*pid) else {
				return false;
			};

			let wake = mask & *m != 0;
			if !wake {
				return true;
			}

			let proc_guard = proc_mutex.lock();
			proc_guard.get_mut().wake();

			false
		});
	}
}

impl Drop for BlockHandler {
	fn drop(&mut self) {
		self.wake_processes(io::POLLERR);
	}
}
