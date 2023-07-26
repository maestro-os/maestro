//! This module implements the macro used to declare a system call.

use proc_macro::TokenStream;
use proc_macro2::Ident;
use proc_macro2::Span;
use quote::quote;
use syn::parse_macro_input;
use syn::AngleBracketedGenericArguments;
use syn::FnArg;
use syn::ItemFn;
use syn::Path;
use syn::PathArguments;
use syn::PathSegment;
use syn::Token;
use syn::Type;
use syn::TypePath;

/// The list of register for each argument, in order.
const REGS: [&'static str; 6] = ["ebx", "ecx", "edx", "esi", "edi", "ebp"];

// TODO Add support for mutable arguments

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
	if input.sig.inputs.len() > REGS.len() {
		panic!(
			"too many arguments for the current target (max: {})",
			REGS.len()
		);
	}

	let args = input
		.sig
		.inputs
		.iter()
		.enumerate()
		.map(|(i, arg)| match arg {
			FnArg::Typed(typed) => {
				let pat = typed.pat.clone();
				let ty = typed.ty.clone();

				let reg_name = Ident::new(REGS[i], Span::call_site());

				(pat, ty, reg_name)
			}

			FnArg::Receiver(_) => panic!("a system call handler cannot have a `self` argument"),
		})
		.collect::<Vec<_>>();

	let mut args_tokens = proc_macro2::TokenStream::new();
	args_tokens.extend(args.iter().map(|(pat, ty, reg_name)| {
		let mut ty = ty.clone();

		// TODO make a cleaner check
		match *ty {
			// Special cast for pointers
			Type::Path(TypePath {
				path: Path {
					ref mut segments, ..
				},
				..
			}) if segments
				.first()
				.map(|s| s.ident.to_string().starts_with("Syscall"))
				.unwrap_or(false) =>
			{
				// Adding colon token to avoid compilation error
				if let PathSegment {
					arguments:
						PathArguments::AngleBracketed(AngleBracketedGenericArguments {
							ref mut colon2_token,
							..
						}),
					..
				} = &mut segments[0]
				{
					*colon2_token = Some(Token![::](Span::call_site()));
				}

				proc_macro2::TokenStream::from(quote! {
					let #pat = #ty::from(regs.#reg_name as usize);
				})
			}

			// Normal, truncating cast
			_ => proc_macro2::TokenStream::from(quote! {
				let #pat = regs.#reg_name as #ty;
			}),
		}
	}));

	let ident = input.sig.ident;
	let code = input.block;

	let toks = if cfg!(feature = "strace") {
		let args_count = input.sig.inputs.len();

		let mut strace_call_format = String::from("[strace PID: {}] {}(");
		for i in 0..args_count {
			if i + 1 < args_count {
				strace_call_format += "{:?}, ";
			} else {
				strace_call_format += "{:?}";
			}
		}
		strace_call_format += ")";

		let strace_args = args.iter().map(|(pat, ..)| pat).collect::<Vec<_>>();

		quote! {
			pub fn #ident(regs: &crate::process::regs::Regs) -> Result<i32, Errno> {
				#args_tokens

				crate::idt::wrap_disable_interrupts(|| {
					let pid = {
						let proc_mutex = crate::process::Process::current().unwrap();
						let proc = proc_mutex.lock();

						proc.pid
					};

					println!(
						#strace_call_format,
						pid,
						stringify!(#ident),
						#(#strace_args),*
					);
				});

				let ret = (|| {
					#code
				})();

				crate::idt::wrap_disable_interrupts(|| {
					let pid = {
						let proc_mutex = crate::process::Process::current().unwrap();
						let proc = proc_mutex.lock();

						proc.pid
					};

					match ret {
						Ok(val) => println!(
							"[strace PID: {}] -> Ok(0x{:x})",
							pid,
							val
						),

						Err(errno) => println!(
							"[strace PID: {}] -> Err({})",
							pid,
							errno
						),
					}
				});

				ret
			}
		}
	} else {
		quote! {
			pub fn #ident(regs: &crate::process::regs::Regs) -> Result<i32, Errno> {
				#args_tokens

				#code
			}
		}
	};

	TokenStream::from(toks)
}
