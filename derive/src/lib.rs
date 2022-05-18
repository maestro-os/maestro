//! This crate implements derive macros for the Maestro kernel.

extern crate proc_macro;

use proc_macro::TokenStream;

/// Definition of a derive macro used to turn a structure into a parsable object for the AML
/// bytecode.
#[proc_macro_derive(AMLParseable)]
pub fn derive_aml_parseable(_stream: TokenStream) -> TokenStream {
    // TODO
	todo!();
}
