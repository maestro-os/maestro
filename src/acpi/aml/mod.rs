//! TODO doc

use core::ops::Range;
use crate::util::container::string::String;
use derive::Parseable;

/// TODO doc
const ZERO_OP: u8 = 0x00;
/// TODO doc
const ONE_OP: u8 = 0x01;
/// TODO doc
const ALIAS_OP: u8 = 0x06;
/// TODO doc
const NAME_OP: u8 = 0x08;
/// TODO doc
const BYTE_PREFIX: u8 = 0x0a;
/// TODO doc
const WORD_PREFIX: u8 = 0x0b;
/// TODO doc
const DWORD_PREFIX: u8 = 0x0c;
/// TODO doc
const STRING_PREFIX: u8 = 0x0d;
/// TODO doc
const QWORD_PREFIX: u8 = 0x0e;
/// TODO doc
const SCOPE_OP: u8 = 0x10;
/// TODO doc
const BUFFER_OP: u8 = 0x11;
/// TODO doc
const PACKAGE_OP: u8 = 0x12;
/// TODO doc
const VAR_PACKAGE_OP: u8 = 0x13;
/// TODO doc
const METHOD_OP: u8 = 0x14;
/// TODO doc
const EXTERNAL_OP: u8 = 0x15;
/// TODO doc
const DUAL_NAME_PREFIX: u8 = 0x2e;
/// TODO doc
const MULTI_NAME_PREFIX: u8 = 0x2f;
/// TODO doc
const DIGIT_CHAR: Range<u8> = 0x30..0x39;
/// TODO doc
const NAME_CHAR: Range<u8> = 0x41..0x5a;
/// TODO doc
const EXT_OP_PREFIX: u8 = 0x5b;
/// TODO doc
const MUTEX_OP: &[u8] = &[0x5b, 0x01];
/// TODO doc
const EVENT_OP: &[u8] = &[0x5b, 0x02];
/// TODO doc
const COND_REF_OF_OP: &[u8] = &[0x5b, 0x12];
/// TODO doc
const CREATE_FIELD_OP: &[u8] = &[0x5b, 0x13];
/// TODO doc
const LOAD_TABLE_OP: &[u8] = &[0x5b, 0x1f];
/// TODO doc
const LOAD_OP: &[u8] = &[0x5b, 0x20];
/// TODO doc
const STALL_OP: &[u8] = &[0x5b, 0x21];
/// TODO doc
const SLEEP_OP: &[u8] = &[0x5b, 0x22];
/// TODO doc
const ACQUIRE_OP: &[u8] = &[0x5b, 0x23];
/// TODO doc
const SIGNAL_OP: &[u8] = &[0x5b, 0x24];
/// TODO doc
const WAIT_OP: &[u8] = &[0x5b, 0x25];
/// TODO doc
const RESET_OP: &[u8] = &[0x5b, 0x26];
/// TODO doc
const RELEASE_OP: &[u8] = &[0x5b, 0x27];
/// TODO doc
const FROM_BCD_OP: &[u8] = &[0x5b, 0x28];
/// TODO doc
const TO_BCD: &[u8] = &[0x5b, 0x29];
/// TODO doc
const REVISION_OP: &[u8] = &[0x5b, 0x30];
/// TODO doc
const DEBUG_OP: &[u8] = &[0x5b, 0x31];
/// TODO doc
const FATAL_OP: &[u8] = &[0x5b, 0x32];
/// TODO doc
const TIMER_OP: &[u8] = &[0x5b, 0x33];
/// TODO doc
const OP_REGION_OP: &[u8] = &[0x5b, 0x80];
/// TODO doc
const FIELD_OP: &[u8] = &[0x5b, 0x81];
/// TODO doc
const DEVICE_OP: &[u8] = &[0x5b, 0x82];
/// TODO doc
const PROCESSOR_OP: &[u8] = &[0x5b, 0x83];
/// TODO doc
const POWER_RES_OP: &[u8] = &[0x5b, 0x84];
/// TODO doc
const THERMAL_ZONE_OP: &[u8] = &[0x5b, 0x85];
/// TODO doc
const INDEX_FIELD_OP: &[u8] = &[0x5b, 0x86];
/// TODO doc
const BANK_FIELD_OP: &[u8] = &[0x5b, 0x87];
/// TODO doc
const DATA_REGION_OP: &[u8] = &[0x5b, 0x88];
/// TODO doc
const ROOT_CHAR: u8 = 0x5c;
/// TODO doc
const PARENT_PREFIX_CHAR: u8 = 0x5e;
/// TODO doc
const NAME_CHAR_: u8 = 0x5f;
/// TODO doc
const LOCAL0_OP: u8 = 0x60;
/// TODO doc
const LOCAL1_OP: u8 = 0x61;
/// TODO doc
const LOCAL2_OP: u8 = 0x62;
/// TODO doc
const LOCAL3_OP: u8 = 0x63;
/// TODO doc
const LOCAL4_OP: u8 = 0x64;
/// TODO doc
const LOCAL5_OP: u8 = 0x65;
/// TODO doc
const LOCAL6_OP: u8 = 0x66;
/// TODO doc
const LOCAL7_OP: u8 = 0x67;
/// TODO doc
const ARG0_OP: u8 = 0x68;
/// TODO doc
const ARG1_OP: u8 = 0x69;
/// TODO doc
const ARG2_OP: u8 = 0x6a;
/// TODO doc
const ARG3_OP: u8 = 0x6b;
/// TODO doc
const ARG4_OP: u8 = 0x6c;
/// TODO doc
const ARG5_OP: u8 = 0x6d;
/// TODO doc
const ARG6_OP: u8 = 0x6e;
/// TODO doc
const STORE_OP: u8 = 0x70;
/// TODO doc
const REF_OF_OP: u8 = 0x71;
/// TODO doc
const ADD_OP: u8 = 0x72;
/// TODO doc
const CONCAT_OP: u8 = 0x73;
/// TODO doc
const SUBTRACT_OP: u8 = 0x74;
/// TODO doc
const INCREMENT_OP: u8 = 0x75;
/// TODO doc
const DECREMENT_OP: u8 = 0x76;
/// TODO doc
const MULTIPLY_OP: u8 = 0x77;
/// TODO doc
const DIVIDE_OP: u8 = 0x78;
/// TODO doc
const SHIFT_LEFT_OP: u8 = 0x79;
/// TODO doc
const SHIFT_RIGHT_OP: u8 = 0x7a;
/// TODO doc
const AND_OP: u8 = 0x7b;
/// TODO doc
const NAND_OP: u8 = 0x7c;
/// TODO doc
const OR_OP: u8 = 0x7d;
/// TODO doc
const NOR_OP: u8 = 0x7e;
/// TODO doc
const XOR_OP: u8 = 0x7f;
/// TODO doc
const NOT_OP: u8 = 0x80;
/// TODO doc
const FIND_SET_LEFT_BIT_OP: u8 = 0x81;
/// TODO doc
const FIND_SET_RIGHT_BIT_OP: u8 = 0x82;
/// TODO doc
const DEREF_OF_OP: u8 = 0x83;
/// TODO doc
const CONCAT_RES_OP: u8 = 0x84;
/// TODO doc
const MOD_OP: u8 = 0x85;
/// TODO doc
const NOTIFY_OP: u8 = 0x86;
/// TODO doc
const SIZE_OF_OP: u8 = 0x87;
/// TODO doc
const INDEX_OP: u8 = 0x88;
/// TODO doc
const MATCH_OP: u8 = 0x89;
/// TODO doc
const CREATE_DWORD_FIELD_OP: u8 = 0x8a;
/// TODO doc
const CREATE_WORD_FIELD_OP: u8 = 0x8b;
/// TODO doc
const CREATE_BYTE_FIELD_OP: u8 = 0x8c;
/// TODO doc
const CREATE_BIT_FIELD_OP: u8 = 0x8d;
/// TODO doc
const OBJECT_TYPE_OP: u8 = 0x8e;
/// TODO doc
const CREATE_QWORD_FIELD_OP: u8 = 0x8f;
/// TODO doc
const LAND_OP: u8 = 0x90;
/// TODO doc
const LOR_OP: u8 = 0x91;
/// TODO doc
const LNOT_OP: u8 = 0x92;
/// TODO doc
const LNOT_EQUAL_OP: &[u8] = &[0x92, 0x93];
/// TODO doc
const LLESS_EQUAL_OP: &[u8] = &[0x92, 0x94];
/// TODO doc
const LGREATER_EQUAL_OP: &[u8] = &[0x92, 0x95];
/// TODO doc
const LEQUAL_OP: u8 = 0x93;
/// TODO doc
const LGREATER_OP: u8 = 0x94;
/// TODO doc
const LLESS_OP: u8 = 0x95;
/// TODO doc
const TO_BUFFER_OP: u8 = 0x96;
/// TODO doc
const TO_DECIMAL_STRING_OP: u8 = 0x97;
/// TODO doc
const TO_HEX_STRING_OP: u8 = 0x98;
/// TODO doc
const TO_INTEGER_OP: u8 = 0x99;
/// TODO doc
const TO_STRING_OP: u8 = 0x9c;
/// TODO doc
const COPY_OBJECT_OP: u8 = 0x9d;
/// TODO doc
const MID_OP: u8 = 0x9e;
/// TODO doc
const CONTINUE_OP: u8 = 0x9f;
/// TODO doc
const IF_OP: u8 = 0xa0;
/// TODO doc
const ELSE_OP: u8 = 0xa1;
/// TODO doc
const WHILE_OP: u8 = 0xa2;
/// TODO doc
const NOOP_OP: u8 = 0xa3;
/// TODO doc
const RETURN_OP: u8 = 0xa4;
/// TODO doc
const BREAK_OP: u8 = 0xa5;
/// TODO doc
const BREAK_POINT_OP: u8 = 0xcc;
/// TODO doc
const ONES_OP: u8 = 0xff;

/// An enumeration representing error messages.
/// An error message can either be allocated or static. This enumeration contains both these
/// possibilities.
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
pub trait Parseable: Sized {
	/// Parses the object from the given bytes `b`.
	/// `off` is the offset in the bytecode during parsing. This value is used only to located
	/// errors.
	/// The function returns an instance of the parsed object and the consumed length.
	/// On parsing error, the function returns an error message.
	fn parse(off: usize, b: &[u8]) -> Result<(Self, usize), Error>;
}

/// Implements the Parseable trait for the given primitive type.
macro_rules! impl_aml_parseable_primitive {
	($type:ty) => {
		impl Parseable for $type {
			fn parse(off: usize, b: &[u8]) -> Result<(Self, usize), Error> {
				let len = core::mem::size_of::<$type>();
				if b.len() < len {
					// TODO Error message
					let err = String::from(b"TODO").map(| msg | {
						Error {
							message: ErrorMessage::Allocated(msg),
							off,
						}
					}).unwrap_or_else(| _ | Error {
						message: ErrorMessage::Static("Allocation error"),
						off,
					});

					return Err(err);
				}

				let mut n: $type = Default::default();
				unsafe {
					core::ptr::copy_nonoverlapping(&b[0], (&mut n) as *mut _ as *mut u8, len);
				}

				Ok((n, len))
			}
		}
	}
}

/// TODO doc
pub type ByteData = u8;
/// TODO doc
pub type WordData = u16;
/// TODO doc
pub type DWordData = u32;
/// TODO doc
pub type QWordData = u64;

/// TODO doc
pub type TableSignature = DWordData;
/// TODO doc
pub type TableLength = DWordData;
/// TODO doc
pub type SpecCompliance = ByteData;
/// TODO doc
pub type CheckSum = ByteData;
/// TODO doc
pub type OemId = [ByteData; 6];
/// TODO doc
pub type OemTableId = [ByteData; 8];
/// TODO doc
pub type OemRevision = DWordData;
/// TODO doc
pub type CreatorId = DWordData;
/// TODO doc
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

/// TODO doc
#[derive(Parseable)]
pub struct TermList {
	// TODO
}

/// Base of the AML Abstract Syntax Tree (AST).
#[derive(Parseable)]
pub struct AMLCode {
	def_block_header: DefBlockHeader,
	term_list: TermList,
}

/// Parses the given AML code.
/// On parsing error, the function returns an error message.
pub fn parse(_aml: &[u8]) -> Result<AMLCode, String> {
	// TODO
	todo!();
}
