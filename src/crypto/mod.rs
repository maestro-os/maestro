//! Cryptographic algorithms and tools.

use crate::errno::AllocResult;

pub mod chacha20;
pub mod checksum;
pub mod rand;

/// Initializes cryptographic features.
pub(crate) fn init() -> AllocResult<()> {
	rand::init()
}
