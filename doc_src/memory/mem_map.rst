Memory map
**********

The memory mapping can be retrived using different ways. Because the kernel is using Multiboot2 to boot, it also uses the memory map given by it.



Multiboot2 mapping
==================

When booting, the bootloader retrieves memory mapping informations.
Memory region types include:

- **Available**: Memory available for use
- **Reserved**: Memory unavailable for use
- **ACPI Reclaimable**: Memory used by ACPI
- **NVS**: Hibernating memory
- **Bad RAM**: Invalid physical memory region



Kernel mapping
==============

Using memory mapping informations, the kernel can reserve memory for its own usage. The mapping is available below:

+----------+------------+-------------------------------------------------------------------------------------+
| From     | To         | Description                                                                         |
+==========+============+=====================================================================================+
| 0x0      | 0x1000     | Non-mapped memory, to make the kernel crash if trying to dereference a NULL pointer |
+----------+------------+-------------------------------------------------------------------------------------+
| 0x1000   | 0x100000   | Various BIOS memory and DMA mappings                                                |
+----------+------------+-------------------------------------------------------------------------------------+
| 0x100000 | x          | Kernel image, write protected                                                       |
+----------+------------+-------------------------------------------------------------------------------------+
| x        | y          | Multiboot data                                                                      |
+----------+------------+-------------------------------------------------------------------------------------+
| y        | z          | Big chunk of memory for the buddy allocator                                         |
+----------+------------+-------------------------------------------------------------------------------------+
| z        | end        | ACPI data and unused memory                                                         |
+----------+------------+-------------------------------------------------------------------------------------+
