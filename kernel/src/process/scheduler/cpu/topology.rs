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

//! CPU topology tree

use crate::{
	process::{Process, scheduler::cpu::PerCpu},
	sync::{once::OnceInit, spin::Spin},
};
use core::{cell::UnsafeCell, hint::likely, ptr};
use utils::{boxed::Box, collections::vec::Vec, errno::AllocResult};

/// CPU topology node
pub struct TopologyNode {
	/// Parent node
	parent: Option<&'static TopologyNode>,
	/// Child nodes
	children: UnsafeCell<Vec<&'static TopologyNode>>,

	/// ID used to avoid duplicate entries when this tree is built
	id: u32,
	/// The node's CPU. This is set only if the node is a leaf
	cpu: Option<&'static PerCpu>,
}

unsafe impl Sync for TopologyNode {}

impl TopologyNode {
	/// Inserts a node in the topology tree. This function is thread-safe.
	///
	/// This function must be used only during boot.
	///
	/// On success, the function returns a reference to the node.
	pub(crate) fn insert(
		&'static self,
		id: u32,
		cpu: Option<&'static PerCpu>,
	) -> AllocResult<&'static Self> {
		// Lock to prevent several cores from adding their topology at the same time
		static LOCK: Spin<()> = Spin::new(());
		let _guard = LOCK.lock();
		let children = unsafe { &mut *self.children.get() };
		// Looks for a node with the same ID
		if let Some(node) = children.iter().find(|node| node.id == id) {
			// There is already a node, no need to insert a new one
			return Ok(node);
		}
		// Insert node
		let node = Box::new(TopologyNode {
			parent: Some(self),
			children: UnsafeCell::new(Vec::new()),

			id,
			cpu,
		})?;
		let node = Box::into_raw(node);
		unsafe {
			let node = &*node;
			// Link back
			if let Some(cpu) = node.cpu {
				OnceInit::init(&cpu.topology_node, node);
			}
			children.push(node)?;
			Ok(node)
		}
	}

	/// Returns the parent node if any
	#[inline]
	pub fn parent(&self) -> Option<&Self> {
		self.parent
	}

	/// Returns the list of child nodes
	#[inline]
	pub fn children(&self) -> &[&Self] {
		unsafe { &*self.children.get() }
	}
}

/// Tree representing the topology of CPU cores
pub static CPU_TOPOLOGY: TopologyNode = TopologyNode {
	parent: None,
	children: UnsafeCell::new(Vec::new()),

	id: 0,
	cpu: None,
};

/// Recursively goes down the tree, looking for a core able to immediately run `proc`
fn find_core(node: &TopologyNode, proc: &Process) -> Option<&'static PerCpu> {
	if let Some(cpu) = &node.cpu {
		// This is a leaf, check if it is suitable
		if cpu.sched.can_immediately_run(proc) {
			return Some(cpu);
		}
	} else {
		// Lookup children
		for node in node.children() {
			find_core(node, proc);
		}
	}
	None
}

/// Finds the closest core able to immediately run `proc`, starting from `start`
pub fn find_closest_core(start: &'static PerCpu, proc: &Process) -> Option<&'static PerCpu> {
	if likely(start.sched.can_immediately_run(proc)) {
		return Some(start);
	}
	let mut node = *start.topology_node;
	while let Some(parent) = node.parent() {
		// Iterate over siblings
		for n in parent.children() {
			// Skip current node
			if ptr::eq(*n, node) {
				continue;
			}
			if let Some(cpu) = find_core(n, proc) {
				return Some(cpu);
			}
		}
		node = parent;
	}
	None
}
