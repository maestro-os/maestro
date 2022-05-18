//! This crate implements derive macros for the Maestro kernel.

#![deny(warnings)]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::Data;
use syn::DataStruct;
use syn::DeriveInput;
use syn::Fields;
use syn::parse_macro_input;

/// Definition of a derive macro used to turn a structure into a parsable object for the AML
/// bytecode.
#[proc_macro_derive(AMLParseable)]
pub fn derive_aml_parseable(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let struct_name = input.ident;

	let fields = match input.data {
		Data::Struct(DataStruct {
			fields: Fields::Named(fields),
			..
		}) => fields.named,

		// TODO Handle enums

		_ => panic!("only structs with named fields can be derived with AMLParseable"),
	};

	let parse_lines = fields.iter().map(| field | {
		let ident = field.ident.as_ref().unwrap();

		quote! {
			let (#ident, child_off) = AMLParseable::parse(&b[off..])?;
			off += child_off;
		}
	});

	let struct_lines = fields.iter().map(| field | {
		let ident = field.ident.as_ref().unwrap();

		quote! {
			#ident,
		}
	});

	let output = quote! {
        impl AMLParseable for #struct_name {
			fn parse(b: &[u8]) -> Result<(Self, usize), String> {
				let mut off: usize = 0;

				#(#parse_lines)*

				let s = Self {
					#(#struct_lines)*
				};

				Ok((s, off))
            }
        }
    };
    TokenStream::from(output)
}
