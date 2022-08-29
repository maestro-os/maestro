//! This module implements randomness functions.

use crate::errno::Errno;
use crate::util::container::hashmap::HashMap;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// The size of the entropy buffer in bytes.
const ENTROPY_BUFFER_SIZE: usize = 32768;

/// The list of random sources.
static RANDOM_SOURCES: Mutex<HashMap<&'static str, SharedPtr<dyn RandomSource>>> =
	Mutex::new(HashMap::new());

/// Trait representing a source of random bytes.
pub trait RandomSource {
	/// Feeds data `data` to the random number generators.
	/// If the entropy buffer is already full, the data is ignored.
	fn feed_entropy(&mut self, data: &[u8]);

	/// Returns the number of available random bytes.
	fn available_bytes(&self) -> usize;

	/// Consumes entropy from the buffer to fill the given `buf`.
	/// The function returns the number of bytes written.
	fn consume_entropy(&mut self, buf: &mut [u8]) -> usize;
}

/// Structure representing a source of random bytes.
/// This source is used by the `/dev/random` device file.
pub struct Random {
	/// The buffer containing entropy.
	buff: RingBuffer<u8>,
}

impl Random {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			buff: RingBuffer::new(ENTROPY_BUFFER_SIZE)?,
		})
	}
}

impl RandomSource for Random {
	fn feed_entropy(&mut self, data: &[u8]) {
		self.buff.write(data);
	}

	fn available_bytes(&self) -> usize {
		// TODO
		usize::MAX
	}

	fn consume_entropy(&mut self, buf: &mut [u8]) -> usize {
		// TODO
		buf.fill(0);
		buf.len()
	}
}

/// Structure representing a source of random bytes.
/// Contrary to `random`, this source doesn't block when entropy is exhausted.
/// This source is used by the `/dev/urandom` device file.
pub struct URandom {
	/// The buffer containing entropy.
	buff: RingBuffer<u8>,
}

impl URandom {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			buff: RingBuffer::new(ENTROPY_BUFFER_SIZE)?,
		})
	}
}

impl RandomSource for URandom {
	fn feed_entropy(&mut self, data: &[u8]) {
		self.buff.write(data);
	}

	fn available_bytes(&self) -> usize {
		// TODO
		usize::MAX
	}

	fn consume_entropy(&mut self, buf: &mut [u8]) -> usize {
		// TODO
		buf.fill(0);
		buf.len()
	}
}

/// Returns the randomness source with the given name `name`. If the source doesn't exist, the
/// function returns None.
pub fn get_source(name: &'static str) -> Option<SharedPtr<dyn RandomSource>> {
	Some(RANDOM_SOURCES.lock().get().get(&name)?.clone())
}

/// Initializes randomness sources.
pub fn init() -> Result<(), Errno> {
	let guard = RANDOM_SOURCES.lock();
	let sources = guard.get_mut();

	sources.insert("random", SharedPtr::new(Random::new()?)?)?;
	sources.insert("urandom", SharedPtr::new(URandom::new()?)?)?;

	Ok(())
}
