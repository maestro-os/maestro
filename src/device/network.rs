//! This module implements network-related devices handling.

pub trait NetworkInterface {
	/// Returns the interface's MAC address.
	fn get_mac(&self) -> [u8; 6];

	// TODO Reading (use interrupts)

	/// Writes the data from `buff` to the interface.
	/// The function returns the number of bytes written.
	fn write(&self, buff: &[u8]) -> usize;
}
