//! Implementation of the memory allocation instrumentation macro.

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Ident, ItemFn};

#[derive(Default)]
struct RawMetadata {
	name: Option<String>,
	op: Option<String>,
	ptr_field: Option<Ident>,
	size_field: Option<Ident>,
}

enum MetadataOp {
	Alloc {
		size_field: Ident,
	},
	Realloc {
		ptr_field: Ident,
		size_field: Ident,
	},
	Free {
		ptr_field: Ident,
		size_field: Option<Ident>,
	},
}

struct Metadata {
	name: String,
	op: MetadataOp,
}

fn parse_metadata(metadata: proc_macro2::TokenStream) -> Metadata {
	let toks: Vec<_> = metadata.into_iter().collect();
	let separator = |t: &TokenTree| matches!(t, TokenTree::Punct(p) if p.as_char() == ',');
	let mut metadata = RawMetadata::default();
	for i in toks.split(separator) {
		if i.is_empty() {
			continue;
		}
		let [TokenTree::Ident(name), TokenTree::Punct(separator), TokenTree::Ident(value)] = i
		else {
			panic!("syntax error");
		};
		if separator.as_char() != '=' {
			panic!("syntax error");
		}
		let name = name.to_string();
		match name.as_str() {
			"name" => metadata.name = Some(value.to_string()),
			"op" => metadata.op = Some(value.to_string()),
			"ptr" => metadata.ptr_field = Some(value.clone()),
			"size" => metadata.size_field = Some(value.clone()),
			_ => panic!("syntax error"),
		}
	}
	let op = metadata.op.expect("missing `op`");
	let op = match op.as_str() {
		"alloc" => MetadataOp::Alloc {
			size_field: metadata.size_field.expect("missing `size`"),
		},
		"realloc" => MetadataOp::Realloc {
			ptr_field: metadata.ptr_field.expect("missing `ptr`"),
			size_field: metadata.size_field.expect("missing `size`"),
		},
		"free" => MetadataOp::Free {
			ptr_field: metadata.ptr_field.expect("missing `ptr`"),
			size_field: metadata.size_field,
		},
		n => panic!("invalid operation `{n}`"),
	};
	Metadata {
		name: metadata.name.expect("missing `name`"),
		op,
	}
}

pub fn instrument_allocator(metadata: TokenStream, input: TokenStream) -> TokenStream {
	let metadata = proc_macro2::TokenStream::from(metadata);
	let metadata = parse_metadata(metadata);
	let mut input = parse_macro_input!(input as ItemFn);
	// Check signature is valid
	if input.sig.constness.is_some() {
		panic!("an allocator function cannot be `const`");
	}
	let name = metadata.name;
	if name.as_bytes().len() > u8::MAX as usize {
		panic!("an allocator name cannot exceed {} bytes", u8::MAX);
	}
	let sample_code = match metadata.op {
		MetadataOp::Alloc {
			size_field,
		} => quote! {
			crate::memory::trace::sample(#name, 0, core::ptr::null(), #size_field.into());
		},
		MetadataOp::Realloc {
			ptr_field,
			size_field,
		} => quote! {
			crate::memory::trace::sample(#name, 1, #ptr_field.as_ptr(), #size_field.into());
		},
		MetadataOp::Free {
			ptr_field,
			size_field: Some(size_field),
		} => quote! {
			crate::memory::trace::sample(#name, 2, #ptr_field.as_ptr(), #size_field.into());
		},
		MetadataOp::Free {
			ptr_field,
			size_field: None,
		} => quote! {
			crate::memory::trace::sample(#name, 2, #ptr_field.as_ptr(), 0);
		},
	};
	let sample_code = syn::parse(
		quote! {
			#[cfg(feature = "memtrace")]
			#sample_code
		}
		.into(),
	)
	.unwrap();
	input.block.stmts.insert(0, sample_code);
	input.into_token_stream().into()
}
