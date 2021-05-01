/// This module implements storage drivers.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::storage::pata::PATAInterface;
use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;

pub mod pata;

/// Trait representing a storage interface. A storage block is the atomic unit for I/O access on
/// the storage device.
pub trait StorageInterface {
	/// Returns the size of the storage blocks in bytes.
	fn get_block_size(&self) -> usize;
	/// Returns the number of storage blocks.
	fn get_blocks_count(&self) -> u64;

	/// Reads `size` blocks from storage at block offset `offset`, writting the data to `buf`.
	fn read(&self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), ()>;
	/// Writes `size` blocks to storage at block offset `offset`, reading the data from `buf`.
	fn write(&self, buf: &[u8], offset: u64, size: u64) -> Result<(), ()>;
}

// TODO Function to add a device

/// Structure managing storage devices.
pub struct StorageManager {
	/// The list of detected interfaces.
	interfaces: Vec<Box<dyn StorageInterface>>,
}

impl StorageManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			interfaces: Vec::new(),
		}
	}

	/// Adds a storage device.
	fn add(&mut self, storage: Box<dyn StorageInterface>) -> Result<(), Errno> {
		// TODO Read to detect partitions

		self.interfaces.push(storage)
	}

	/// Fills a random buffer `buff` of size `size` with seed `seed`.
	#[cfg(config_debug_storagetest)]
	fn random_block(size: usize, buff: &mut [u8], seed: u32) -> u32 {
		let mut s = seed;

		for i in 0..size {
			s = crate::util::math::pseudo_rand(s, 22, 44, 100);
			buff[i] = (s & 0xff) as u8;
		}

		s
	}

	/// Tests the given interface with the given interface `interface`.
	/// `seed` is the seed for pseudo random generation. The function will set this variable to
	/// another value for the next iteration.
	#[cfg(config_debug_storagetest)]
	fn test_interface(interface: &mut dyn StorageInterface, seed: &mut u32) -> bool {
		let block_size = interface.get_block_size();
		let mut s = *seed;
		for i in 0..interface.get_blocks_count() {
			let mut buff: [u8; 512] = [0; 512]; // TODO Set to block size
			s = Self::random_block(block_size, &mut buff, s);
			interface.write(&buff, i, block_size as _).unwrap();
		}

		s = *seed;
		for i in 0..interface.get_blocks_count() {
			let mut buff: [u8; 512] = [0; 512]; // TODO Set to block size
			s = Self::random_block(interface.get_block_size(), &mut buff, s);

			let mut buf: [u8; 512] = [0; 512]; // TODO Set to block size
			interface.read(&mut buf, i, block_size as _).unwrap();

			if buf != buff {
				return false;
			}
		}

		*seed = crate::util::math::pseudo_rand(*seed, 11, 22, 100);
		true
	}

	/// Tests every storage drivers on every storage devices.
	/// The execution of this function removes all the data on every connected writable disks, so
	/// it must be used carefully.
	#[cfg(config_debug_storagetest)]
	pub fn test(&mut self) {
		crate::println!("Running disks tests... ({} devices)", self.interfaces.len());

		let mut seed = 42;
		let iterations_count = 100;
		for i in 0..iterations_count {
			for j in 0..self.interfaces.len() {
				crate::print!("Iteration: {}/{}; Device: {}/{}",
					i + 1, iterations_count,
					j + 1, self.interfaces.len());

				let interface = &mut self.interfaces[j];
				if !Self::test_interface(interface.as_mut(), &mut seed) {
					crate::println!("Disk test failed!");
					unsafe {
						crate::kernel_halt();
					}
				}
			}

			crate::print!("\r");
		}

		crate::println!("Done!");
		unsafe {
			crate::kernel_halt();
		}
	}
}

impl DeviceManager for StorageManager {
	fn legacy_detect(&mut self) -> Result<(), Errno> {
		// TODO Detect floppy disks

		for i in 0..4 {
			let secondary = (i & 0b01) != 0;
			let slave = (i & 0b10) != 0;

			if let Ok(dev) = PATAInterface::new(secondary, slave) {
				self.add(Box::new(dev)?)?;
			}
		}

		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}
