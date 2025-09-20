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

//! This module implements Serial port communications.

use crate::{
	arch::x86::io::{inb, outb},
	sync::spin::Spin,
};

/// The offset of COM1 registers.
pub const COM1: u16 = 0x3f8;
/// The offset of COM2 registers.
pub const COM2: u16 = 0x2f8;
/// The offset of COM3 registers.
pub const COM3: u16 = 0x3e8;
/// The offset of COM4 registers.
pub const COM4: u16 = 0x2e8;

/// When DLAB = 0: Data register
const DATA_REG_OFF: u16 = 0;
/// When DLAB = 0: Interrupt Enable Register
const INTERRUPT_REG_OFF: u16 = 1;
/// When DLAB = 1: least significant byte of the divisor value
const DIVISOR_LO_REG_OFF: u16 = 0;
/// When DLAB = 1: most significant byte of the divisor value
const DIVISOR_HI_REG_OFF: u16 = 1;
/// Interrupt Identification and FIFO control registers
const II_FIFO_REG_OFF: u16 = 2;
/// Line Control Register
const LINE_CTRL_REG_OFF: u16 = 3;
/// Modem Control Register
const MODEM_CTRL_REG_OFF: u16 = 4;
/// Line Status Register
const LINE_STATUS_REG_OFF: u16 = 5;
/// Modem Status Register
const MODEM_STATUS_REG_OFF: u16 = 6;
/// Scratch Register
const SCRATCH_REG_OFF: u16 = 7;

/// Bit of the Interrupt Enable Register telling whether data is available.
const INTERRUPT_DATA_AVAILABLE: u8 = 0b1;
/// Bit of the Interrupt Enable Register telling whether the transmitter is
/// empty.
const INTERRUPT_TRANSMITTER_EMPTY: u8 = 0b10;
/// Bit of the Interrupt Enable Register telling whether an interrupt error
/// happened.
const INTERRUPT_ERROR: u8 = 0b100;
/// Bit of the Interrupt Enable Register telling whether the interrupt status
/// changed.
const INTERRUPT_STATUS_CHANGE: u8 = 0b1000;

/// The offset of the DLAB bit in the line control register.
const DLAB: u8 = 1 << 7;

/// Bit of the Line Status Register telling whether data is available to be
/// read.
const LINE_STATUS_DR: u8 = 0b1;
/// Bit of the Line Status Register telling whether data has been lost.
const LINE_STATUS_OE: u8 = 0b10;
/// Bit of the Line Status Register telling whether a transmission error was
/// detect by parity.
const LINE_STATUS_PE: u8 = 0b100;
/// Bit of the Line Status Register telling whether a framing error was
/// detected.
const LINE_STATUS_FE: u8 = 0b1000;
/// Bit of the Line Status Register telling whether there is a break in data
/// input.
const LINE_STATUS_BI: u8 = 0b10000;
/// Bit of the Line Status Register telling whether the transmission buffer is
/// empty.
const LINE_STATUS_THRE: u8 = 0b100000;
/// Bit of the Line Status Register telling whether the transmitter is idling.
const LINE_STATUS_TEMT: u8 = 0b1000000;
/// Bit of the Line Status Register telling whether there is an error with a
/// word in the input buffer.
const LINE_STATUS_IE: u8 = 0b10000000;

/// The UART's frequency.
const UART_FREQUENCY: u32 = 115200; // TODO Replace by a rational number?

/// A serial communication port.
pub struct Serial {
	/// The offset of the port's I/O registers.
	regs_off: u16,
	/// Tells whether the port is active (if not need, probing to check).
	active: bool,
}

impl Serial {
	/// Tests whether the current serial port exists.
	fn probe(&mut self) -> bool {
		unsafe {
			outb(self.regs_off + INTERRUPT_REG_OFF, 0x00);
			self.set_baud_rate(38400);
			outb(self.regs_off + LINE_CTRL_REG_OFF, 0x03);
			outb(self.regs_off + II_FIFO_REG_OFF, 0xc7);
			outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x0b);
			outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x1e);
			outb(self.regs_off + DATA_REG_OFF, 0xae);

			if inb(self.regs_off + DATA_REG_OFF) != 0xae {
				return false;
			}

			outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x0f);
		}

		true
	}

	/// Creates a new instance for the specified port.
	///
	/// If the port doesn't exist, the function returns `None`.
	const fn from_port(port: u16) -> Serial {
		Self {
			regs_off: port,
			active: false,
		}
	}

	// TODO make pub? (must check the port is active before, without causing a stack overflow)
	/// Sets the port's baud rate.
	///
	/// If the baud rate is not supported, the function approximates it to the nearest supported
	/// value.
	///
	/// If the port does not exist, the function does nothing.
	fn set_baud_rate(&mut self, baud: u32) {
		let div = (UART_FREQUENCY / baud) as u16;
		unsafe {
			let line_ctrl = inb(self.regs_off + LINE_CTRL_REG_OFF);
			outb(self.regs_off + LINE_CTRL_REG_OFF, line_ctrl | DLAB);

			outb(self.regs_off + DIVISOR_LO_REG_OFF, (div & 0xff) as _);
			outb(self.regs_off + DIVISOR_HI_REG_OFF, ((div >> 8) & 0xff) as _);

			outb(self.regs_off + LINE_CTRL_REG_OFF, line_ctrl & !DLAB);
		}
	}

	// TODO read

	/// Tells whether the transmission buffer is empty.
	fn is_transmit_empty(&self) -> bool {
		(unsafe { inb(self.regs_off + LINE_STATUS_REG_OFF) } & LINE_STATUS_THRE) != 0
	}

	/// Writes the given buffer to the port's output.
	///
	/// If the port does not exist, the function does nothing.
	pub fn write(&mut self, buff: &[u8]) {
		if !self.active {
			self.active = self.probe();
		}
		if !self.active {
			return;
		}

		for b in buff {
			while !self.is_transmit_empty() {}
			unsafe {
				outb(self.regs_off + DATA_REG_OFF, *b);
			}
		}
	}
}

/// The list of serial ports.
pub static PORTS: [Spin<Serial>; 4] = [
	Spin::new(Serial::from_port(COM1)),
	Spin::new(Serial::from_port(COM2)),
	Spin::new(Serial::from_port(COM3)),
	Spin::new(Serial::from_port(COM4)),
];
