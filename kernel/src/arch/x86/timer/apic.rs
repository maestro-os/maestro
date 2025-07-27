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
		apic::{LVT_MASKED, REG_LVT_TIMER, REG_TIMER_DIVIDE, REG_TIMER_INIT_COUNT, get_base_addr},
		hlt,
		timer::hpet,
	},
	int,
	int::CallbackResult,
	memory::PhysAddr,
	println,
	process::scheduler::core_local,
};
use core::{hint::likely, sync::atomic::Ordering::Relaxed};
use utils::errno::AllocResult;

/// Measures and stores the frequency of the APIC timer, using the HPET.
pub(crate) fn calibrate_hpet() -> AllocResult<()> {
	let base_addr = PhysAddr(get_base_addr())
		.kernel_to_virtual()
		.unwrap()
		.as_ptr();
	// Setup APIC timer interrupt vector
	let _hook = int::register_callback(0x20, |_, _, _, _| {
		let counter = hpet::read_counter();
		// Use as a temporary storage to pass the value outside the interrupt handler. This will
		// be modified later
		core_local().time_per_jiffy.store(counter, Relaxed);
		CallbackResult::Continue
	})?
	.unwrap(); // TODO determine the ID to use and map it to the APIC
	// The amount of ticks over which we calibrate
	const APIC_TICKS: u32 = 0x100000;
	// Setup APIC
	let hpet_before = unsafe {
		// Use divider `16`
		apic::write_reg(base_addr, REG_TIMER_DIVIDE, 3);
		hpet::set_enabled(true);
		let hpet_before = hpet::read_counter();
		apic::write_reg(base_addr, REG_TIMER_INIT_COUNT, APIC_TICKS);
		hpet_before
	};
	let time_per_jiffy = &core_local().time_per_jiffy;
	// Wait for the APIC to fire an interrupt
	while likely(time_per_jiffy.load(Relaxed) == 0) {
		hlt();
	}
	let hpet_after = time_per_jiffy.load(Relaxed);
	// Adjust with the beginning value of the HPET counter
	let tpj = unsafe {
		// Disable the APIC timer
		apic::write_reg(base_addr, REG_LVT_TIMER, LVT_MASKED);
		// Compute the amount of time
		let hpet_delta = hpet_after - hpet_before;
		let period = hpet_delta * hpet::INFO.tick_period as u64;
		period / APIC_TICKS as u64
	};
	println!("tpj {tpj}");
	time_per_jiffy.store(tpj, Relaxed);
	Ok(())
}

/// Measures and stores the frequency of the APIC timer, using the PIT.
pub(crate) fn calibrate_pit() {
	todo!()
}
