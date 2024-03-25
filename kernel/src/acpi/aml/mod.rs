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

//! ACPI Machine Language (AML) is a bytecode language used by ACPI to describe programs that allow
//! retrieving informations on the system in order to used ACPI features.

mod named_obj;
mod namespace_modifier;
mod term_obj;
mod type1_opcode;
mod type2_opcode;

use core::ops::Range;
use macros::Parseable;
use term_obj::TermList;
use utils::collections::string::String;

const ZERO_OP: u8 = 0x00;
const ONE_OP: u8 = 0x01;
const ALIAS_OP: u8 = 0x06;
const NAME_OP: u8 = 0x08;
const BYTE_PREFIX: u8 = 0x0a;
const WORD_PREFIX: u8 = 0x0b;
const DWORD_PREFIX: u8 = 0x0c;
const STRING_PREFIX: u8 = 0x0d;
const QWORD_PREFIX: u8 = 0x0e;
const SCOPE_OP: u8 = 0x10;
const BUFFER_OP: u8 = 0x11;
const PACKAGE_OP: u8 = 0x12;
const VAR_PACKAGE_OP: u8 = 0x13;
const METHOD_OP: u8 = 0x14;
const EXTERNAL_OP: u8 = 0x15;
const DUAL_NAME_PREFIX: u8 = 0x2e;
const MULTI_NAME_PREFIX: u8 = 0x2f;
const DIGIT_CHAR: Range<u8> = 0x30..0x39;
const NAME_CHAR: Range<u8> = 0x41..0x5a;
const EXT_OP_PREFIX: u8 = 0x5b;
const MUTEX_OP: &[u8] = &[0x5b, 0x01];
const EVENT_OP: &[u8] = &[0x5b, 0x02];
const COND_REF_OF_OP: &[u8] = &[0x5b, 0x12];
const CREATE_FIELD_OP: &[u8] = &[0x5b, 0x13];
const LOAD_TABLE_OP: &[u8] = &[0x5b, 0x1f];
const LOAD_OP: &[u8] = &[0x5b, 0x20];
const STALL_OP: &[u8] = &[0x5b, 0x21];
const SLEEP_OP: &[u8] = &[0x5b, 0x22];
const ACQUIRE_OP: &[u8] = &[0x5b, 0x23];
const SIGNAL_OP: &[u8] = &[0x5b, 0x24];
const WAIT_OP: &[u8] = &[0x5b, 0x25];
const RESET_OP: &[u8] = &[0x5b, 0x26];
const RELEASE_OP: &[u8] = &[0x5b, 0x27];
const FROM_BCD_OP: &[u8] = &[0x5b, 0x28];
const TO_BCD: &[u8] = &[0x5b, 0x29];
const REVISION_OP: &[u8] = &[0x5b, 0x30];
const DEBUG_OP: &[u8] = &[0x5b, 0x31];
const FATAL_OP: &[u8] = &[0x5b, 0x32];
const TIMER_OP: &[u8] = &[0x5b, 0x33];
const OP_REGION_OP: &[u8] = &[0x5b, 0x80];
const FIELD_OP: &[u8] = &[0x5b, 0x81];
const DEVICE_OP: &[u8] = &[0x5b, 0x82];
const PROCESSOR_OP: &[u8] = &[0x5b, 0x83];
const POWER_RES_OP: &[u8] = &[0x5b, 0x84];
const THERMAL_ZONE_OP: &[u8] = &[0x5b, 0x85];
const INDEX_FIELD_OP: &[u8] = &[0x5b, 0x86];
const BANK_FIELD_OP: &[u8] = &[0x5b, 0x87];
const DATA_REGION_OP: &[u8] = &[0x5b, 0x88];
const ROOT_CHAR: u8 = 0x5c;
const PARENT_PREFIX_CHAR: u8 = 0x5e;
const NAME_CHAR_: u8 = 0x5f;
const LOCAL0_OP: u8 = 0x60;
const LOCAL1_OP: u8 = 0x61;
const LOCAL2_OP: u8 = 0x62;
const LOCAL3_OP: u8 = 0x63;
const LOCAL4_OP: u8 = 0x64;
const LOCAL5_OP: u8 = 0x65;
const LOCAL6_OP: u8 = 0x66;
const LOCAL7_OP: u8 = 0x67;
const ARG0_OP: u8 = 0x68;
const ARG1_OP: u8 = 0x69;
const ARG2_OP: u8 = 0x6a;
const ARG3_OP: u8 = 0x6b;
const ARG4_OP: u8 = 0x6c;
const ARG5_OP: u8 = 0x6d;
const ARG6_OP: u8 = 0x6e;
const STORE_OP: u8 = 0x70;
const REF_OF_OP: u8 = 0x71;
const ADD_OP: u8 = 0x72;
const CONCAT_OP: u8 = 0x73;
const SUBTRACT_OP: u8 = 0x74;
const INCREMENT_OP: u8 = 0x75;
const DECREMENT_OP: u8 = 0x76;
const MULTIPLY_OP: u8 = 0x77;
const DIVIDE_OP: u8 = 0x78;
const SHIFT_LEFT_OP: u8 = 0x79;
const SHIFT_RIGHT_OP: u8 = 0x7a;
const AND_OP: u8 = 0x7b;
const NAND_OP: u8 = 0x7c;
const OR_OP: u8 = 0x7d;
const NOR_OP: u8 = 0x7e;
const XOR_OP: u8 = 0x7f;
const NOT_OP: u8 = 0x80;
const FIND_SET_LEFT_BIT_OP: u8 = 0x81;
const FIND_SET_RIGHT_BIT_OP: u8 = 0x82;
const DEREF_OF_OP: u8 = 0x83;
const CONCAT_RES_OP: u8 = 0x84;
const MOD_OP: u8 = 0x85;
const NOTIFY_OP: u8 = 0x86;
const SIZE_OF_OP: u8 = 0x87;
const INDEX_OP: u8 = 0x88;
const MATCH_OP: u8 = 0x89;
const CREATE_DWORD_FIELD_OP: u8 = 0x8a;
const CREATE_WORD_FIELD_OP: u8 = 0x8b;
const CREATE_BYTE_FIELD_OP: u8 = 0x8c;
const CREATE_BIT_FIELD_OP: u8 = 0x8d;
const OBJECT_TYPE_OP: u8 = 0x8e;
const CREATE_QWORD_FIELD_OP: u8 = 0x8f;
const LAND_OP: u8 = 0x90;
const LOR_OP: u8 = 0x91;
const LNOT_OP: u8 = 0x92;
const LNOT_EQUAL_OP: &[u8] = &[0x92, 0x93];
const LLESS_EQUAL_OP: &[u8] = &[0x92, 0x94];
const LGREATER_EQUAL_OP: &[u8] = &[0x92, 0x95];
const LEQUAL_OP: u8 = 0x93;
const LGREATER_OP: u8 = 0x94;
const LLESS_OP: u8 = 0x95;
const TO_BUFFER_OP: u8 = 0x96;
const TO_DECIMAL_STRING_OP: u8 = 0x97;
const TO_HEX_STRING_OP: u8 = 0x98;
const TO_INTEGER_OP: u8 = 0x99;
const TO_STRING_OP: u8 = 0x9c;
const COPY_OBJECT_OP: u8 = 0x9d;
const MID_OP: u8 = 0x9e;
const CONTINUE_OP: u8 = 0x9f;
const IF_OP: u8 = 0xa0;
const ELSE_OP: u8 = 0xa1;
const WHILE_OP: u8 = 0xa2;
const NOOP_OP: u8 = 0xa3;
const RETURN_OP: u8 = 0xa4;
const BREAK_OP: u8 = 0xa5;
const BREAK_POINT_OP: u8 = 0xcc;
const ONES_OP: u8 = 0xff;

/// An enumeration representing error messages.
///
/// An error message can either be allocated or static.
///
/// This enumeration contains both these possibilities.
pub enum ErrorMessage {
	/// Allocated error message.
	Allocated(String),
	/// Static error message.
	Static(&'static str),
}

/// Structure representing an AML parse error.
pub struct Error {
	/// The error message.
	message: ErrorMessage,
	/// The offset of the error in the bytecode.
	off: usize,
}

/// Trait representing a parseable object.
pub trait AMLParseable: Sized {
	/// Parses the object from the given bytes `b`.
	///
	/// `off` is the offset in the bytecode during parsing.
	/// This value is used only to locate errors.
	///
	/// The function returns an instance of the parsed object and the consumed length.
	/// On parsing error, the function returns an error message.
	fn parse(off: usize, b: &[u8]) -> Result<Option<(Self, usize)>, Error>;
}

/// Implements the AMLParseable trait for the given primitive type.
macro_rules! impl_aml_parseable_primitive {
	($type:ty) => {
		impl AMLParseable for $type {
			fn parse(off: usize, b: &[u8]) -> Result<Option<(Self, usize)>, Error> {
				let len = core::mem::size_of::<$type>();
				if b.len() < len {
					// TODO Error message
					let err = String::try_from(b"TODO")
						.map(|msg| Error {
							message: ErrorMessage::Allocated(msg),
							off,
						})
						.unwrap_or_else(|_| Error {
							message: ErrorMessage::Static("Allocation error"),
							off,
						});

					return Err(err);
				}

				let mut n: $type = Default::default();
				unsafe {
					core::ptr::copy_nonoverlapping(&b[0], (&mut n) as *mut _ as *mut u8, len);
				}

				Ok(Some((n, len)))
			}
		}
	};
}

pub type ByteData = u8;
pub type WordData = u16;
pub type DWordData = u32;
pub type QWordData = u64;

pub type TableSignature = DWordData;
pub type TableLength = DWordData;
pub type SpecCompliance = ByteData;
pub type CheckSum = ByteData;
pub type OemId = [ByteData; 6];
pub type OemTableId = [ByteData; 8];
pub type OemRevision = DWordData;
pub type CreatorId = DWordData;
pub type CreatorRevision = DWordData;

// Implementations for primitive types
impl_aml_parseable_primitive!(u8);
impl_aml_parseable_primitive!(i8);
impl_aml_parseable_primitive!(u16);
impl_aml_parseable_primitive!(i16);
impl_aml_parseable_primitive!(u32);
impl_aml_parseable_primitive!(i32);
impl_aml_parseable_primitive!(u64);
impl_aml_parseable_primitive!(i64);

// Implementations for array types
impl_aml_parseable_primitive!(OemId);
impl_aml_parseable_primitive!(OemTableId);

/// TODO doc
#[derive(Parseable)]
pub struct DefBlockHeader {
	/// TODO doc
	signature: TableSignature,
	/// TODO doc
	length: TableLength,
	/// TODO doc
	compliance: SpecCompliance,
	/// TODO doc
	checksum: CheckSum,
	/// TODO doc
	oem_id: OemId,
	/// TODO doc
	oem_table_id: OemTableId,
	/// TODO doc
	oem_revision: OemRevision,
	/// TODO doc
	creator_id: CreatorId,
	/// TODO doc
	creator_revision: CreatorRevision,
}

/// Base of the AML Abstract Syntax Tree (AST).
#[derive(Parseable)]
pub struct AMLCode {
	def_block_header: DefBlockHeader,
	term_list: TermList,
}

/// Parses the given AML code.
///
/// On parsing error, the function returns an error message.
pub fn parse(_aml: &[u8]) -> Result<AMLCode, String> {
	// TODO
	todo!();
}
