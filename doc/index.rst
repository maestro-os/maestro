.. Maestro documentation master file, created by
   sphinx-quickstart on Mon Jun 22 00:19:04 2020.
   You can adapt this file completely to your liking, but it should at least
   contain the root `toctree` directive.

Maestro documentation
=====================

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   dependencies
   compilation
   booting

   arch/x86

   debug

   TTY
   VGA
   PS2/keyboard
   PS2/mouse

   memory/mem_map
   memory/buddy_alloc
   memory/slab_alloc
   memory/kmalloc
   memory/vmem
   memory/mem_space

   interruptions
   interruptions/syscalls

   process
   process/scheduling
   process/signals
   process/fork

   ELF

   cpuid

   pci

   disk/mbr
   disk/gpt

   filesystem/ext2
   filesystem/vfs

   cmos

   ACPI/AML



Overview
========

This documentation describes the way the Maestro kernel works and its interfaces. The target audience for these documents are kernel or module developers.



License
=======

The kernel and the documentation is under MIT license.



Indices and tables
==================

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
