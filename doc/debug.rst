Debug
*****

This section describes debugging features integrated to the kernel. These features are not intended for release builds and are thus disabled by default.

Debugging features are implemented in directory **src/debug/**.



Enabling debugging mode
=======================

Debugging mode is enabled through macros, they can be defined using option **-D** in the compilation command line.

Available debugging flags (macros) are:

- **KERNEL_DEBUG**: Enables general debugging features
- **KERNEL_DEBUG_SANITY**: Enables parameters sanity checks
- **KERNEL_DEBUG_SELFTEST**: Enables kernel selftesting
- **KERNEL_DEBUG_SPINLOCK**: Enables spinlock logging



General debugging
=================

General debugging features include:

- Assertions
- Registers printing
- Memory printing
- Callstack printing
- Profiler



Assertions
----------

Assertions allow to check a condition at a specific moment of execution. If the assertion fails, the kernel stops, printing some informations.
This is especially usefull when a variable is assumed to at a certain value. If the condition is not fullfilled, something went wrong before the assertion.

Assertions can be used with macro **debug_assert**.



Registers printing
------------------

Registers printing allows to print the value into the strucutre **regs** given in parameters. This is usefull when debugging context switching.

Registers printing can be used with function **print_regs**.



Memory printing
---------------

Memory printing is about dumping the content of a memory block at the specified pointer. This feature can be used with function **print_memory**.



Callstack printing
-------------------

Callstack printing uses the ELF structures of the kernel image to check which functions were called to reach current execution state.
The kernel image must not be stripped to print the stack for obvious reasons.

The stack can be printed using function **print_callstack**.



Profiler
--------

Profiler is a work-in-progress feature which will eventually allow to check which part of the execution takes the more CPU time by interrupting execution at a regular interval and checking the callstack.



Selftesting
===========

Selftesting is a special debugging mode. It's an attempt of unit testing for various features of the kernel. It is implemented in directory **src/selftest/**.

When running selftests, the kernel only enables minimal features. This mode is mostly usefull to test memory allocation.

Although selftesting might give usefull informations on whether a features gives the expected result, it's likely to be unstable as a test can corrupt the kernel's memory and thus, modify the results of the following tests.
This risk is minimized by using paging to protect important portions of memory, but for this reason, paging itself cannot be tested.



Spinlock logging
================

Spinlock logging feature allows to write on the TTY whenever a spinlock is aquired or released, it is usefull to diagnose deadlocks.
