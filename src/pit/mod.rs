//! This module handles the PIT (Programmable Interrupt Timer) which allows to trigger
//! interruptions at a fixed interval.

use crate::idt;
use crate::io;
use crate::util::lock::*;
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

/// Select PIT channel 0.
const SELECT_CHANNEL_0: u8 = 0b00 << 6;
/// Select PIT channel 1.
const SELECT_CHANNEL_1: u8 = 0b01 << 6;
/// Select PIT channel 2.
const SELECT_CHANNEL_2: u8 = 0b10 << 6;
/// The read back command, used to read the current state of the PIT (doesn't work on 8253 and
/// older).
const READ_BACK_COMMAND: u8 = 0b11 << 6;

/// Tells the PIT to copy the current count to the latch register to be read by the CPU.
const ACCESS_LATCH_COUNT_VALUE: u8 = 0b00 << 4;
/// Tells the PIT to read only the lowest 8 bits of the counter value.
const ACCESS_LOBYTE: u8 = 0b01 << 4;
/// Tells the PIT to read only the highest 8 bits of the counter value.
const ACCESS_HIBYTE: u8 = 0b10 << 4;
/// Tells the PIT to read the whole counter value.
const ACCESS_LOBYTE_HIBYTE: u8 = 0b11 << 4;

/// Interrupt on terminal count.
const MODE_0: u8 = 0b000 << 1;
/// Hardware re-triggerable one-shot.
const MODE_1: u8 = 0b001 << 1;
/// Rate generator.
const MODE_2: u8 = 0b010 << 1;
/// Square wave generator.
const MODE_3: u8 = 0b011 << 1;
/// Software triggered strobe.
const MODE_4: u8 = 0b100 << 1;
/// Hardware triggered strobe.
const MODE_5: u8 = 0b101 << 1;

/// Tells whether the BCD mode is enabled.
const BCD_MODE: u8 = 0b1;

/// The base frequency of the PIT.
const BASE_FREQUENCY: Frequency = 1193180;

/// The current frequency of the PIT.
static CURRENT_FREQUENCY: Mutex<Frequency> = Mutex::new(0);

/// Initializes the PIT.
/// This function disables interrupts.
pub fn init() {
	idt::wrap_disable_interrupts(|| {
		unsafe {
			io::outb(PIT_COMMAND, SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_2);
			io::outb(PIT_COMMAND, SELECT_CHANNEL_2 | ACCESS_LOBYTE_HIBYTE | MODE_2);
		}

		set_frequency(1); // TODO
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
	let mut guard = CURRENT_FREQUENCY.lock();
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
