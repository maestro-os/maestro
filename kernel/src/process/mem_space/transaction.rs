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
use crate::memory::{vmem::VMem, VirtAddr};
use core::{alloc::AllocError, hash::Hash, mem};
use utils::{
	collections::{
		btreemap::BTreeMap,
		hashmap::{Entry, HashMap},
	},
	errno::AllocResult,
};

/// Applies the difference in `complement` to rollback operations.
///
/// If the complement does not correspond to `on`, the function might panic.
fn rollback<K: Ord + Hash, V>(on: &mut BTreeMap<K, V>, complement: HashMap<K, Option<V>>) {
	for (key, value) in complement {
		rollback_impl(on, key, value);
	}
}

#[cold]
fn rollback_impl<K: Ord + Hash, V>(on: &mut BTreeMap<K, V>, key: K, value: Option<V>) {
	let _ = match value {
		// Insertion cannot fail since `on` is guaranteed to already contain the key
		Some(value) => on.insert(key, value).unwrap(),
		None => on.remove(&key),
	};
}

/// Insert an element in the [`BTreeMap`] `on`, together with rollback data.
///
/// `complement` is the complement used for rollback.
///
/// The `discard` list is also updated to avoid discarding an element that is being replaced by the
/// insertion.
fn insert<K: Clone + Ord + Hash, V>(
	key: K,
	value: V,
	on: &mut BTreeMap<K, V>,
	complement: &mut HashMap<K, Option<V>>,
	discard: &mut HashMap<K, ()>,
) -> AllocResult<()> {
	// Insert new value and get previous
	let old = on.insert(key.clone(), value)?;
	// If no value is already present in the complement for the key, insert the old value
	if let Entry::Vacant(entry) = complement.entry(key.clone()) {
		// Insert `None` to allocate first so that `old` is not dropped on failure
		let Ok(val) = entry.insert(None) else {
			// Memory allocation failure: rollback `on` for this element
			complement.remove(&key);
			rollback_impl(on, key, old);
			return Err(AllocError);
		};
		// Then insert the actual value
		*val = old;
	}
	// Do not discard an element that is being replaced by the insertion
	discard.remove(&key);
	Ok(())
}

/// A transaction to be performed on a memory space.
///
/// Since mapping or unmapping memory required separate insert and remove operations, and insert
/// operations can fail, it is necessary to ensure every operation are performed, or rollback to
/// avoid inconsistent states.
#[must_use = "A transaction must be committed, or its result is discarded"]
pub(super) struct MemSpaceTransaction<'m> {
	/// The memory space on which the transaction applies.
	pub mem_space_state: &'m mut MemSpaceState,
	/// The virtual memory context on which this transaction applies.
	vmem: &'m mut VMem,

	/// The complement used to restore `gaps` on rollback.
	gaps_complement: HashMap<VirtAddr, Option<MemGap>>,
	/// The complement used to restore `mappings` on rollback.
	mappings_complement: HashMap<*mut u8, Option<MemMapping>>,

	/// The list of gaps that must be discarded on commit.
	gaps_discard: HashMap<VirtAddr, ()>,
	/// The list of mappings that must be discarded on commit.
	mappings_discard: HashMap<*mut u8, ()>,

	/// The new value for the `vmem_usage` field.
	vmem_usage: usize,
}

impl<'m> MemSpaceTransaction<'m> {
	/// Begins a new transaction for the given memory space.
	pub fn new(mem_space_state: &'m mut MemSpaceState, vmem: &'m mut VMem) -> Self {
		let vmem_usage = mem_space_state.vmem_usage;
		Self {
			mem_space_state,
			vmem,

			gaps_complement: Default::default(),
			mappings_complement: Default::default(),

			gaps_discard: Default::default(),
			mappings_discard: Default::default(),

			vmem_usage,
		}
	}

	/// Inserts the given gap into the state.
	///
	/// On failure, the transaction is dropped and rolled back.
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
	/// On failure, the transaction is dropped and rolled back.
	pub fn remove_gap(&mut self, gap_begin: VirtAddr) -> AllocResult<()> {
		if let Some(gap) = self.mem_space_state.gaps.get(&gap_begin) {
			self.gaps_discard.insert(gap.get_begin(), ())?;
		}
		Ok(())
	}

	/// Inserts the given mapping into the state.
	///
	/// On failure, the transaction is dropped and rolled back.
	pub fn insert_mapping(&mut self, mapping: MemMapping) -> AllocResult<()> {
		let size = mapping.get_size().get();
		insert(
			mapping.get_addr(),
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
	/// On failure, the transaction is dropped and rolled back.
	pub fn remove_mapping(&mut self, mapping_begin: *mut u8) -> AllocResult<()> {
		if let Some(mapping) = self.mem_space_state.mappings.get(&mapping_begin) {
			self.mappings_discard.insert(mapping_begin, ())?;
			// Apply to vmem. No rollback is required since this would be corrected by a page fault
			self.vmem
				.unmap_range(VirtAddr::from(mapping.get_addr()), mapping.get_size().get());
			// Update usage
			self.vmem_usage -= mapping.get_size().get();
		}
		Ok(())
	}

	/// Commits the transaction.
	pub fn commit(mut self) {
		// Cancel rollback
		self.gaps_complement.clear();
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
	}
}

impl Drop for MemSpaceTransaction<'_> {
	fn drop(&mut self) {
		// If the transaction was not committed, rollback
		let gaps_complement = mem::take(&mut self.gaps_complement);
		rollback(&mut self.mem_space_state.gaps, gaps_complement);
		let mappings_complement = mem::take(&mut self.mappings_complement);
		rollback(&mut self.mem_space_state.mappings, mappings_complement);
	}
}
