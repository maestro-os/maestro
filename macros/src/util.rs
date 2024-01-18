//! Utility functions.

use proc_macro2::TokenTree;
use syn::{AttrStyle, Attribute, Meta, MetaList, Path};

/// Tells whether the list of attributes contains `repr(C)`.
pub fn has_repr_c(attrs: &[Attribute]) -> bool {
	attrs
		.iter()
		.filter_map(|attr| {
			if !matches!(attr.style, AttrStyle::Outer) {
				return None;
			}
			let Path {
				leading_colon: None,
				segments,
				..
			} = attr.path()
			else {
				return None;
			};
			if segments.len() != 1 {
				return None;
			}
			let seg = segments.first().unwrap();
			if !seg.arguments.is_empty() {
				return None;
			}
			if seg.ident != "repr" {
				return None;
			}
			let Meta::List(MetaList {
				ref tokens, ..
			}) = attr.meta
			else {
				return None;
			};
			Some(tokens.clone())
		})
		.flat_map(|tokens| tokens.into_iter())
		.filter_map(|tok| {
			if let TokenTree::Ident(ident) = tok {
				Some(ident)
			} else {
				None
			}
		})
		.any(|ident| ident == "C")
}
