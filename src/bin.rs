//! This module exists only to import symbols from the kernel that has been compiled as a library.

#![no_std]
#![no_main]
// Force presence of the test code for both `cargo test` and `cargo clippy --tests`
#![feature(custom_test_frameworks)]
#![test_runner(kernel::selftest::runner)]

extern crate kernel;
