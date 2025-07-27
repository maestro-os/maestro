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
			LVT_MASKED, LVT_ONESHOT, REG_LVT_TIMER, REG_TIMER_CURRENT_COUNT, REG_TIMER_DIVIDE,
			REG_TIMER_INIT_COUNT, get_base_addr,
		},
		timer::hpet,
	},
	memory::PhysAddr,
	println,
};
use core::{hint, hint::likely};
use utils::errno::AllocResult;

/// Measures and stores the frequency of the APIC timer, using the HPET.
pub(crate) fn calibrate_hpet() -> AllocResult<()> {
	let base_addr = PhysAddr(get_base_addr())
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// The amount of ticks over which we calibrate
	const APIC_TICKS: u32 = 0x10000;
	// Setup APIC
	let period = unsafe {
		// Use divider `16`
		apic::write_reg(base_addr, REG_TIMER_DIVIDE, 3);
		hpet::set_enabled(true);
		let hpet_before = hpet::read_counter();
		apic::write_reg(base_addr, REG_TIMER_INIT_COUNT, APIC_TICKS);
		apic::write_reg(base_addr, REG_LVT_TIMER, LVT_ONESHOT | LVT_MASKED);
		// Wait for the APIC counter to reach zero
		while likely(apic::read_reg(base_addr, REG_TIMER_CURRENT_COUNT) != 0) {
			hint::spin_loop();
		}
		// Compute elapsed time
		let hpet_delta = hpet::read_counter() - hpet_before;
		hpet::set_enabled(false);
		let period = hpet_delta * hpet::INFO.tick_period as u64;
		period / APIC_TICKS as u64
	};
	println!("period per tick {period}");
	// TODO store
	Ok(())
}

/// Measures and stores the frequency of the APIC timer, using the PIT.
pub(crate) fn calibrate_pit() {
	todo!()
}
