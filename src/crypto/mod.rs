//! This module implements cryptographic tools.

pub mod chacha20;
pub mod checksum;
pub mod rand;

use crate::errno::Errno;

/// Initializes cryptographic features.
pub fn init() -> Result<(), Errno> {
	rand::init()
}
