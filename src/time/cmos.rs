/// This module implements CMOS control features.
/// The CMOS is a persistent memory used to store some BIOS settings.

use crate::idt;
use crate::io;
use super::Timestamp;

/// The ID of the port used to select the CMOS register to read.
const SELECT_PORT: u16 = 0x70;
/// The ID of the port to read or write a CMOS port previously selected.
const VALUE_PORT: u16 = 0x71;

/// The ID of the register storing the current time second.
const SECOND_REGISTER: u8 = 0x00;
/// The ID of the register storing the current time minute.
const MINUTE_REGISTER: u8 = 0x02;
/// The ID of the register storing the current time hour.
const HOUR_REGISTER: u8 = 0x04;
/// The ID of the register storing the current time day of month.
const DAY_OF_MONTH_REGISTER: u8 = 0x07;
/// The ID of the register storing the current time month.
const MONTH_REGISTER: u8 = 0x08;
/// The ID of the register storing the current time year.
const YEAR_REGISTER: u8 = 0x09;
/// The ID of the register storing the current time century.
const CENTURY_REGISTER: u8 = 0x32;

/// The ID of the status register A.
const STATUS_A_REGISTER: u8 = 0x0a;
/// The ID of the status register B.
const STATUS_B_REGISTER: u8 = 0x0b;

/// Bit of status register A, tells whether the time is being updated.
const UPDATE_FLAG: u8 = 1 << 7;
/// Bit of status register B, tells whether the 24 hour format is set.
const FORMAT_24_FLAG: u8 = 1 << 1;
/// Bit of status register B, tells whether binary mode is set.
const FORMAT_BCD_FLAG: u8 = 1 << 2;

/// The ID of the register used to store the Floopy Drive type.
const FLOPPY_DRIVE_REGISTER: u8 = 0x10;

/// Reads the register `reg` and returns the value.
fn read(reg: u8) -> u8 {
	unsafe {
		io::outb(SELECT_PORT, (1 << 7) | reg);
		io::inb(VALUE_PORT)
	}
}

/// Enumeration representing a drive type.
pub enum FloppyDriveType {
	/// No drive is present.
	NoDrive,
	/// 360KB, 5.25 inches
	Type360kb525,
	/// 1200KB, 5.25 inches
	Type1200kb525,
	/// 720KB, 3.5 inches
	Type720kb350,
	/// 1440KB, 3.5 inches
	Type1440kb350,
	/// 2880KB, 3.5 inches
	Type2880kb350,
}

/// Structure representing the state of the floppy drives.
pub struct FloppyDrives {
	/// The type of the master floppy drive.
	master: FloppyDriveType,
	/// The type of the slave floppy drive.
	slave: FloppyDriveType,
}

/// Converts the given number to the associated floppy type.
fn floppy_type_from_number(n: u8) -> FloppyDriveType {
	match n {
		1 => FloppyDriveType::Type360kb525,
		2 => FloppyDriveType::Type1200kb525,
		3 => FloppyDriveType::Type720kb350,
		4 => FloppyDriveType::Type1440kb350,
		5 => FloppyDriveType::Type2880kb350,
		_ => FloppyDriveType::NoDrive,
	}
}

/// Returns the state of the floppy drives.
pub fn get_floppy_type() -> FloppyDrives {
	let floppy_state = read(FLOPPY_DRIVE_REGISTER);
	let master_state = (floppy_state >> 4) & 0xf;
	let slave_state = floppy_state & 0xf;

	FloppyDrives {
		master: floppy_type_from_number(master_state),
		slave: floppy_type_from_number(slave_state),
	}
}

/// Tells whether the CMOS is ready for time reading.
fn is_time_ready() -> bool {
	read(STATUS_A_REGISTER) & UPDATE_FLAG == 0
}

/// Waits for the CMOS to be ready for reading the time.
fn time_wait() {
	while is_time_ready() {}
	while !is_time_ready() {}
}

/// Tells whether the given year is a leap year or not.
fn is_leap_year(year: u32) -> bool {
	if year % 4 != 0 {
		false
	} else if year % 100 != 0 {
		true
	} else if year % 400 != 0 {
		false
	} else {
		true
	}
}

// TODO Compute in constant time
/// Returns the number of leap years between the two years.
/// `y0` and `y1` are the range in years. If `y1` is greater than `y0`, then the behaviour is
/// undefined.
fn leap_years_between(y0: u32, y1: u32) -> u32 {
	let mut n = 0;
	for i in y1..y0 {
		if is_leap_year(i) {
			n += 1;
		}
	}
	n
}

/// Returns the number of days since epoch from the year, month and day of the month.
fn get_days_since_epoch(year: u32, month: u32, day: u32) -> u32 {
	let year_days = (year - 1970) * 365 + leap_years_between(year, 1970);
	let month_days = (((month + 1) / 2) * 31) + ((month / 2) * 30);
	year_days + month_days + day
}

/// Returns the current timestamp in seconds from the CMOS time.
/// The function might wait up to 1 second for the CMOS to be ready for reading.
/// The function also disables maskable interrupts during its execution.
/// `century_register` tells whether the century register exists or not. If it doesn't exist, the
/// 21st century is taken.
pub fn get_time(century_register: bool) -> Timestamp {
	let mut timestamp: Timestamp = 0;

	idt::wrap_disable_interrupts(|| {
		time_wait();
		let mut second = read(SECOND_REGISTER) as u32;
		let mut minute = read(MINUTE_REGISTER) as u32;
		let mut hour = read(HOUR_REGISTER) as u32;
		let mut day = read(DAY_OF_MONTH_REGISTER) as u32;
		let mut month = read(MONTH_REGISTER) as u32;
		let mut year = read(YEAR_REGISTER) as u32;
		let mut century = if century_register {
			read(CENTURY_REGISTER)
		} else {
			20
		} as u32;

		let status_b = read(STATUS_B_REGISTER);
		if status_b & FORMAT_BCD_FLAG == 0 {
			second = (second & 0x0f) + ((second / 16) * 10);
        	minute = (minute & 0x0f) + ((minute / 16) * 10);
        	hour = ((hour & 0x0f) + (((hour & 0x70) / 16) * 10)) | (hour & 0x80);
        	day = (day & 0x0f) + ((day / 16) * 10);
        	month = (month & 0x0f) + ((month / 16) * 10);
        	year = (year & 0x0f) + ((year / 16) * 10);
        	if century_register {
            	century = (century & 0x0f) + (century / 16) * 10;
        	}
		}

		if (status_b & FORMAT_24_FLAG) == 0 && (hour & 0x80) != 0 {
			hour = ((hour & 0x7f) + 12) % 24;
		}

		year += century * 100;

		let days_since_epoch = get_days_since_epoch(year, month - 1, day - 3); // TODO Determine why `- 3` instead of `- 1`
		timestamp = days_since_epoch * 86400 + hour * 3600 + minute * 60 + second;
	});

	timestamp
}
