//! This crate implements derive macros for the Maestro kernel.

#![deny(warnings)]

extern crate proc_macro;

mod aml;

use proc_macro::TokenStream;

/// Definition of a derive macro used to turn a structure into a parsable object for the AML
/// bytecode.
#[proc_macro_derive(AMLParseable)]
pub fn derive_aml_parseable(input: TokenStream) -> TokenStream {
	aml::derive_parseable(input)
}
