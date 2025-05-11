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

//! Read-Copy-Update allows several threads to read and update data structures concurrently without
//! using locks.

use core::{
	mem,
	ptr::NonNull,
	sync::atomic::{
		AtomicPtr,
		Ordering::{Acquire, Relaxed, SeqCst},
	},
};
use utils::ptr::arc::{Arc, ArcInner};

/// An [`Arc`], behind a RCU.
pub struct RcuArc<T>(RcuOptionArc<T>);

impl<T> RcuArc<T> {
	/// Creates a new instance.
	#[inline]
	pub fn new(arc: Arc<T>) -> Self {
		Self(RcuOptionArc::new(Some(arc)))
	}

	/// Returns a reference to the inner [`Arc`].
	#[inline]
	pub fn get(&self) -> Arc<T> {
		let arc = self.0.get();
		unsafe { arc.unwrap_unchecked() }
	}

	/// Atomically swap the inner [`Arc`] for the given `other`.
	#[inline]
	pub fn swap(&self, other: Arc<T>) -> Arc<T> {
		let arc = self.0.swap(Some(other));
		unsafe { arc.unwrap_unchecked() }
	}
}

unsafe impl<T> Send for RcuArc<T> {}

unsafe impl<T> Sync for RcuArc<T> {}

/// An `Option<Arc<...>>`, behind a RCU.
pub struct RcuOptionArc<T> {
	inner: AtomicPtr<ArcInner<T>>,
}

impl<T> RcuOptionArc<T> {
	/// Creates a new instance.
	#[inline]
	pub fn new(arc: Option<Arc<T>>) -> Self {
		let inner = arc
			.map(|a| AtomicPtr::new(a.inner.as_ptr()))
			.unwrap_or_default();
		Self {
			inner,
		}
	}

	/// Returns a reference to the inner [`Arc`].
	pub fn get(&self) -> Option<Arc<T>> {
		// TODO enter RCU read critical section
		let inner = self.inner.load(Acquire);
		NonNull::new(inner).map(|inner| {
			let inner_ref = unsafe { inner.as_ref() };
			inner_ref.ref_count.fetch_add(1, Relaxed);
			Arc {
				inner,
			}
		})
		// TODO exit RCU read critical section before returning
	}

	/// Atomically swap the inner [`Arc`] for the given `other`.
	pub fn swap(&self, other: Option<Arc<T>>) -> Option<Arc<T>> {
		let new = other
			.as_ref()
			.map(|arc| arc.inner.as_ptr())
			.unwrap_or_default();
		// avoid decrementing reference counter
		mem::forget(other);
		let old = self.inner.swap(new, SeqCst);
		NonNull::new(old).map(|inner| {
			// TODO RCU sync
			Arc {
				inner,
			}
		})
	}
}

unsafe impl<T> Send for RcuOptionArc<T> {}

unsafe impl<T> Sync for RcuOptionArc<T> {}

impl<T> Drop for RcuOptionArc<T> {
	fn drop(&mut self) {
		let inner = self.inner.load(Relaxed);
		if let Some(inner) = NonNull::new(inner) {
			// decrement reference counter
			drop(Arc {
				inner,
			});
		}
	}
}
