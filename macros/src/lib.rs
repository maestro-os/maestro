//! This crate implements derive macros for the Maestro kernel.

#![feature(iter_intersperse)]
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
