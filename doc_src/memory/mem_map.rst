Memory map
**********

The kernel is using the memory mapping given by Multiboot2. To determine the parts of the memory which can be used for physical memory allocations.



Multiboot2 mapping
==================

When booting, the bootloader retrieves memory mapping informations.
Memory region types include:

- **Available**: Memory available for use
- **Reserved**: Memory unavailable for use
- **ACPI Reclaimable**: Memory used by ACPI
- **NVS**: Hibernating memory
- **Bad RAM**: Invalid physical memory region



Physical memory mapping (x86)
=============================

+----------+------------+---------------------------------------------+
| From     | To         | Description                                 |
+==========+============+=============================================+
| 0x0      | 0x800      | Nothing in particular                       |
+----------+------------+---------------------------------------------+
| 0x800    | 0x1000     | The Global Descriptor table                 |
+----------+------------+---------------------------------------------+
| 0x1000   | 0x100000   | Various BIOS memory and DMA mappings        |
+----------+------------+---------------------------------------------+
| 0x100000 | x          | Kernel image                                |
+----------+------------+---------------------------------------------+
| x        | y          | Multiboot data                              |
+----------+------------+---------------------------------------------+
| y        | z          | Big chunk of memory for the buddy allocator |
+----------+------------+---------------------------------------------+
| z        | end        | ACPI data and unused memory                 |
+----------+------------+---------------------------------------------+
