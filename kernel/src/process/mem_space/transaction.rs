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

use super::{gap::MemGap, mapping::MemMapping, MemSpaceState};
use crate::memory::vmem::{VMem, VMemTransaction};
use core::{alloc::AllocError, ffi::c_void, hash::Hash, mem, num::NonZeroUsize};
use utils::{
	collections::{btreemap::BTreeMap, hashmap::HashMap},
	errno,
	errno::AllocResult,
};

/// Applies the difference in `complement` to rollback operations.
///
/// If the complement does not correspond to `on`, the function might panic.
#[cold]
pub fn rollback<K: Ord + Hash, V>(on: &mut BTreeMap<K, V>, complement: HashMap<K, Option<V>>) {
	for (key, value) in complement {
		rollback_impl(on, key, value);
	}
}

#[cold]
fn rollback_impl<K: Ord + Hash, V>(on: &mut BTreeMap<K, V>, key: K, value: Option<V>) {
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

/// Insert an element in the [`BTreeMap`] `on`, together with rollback data.
///
/// `complement` is the complement used for rollback. The function inserts an element
///
/// The `discard` list is also updated to avoid discarding an element that would be restored by the
/// complement.
fn insert<K: Clone + Ord + Hash, V>(
	key: K,
	value: V,
	on: &mut BTreeMap<K, V>,
	complement: &mut HashMap<K, Option<V>>,
	discard: &mut HashMap<K, ()>,
) -> AllocResult<()> {
	// Insert new value and get previous
	let old = on.insert(key.clone(), value)?;
	// Insert `None` to reserve memory without dropping `old` on failure
	let Ok(val) = complement.entry(key.clone()).or_insert(None) else {
		// Memory allocation failure: rollback `on` for this element
		rollback_impl(on, key, old);
		return Err(AllocError);
	};
	// Write the actual complement value
	*val = old;
	// Do not discard an element that is to be restored by the complement
	discard.remove(&key);
	Ok(())
}

/// A transaction to be performed on a memory space.
///
/// Since mapping or unmapping memory required separate insert and remove operations, and insert
/// operations can fail, it is necessary to ensure every operation are performed, or rollback to
/// avoid inconsistent states.
#[must_use = "A transaction must be committed, or its result is discarded"]
pub(super) struct MemSpaceTransaction<'m, 'v> {
	/// The memory space on which the transaction applies.
	pub mem_space_state: &'m mut MemSpaceState,
	/// The virtual memory transaction on which this transaction applies.
	vmem_transaction: VMemTransaction<'v, false>,

	/// The complement used to restore `gaps` on rollback.
	gaps_complement: HashMap<*const c_void, Option<MemGap>>,
	/// The complement used to restore `gaps_size` on rollback.
	gaps_size_complement: HashMap<(NonZeroUsize, *const c_void), Option<()>>,
	/// The complement used to restore `mappings` on rollback.
	mappings_complement: HashMap<*const c_void, Option<MemMapping>>,

	/// The list of gaps that must be discarded on commit.
	gaps_discard: HashMap<*const c_void, ()>,
	/// The list of mappings that must be discarded on commit.
	mappings_discard: HashMap<*const c_void, ()>,

	/// The new value for the `vmem_usage` field.
	vmem_usage: usize,
}

impl<'m, 'v> MemSpaceTransaction<'m, 'v> {
	/// Begins a new transaction for the given memory space.
	pub fn new(mem_space_state: &'m mut MemSpaceState, vmem: &'v mut VMem<false>) -> Self {
		let vmem_usage = mem_space_state.vmem_usage;
		Self {
			mem_space_state,
			vmem_transaction: vmem.transaction(),

			gaps_complement: Default::default(),
			gaps_size_complement: Default::default(),
			mappings_complement: Default::default(),

			gaps_discard: Default::default(),
			mappings_discard: Default::default(),

			vmem_usage,
		}
	}

	/// Inserts the given gap into the state.
	///
	/// On failure, the transaction is dropped and rollbacked.
	pub fn insert_gap(&mut self, gap: MemGap) -> AllocResult<()> {
		insert(
			gap.get_begin(),
			gap,
			&mut self.mem_space_state.gaps,
			&mut self.gaps_complement,
			&mut self.gaps_discard,
		)?;
		Ok(())
	}

	/// Removes the gap beginning at the given address from the state.
	///
	/// On failure, the transaction is dropped and rollbacked.
	pub fn remove_gap(&mut self, gap_begin: *const c_void) -> AllocResult<()> {
		if let Some(gap) = self.mem_space_state.gaps.get(&gap_begin) {
			self.gaps_discard.insert(gap.get_begin(), ())?;
		}
		Ok(())
	}

	/// Inserts the given mapping into the state.
	///
	/// On failure, the transaction is dropped and rollbacked.
	pub fn insert_mapping(&mut self, mut mapping: MemMapping) -> AllocResult<()> {
		let size = mapping.get_size().get();
		mapping.apply_to(&mut self.vmem_transaction)?;
		insert(
			mapping.get_begin(),
			mapping,
			&mut self.mem_space_state.mappings,
			&mut self.mappings_complement,
			&mut self.mappings_discard,
		)?;
		self.vmem_usage += size;
		Ok(())
	}

	/// Removes the mapping beginning at the given address from the state.
	///
	/// On failure, the transaction is dropped and rollbacked.
	pub fn remove_mapping(&mut self, mapping_begin: *const c_void) -> AllocResult<()> {
		if let Some(mapping) = self.mem_space_state.mappings.get(&mapping_begin) {
			self.mappings_discard.insert(mapping_begin, ())?;
			let size = mapping.get_size().get();
			// Apply to vmem
			let res = mapping.unmap(0..size, &mut self.vmem_transaction);
			// Ignore disk I/O errors
			if matches!(res, Err(e) if e.as_int() == errno::ENOMEM) {
				return Err(AllocError);
			}
			// Update usage
			self.vmem_usage -= size;
		}
		Ok(())
	}

	/// Commits the transaction.
	pub fn commit(&mut self) {
		// Cancel rollback
		self.gaps_complement.clear();
		self.gaps_size_complement.clear();
		self.mappings_complement.clear();
		// Discard gaps
		for (ptr, _) in self.gaps_discard.iter() {
			self.mem_space_state.gaps.remove(ptr);
		}
		// Discard mappings
		for (ptr, _) in self.mappings_discard.iter() {
			self.mem_space_state.mappings.remove(ptr);
		}
		// Update vmem
		self.mem_space_state.vmem_usage = self.vmem_usage;
		self.vmem_transaction.commit();
	}
}

impl<'m, 'v> Drop for MemSpaceTransaction<'m, 'v> {
	fn drop(&mut self) {
		// If the transaction was not committed, rollback
		let gaps_complement = mem::take(&mut self.gaps_complement);
		rollback(&mut self.mem_space_state.gaps, gaps_complement);
		let mappings_complement = mem::take(&mut self.mappings_complement);
		rollback(&mut self.mem_space_state.mappings, mappings_complement);
	}
}
