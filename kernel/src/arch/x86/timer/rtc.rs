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

//! RTC (Real Time Clock) implementation.

use crate::{
	acpi,
	arch::x86::{
		idt::disable_int,
		io::{inb, outb},
	},
	println,
	time::unit::Timestamp,
};
use core::{hint, hint::unlikely};

/// The ID of the port used to select the CMOS register to read.
const SELECT_PORT: u16 = 0x70;
/// The ID of the port to read or write a CMOS port previously selected.
const VALUE_PORT: u16 = 0x71;

/// The ID of the status register A.
const STATUS_A_REGISTER: u8 = 0x0a;
/// The ID of the status register B.
const STATUS_B_REGISTER: u8 = 0x0b;
/// The ID of the status register C.
const STATUS_C_REGISTER: u8 = 0x0c;

/// Interrupt vector for the RTC.
pub const INTERRUPT_VECTOR: u8 = 0x28;

/// Reads the value of a register
fn read_reg(reg: u8) -> u8 {
	unsafe {
		outb(SELECT_PORT, reg);
		inb(VALUE_PORT)
	}
}

/// Enables or disables the RTC.
pub fn set_enabled(enable: bool) {
	disable_int(|| unsafe {
		let prev = read_reg(STATUS_B_REGISTER | 0x80);
		outb(SELECT_PORT, STATUS_B_REGISTER | 0x80);
		if enable {
			outb(VALUE_PORT, prev | 0x40);
		} else {
			outb(VALUE_PORT, prev & !0x40);
		}
	});
}

/// Sets the RTC's frequency.
pub fn set_frequency(freq: u32) {
	let rate = (32768u32 / freq).trailing_zeros() as u8 + 1;
	disable_int(|| unsafe {
		let prev = read_reg(STATUS_A_REGISTER | 0x80);
		outb(SELECT_PORT, STATUS_A_REGISTER | 0x80);
		outb(VALUE_PORT, (prev & 0xf0) | (rate & 0x0f));
	});
}

/// Allows the next RTC tick to happen.
pub fn reset() {
	read_reg(STATUS_C_REGISTER);
}

/// Tells whether an update of the clock is currently in progress
fn is_update_in_progress() -> bool {
	unsafe {
		outb(SELECT_PORT, STATUS_A_REGISTER);
		inb(VALUE_PORT) & 0x80 != 0
	}
}

/// Reads time from registers
fn read_time_regs(time: &mut [u8; 7], century_reg: u8) {
	while unlikely(is_update_in_progress()) {
		hint::spin_loop();
	}
	time[0] = read_reg(0x0);
	time[1] = read_reg(0x2);
	time[2] = read_reg(0x4);
	time[3] = read_reg(0x7);
	time[4] = read_reg(0x8);
	time[5] = read_reg(0x9);
	time[6] = if century_reg != 0 {
		read_reg(century_reg)
	} else {
		0
	};
}

fn secs_through_year(year: u64, leap: &mut bool) -> u64 {
	let cycles = (year - 2000) / 400;
	let rem = (year - 2000) % 400;
	let mut centuries = 0;
	let mut leaps = 0;
	if rem == 0 {
		*leap = true;
	} else {
		centuries = rem / 100;
		let rem = rem % 100;
		if rem != 0 {
			leaps = rem / 4;
			*leap = rem % 4 == 0;
		}
	}
	leaps += 97 * cycles + 24 * centuries - if *leap { 1 } else { 0 };
	(year - 2000) * 31536000 + leaps * 86400 + 946684800 + 86400
}

fn secs_through_month(month: u8, leap: bool) -> u64 {
	const SECS_THROUGH_MONTH: [u64; 12] = [
		0,
		31 * 86400,
		59 * 86400,
		90 * 86400,
		120 * 86400,
		151 * 86400,
		181 * 86400,
		212 * 86400,
		243 * 86400,
		273 * 86400,
		304 * 86400,
		334 * 86400,
	];
	let mut t = SECS_THROUGH_MONTH[month as usize - 1];
	if leap && month >= 2 {
		t += 86400;
	}
	t
}

// The code of this function and the functions it calls is highly inspired from the musl C library
// (function `mktime`)
/// Compute the Unix timestamp from the given `time`
fn date_to_ts(time: &[u8; 7]) -> Timestamp {
	let full_year = time[6] as u64 * 100 + time[5] as u64;
	let mut t = 0;
	let mut leap = false;
	t += secs_through_year(full_year, &mut leap);
	t += secs_through_month(time[4], leap);
	t += 86400 * (time[3] as u64 - 1);
	t += 3600 * time[2] as u64;
	t += 60 * time[1] as u64;
	t += time[0] as u64;
	t
}

/// Reads the current time from the RTC.
pub fn read_time() -> Timestamp {
	let century_reg = acpi::rtc_century_register();
	// Read a first time
	let mut time = [0; 7];
	read_time_regs(&mut time, century_reg);
	// Read in a loop until two reads in a row give the same result
	loop {
		let last_time = time;
		read_time_regs(&mut time, century_reg);
		if time == last_time {
			break;
		}
	}
	let reg_b = read_reg(STATUS_B_REGISTER);
	// Convert BCD to binary if necessary
	if reg_b & 0x04 == 0 {
		time[0] = (time[0] & 0x0f) + ((time[0] / 16) * 10);
		time[1] = (time[1] & 0x0f) + ((time[1] / 16) * 10);
		time[2] = ((time[2] & 0x0f) + (((time[2] & 0x70) / 16) * 10)) | (time[2] & 0x80);
		time[3] = (time[3] & 0x0f) + ((time[3] / 16) * 10);
		time[4] = (time[4] & 0x0f) + ((time[4] / 16) * 10);
		time[5] = (time[5] & 0x0f) + ((time[5] / 16) * 10);
		if century_reg != 0 {
			time[6] = (time[6] & 0x0f) + ((time[6] / 16) * 10);
		}
	}
	// Convert 12-hour clock to 24-hour clock if necessary
	if reg_b & 0x2 == 0 && time[2] & 0x80 != 0 {
		time[2] = ((time[2] & 0x7f) + 12) % 24;
	}
	// Adjust if century register is not present. We won't support computers without a century
	// register after the year 2099
	if century_reg == 0 {
		time[6] = 20;
	}
	let ts = date_to_ts(&time) * 1_000_000_000;
	println!("ts {ts}");
	ts
}
