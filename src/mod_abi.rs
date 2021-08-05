//! This module is used as a base for the compilation of a library meant to link the kernel
//! modules.

#![no_std]

#![feature(allow_internal_unstable)]
#![feature(asm)]
#![feature(coerce_unsized)]
#![feature(const_fn_trait_bound)]
#![feature(const_maybe_uninit_assume_init)]
#![feature(const_mut_refs)]
#![feature(const_ptr_offset)]
#![feature(const_raw_ptr_deref)]
#![feature(core_intrinsics)]
#![feature(custom_test_frameworks)]
#![feature(dispatch_from_dyn)]
#![feature(fundamental)]
#![feature(lang_items)]
#![feature(llvm_asm)]
#![feature(maybe_uninit_extra)]
#![feature(panic_info_message)]
#![feature(slice_ptr_get)]
#![feature(stmt_expr_attributes)]
#![feature(unsize)]

#![deny(warnings)]
#![allow(dead_code)]
#![allow(unused_macros)]

// Below are the modules that are made public to the kernel modules
pub mod acpi;
pub mod cmdline;
pub mod debug;
pub mod device;
pub mod elf;
pub mod errno;
pub mod event;
pub mod file;
pub mod gdt;
#[macro_use]
pub mod idt;
pub mod io;
pub mod kern;
pub mod limits;
pub mod logger;
pub mod memory;
pub mod module;
pub mod multiboot;
#[macro_use]
pub mod panic;
pub mod pit;
#[macro_use]
pub mod print;
pub mod process;
pub mod selftest;
pub mod syscall;
pub mod time;
pub mod tty;
#[macro_use]
pub mod util;
#[macro_use]
pub mod vga;
