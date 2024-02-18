//! Memory usage tracing utility functions.

use core::ffi::c_void;
use crate::device::serial;

/// Writes a memory tracing sample to the **COM4** serial port.
///
/// Arguments:
/// - `allocator` is the name of the allocator.
/// - `op` is the operation number.
/// - `ptr` is the affected pointer.
/// - `size` is the new size of the allocation. The unit is dependent on the allocator.
pub fn sample(allocator: &str, op: u8, ptr: *const c_void, size: usize) {
    // COM4
    let mut serial = serial::PORTS[3].lock();
    // Write name of allocator
    serial.write(&(allocator.len() as u64).to_le_bytes());
    serial.write(allocator.as_bytes());
    // Write op
    serial.write(&[op]);
    // Write ptr and size
    serial.write(&(ptr as u64).to_le_bytes());
    serial.write(&(size as u64).to_le_bytes());
}
