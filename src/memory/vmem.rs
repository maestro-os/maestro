/*
 * TODO Doc
 */

type Vmem = *const u32;
type MutVmem = *mut u32;

extern "C" {
	pub fn cr0_get() -> u32;
	pub fn cr0_set(flags: u32);
	pub fn cr0_clear(flags: u32);
	pub fn cr2_get() -> u32;
	pub fn cr3_get() -> u32;

	fn paging_enable(directory: *const u32);
	fn paging_disable();
	fn tlb_reload();
}

// TODO
/*fn init() -> Vmem;
fn kernel();
fn resolve(vmem: Vmem, ptr: *const Void) -> *const u32;
fn is_mapped(vmem: Vmem, ptr: *const Void) -> bool;
fn contains(vmem: Vmem, ptr: *const Void, size: usize) -> bool;
fn translate(vmem: Vmem, ptr: *const Void) -> *const Void;
fn get_entry(vmem: Vmem, ptr: *const Void) -> u32;
fn map(vmem: Vmem, physaddr: *const Void, virtaddr: *const Void, flags: u32);
fn map_pse(vmem: Vmem, physaddr: *const Void, virtaddr: *const Void, flags: u32);
fn map_range(vmem: Vmem, physaddr: *const Void, virtaddr: *const Void, pages: usize, flags: u32);
fn identity(vmem: Vmem, page: *const Void, flags: u32);
fn identity_pse(vmem: Vmem, page: *const Void, flags: u32);
fn identity_range(vmem: Vmem, from: *const Void, pages: usize, flags: u32);
fn unmap(vmem: Vmem, virtaddr: *const Void);
fn unmap_range(vmem: Vmem, virtaddr: *const Void, pages: usize);
fn clone(vmem: Vmem) -> Vmem;
fn flush(vmem: Vmem);
fn destroy(vmem: Vmem);*/
