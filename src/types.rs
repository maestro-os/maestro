//! This module implements C types.

#![allow(non_camel_case_types)]

// Signed primitives

pub type c_char = i8;
pub type c_short = i16;
pub type c_int = i32;

#[cfg(any(target_pointer_width = "32"))]
pub type c_long = i32;
#[cfg(any(target_pointer_width = "64"))]
pub type c_long = i64;

// Unsigned primitives

pub type c_uchar = u8;
pub type c_ushort = u16;
pub type c_uint = u32;

#[cfg(any(target_pointer_width = "32"))]
pub type c_ulong = u32;
#[cfg(any(target_pointer_width = "64"))]
pub type c_ulong = u64;
