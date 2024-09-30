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

//! This module implements macros used to parse AML bytecode.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields};

/// Returns the parse code for the given set of fields.
fn parse_expr(fields: &Fields) -> proc_macro2::TokenStream {
	match fields {
		Fields::Named(fields) => {
			let parse_lines = fields.named.iter().map(|field| {
				let ident = field.ident.as_ref().unwrap();

				quote! {
					let #ident = match AMLParseable::parse(off + curr_off, &b[curr_off..])? {
						Some((child, child_off)) => {
							curr_off += child_off;
							child
						},

						None => return Ok(None),
					};
				}
			});

			quote! {
				#(#parse_lines)*
			}
		}

		Fields::Unnamed(fields) => {
			let parse_lines = fields.unnamed.iter().enumerate().map(|(i, _)| {
				// TODO Fix span
				let ident = Ident::new(format!("field{}", i).as_str(), Span::call_site());

				quote! {
					let #ident = match AMLParseable::parse(off + curr_off, &b[curr_off..])? {
						Some((child, child_off)) => {
							curr_off += child_off;
							child
						},

						None => return Ok(None),
					};
				}
			});

			quote! {
				#(#parse_lines)*
			}
		}

		Fields::Unit => quote! {},
	}
}

// TODO Clean
/// TODO doc
pub fn derive_parseable(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	let ident = input.ident;

	let output = match input.data {
		Data::Struct(DataStruct {
			fields: Fields::Named(fields),
			..
		}) => {
			let parse_lines = parse_expr(&Fields::Named(fields.clone()));

			// TODO Streamline
			let struct_lines = fields.named.iter().map(|field| {
				let ident = field.ident.as_ref().unwrap();
				quote! { #ident, }
			});

			quote! {
				impl AMLParseable for #ident {
					fn parse(off: usize, b: &[u8]) -> Result<Option<(Self, usize)>, Error> {
						let mut curr_off: usize = 0;

						#parse_lines

						Ok(Some((Self {
							#(#struct_lines)*
						}, curr_off)))
					}
				}
			}
		}

		Data::Enum(DataEnum {
			variants, ..
		}) => {
			let parse_lines = variants.iter().map(|v| {
				let ident = v.ident.clone();
				let parse_lines = parse_expr(&v.fields);

				// TODO Streamline
				let struct_lines = match &v.fields {
					Fields::Named(fields) => {
						let fields = fields.named.iter().map(|field| {
							let ident = field.ident.as_ref().unwrap();
							quote! { #ident, }
						});

						quote! { #(#fields)* }
					}

					Fields::Unnamed(fields) => {
						let fields = fields.unnamed.iter().enumerate().map(|(i, _)| {
							// TODO Fix span
							let ident =
								Ident::new(format!("field{}", i).as_str(), Span::call_site());
							quote! { #ident, }
						});

						quote! { #(#fields)* }
					}

					Fields::Unit => quote! {},
				};

				let s = match v.fields {
					Fields::Named(_) => quote! {
						Self::#ident {
							#struct_lines
						}
					},

					Fields::Unnamed(_) => quote! {
						Self::#ident(
							#struct_lines
						)
					},

					Fields::Unit => quote! {
						Self::#ident
					},
				};

				quote! {
					let s = (|| {
						let mut curr_off: usize = 0;

						#parse_lines

						Ok(Some((#s, curr_off)))
					})()?;

					if let Some((child, child_off)) = s {
						return Ok(Some((child, curr_off)));
					}
				}
			});

			quote! {
				impl AMLParseable for #ident {
					fn parse(off: usize, b: &[u8]) -> Result<Option<(Self, usize)>, Error> {
						let mut curr_off: usize = 0;

						#(#parse_lines)*

						Ok(None)
					}
				}
			}
		}

		_ => panic!("only structs and enums can be derived with Parseable"),
	};

	TokenStream::from(output)
}
