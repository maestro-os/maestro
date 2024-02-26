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

//! Implementation of memory space transactions to modify [`MemSpaceState`] atomically.

use super::{gap::MemGap, mapping::MemMapping, MemSpace, MemSpaceState};
use crate::{
	errno,
	errno::{AllocError, AllocResult, EResult},
	util::collections::{
		btreemap::{BTreeMap, Entry},
		vec::Vec,
	},
};
use core::{ffi::c_void, mem, num::NonZeroUsize};

/// Applies the difference in `complement` to rollback a [`union`] operation.
///
/// If the complement does not correspond to `on`, the function might panic.
pub fn rollback_union<K: Ord, V>(on: &mut BTreeMap<K, V>, complement: Vec<(K, Option<V>)>) {
	for (key, value) in complement {
		// Apply diff
		match value {
			// Insertion cannot fail since `on` is guaranteed to already contain the key
			Some(value) => {
				on.insert(key, value).unwrap();
			}
			None => {
				on.remove(&key);
			}
		}
	}
}

fn union_impl<K: Clone + Ord, V>(
	from: BTreeMap<K, V>,
	to: &mut BTreeMap<K, V>,
	complement: &mut Vec<(K, Option<V>)>,
) -> AllocResult<()> {
	for (key, value) in from {
		// Insert new value and get previous
		let old = match to.entry(key.clone()) {
			Entry::Occupied(mut e) => Some(e.insert(value)),
			Entry::Vacant(e) => {
				e.insert(value)?;
				None
			}
		};
		// Keep previous value in complement vector
		complement.push((key.clone(), old))?;
	}
	Ok(())
}

/// Clones and inserts all elements of `from` to `to`.
///
/// The function returns the complement to be used to roll back the union with [`rollback`].
pub fn union<K: Clone + Ord, V>(
	from: BTreeMap<K, V>,
	to: &mut BTreeMap<K, V>,
) -> AllocResult<Vec<(K, Option<V>)>> {
	let mut complement = Vec::with_capacity(from.len())?;
	match union_impl(from, to, &mut complement) {
		Ok(_) => Ok(complement),
		Err(_) => {
			rollback_union(to, complement);
			Err(AllocError)
		}
	}
}

/// A transaction to be performed on a memory space.
///
/// Since mapping or unmapping memory required separate insert and remove operations, and insert
/// operations can fail, it is necessary to ensure every operation are performed, or rollback to
/// avoid inconsistent states.
#[derive(Default)]
pub struct MemSpaceTransaction {
	/// Buffer used to store insertions.
	pub buffer_state: MemSpaceState,
	/// The list of mappings to remove, by address.
	pub remove_mappings: Vec<*const c_void>,
	/// The list of gaps to remove, by address.
	pub remove_gaps: Vec<*const c_void>,
}

/// Data used for transaction rollback.
///
/// This structure is used in the implementation of [`MemSpaceTransaction`].
#[derive(Default)]
struct RollbackData {
	gaps_complement: Vec<(*const c_void, Option<MemGap>)>,
	gaps_size_complement: Vec<((NonZeroUsize, *const c_void), Option<()>)>,
	mappings_complement: Vec<(*const c_void, Option<MemMapping>)>,
}

impl MemSpaceTransaction {
	/// Rollbacks modifications made by [`commit_impl`].
	#[cold]
	fn rollback(on: &mut MemSpace, rollback_data: RollbackData) {
		rollback_union(&mut on.state.gaps, rollback_data.gaps_complement);
		rollback_union(&mut on.state.gaps_size, rollback_data.gaps_size_complement);
		rollback_union(&mut on.state.mappings, rollback_data.mappings_complement);
	}

	fn commit_impl(&mut self, on: &mut MemSpace, rollback_data: &mut RollbackData) -> EResult<()> {
		// Apply changes on the virtual memory context
		let mut vmem_transaction = on.vmem.transaction();
		// Unmap on virtual memory context
		let iter = self
			.remove_mappings
			.iter()
			.cloned()
			.filter_map(|m| self.buffer_state.mappings.get(&(m as _)));
		for m in iter {
			let size = m.get_size().get();
			m.unmap(0..size, &mut vmem_transaction)?;
		}
		// Map on virtual memory context
		for (_, m) in self.buffer_state.mappings.iter() {
			m.map_default(&mut vmem_transaction)?;
		}
		// Update memory space structures
		let gaps = mem::take(&mut self.buffer_state.gaps);
		rollback_data.gaps_complement = union(gaps, &mut on.state.gaps)?;
		let gaps_size = mem::take(&mut self.buffer_state.gaps_size);
		rollback_data.gaps_size_complement = union(gaps_size, &mut on.state.gaps_size)?;
		let mappings = mem::take(&mut self.buffer_state.mappings);
		rollback_data.mappings_complement = union(mappings, &mut on.state.mappings)?;
		// Here, all fallible operations have been performed successfully
		vmem_transaction.commit();
		// Removals can be performed after because removals that overlap with insertions have been
		// removed. This reduces the complexity of the rollback operation since removals cannot
		// fail
		for m in self.remove_mappings.iter().cloned() {
			on.state.mappings.remove(&(m as _));
		}
		for g in self.remove_gaps.iter().cloned() {
			on.state.remove_gap(g as _);
		}
		Ok(())
	}

	/// Commits the transaction on the given state.
	pub fn commit(mut self, on: &mut MemSpace) -> AllocResult<()> {
		// Filter out remove orders that are overlapping with insert orders
		self.remove_mappings
			.retain(|m| !self.buffer_state.mappings.contains_key(&(*m as _)));
		self.remove_gaps
			.retain(|g| !self.buffer_state.gaps.contains_key(&(*g as _)));
		// Commit
		let mut rollback_data = RollbackData::default();
		let res = self.commit_impl(on, &mut rollback_data);
		// On allocation failure, rollback
		// Other kind of errors may appear but have to be ignored, such as I/O error on disk
		match res {
			Err(e) if e.as_int() == errno::ENOMEM => {
				Self::rollback(on, rollback_data);
				Err(AllocError)
			}
			_ => Ok(()),
		}
	}
}
