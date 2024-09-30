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
