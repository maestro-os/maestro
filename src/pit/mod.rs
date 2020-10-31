/*
 * TODO doc
 */

use crate::io;
use crate::util;

pub type Frequency = u32;

/*
 * TODO doc
 */
const CHANNEL_0: u16 = 0x40;
/*
 * TODO doc
 */
const CHANNEL_1: u16 = 0x41;
/*
 * TODO doc
 */
const CHANNEL_2: u16 = 0x42;
/*
 * TODO doc
 */
const COMMAND: u16 = 0x43;

/*
 * The command to enable the PC speaker.
 */
const BEEPER_ENABLE: u8 = 0x61;

/*
 * TODO doc
 */
const SELECT_CHANNEL_0: u8 = 0x0;
/*
 * TODO doc
 */
const SELECT_CHANNEL_1: u8 = 0x40;
/*
 * TODO doc
 */
const SELECT_CHANNEL_2: u8 = 0x80;
/*
 * TODO doc
 */
const READ_BACK_COMMAND: u8 = 0xc0;

/*
 * TODO doc
 */
const ACCESS_LATCH_COUNT_VALUE: u8 = 0x0;
/*
 * TODO doc
 */
const ACCESS_LOBYTE: u8 = 0x10;
/*
 * TODO doc
 */
const ACCESS_HIBYTE: u8 = 0x20;
/*
 * TODO doc
 */
const ACCESS_LOBYTE_HIBYTE: u8 = 0x30;

/*
 * TODO doc
 */
const MODE_0: u8 = 0x0;
/*
 * TODO doc
 */
const MODE_1: u8 = 0x1;
/*
 * TODO doc
 */
const MODE_2: u8 = 0x2;
/*
 * TODO doc
 */
const MODE_3: u8 = 0x3;
/*
 * TODO doc
 */
const MODE_4: u8 = 0x4;
/*
 * TODO doc
 */
const MODE_5: u8 = 0x5;

/*
 * The base frequency of the PIT.
 */
const BASE_FREQUENCY: Frequency = 1193180;

/*
 * The current frequency of the PIT in hertz.
 */
static mut CURRENT_FREQUENCY: Frequency = 0;

/*
 * Initializes the PIT.
 * This function disables interrupts.
 */
pub fn init() {
	crate::cli!();

	unsafe {
		io::outb(COMMAND, SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_4);
		io::outb(COMMAND, SELECT_CHANNEL_2 | ACCESS_LOBYTE_HIBYTE | MODE_4);
	}
}

/*
 * Sets the PIT divider value to `count`.
 * This function disables interrupts.
 */
pub fn set_value(count: u16) {
	crate::cli!();

	unsafe {
		io::outb(CHANNEL_0, (count & 0xff) as u8);
		io::outb(CHANNEL_0, ((count >> 8) & 0xff) as u8);
	}

	// TODO Enable interrupts back if they were enabled in the first place?
}

/*
 * Sets the current frequency of the PIT to `frequency` in hertz.
 * This function disables interrupts.
 */
pub fn set_frequency(frequency: Frequency) {
	unsafe {
		CURRENT_FREQUENCY = frequency;
	}

	let mut c = {
		if frequency != 0 {
			util::ceil_division(BASE_FREQUENCY, frequency)
		} else {
			0
		}
	};
	if c & !0xffff != 0 {
		c = 0;
	}
	set_value(c as u16);
}
