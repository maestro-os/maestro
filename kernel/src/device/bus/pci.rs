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

//! The PCI (Peripheral Component Interconnect) is a bus which allows to attach
//! hardware devices on the motherboard.
//!
//! This module allows to retrieve informations on the devices attached to the computer's PCI.
//!
//! The device ID, vendor ID, class and subclass of a device allows to determine
//! which driver is required for the device.
//!
//! A PCI device can specify one or several BARs (Base Address Registers). They
//! specify the address of the device's registers in memory, allowing
//! communications through DMA (Direct Memory Access).

use crate::{
	device::{
		bar::{BARType, BAR},
		driver, manager,
		manager::PhysicalDevice,
		DeviceManager,
	},
	io, memory,
	memory::mmio::MMIO,
};
use core::{cmp::min, mem::size_of};
use utils::{
	collections::vec::Vec,
	errno::{CollectResult, EResult},
};

/// The port used to specify the configuration address.
const CONFIG_ADDRESS_PORT: u16 = 0xcf8;
/// The port used to retrieve the devices informations.
const CONFIG_DATA_PORT: u16 = 0xcfc;

/// Device class: Unclassified
pub const CLASS_UNCLASSIFIED: u16 = 0x00;
/// Device class: Mass Storage Controller
pub const CLASS_MASS_STORAGE_CONTROLLER: u16 = 0x01;
/// Device class: Network Controller
pub const CLASS_NETWORK_CONTROLLER: u16 = 0x02;
/// Device class: Display Controller
pub const CLASS_DISPLAY_CONTROLLER: u16 = 0x03;
/// Device class: Multimedia Controller
pub const CLASS_MULTIMEDIA_CONTROLLER: u16 = 0x04;
/// Device class: Memory Controller
pub const CLASS_MEMORY_CONTROLLER: u16 = 0x05;
/// Device class: Bridge
pub const CLASS_BRIDGE: u16 = 0x06;
/// Device class: Simple Communication Controller
pub const CLASS_SIMPLE_COMMUNICATION_CONTROLLER: u16 = 0x07;
/// Device class: Base System Peripheral
pub const CLASS_BASE_SYSTEM_PERIPHERAL: u16 = 0x08;
/// Device class: Input Device Controller
pub const CLASS_INPUT_DEVICE_CONTROLLER: u16 = 0x09;
/// Device class: Docking Station
pub const CLASS_DOCKING_STATION: u16 = 0x0a;
/// Device class: Processor
pub const CLASS_PROCESSOR: u16 = 0x0b;
/// Device class: Serial Bus Controller
pub const CLASS_SERIAL_BUS_CONTROLLER: u16 = 0x0c;
/// Device class: Wireless Controller
pub const CLASS_WIRELESS_CONTROLLER: u16 = 0x0d;
/// Device class: Intelligent Controller
pub const CLASS_INTELLIGENT_CONTROLLER: u16 = 0x0e;
/// Device class: Satellite Communication Controller
pub const CLASS_SATELLITE_COMMUNICATION_CONTROLLER: u16 = 0x0f;
/// Device class: Encryption Controller
pub const CLASS_ENCRYPTION_CONTROLLER: u16 = 0x10;
/// Device class: Signal Processing Controller
pub const CLASS_SIGNAL_PROCESSING_CONTROLLER: u16 = 0x11;
/// Device class: Processing Accelerator
pub const CLASS_PROCESSING_ACCELERATOR: u16 = 0x12;
/// Device class: Non-Essential Instrumentation
pub const CLASS_NON_ESSENTIAL_INSTRUMENTATION: u16 = 0x13;
/// Device class: Co-Processor
pub const CLASS_CO_PROCESSOR: u16 = 0x40;
/// Device class: Unassigned
pub const CLASS_UNASSIGNED: u16 = 0xff;

/// Reads 32 bits from the PCI register specified by `bus`, `device`, `func` and
/// `reg_off`.
fn read_long(bus: u8, device: u8, func: u8, reg_off: u8) -> u32 {
	// The PCI address
	let addr = ((bus as u32) << 16)
		| ((device as u32) << 11)
		| ((func as u32) << 8)
		| ((reg_off as u32 * size_of::<u32>() as u32) & 0xff)
		| 0x80000000;

	unsafe {
		// Set the address
		io::outl(CONFIG_ADDRESS_PORT, addr);
		// Read the value
		io::inl(CONFIG_DATA_PORT)
	}
}

/// Writes 32 bits from `value` into the PCI register specified by `bus`,
/// `device`, `func` and `reg_off`.
fn write_long(bus: u8, device: u8, func: u8, reg_off: u8, value: u32) {
	// The PCI address
	let addr = ((bus as u32) << 16)
		| ((device as u32) << 11)
		| ((func as u32) << 8)
		| ((reg_off as u32 * size_of::<u32>() as u32) & 0xff)
		| 0x80000000;

	unsafe {
		// Set the address
		io::outl(CONFIG_ADDRESS_PORT, addr);
		// Write the value
		io::outl(CONFIG_DATA_PORT, value);
	}
}

/// Reads PCI configuration and writes it into `buf`.
///
/// Arguments:
/// - `bus` is the bus number.
/// - `device` is the device number.
/// - `func` is the function number.
/// - `off` is the register offset.
/// - `buf` is the data buffer to write to.
fn read_data(bus: u8, device: u8, func: u8, off: usize, buf: &mut [u32]) {
	let end = min(off + buf.len(), 0x12);
	for (buf_off, reg_off) in (off..end).enumerate() {
		buf[buf_off] = read_long(bus, device, func, reg_off as _);
	}
}

/// Writes PCI configuration from `buf`.
///
/// Arguments:
/// - `bus` is the bus number.
/// - `device` is the device number.
/// - `func` is the function number.
/// - `off` is the register offset.
/// - `buf` is the data buffer to read from.
fn write_data(bus: u8, device: u8, func: u8, off: usize, buf: &[u32]) {
	let end = min(off + buf.len(), 16);
	for (buf_off, reg_off) in (off..end).enumerate() {
		write_long(bus, device, func, reg_off as _, buf[buf_off]);
	}
}

/// Structure representing a device attached to the PCI bus.
pub struct PCIDevice {
	/// The PCI bus of the device.
	bus: u8,
	/// The offset of the device on the bus.
	device: u8,
	/// The function number of the device.
	function: u8,

	/// The device's ID.
	device_id: u16,
	/// The device's vendor ID.
	vendor_id: u16,

	/// The command register.
	command: u16,
	/// The status register.
	status: u16,

	/// The device's class code, telling what the device is.
	class: u8,
	/// The device's subclass code, giving more informations on the device.
	subclass: u8,
	/// Value giving more informations on the device's compatibilities.
	prog_if: u8,
	/// The device's revision ID.
	revision_id: u8,

	/// Built-In Self Test status.
	bist: u8,
	/// Defines the header type of the device, to determine what informations
	/// follow.
	header_type: u8,
	/// Specifies the latency timer in units of PCI bus clocks.
	latency_timer: u8,
	/// Specifies the system cache line size in 32-bit units.
	cache_line_size: u8,

	/// Additional informations about the device.
	info: [u32; 12],

	/// The list of BARs for the device.
	bars: Vec<Option<BAR>>,
	/// The list of MMIOs associated with the device's BARs.
	mmios: Vec<MMIO>,
}

impl PCIDevice {
	/// Returns the maximum number of BARs for the current device.
	fn get_max_bars_count(&self) -> u8 {
		match self.header_type {
			0x00 => 6,
			0x01 => 2,

			_ => 0,
		}
	}

	/// Returns the offset of the register for the `n`th BAR.
	fn get_bar_reg_off(&self, n: u8) -> Option<u16> {
		if n < self.get_max_bars_count() {
			Some(0x4 + n as u16)
		} else {
			None
		}
	}

	/// Returns the size of the address space of the `n`th BAR.
	///
	/// `io` tells whether the BAR is in I/O space.
	fn get_bar_size(&self, n: u8, io: bool) -> Option<usize> {
		let reg_off = self.get_bar_reg_off(n)?;
		// Saving the register
		let save = read_long(self.bus, self.device, self.function, reg_off as _);

		// Writing all 1s on register
		write_long(self.bus, self.device, self.function, reg_off as _, !0u32);

		let mut size =
			(!read_long(self.bus, self.device, self.function, reg_off as _)).wrapping_add(1);
		if io {
			size &= 0xffff;
		}

		// Restoring the register's value
		write_long(self.bus, self.device, self.function, reg_off as _, save);

		Some(size as _)
	}

	/// Loads and returns the `n`th BAR.
	///
	/// If it doesn't exist, the function returns `None`.
	///
	/// A BAR may be accompanied with an MMIO, allowing to map a portion of virtual memory in order
	/// to make the BAR accessible.
	///
	/// Dropping the MMIO makes using the associated BAR an undefined behaviour.
	fn load_bar(&self, n: u8) -> EResult<Option<(BAR, Option<MMIO>)>> {
		let Some(bar_off) = self.get_bar_reg_off(n) else {
			return Ok(None);
		};

		// The BAR's value
		let value = read_long(self.bus, self.device, self.function, bar_off as _);
		// Tells whether the BAR is in IO space.
		let io = (value & 0b1) != 0;
		// The address space's size
		let size = self.get_bar_size(n, io).unwrap();

		if !io {
			let type_ = match ((value >> 1) & 0b11) as u8 {
				0x0 => BARType::Size32,
				0x2 => BARType::Size64,

				_ => return Ok(None),
			};
			let mut address = match type_ {
				BARType::Size32 => (value & 0xfffffff0) as u64,

				BARType::Size64 => {
					let Some(next_bar_off) = self.get_bar_reg_off(n + 1) else {
						return Ok(None);
					};

					// The next BAR's value
					let next_value =
						read_long(self.bus, self.device, self.function, next_bar_off as _);
					(value & 0xfffffff0) as u64 | ((next_value as u64) << 32)
				}
			};
			if address == 0 {
				return Ok(None);
			}

			let prefetchable = value & 0b1000 != 0;

			// Create MMIO
			let pages = size.div_ceil(memory::PAGE_SIZE);
			let mut mmio = MMIO::new(address as _, pages, prefetchable)?;
			address = mmio.as_mut_ptr() as _;

			Ok(Some((
				BAR::MemorySpace {
					type_,
					prefetchable,

					address,

					size,
				},
				Some(mmio),
			)))
		} else {
			let address = (value & 0xfffffffc) as u64;
			if address == 0 {
				return Ok(None);
			}

			Ok(Some((
				BAR::IOSpace {
					address,

					size,
				},
				None,
			)))
		}
	}

	/// Creates a new instance of PCI device.
	///
	/// Arguments:
	/// - `bus` is the PCI bus.
	/// - `device` is the device number on the bus.
	/// - `function` is the function number on the device.
	/// - `data` is the data returned by the PCI.
	fn new(bus: u8, device: u8, function: u8, data: &[u32; 16]) -> EResult<Self> {
		let mut dev = Self {
			bus,
			device,
			function,

			vendor_id: (data[0] & 0xffff) as _,
			device_id: ((data[0] >> 16) & 0xffff) as _,

			command: (data[1] & 0xffff) as _,
			status: ((data[1] >> 16) & 0xffff) as _,

			class: ((data[2] >> 24) & 0xff) as _,
			subclass: ((data[2] >> 16) & 0xff) as _,
			prog_if: ((data[2] >> 8) & 0xff) as _,
			revision_id: (data[2] & 0xff) as _,

			bist: ((data[3] >> 24) & 0xff) as _,
			header_type: ((data[3] >> 16) & 0xff) as _,
			latency_timer: ((data[3] >> 8) & 0xff) as _,
			cache_line_size: (data[3] & 0xff) as _,

			info: [
				data[4], data[5], data[6], data[7], data[8], data[9], data[10], data[11],
				data[12], data[13], data[14], data[15],
			],

			bars: Vec::new(),
			mmios: Vec::new(),
		};

		// Load BARs
		let mut i = 0;
		while i < dev.get_max_bars_count() {
			let bar = if let Some((bar, mmio)) = dev.load_bar(i)? {
				// Skip the next BAR if necessary
				if let BAR::MemorySpace {
					type_: BARType::Size64,
					..
				} = &bar
				{
					i += 1;
				}
				if let Some(mmio) = mmio {
					dev.mmios.push(mmio)?;
				}
				Some(bar)
			} else {
				None
			};
			dev.bars.push(bar)?;

			i += 1;
		}

		Ok(dev)
	}

	/// Returns the PCI bus ID.
	#[inline(always)]
	pub fn get_bus(&self) -> u8 {
		self.bus
	}

	/// Returns the PCI device ID.
	#[inline(always)]
	pub fn get_device(&self) -> u8 {
		self.device
	}

	/// Returns the PCI function ID.
	#[inline(always)]
	pub fn get_function(&self) -> u8 {
		self.function
	}

	/// Returns the header type of the device.
	#[inline(always)]
	pub fn get_header_type(&self) -> u8 {
		// Clear the Multi-Function flag
		self.header_type & 0b01111111
	}
}

impl PhysicalDevice for PCIDevice {
	fn get_device_id(&self) -> u16 {
		self.device_id
	}

	fn get_vendor_id(&self) -> u16 {
		self.vendor_id
	}

	fn get_command_reg(&self) -> Option<u16> {
		Some(self.command)
	}

	fn get_status_reg(&self) -> Option<u16> {
		Some(self.status)
	}

	fn get_class(&self) -> u16 {
		self.class as _
	}

	fn get_subclass(&self) -> u16 {
		self.subclass as _
	}

	fn get_prog_if(&self) -> u8 {
		self.prog_if
	}

	fn is_hotplug(&self) -> bool {
		false
	}

	fn get_bars(&self) -> &[Option<BAR>] {
		&self.bars
	}

	fn get_interrupt_line(&self) -> Option<u8> {
		let n = (self.info[11] & 0xff) as u8;
		if n != 0xff {
			Some(n)
		} else {
			None
		}
	}

	fn get_interrupt_pin(&self) -> Option<u8> {
		let n = ((self.info[11] >> 8) & 0xff) as u8;
		if n != 0 {
			Some(n)
		} else {
			None
		}
	}
}

/// This manager handles every devices connected to the PCI bus.
///
/// Since the PCI bus is not a hotplug bus, calling `on_unplug` on this structure has no effect.
pub struct PCIManager {
	/// The list of PCI devices.
	devices: Vec<PCIDevice>,
}

impl PCIManager {
	/// Creates a new instance.
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		Self {
			devices: Vec::new(),
		}
	}

	/// Scans for PCI devices and registers them on the manager.
	///
	/// If the PCI has already been scanned, this function does nothing.
	pub fn scan(&mut self) -> EResult<()> {
		// Avoid calling `on_plug` twice for the same devices
		if !self.devices.is_empty() {
			return Ok(());
		}

		// Iterate buses
		self.devices = (0..=255u8)
			// Iterate devices on bus
			.flat_map(|bus| (0..32u8).map(move |device| (bus, device)))
			// If the device doesn't exist, ignore
			.filter(|(bus, device)| {
				let vendor_id = read_long(*bus, *device, 0, 0) & 0xffff;
				vendor_id != 0xffff
			})
			// Read device's PCI data
			.flat_map(|(bus, device)| {
				// Read device's PCI data
				let mut data: [u32; 16] = [0; 16];
				read_data(bus, device, 0, 0, &mut data);

				let header_type = ((data[3] >> 16) & 0xff) as u8;
				let max_func_count = {
					if header_type & 0x80 != 0 {
						// Multi-function device
						8u8
					} else {
						// Single-function device
						1u8
					}
				};

				(0..max_func_count).map(move |func| (bus, device, func))
			})
			// If the function doesn't exist, ignore
			.filter(|(bus, device, func)| {
				let vendor_id = read_long(*bus, *device, *func, 0) & 0xffff;
				vendor_id != 0xffff
			})
			// Iterate functions
			.map(|(bus, device, func)| {
				// Read function's PCI data
				let mut data: [u32; 16] = [0; 16];
				read_data(bus, device, func, 0, &mut data);

				// Enable Memory space and I/O space for BARs
				data[1] |= 0b11;
				write_long(bus, device, func, 0x1, data[1]);

				// Register the device
				let dev = PCIDevice::new(bus, device, func, &data)?;
				driver::on_plug(&dev);
				manager::on_plug(&dev)?;
				Ok(dev)
			})
			.collect::<EResult<CollectResult<_>>>()?
			.0?;
		Ok(())
	}

	/// Returns the list of PCI devices.
	///
	/// If the PCI hasn't been scanned, the function returns an empty vector.
	#[inline(always)]
	pub fn get_devices(&self) -> &Vec<PCIDevice> {
		&self.devices
	}
}

impl DeviceManager for PCIManager {
	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		Ok(())
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		Ok(())
	}
}
