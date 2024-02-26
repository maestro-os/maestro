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

//! This crate implements derive macros for the Maestro kernel.

#![deny(warnings)]

extern crate proc_macro;

mod aml;
mod syscall;

use proc_macro::TokenStream;

/// Definition of a derive macro used to turn a structure into a parsable object for the AML
/// bytecode.
///
/// TODO further document
#[proc_macro_derive(Parseable)]
pub fn derive_aml_parseable(input: TokenStream) -> TokenStream {
	aml::derive_parseable(input)
}

/// Attribute macro to declare a system call.
///
/// This macro allows to take the system call's arguments directly instead of taking the process's
/// registers.
#[proc_macro_attribute]
pub fn syscall(_metadata: TokenStream, input: TokenStream) -> TokenStream {
	syscall::syscall(input)
}
