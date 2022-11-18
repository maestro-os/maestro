//! This module implements the macro used to declare a system call.

use proc_macro2::Ident;
use proc_macro2::Span;
use proc_macro::TokenStream;
use quote::quote;
use syn::FnArg;
use syn::ItemFn;
use syn::parse_macro_input;

/// The list of register for each argument, in order.
const REGS: [&'static str; 6] = [
	"ebx",
	"ecx",
	"edx",
	"esi",
	"edi",
	"ebp"
];

/// Implementation of the syscall macro.
pub fn syscall(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	// Check signature is valid
	if input.sig.constness.is_some() {
		panic!("a system call handler cannot be const");
	}
	if !input.sig.generics.params.is_empty() {
		panic!("a system call cannot have generic arguments");
	}
	if input.sig.variadic.is_some() {
		panic!("a system call handler cannot have variadic arguments");
	}
	if input.sig.inputs.len() >= REGS.len() {
		panic!("too many arguments for the current target (max: {})", REGS.len());
	}

	let mut args = proc_macro2::TokenStream::new();
	args.extend(input.sig.inputs.iter()
		.enumerate()
		.map(|(i, arg)| match arg {
			FnArg::Typed(typed) => {
				let pat = typed.pat.clone();
				let ty = typed.ty.clone();

				// TODO Check type is supported
				// Supported list:
				// - c_char
				// - c_double
				// - c_float
				// - c_int
				// - c_long
				// - c_longlong
				// - c_schar
				// - c_short
				// - c_uchar
				// - c_uint
				// - c_ulong
				// - c_ulonglong
				// - c_ushort
				// - c_void
				// - u8
				// - i8
				// - u16
				// - i16
				// - u32
				// - i32
				// - u64
				// - i64
				// - all pointers
				// - Syscall*

				let reg_name = Ident::new(REGS[i], Span::call_site());

				proc_macro2::TokenStream::from(quote! {
					let #pat = regs.#reg_name as #ty;
				})
			},

			FnArg::Receiver(_) => panic!("a system call handler cannot have a `self` argument"),
		}));

	let ident = input.sig.ident;
	let code = input.block;

	TokenStream::from(quote! {
		pub fn #ident(regs: &crate::process::regs::Regs) -> Result<i32, Errno> {
			#args

			#code
		}
	})
}
