//! Implementation of the memory allocation instrumentation macro.

use proc_macro::TokenStream;
use proc_macro2::TokenTree;
use quote::{quote, ToTokens};
use syn::{parse::Parser, parse_macro_input, Block, FnArg, Ident, ItemFn, Pat, PatIdent, Type};

#[derive(Default)]
struct RawMetadata {
	name: Option<String>,
	op: Option<String>,
	ptr_field: Option<Ident>,
	size_field: Option<Ident>,
	size_scale: Option<String>,
}

enum MetadataOp {
	Alloc,
	Realloc,
	Free,
}

enum Scale {
	Linear,
	Log2,
}

struct Metadata {
	name: String,
	op: MetadataOp,
	ptr_field: Option<Ident>,
	size_field: Option<Ident>,
	size_scale: Scale,
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
			"scale" => metadata.size_scale = Some(value.to_string()),
			_ => panic!("syntax error"),
		}
	}
	let op = metadata.op.expect("missing `op`");
	let ptr_field;
	let size_field;
	let op = match op.as_str() {
		"alloc" => {
			ptr_field = None;
			size_field = Some(metadata.size_field.expect("missing `size`"));
			MetadataOp::Alloc
		}
		"realloc" => {
			ptr_field = Some(metadata.ptr_field.expect("missing `ptr`"));
			size_field = Some(metadata.size_field.expect("missing `size`"));
			MetadataOp::Realloc
		}
		"free" => {
			ptr_field = Some(metadata.ptr_field.expect("missing `ptr`"));
			size_field = metadata.size_field;
			MetadataOp::Free
		}
		n => panic!("invalid operation `{n}`"),
	};
	let size_scale = match metadata.size_scale.as_deref() {
		Some("linear") | None => Scale::Linear,
		Some("log2") => Scale::Log2,
		_ => panic!("invalid scale"),
	};
	Metadata {
		name: metadata.name.expect("missing `name`"),
		op,
		ptr_field,
		size_field,
		size_scale,
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
	// Generate sampling code
	let ptr_field = metadata
		.ptr_field
		.map(|ptr_field| {
			let ptr_nonnull = input
				.sig
				.inputs
				.iter()
				.filter_map(|arg| match arg {
					FnArg::Typed(p) => Some(p),
					_ => None,
				})
				.find(
					|p| matches!(&*p.pat, Pat::Ident(PatIdent { ident, .. }) if *ident == ptr_field),
				)
				.map(|p| !matches!(&*p.ty, Type::Ptr(_)))
				.unwrap_or(false);
			if ptr_nonnull {
				quote! {
					#ptr_field.as_ptr()
				}
			} else {
				quote! {
					#ptr_field
				}
			}
		})
		.unwrap_or(quote! {
			core::ptr::null()
		});
	let size_field = metadata
		.size_field
		.map(|size_field| match metadata.size_scale {
			Scale::Linear => quote! {
				#size_field.into()
			},
			Scale::Log2 => quote! {
				1usize << #size_field
			},
		})
		.unwrap_or(quote! {
			0
		});
	let stmts = input.block.stmts;
	let stmts = match metadata.op {
		MetadataOp::Alloc => {
			quote! {
				let ptr = {
					#(#stmts)*
				};
				#[cfg(feature = "memtrace")]
				if let Ok(ptr) = ptr {
					crate::memory::trace::sample(#name, 0, ptr.as_ptr(), #size_field);
				}
				ptr
			}
		}
		MetadataOp::Realloc => quote! {
			#[cfg(feature = "memtrace")]
			crate::memory::trace::sample(#name, 1, #ptr_field, #size_field);
			#(#stmts)*
		},
		MetadataOp::Free => quote! {
			#[cfg(feature = "memtrace")]
			crate::memory::trace::sample(#name, 2, #ptr_field, #size_field);
			#(#stmts)*
		},
	};
	input.block.stmts = Block::parse_within.parse(stmts.into()).unwrap();
	input.into_token_stream().into()
}
