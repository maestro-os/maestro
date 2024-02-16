//! Implementation of memory space transactions to modify [`MemSpaceState`] atomically.

use super::{gap::MemGap, mapping::MemMapping, MemSpaceState};
use crate::{
	errno::{AllocError, AllocResult},
	util::{
		collections::{
			btreemap::{BTreeMap, Entry},
			vec::Vec,
		},
		TryClone,
	},
};
use core::{ffi::c_void, mem, num::NonZeroUsize};

/// Clones and inserts all elements of `from` to `to`.
///
/// The `complement` vector stores the complement of modifications to be used by [`rollback`].
///
/// **Warning**: on memory allocation failure, `to` is left altered mid-way.
fn union<K, V>(
	from: BTreeMap<K, V>,
	to: &mut BTreeMap<K, V>,
	complement: &mut Vec<(K, Option<V>)>,
) -> AllocResult<()>
where
	K: Clone + Ord,
	V: TryClone<Error = AllocError>,
{
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

/// Applies the difference in `complement` to rollback a [`union`] operation.
///
/// If the complement does not correspond to `on`, the function might panic.
fn rollback_union<K: Ord, V>(on: &mut BTreeMap<K, V>, complement: Vec<(K, Option<V>)>) {
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
	gaps_complement: Vec<(*mut c_void, Option<MemGap>)>,
	gaps_size_complement: Vec<((NonZeroUsize, *mut c_void), Option<()>)>,
	mappings_complement: Vec<(*mut c_void, Option<MemMapping>)>,
}

impl MemSpaceTransaction {
	/// Rollbacks modifications made by [`commit_impl`].
	#[cold]
	fn rollback(on: &mut MemSpaceState, rollback_data: RollbackData) {
		rollback_union(&mut on.gaps, rollback_data.gaps_complement);
		rollback_union(&mut on.gaps_size, rollback_data.gaps_size_complement);
		rollback_union(&mut on.mappings, rollback_data.mappings_complement);
	}

	fn commit_impl(
		&mut self,
		on: &mut MemSpaceState,
		rollback_data: &mut RollbackData,
	) -> AllocResult<()> {
		// Insertions
		let gaps = mem::replace(&mut self.buffer_state.gaps, BTreeMap::new());
		union(gaps, &mut on.gaps, &mut rollback_data.gaps_complement)?;
		let gaps_size = mem::replace(&mut self.buffer_state.gaps_size, BTreeMap::new());
		union(
			gaps_size,
			&mut on.gaps_size,
			&mut rollback_data.gaps_size_complement,
		)?;
		let mappings = mem::replace(&mut self.buffer_state.mappings, BTreeMap::new());
		union(
			mappings,
			&mut on.mappings,
			&mut rollback_data.mappings_complement,
		)?;
		// Removals can be performed after because removals that overlap with insertions have been
		// removed. This reduces the complexity of the rollback operation since removals cannot
		// fail
		for m in self.remove_mappings.iter().cloned() {
			on.mappings.remove(&(m as _));
		}
		for g in self.remove_gaps.iter().cloned() {
			on.remove_gap(g as _);
		}
		Ok(())
	}

	/// Commits the transaction on the given state.
	pub fn commit(mut self, on: &mut MemSpaceState) -> AllocResult<()> {
		// Filter out remove orders that are overlapping with insert orders
		self.remove_mappings
			.retain(|m| !self.buffer_state.mappings.contains_key(&(*m as _)));
		self.remove_gaps
			.retain(|g| !self.buffer_state.gaps.contains_key(&(*g as _)));
		// Commit
		let mut rollback_data = RollbackData::default();
		let res = self.commit_impl(on, &mut rollback_data);
		if res.is_err() {
			Self::rollback(on, rollback_data);
		}
		res
	}
}
