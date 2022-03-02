//! An INode is an identifier allowing to locate a file on a filesystem.

use core::any::Any;

/// Trait representing an INode. A structure implementing this trait can contain
/// filesystem-specific data.
pub trait INode: Any {}
