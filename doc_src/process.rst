Process
*******

A process is a task running a program. It can be paused and resume, and communicates with the kernel using system calls.



Virtual memory
==============

Each process has its own virtual memory on which it can allocate mappings through syscalls.



Memory layout
-------------

The layout of the virtual memory of each process is the following:

+------------+------------+---------------------------------------------------------------------------------------------+
| Begin      | End        | Description                                                                                 |
+============+============+=============================================================================================+
| 0x0        | 0x1000     | The first page of memory is not available for any usage since it contains the NULL pointer. |
+------------+------------+---------------------------------------------------------------------------------------------+
| 0x1000     | TODO       | ELF sections data (code, initialized and uninitialized data, etc...)                        |
+------------+------------+---------------------------------------------------------------------------------------------+
| TODO       | TODO       | Heap/Allocatable memory                                                                     |
+------------+------------+---------------------------------------------------------------------------------------------+
| TODO       | TODO       | Shared libraries                                                                            |
+------------+------------+---------------------------------------------------------------------------------------------+
| TODO       | TODO       | Userspace Stack                                                                             |
+------------+------------+---------------------------------------------------------------------------------------------+
| TODO       | 0xc0000000 | argv, environ                                                                               |
+------------+------------+---------------------------------------------------------------------------------------------+
| 0xc0000000 | Memory end | Kernel memory                                                                               |
+------------+------------+---------------------------------------------------------------------------------------------+

TODO: Find a place for kernelside stack



Scheduling
==========

Processes scheduling allows to share the CPU time of the host machine between each processes.
Every processes may have different priority levels, the greater the priority, the more CPU time the process gets.

The kernel uses the PIT to tick the processes scheduler at a fixed interval. At each tick, the scheduler determines the next process to be resumed.
Each process stores the number of ticks it has been running, if the process is selected, its counter increments. If another process is select, the counter of the previous process is set to 0.

The number of ticks a process has to run before switching to another is computed relative to its priority. This is done by linear interpolation between the average priority over every running processes and the priority of the process which has the greatest:

.. math::

    \tau = \frac{P - P_{avg}}{P_{max} - P_{avg}} \\
    T = (1 - \tau) T_{avg} + \tau T_{max}

Where,

- :math:`P` is the priority of the process
- :math:`P_{avg}` is the average priority of the running processes
- :math:`P_{avg}` is the greatest priority of the running processes
- :math:`T_{avg}` is a constant which defines the number of ticks for the average process priority
- :math:`T_{max}` is a constant which defines the number of ticks for the process with the greatest priority
- :math:`T` is the number of ticks the process will run.



Signals
=======

TODO



Fork
====

Forking is the action of duplicating a process to obtain two identical processes with identical memories, registers, file descriptors, PGIDs, SIDs, etc... except they have different PIDs.
When process A forks and creates process B, A becomes the parent process of B.
A process can fork using the system call ``fork``.
