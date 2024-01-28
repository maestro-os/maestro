//! TODO doc

use super::{
	named_obj::NamedObj, namespace_modifier::NameSpaceModifierObj, type1_opcode::Type1Opcode,
	type2_opcode::Type2Opcode, AMLParseable, Error,
};
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
