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

//! This module implements the macro used to declare a system call.

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use std::iter;
use syn::{
	parse_macro_input, AngleBracketedGenericArguments, FnArg, ItemFn, Path, PathArguments,
	PathSegment, Token, Type, TypePath,
};

/// The list of register for each argument, in order.
const REGS: [&str; 6] = ["ebx", "ecx", "edx", "esi", "edi", "ebp"];

// TODO Add support for mutable arguments

/// Implementation of the syscall macro.
pub fn syscall(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	// Check signature is valid
	if input.sig.constness.is_some() {
		panic!("a system call handler cannot be `const`");
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
				let pat = &typed.pat;
				let ty = &typed.ty;
				let reg_name = Ident::new(REGS[i], Span::call_site());
				(pat, ty, reg_name)
			}
			FnArg::Receiver(_) => panic!("a system call handler cannot have a `self` argument"),
		})
		.collect::<Vec<_>>();

	let args_tokens: proc_macro2::TokenStream = args
		.iter()
		.map(|(pat, ty, reg_name)| {
			let mut ty = (*ty).clone();
			match ty.as_mut() {
				// Special cast for userspace pointers
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
					// Add colon token to avoid compilation error
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
					quote! {
						let #pat = #ty::from(regs.#reg_name as usize);
					}
				}
				// Normal argument
				ty => quote! {
					let #pat = regs.#reg_name as #ty;
				},
			}
		})
		.collect();

	let ident = input.sig.ident;
	let code = input.block;

	let toks = if cfg!(feature = "strace") {
		let args_count = input.sig.inputs.len();
		let strace_call_format: String = iter::once("[strace PID: {}] {}(")
			.chain((0..args_count).map(|_| "{:?}").intersperse(", "))
			.chain(iter::once(")"))
			.collect();
		let strace_args = args.iter().map(|(pat, ..)| pat).collect::<Vec<_>>();

		quote! {
			pub fn #ident(regs: &crate::process::regs::Regs) -> Result<i32, Errno> {
				#args_tokens
				crate::idt::wrap_disable_interrupts(|| {
					let pid = {
						crate::process::Process::current_assert().lock().pid
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
						crate::process::Process::current_assert().lock().pid
					};
					match ret {
						Ok(val) => println!("[strace PID: {pid}] -> Ok(0x{val:x})"),
						Err(errno) => println!("[strace PID: {pid}] -> Err({errno})"),
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
