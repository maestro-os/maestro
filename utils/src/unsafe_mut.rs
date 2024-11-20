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

//! Unsafe mutable wrapper.

use core::cell::UnsafeCell;

/// Wrapper allowing safe immutable accesses, or unsafe mutable accesses at the same time.
pub struct UnsafeMut<T>(UnsafeCell<T>);

impl<T> UnsafeMut<T> {
    /// Creates a new instance.
    pub fn new(val: T) -> Self {
        Self(UnsafeCell::new(val))
    }
    
    /// Returns an immutable reference.
    pub fn get(&self) -> &T {
        unsafe { &*self.0.get() }
    }

    /// Returns a mutable reference.
    /// 
    /// # Safety
    /// 
    /// The caller must ensure no other thread is accessing the value at the same time.
    #[allow(clippy::mut_from_ref)]
    pub unsafe fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.0.get() }
    }
}