.. Maestro documentation master file, created by
   sphinx-quickstart on Mon Jun 22 00:19:04 2020.
   You can adapt this file completely to your liking, but it should at least
   contain the root `toctree` directive.

Maestro documentation
*********************

.. toctree::
   :maxdepth: 2
   :caption: Contents:

   external_doc
   dependencies
   compilation
   selftest
   debug

   booting
   distribution

   VGA
   TTY
   PS2/keyboard
   PS2/mouse

   memory/a20
   memory/mem_map
   memory/buddy_alloc
   memory/kmalloc
   memory/vmem
   memory/mem_space

   interruptions
   task_switching

   process

   device/list

   ELF
   modules

   ACPI/AML
   CMOS
   PCI
   cpuid



Overview
========

Maestro is a Unix-like kernel written in Rust. It follows the POSIX specifications.



Intented audience
=================

This documentation describes the way the Maestro kernel and its interfaces work. The targeted audience for these documents are kernel and module developers.



Interface references
====================

The references to the kernel's internals and module interfaces can be found `here <references/kernel/index.html>`_.



License
=======

The kernel and this documentation are under MIT license.



Indices and tables
==================

* :ref:`genindex`
* :ref:`search`
