//! TODO doc

use core::ops::Range;
use crate::util::container::string::String;
use derive::AMLParseable;

/// TODO doc
const ZERO_OP: u8 = 0x00;
/// TODO doc
const OneOp: u8 = 0x01;
/// TODO doc
const AliasOp: u8 = 0x06;
/// TODO doc
const NameOp: u8 = 0x08;
/// TODO doc
const BytePrefix: u8 = 0x0a;
/// TODO doc
const WordPrefix: u8 = 0x0b;
/// TODO doc
const DWordPrefix: u8 = 0x0c;
/// TODO doc
const StringPrefix: u8 = 0x0d;
/// TODO doc
const QWordPrefix: u8 = 0x0e;
/// TODO doc
const ScopeOp: u8 = 0x10;
/// TODO doc
const BufferOp: u8 = 0x11;
/// TODO doc
const PackageOp: u8 = 0x12;
/// TODO doc
const VarPackageOp: u8 = 0x13;
/// TODO doc
const MethodOp: u8 = 0x14;
/// TODO doc
const ExternalOp: u8 = 0x15;
/// TODO doc
const DualNamePrefix: u8 = 0x2e;
/// TODO doc
const MultiNamePrefix: u8 = 0x2f;
/// TODO doc
const DigitChar: Range<u8> = 0x30..0x39;
/// TODO doc
const NameChar: Range<u8> = 0x41..0x5a;
/// TODO doc
const ExtOpPrefix: u8 = 0x5b;
/// TODO doc
const MutexOp: &[u8] = &[0x5b, 0x01];
/// TODO doc
const EventOp: &[u8] = &[0x5b, 0x02];
/// TODO doc
const CondRefOfOp: &[u8] = &[0x5b, 0x12];
/// TODO doc
const CreateFieldOp: &[u8] = &[0x5b, 0x13];
/// TODO doc
const LoadTableOp: &[u8] = &[0x5b, 0x1f];
/// TODO doc
const LoadOp: &[u8] = &[0x5b, 0x20];
/// TODO doc
const StallOp: &[u8] = &[0x5b, 0x21];
/// TODO doc
const SleepOp: &[u8] = &[0x5b, 0x22];
/// TODO doc
const AcquireOp: &[u8] = &[0x5b, 0x23];
/// TODO doc
const SignalOp: &[u8] = &[0x5b, 0x24];
/// TODO doc
const WaitOp: &[u8] = &[0x5b, 0x25];
/// TODO doc
const ResetOp: &[u8] = &[0x5b, 0x26];
/// TODO doc
const ReleaseOp: &[u8] = &[0x5b, 0x27];
/// TODO doc
const FromBCDOp: &[u8] = &[0x5b, 0x28];
/// TODO doc
const ToBCD: &[u8] = &[0x5b, 0x29];
/// TODO doc
const RevisionOp: &[u8] = &[0x5b, 0x30];
/// TODO doc
const DebugOp: &[u8] = &[0x5b, 0x31];
/// TODO doc
const FatalOp: &[u8] = &[0x5b, 0x32];
/// TODO doc
const TimerOp: &[u8] = &[0x5b, 0x33];
/// TODO doc
const OpRegionOp: &[u8] = &[0x5b, 0x80];
/// TODO doc
const FieldOp: &[u8] = &[0x5b, 0x81];
/// TODO doc
const DeviceOp: &[u8] = &[0x5b, 0x82];
/// TODO doc
const ProcessorOp: &[u8] = &[0x5b, 0x83];
/// TODO doc
const PowerResOp: &[u8] = &[0x5b, 0x84];
/// TODO doc
const ThermalZoneOp: &[u8] = &[0x5b, 0x85];
/// TODO doc
const IndexFieldOp: &[u8] = &[0x5b, 0x86];
/// TODO doc
const BankFieldOp: &[u8] = &[0x5b, 0x87];
/// TODO doc
const DataRegionOp: &[u8] = &[0x5b, 0x88];
/// TODO doc
const RootChar: u8 = 0x5c;
/// TODO doc
const ParentPrefixChar: u8 = 0x5e;
/// TODO doc
const NameChar_: u8 = 0x5f;
/// TODO doc
const Local0Op: u8 = 0x60;
/// TODO doc
const Local1Op: u8 = 0x61;
/// TODO doc
const Local2Op: u8 = 0x62;
/// TODO doc
const Local3Op: u8 = 0x63;
/// TODO doc
const Local4Op: u8 = 0x64;
/// TODO doc
const Local5Op: u8 = 0x65;
/// TODO doc
const Local6Op: u8 = 0x66;
/// TODO doc
const Local7Op: u8 = 0x67;
/// TODO doc
const Arg0Op: u8 = 0x68;
/// TODO doc
const Arg1Op: u8 = 0x69;
/// TODO doc
const Arg2Op: u8 = 0x6a;
/// TODO doc
const Arg3Op: u8 = 0x6b;
/// TODO doc
const Arg4Op: u8 = 0x6c;
/// TODO doc
const Arg5Op: u8 = 0x6d;
/// TODO doc
const Arg6Op: u8 = 0x6e;
/// TODO doc
const StoreOp: u8 = 0x70;
/// TODO doc
const RefOfOp: u8 = 0x71;
/// TODO doc
const AddOp: u8 = 0x72;
/// TODO doc
const ConcatOp: u8 = 0x73;
/// TODO doc
const SubtractOp: u8 = 0x74;
/// TODO doc
const IncrementOp: u8 = 0x75;
/// TODO doc
const DecrementOp: u8 = 0x76;
/// TODO doc
const MultiplyOp: u8 = 0x77;
/// TODO doc
const DivideOp: u8 = 0x78;
/// TODO doc
const ShiftLeftOp: u8 = 0x79;
/// TODO doc
const ShiftRightOp: u8 = 0x7a;
/// TODO doc
const AndOp: u8 = 0x7b;
/// TODO doc
const NandOp: u8 = 0x7c;
/// TODO doc
const OrOp: u8 = 0x7d;
/// TODO doc
const NorOp: u8 = 0x7e;
/// TODO doc
const XorOp: u8 = 0x7f;
/// TODO doc
const NotOp: u8 = 0x80;
/// TODO doc
const FindSetLeftBitOp: u8 = 0x81;
/// TODO doc
const FindSetRightBitOp: u8 = 0x82;
/// TODO doc
const DerefOfOp: u8 = 0x83;
/// TODO doc
const ConcatResOp: u8 = 0x84;
/// TODO doc
const ModOp: u8 = 0x85;
/// TODO doc
const NotifyOp: u8 = 0x86;
/// TODO doc
const SizeOfOp: u8 = 0x87;
/// TODO doc
const IndexOp: u8 = 0x88;
/// TODO doc
const MatchOp: u8 = 0x89;
/// TODO doc
const CreateDWordFieldOp: u8 = 0x8a;
/// TODO doc
const CreateWordFieldOp: u8 = 0x8b;
/// TODO doc
const CreateByteFieldOp: u8 = 0x8c;
/// TODO doc
const CreateBitFieldOp: u8 = 0x8d;
/// TODO doc
const ObjectTypeOp: u8 = 0x8e;
/// TODO doc
const CreateQWordFieldOp: u8 = 0x8f;
/// TODO doc
const LandOp: u8 = 0x90;
/// TODO doc
const LorOp: u8 = 0x91;
/// TODO doc
const LnotOp: u8 = 0x92;
/// TODO doc
const LNotEqualOp: &[u8] = &[0x92, 0x93];
/// TODO doc
const LLessEqualOp: &[u8] = &[0x92, 0x94];
/// TODO doc
const LGreaterEqualOp: &[u8] = &[0x92, 0x95];
/// TODO doc
const LEqualOp: u8 = 0x93;
/// TODO doc
const LGreaterOp: u8 = 0x94;
/// TODO doc
const LLessOp: u8 = 0x95;
/// TODO doc
const ToBufferOp: u8 = 0x96;
/// TODO doc
const ToDecimalStringOp: u8 = 0x97;
/// TODO doc
const ToHexStringOp: u8 = 0x98;
/// TODO doc
const ToIntegerOp: u8 = 0x99;
/// TODO doc
const ToStringOp: u8 = 0x9c;
/// TODO doc
const CopyObjectOp: u8 = 0x9d;
/// TODO doc
const MidOp: u8 = 0x9e;
/// TODO doc
const ContinueOp: u8 = 0x9f;
/// TODO doc
const IfOp: u8 = 0xa0;
/// TODO doc
const ElseOp: u8 = 0xa1;
/// TODO doc
const WhileOp: u8 = 0xa2;
/// TODO doc
const NoopOp: u8 = 0xa3;
/// TODO doc
const ReturnOp: u8 = 0xa4;
/// TODO doc
const BreakOp: u8 = 0xa5;
/// TODO doc
const BreakPointOp: u8 = 0xcc;
/// TODO doc
const OnesOp: u8 = 0xff;

/// Trait representing a parseable object.
pub trait AMLParseable: Sized {
	/// Parses the object from the given bytes `b`.
	/// The function returns an instance of the parsed object and the consumed length.
	/// On parsing error, the function returns an error message.
	fn parse(b: &[u8]) -> Result<(Self, usize), String>;
}

/// Base of the AML Abstract Syntax Tree (AST).
#[derive(AMLParseable)]
pub struct AMLCode {
	// TODO
	/*def_block_header: DefBlockHeader,
	term_list: TermList,*/
}

/// Parses the given AML code.
/// On parsing error, the function returns an error message.
pub fn parse(_aml: &[u8]) -> Result<AMLCode, String> {
	// TODO
	todo!();
}
