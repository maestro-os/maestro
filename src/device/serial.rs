//! This module implements Serial port communications.

use crate::io;
use crate::util::lock::Mutex;

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
/// Bit of the Interrupt Enable Register telling whether the transmitter is empty.
const INTERRUPT_TRANSMITTER_EMPTY: u8 = 0b10;
/// Bit of the Interrupt Enable Register telling whether an interrupt error happened.
const INTERRUPT_ERROR: u8 = 0b100;
/// Bit of the Interrupt Enable Register telling whether the interrupt status changed.
const INTERRUPT_STATUS_CHANGE: u8 = 0b1000;

/// The offset of the DLAB bit in the line control register.
const DLAB: u8 = 1 << 7;

/// Bit of the Line Status Register telling whether data is available to be read.
const LINE_STATUS_DR: u8 = 0b1;
/// Bit of the Line Status Register telling whether data has been lost.
const LINE_STATUS_OE: u8 = 0b10;
/// Bit of the Line Status Register telling whether a transmission error was detect by parity.
const LINE_STATUS_PE: u8 = 0b100;
/// Bit of the Line Status Register telling whether a framing error was detected.
const LINE_STATUS_FE: u8 = 0b1000;
/// Bit of the Line Status Register telling whether there is a break in data input.
const LINE_STATUS_BI: u8 = 0b10000;
/// Bit of the Line Status Register telling whether the transmission buffer is empty.
const LINE_STATUS_THRE: u8 = 0b100000;
/// Bit of the Line Status Register telling whether the transmitter is idling.
const LINE_STATUS_TEMT: u8 = 0b1000000;
/// Bit of the Line Status Register telling whether there is an error with a word in the input
/// buffer.
const LINE_STATUS_IE: u8 = 0b10000000;

/// The UART's frequency.
const UART_FREQUENCY: u32 = 115200; // TODO Replace by a rational number?

// TODO Add feature to avoid multiple instances on one port

/// Structure representing a serial communication port.
pub struct Serial {
	/// The offset of the port's I/O registers.
	regs_off: u16,
}

impl Serial {
	/// Tests whether the given port exists.
	fn probe(&mut self) -> bool {
		unsafe {
			io::outb(self.regs_off + INTERRUPT_REG_OFF, 0x00);
			self.set_baud_rate(38400);
			io::outb(self.regs_off + LINE_CTRL_REG_OFF, 0x03);
			io::outb(self.regs_off + II_FIFO_REG_OFF, 0xc7);
			io::outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x0b);
			io::outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x1e);
			io::outb(self.regs_off + DATA_REG_OFF, 0xae);

			if io::inb(self.regs_off + DATA_REG_OFF) != 0xae {
				return false;
			}

			io::outb(self.regs_off + MODEM_CTRL_REG_OFF, 0x0f);
		}

		true
	}

	/// Creates a new instance for the specified port. If the port doesn't exist, the function
	/// returns None.
	fn from_port(port: u16) -> Option<Serial> {
		let mut s = Self {
			regs_off: port,
		};

		if s.probe() {
			Some(s)
		} else {
			None
		}
	}

	/// Sets the port's baud rate. If the baud rate is not supported, the function approximates it
	/// to the nearest supported value.
	pub fn set_baud_rate(&mut self, baud: u32) {
		let div = (UART_FREQUENCY / baud) as u16;

		unsafe {
			let line_ctrl = io::inb(self.regs_off + LINE_CTRL_REG_OFF);
			io::outb(self.regs_off + LINE_CTRL_REG_OFF, line_ctrl | DLAB);

			io::outb(self.regs_off + DIVISOR_LO_REG_OFF, (div & 0xff) as _);
			io::outb(self.regs_off + DIVISOR_HI_REG_OFF, ((div >> 8) & 0xff) as _);

			io::outb(self.regs_off + LINE_CTRL_REG_OFF, line_ctrl & !DLAB);
		}
	}

	/// Tells whether the transmission buffer is empty.
	fn is_transmit_empty(&self) -> bool {
		(unsafe {
			io::inb(self.regs_off + LINE_STATUS_REG_OFF)
		} & LINE_STATUS_THRE) != 0
	}

	// TODO read

	/// Writes the given buffer to the port's output.
	pub fn write(&mut self, buff: &[u8]) {
		for b in buff {
			while !self.is_transmit_empty() {}

			unsafe {
				io::outb(self.regs_off + DATA_REG_OFF, *b);
			}
		}
	}
}

/// The list of serial ports.
static mut PORTS: [Option<Mutex<Serial>>; 4] = [
	None,
	None,
	None,
	None,
];

/// Returns an instance to an object allowing to use the given serial communication port. If the
/// port is not initialized, the function tries to do it. If the port doesn't exist, the function
/// returns None.
pub fn get(port: u16) -> Option<&'static mut Mutex<Serial>> {
	let i = match port {
		COM1 => 0,
		COM2 => 1,
		COM3 => 2,
		COM4 => 3,

		_ => return None,
	};

	let ports = unsafe { // Safe because using Mutex
		&mut PORTS
	};
	if ports[i].is_none() {
		if let Some(s) = Serial::from_port(port) {
			ports[i] = Some(Mutex::new(s));
		}
	}

	ports[i].as_mut()
}
