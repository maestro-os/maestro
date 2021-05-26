//! This module handles the PIT (Programmable Interrupt Timer) which allows to trigger
//! interruptions at a fixed interval.

use crate::idt;
use crate::io;
use crate::util::lock::mutex::*;
use crate::util::math;

/// The type representing the frequency of the PIT in Hertz.
pub type Frequency = u32;

/// PIT channel number 0.
const CHANNEL_0: u16 = 0x40;
/// PIT channel number 1.
const CHANNEL_1: u16 = 0x41;
/// PIT channel number 2.
const CHANNEL_2: u16 = 0x42;
/// The port to send a command to the PIT.
const PIT_COMMAND: u16 = 0x43;

/// The command to enable the PC speaker.
const BEEPER_ENABLE_COMMAND: u8 = 0x61;

/// TODO doc
const SELECT_CHANNEL_0: u8 = 0x0;
/// TODO doc
const SELECT_CHANNEL_1: u8 = 0x40;
/// TODO doc
const SELECT_CHANNEL_2: u8 = 0x80;
/// TODO doc
const READ_BACK_COMMAND: u8 = 0xc0;

/// TODO doc
const ACCESS_LATCH_COUNT_VALUE: u8 = 0x0;
/// TODO doc
const ACCESS_LOBYTE: u8 = 0x10;
/// TODO doc
const ACCESS_HIBYTE: u8 = 0x20;
/// TODO doc
const ACCESS_LOBYTE_HIBYTE: u8 = 0x30;

/// TODO doc
const MODE_0: u8 = 0x0;
/// TODO doc
const MODE_1: u8 = 0x1;
/// TODO doc
const MODE_2: u8 = 0x2;
/// TODO doc
const MODE_3: u8 = 0x3;
/// TODO doc
const MODE_4: u8 = 0x4;
/// TODO doc
const MODE_5: u8 = 0x5;

/// The base frequency of the PIT.
const BASE_FREQUENCY: Frequency = 1193180;

/// The current frequency of the PIT.
static mut CURRENT_FREQUENCY: Mutex::<Frequency> = Mutex::new(0);

/// Initializes the PIT.
/// This function disables interrupts.
pub fn init() {
	idt::wrap_disable_interrupts(|| {
		unsafe {
			io::outb(PIT_COMMAND, SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_4);
			io::outb(PIT_COMMAND, SELECT_CHANNEL_2 | ACCESS_LOBYTE_HIBYTE | MODE_4);
		}
	});
}

/// Sets the PIT divider value to `count`.
/// This function disables interrupts.
pub fn set_value(count: u16) {
	idt::wrap_disable_interrupts(|| {
		unsafe {
			io::outb(CHANNEL_0, (count & 0xff) as u8);
			io::outb(CHANNEL_0, ((count >> 8) & 0xff) as u8);
		}
	});
}

/// Sets the current frequency of the PIT to `frequency` in hertz.
/// This function disables interrupts.
pub fn set_frequency(frequency: Frequency) {
	let m = unsafe { // Safe because using a Mutex
		&mut CURRENT_FREQUENCY
	};
	let mut guard = MutexGuard::new(m);
	*guard.get_mut() = frequency;

	let mut c = if frequency != 0 {
		math::ceil_division(BASE_FREQUENCY, frequency)
	} else {
		0
	};
	c &= 0xffff;
	if c & !0xffff != 0 {
		c = 0;
	}
	set_value(c as u16);
}

/// Makes PC speaker ring the bell.
pub fn beep() {
	// TODO
}
