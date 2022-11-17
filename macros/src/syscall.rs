//! This module implements the macro used to declare a system call.

use proc_macro::TokenStream;
use syn::Expr;
use syn::ExprCall;
use syn::ItemFn;
use syn::Stmt;
use syn::parse_macro_input;

/// Implementation of the syscall macro.
pub fn syscall(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as ItemFn);

	match input.block.stmts.first() {
		Some(Stmt::Semi(Expr::Call(ExprCall {
			args,
			..
		}), _)) => {
			for _arg in args {
				// TODO
				eprintln!("arg");
			}
		},

		_ => panic!(), // TODO message
	}

	// TODO
	todo!();
}
