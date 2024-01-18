//! This crate implements derive macros for the Maestro kernel.

#![feature(iter_intersperse)]
#![deny(warnings)]

extern crate proc_macro;

mod aml;
mod syscall;
mod util;

use crate::util::has_repr_c;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;

/// Implements `AnyRepr`, making necessary safety checks.
#[proc_macro_derive(AnyRepr)]
pub fn any_repr(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;

	if !has_repr_c(&input.attrs) {
		panic!("{ident} is not suitable for the trait `AnyRepr`");
	}

	let toks = quote! {
		unsafe impl crate::util::bytes::AnyRepr for #ident {}
	};
	TokenStream::from(toks)
}

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
