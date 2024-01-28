//! This module implements utility functions for files manipulations.

use super::{
	path::{Component, Path, PathBuf},
	FileContent,
};
use crate::{
	errno,
	errno::EResult,
	file::{perm::AccessProfile, vfs, vfs::ResolutionSettings},
	util::container::{hashmap::HashMap, string::String},
};

/// Creates the directories necessary to reach path `path`.
///
/// If relative, the path is taken from the root.
pub fn create_dirs(path: &Path) -> EResult<()> {
	// Path of the parent directory
	let mut p = PathBuf::root();
	for comp in path.components() {
		let Component::Normal(name) = &comp else {
			continue;
		};
		if let Ok(parent_mutex) = vfs::get_file_from_path(&p, &ResolutionSettings::kernel_follow())
		{
			let mut parent = parent_mutex.lock();
			let res = vfs::create_file(
				&mut parent,
				String::try_from(*name)?,
				&AccessProfile::KERNEL,
				0o755,
				FileContent::Directory(HashMap::new()),
			);
			match res {
				Err(e) if e.as_int() != errno::EEXIST => return Err(e),
				_ => {}
			}
		}
		p = p.join(comp)?;
	}
	Ok(())
}
