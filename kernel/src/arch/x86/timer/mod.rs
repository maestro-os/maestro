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

//! x86 timers implementation.
//!
//! The following timers are available:
//! - PIT (legacy)
//! - RTC (legacy)
//! - APIC
//! - HPET
//!
//! If the APIC is present, its timer shall be used for scheduling. If not, the kernel fallbacks on
//! the PIT.
//!
//! Since we do not know the frequency of the APIC timer, we need to use another timer with a known
//! frequency to measure it. This is called **calibration**.
//!
//! The kernel will attempt to detect the presence of an HPET.
//!
//! TODO: if the HPET is net present, fallback on the PIT

// TODO calibrate the TSC if present and use it for timekeeping.
// If the TSC is unavailable, fallback in this order:
// - HPET
// - APIC
// - RTC
// - PIT

use crate::{
	acpi,
	arch::{x86, x86::timer::hpet::AcpiHpet},
};
use utils::errno::AllocResult;

pub mod apic;
pub mod hpet;
pub mod pit;
pub mod rtc;

/// Makes the current CPU cores wait for at least `ms` milliseconds.
#[inline]
pub fn mdelay(ms: u32) {
	ndelay(ms * 1_000_000)
}

/// Makes the current CPU cores wait for at least `us` microseconds.
#[inline]
pub fn udelay(us: u32) {
	ndelay(us * 1000)
}

/// Makes the current CPU cores wait for at least `ns` nanoseconds.
pub fn ndelay(ns: u32) {
	if x86::apic::is_present() {
		apic::ndelay(ns);
	} else {
		todo!() // use PIT
	}
}

/// Initializes x86 timers.
pub(crate) fn init() -> AllocResult<()> {
	if !x86::apic::is_present() {
		// We assume the PIT is the only timer present
		pit::init(10);
		return Ok(());
	}
	// Initialize a known-frequency timer
	if let Some(hpet) = acpi::get_table::<AcpiHpet>() {
		hpet::init(hpet);
		apic::calibrate_hpet()?;
	} else {
		// No HPET, we assume the PIT is present
		pit::init(10);
		apic::calibrate_pit();
	}
	Ok(())
}
