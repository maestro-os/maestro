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
//! - APIC
//! - HPET
//!
//! If the APIC is present, its timer shall be used for scheduling. If not, the kernel fallbacks on
//! the PIT.
//!
//! Since we do not know the frequency of the APIC timer, we need to use another timer with a known
//! frequency to measure it. This is called **calibration**.
//!
//! The kernel will attempt to detect the presence of an HPET. If not present, it will then
//! fallback on the PIT.

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

pub mod apic;
pub mod hpet;
pub mod pit;
mod rtc;

/// Initializes x86 timers.
pub(crate) fn init() {
	if !x86::apic::is_present() {
		// We assume the PIT is the only timer present
		pit::init(1); // TODO choose another frequency
		return;
	}
	// Detect HPET
	let acpi_hpet = acpi::get_table::<AcpiHpet>();
	// Initialize a known-frequency timer
	if let Some(hpet) = acpi_hpet {
		let hpet = hpet::init(hpet);
		apic::calibrate(hpet);
	} else {
		let pit = pit::init(1); // TODO choose another frequency
		apic::calibrate(pit);
	}
}
