//! TODO doc

use super::named_obj::NamedObj;
use super::namespace_modifier::NameSpaceModifierObj;
use super::type1_opcode::Type1Opcode;
use super::type2_opcode::Type2Opcode;
use super::AMLParseable;
use super::Error;
use macros::Parseable;

/// TODO doc
#[derive(Parseable)]
pub enum Object {
	NameSpaceModifierObj(NameSpaceModifierObj),
	NamedObj(NamedObj),
}

/// TODO doc
#[derive(Parseable)]
pub enum TermObject {
	Object(Object),
	Type1Opcode(Type1Opcode),
	Type2Opcode(Type2Opcode),
}

/// TODO doc
#[derive(Parseable)]
pub struct TermList {
	// TODO objects: Vec<TermObject>,
}
