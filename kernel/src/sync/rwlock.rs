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

//! Read-write locks.

// This implementation is highly inspired from the Rust standard library

use core::{
	cell::UnsafeCell,
	hint,
	ops::{Deref, DerefMut},
	ptr::NonNull,
	sync::atomic::{
		AtomicU32,
		Ordering::{Acquire, Relaxed, Release},
	},
};

/// Mask of bits representing the number of readers holding the lock.
const MASK: u32 = (1 << 30) - 1;
const MAX_READERS: u32 = MASK - 1;
/// Value telling the lock is locked for writing.
const WRITE_LOCKED: u32 = MASK;
/// Flag telling readers are waiting for the lock.
const READERS_WAITING: u32 = 1 << 30;
/// Flag telling writers are waiting for the lock.
const WRITERS_WAITING: u32 = 1 << 31;

#[inline]
fn has_readers_waiting(state: u32) -> bool {
	state & READERS_WAITING != 0
}

#[inline]
fn has_writers_waiting(state: u32) -> bool {
	state & WRITERS_WAITING != 0
}

#[inline]
fn is_read_lockable(state: u32) -> bool {
	state & MASK < MAX_READERS && !has_readers_waiting(state) && !has_writers_waiting(state)
}

#[inline]
fn is_write_locked(state: u32) -> bool {
	state & MASK == WRITE_LOCKED
}

#[inline]
fn is_unlocked(state: u32) -> bool {
	state & MASK == 0
}

/// Read-write lock, allowing either several concurrent readers or a single writer.
#[derive(Default)]
pub struct RwLock<T: ?Sized> {
	/// The state of the lock.
	///
	/// - Bits 0..30:
	///     - `0`: unlocked
	///     - `1..=0x3ffffffe`: locked by `n` readers
	///     - `0x3fffffff`: locked by a writer
	/// - Bit 30: Readers are waiting on the lock.
	/// - Bit 31: Writers are waiting on the lock.
	state: AtomicU32,
	/// The lock's data.
	data: UnsafeCell<T>,
}

impl<T> RwLock<T> {
	/// Creates a new lock.
	pub fn new(value: T) -> Self {
		Self {
			state: AtomicU32::new(0),
			data: UnsafeCell::new(value),
		}
	}
}

impl<T: ?Sized> RwLock<T> {
	/// Spins until `f` returns `true`. The argument to `f` is the state of the lock.
	///
	/// The function returns the locks' state.
	#[inline]
	fn spin_until<F: Fn(u32) -> bool>(&self, f: F) -> u32 {
		loop {
			let state = self.state.load(Relaxed);
			if f(state) {
				return state;
			}
			hint::spin_loop();
		}
	}

	#[cold]
	fn read_contended(&self) {
		let mut state = self.spin_until(|state| {
			!is_write_locked(state) || has_readers_waiting(state) || has_writers_waiting(state)
		});
		loop {
			// Try to lock
			if is_read_lockable(state) {
				match self
					.state
					.compare_exchange_weak(state, state + 1, Acquire, Relaxed)
				{
					Ok(_) => return, // Locked
					Err(s) => {
						state = s;
						continue;
					}
				}
			}
			// Check for overflow
			if state & MASK == MAX_READERS {
				panic!("too many readers on RwLock");
			}
			// TODO make the current thread sleep
		}
	}

	/// Locks for read access, blocking the current thread until it can be acquired.
	pub fn read(&self) -> ReadGuard<'_, T> {
		let state = self.state.load(Relaxed);
		if !is_read_lockable(state)
			|| self
				.state
				.compare_exchange_weak(state, state + 1, Acquire, Relaxed)
				.is_err()
		{
			self.read_contended();
		}
		ReadGuard {
			lock: self,
			data: NonNull::new(self.data.get()).unwrap(),
		}
	}

	#[inline]
	fn read_unlock(&self) {
		let state = self.state.fetch_sub(1, Release) - 1;
		debug_assert!(!has_readers_waiting(state) || has_writers_waiting(state));
		// TODO if the lock is unlocked and has other threads waiting, wake them
	}

	#[cold]
	fn write_contended(&self) {
		let mut state = self.spin_until(|state| is_unlocked(state) || !has_writers_waiting(state));
		loop {
			if is_unlocked(state) {
				match self.state.compare_exchange_weak(
					state,
					state | WRITE_LOCKED,
					Acquire,
					Relaxed,
				) {
					Ok(_) => return, // Locked
					Err(s) => {
						state = s;
						continue;
					}
				}
			}
			// Indicate we are waiting on the lock
			if !has_writers_waiting(state) {
				if let Err(s) = self.state.compare_exchange_weak(
					state,
					state | WRITERS_WAITING,
					Relaxed,
					Relaxed,
				) {
					state = s;
					continue;
				}
			}
			// TODO make the current thread sleep
		}
	}

	/// Locks for write access, blocking the current thread until it can be acquired.
	pub fn write(&self) -> WriteGuard<'_, T> {
		if self
			.state
			.compare_exchange_weak(0, WRITE_LOCKED, Acquire, Relaxed)
			.is_err()
		{
			self.write_contended();
		}
		WriteGuard {
			lock: self,
		}
	}

	#[inline]
	fn write_unlock(&self) {
		let state = self.state.fetch_sub(WRITE_LOCKED, Release) - WRITE_LOCKED;
		debug_assert!(is_unlocked(state));
		// TODO if the lock has other threads waiting, wake them
	}
}

unsafe impl<T: ?Sized> Send for RwLock<T> {}

unsafe impl<T: ?Sized> Sync for RwLock<T> {}

/// Guard of [`RwLock`] reader.
pub struct ReadGuard<'a, T: ?Sized> {
	lock: &'a RwLock<T>,
	// Using a pointer instead of a reference to avoid `noalias` violations, since the structure
	// holds immutability only until it drops (while other locks might still need it).
	data: NonNull<T>,
}

impl<T: ?Sized> Deref for ReadGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { self.data.as_ref() }
	}
}

impl<T: ?Sized> !Send for ReadGuard<'_, T> {}

unsafe impl<T: ?Sized + Sync> Sync for ReadGuard<'_, T> {}

impl<T: ?Sized> Drop for ReadGuard<'_, T> {
	fn drop(&mut self) {
		self.lock.read_unlock();
	}
}

/// Guard of [`RwLock`] writer.
pub struct WriteGuard<'a, T: ?Sized> {
	lock: &'a RwLock<T>,
}

impl<T: ?Sized> Deref for WriteGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.lock.data.get() }
	}
}

impl<T: ?Sized> DerefMut for WriteGuard<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.lock.data.get() }
	}
}

impl<T: ?Sized> !Send for WriteGuard<'_, T> {}

unsafe impl<T: ?Sized + Sync> Sync for WriteGuard<'_, T> {}

impl<T: ?Sized> Drop for WriteGuard<'_, T> {
	fn drop(&mut self) {
		self.lock.write_unlock();
	}
}
