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

//! APIC local timer implementation.
//!
//! See the Intel Software Developer Manual for specifications.

use crate::{
	arch::x86::{
		apic,
		apic::{
			LVT_MASKED, REG_LVT_TIMER, REG_TIMER_CURRENT_COUNT, REG_TIMER_DIVIDE,
			REG_TIMER_INIT_COUNT, get_base_addr,
		},
	},
	memory::PhysAddr,
	sync::once::OnceInit,
	time::HwTimer,
};

/// The number of measured ticks during calibration.
static MEASURED_TICKS: OnceInit<u32> = unsafe { OnceInit::new() };

/// Initializes the APIC timer, using `timer` (whose frequency is known) for calibration.
pub(crate) fn calibrate<T: HwTimer>(_timer: T) {
	let base_addr = PhysAddr(get_base_addr())
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	unsafe {
		// Use divider `16`
		apic::write_reg(base_addr, REG_TIMER_DIVIDE, 3);
		// TODO init timer to sleep for 10ms
		// Init counter to `-1`
		apic::write_reg(base_addr, REG_TIMER_INIT_COUNT, 0xffffffff);
		// TODO sleep for 10ms
		apic::write_reg(base_addr, REG_LVT_TIMER, LVT_MASKED);
		// Read the number of ticks in 10ms
		let ticks = 0xffffffff - apic::read_reg(base_addr, REG_TIMER_CURRENT_COUNT);
		OnceInit::init(&MEASURED_TICKS, ticks);
	}
}
