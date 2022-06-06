//! This module monitors the resource usage of a process.

use crate::time::unit::Timeval;

/// Structure storing the usage of each resources by a process.
#[derive(Clone, Default)]
pub struct RUsage {
	/// User CPU time used.
	ru_utime: Timeval,
	/// System CPU time used.
	ru_stime: Timeval,
	/// Maximum resident set size.
	ru_maxrss: i32,
	/// Integral shared memory size.
	ru_ixrss: i32,
	/// Integral unshared data size.
	ru_idrss: i32,
	/// Integral unshared stack size.
	ru_isrss: i32,
	/// Page reclaims (soft page faults).
	ru_minflt: i32,
	/// Page faults (hard page faults).
	ru_majflt: i32,
	/// Swaps.
	ru_nswap: i32,
	/// Block input operations.
	ru_inblock: i32,
	/// Block output operations.
	ru_oublock: i32,
	/// IPC messages sent.
	ru_msgsnd: i32,
	/// IPC messages received.
	ru_msgrcv: i32,
	/// Signals received.
	ru_nsignals: i32,
	/// Voluntary context switches.
	ru_nvcsw: i32,
	/// Involuntary context switches.
	ru_nivcsw: i32,
}

// TODO Place calls in kernel's code to update usage
