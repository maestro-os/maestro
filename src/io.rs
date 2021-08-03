//! The I/O functions allow to communicate with the other components on the system.

// TODO Write in Rust instead of C
extern "C" {
	/// Inputs a byte from the specified port.
	pub fn inb(port: u16) -> u8;
	/// Inputs a word from the specified port.
	pub fn inw(port: u16) -> u16;
	/// Inputs a long from the specified port.
	pub fn inl(port: u16) -> u32;
	/// Outputs a byte to the specified port.
	pub fn outb(port: u16, value: u8);
	/// Outputs a word to the specified port.
	pub fn outw(port: u16, value: u16);
	/// Outputs a long to the specified port.
	pub fn outl(port: u16, value: u32);
}
