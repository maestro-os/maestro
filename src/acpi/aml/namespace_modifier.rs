//! TODO doc

use super::AMLParseable;
use super::Error;
use macros::Parseable;

/// TODO doc
#[derive(Parseable)]
pub struct DefAlias {
	// TODO
}

/// TODO doc
#[derive(Parseable)]
pub struct DefName {
	// TODO
}

/// TODO doc
#[derive(Parseable)]
pub struct DefScope {
	// TODO
}

/// TODO doc
#[allow(clippy::enum_variant_names)]
#[derive(Parseable)]
pub enum NameSpaceModifierObj {
	DefAlias(DefAlias),
	DefName(DefAlias),
	DefScope(DefAlias),
}
