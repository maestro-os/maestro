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

#![feature(iter_intersperse)]
#![deny(warnings)]

extern crate proc_macro;

mod allocator;
mod aml;
mod syscall;
mod util;

use crate::util::has_repr_c;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Implements `AnyRepr`, making necessary safety checks.
#[proc_macro_derive(AnyRepr)]
pub fn any_repr(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;

	if !has_repr_c(&input.attrs) {
		panic!("{ident} is not suitable for the trait `AnyRepr`");
	}

	let toks = quote! {
		unsafe impl utils::bytes::AnyRepr for #ident {}
	};
	TokenStream::from(toks)
}

/// Instrumentation macro for memory allocators.
///
/// This macro allows to trace memory allocations/reallocations/frees in order to determine which
/// portion of the codebase consume the most memory, and to help finding memory leaks.
///
/// The macro takes the following attributes:
/// - `name`: the name of the allocator
/// - `op`: the operation the function performs (either `alloc`, `realloc` or `free`)
/// - `ptr` (required for `realloc` and `free`): the field specifying the pointer
/// - `size` (required for `alloc` and `free`): the field specifiying the size of the allocation
/// - `scale` (optional, defaults to `linear`): the scale of the allocation, either:
///     - `linear`: the size is taken as is in the recorded sample
///     - `log2`: the size is put to the power of two (`2^^n`) before begin recordedsaved in the
///    sample
///
/// Example:
/// ```rust
/// #[instrument_allocator(name = buddy, op = alloc, size = order, scale = log2)]
/// ```
#[proc_macro_attribute]
pub fn instrument_allocator(metadata: TokenStream, input: TokenStream) -> TokenStream {
	allocator::instrument_allocator(metadata, input)
}

/// Definition of a derive macro used to turn a structure into a parsable object for the AML
/// bytecode.
///
/// TODO further document
#[proc_macro_derive(Parseable)]
pub fn aml_parseable(input: TokenStream) -> TokenStream {
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
