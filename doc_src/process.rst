Process
*******

A process is a task running a program. It can be paused and resume, and communicates with the kernel using system calls.

For process API references, check the documentation of module `process`.



Process state diagram
=====================

A process always has a state. Either:
- **Running**: The process is running. When in this state, the process may be in fact paused, waiting to be resumed by the scheduler.
- **Sleeping**: The process is waiting for a resource. In this state, the process is not being executed and needs to be waked up by the kernel.
- **Stopped**: The process is stopped and needs to be resumed by a signal.
- **Zombie**: The process has been killed. Most of its resources have been freed and is waiting for the parent process to retrieve its exit code.

Diagram of state transitions for a process:

.. math::
	
	\begin{tikzpicture} [node distance = 2cm, on grid]

	\node (q0) [state] {$Running$};
	\node (q1) [state, above = of q0] {$Sleeping$};
	\node (q2) [state, right = of q0] {$Stopped$};
	\node (q3) [state, left = of q0] {$Zombie$};

	\path [-stealth, thick]
		(q0) edge node {} (q1)
		(q1) edge node {} (q0)
		(q0) edge node {} (q2)
		(q1) edge node {} (q2)
		(q2) edge node {} (q0)
		(q0) edge node {} (q3)
		(q1) edge node {} (q3);

	\end{tikzpicture}



Virtual memory
==============

Each process has its own virtual memory on which it can allocate mappings through syscalls.



Memory layout (x86)
-------------------

The layout of the virtual memory of each process is the following:

+------------+------------+---------------------------------------------------------------------------------------------+
| Begin      | End        | Description                                                                                 |
+============+============+=============================================================================================+
| 0x0        | 0x1000     | The first page of memory is not available for any usage since it contains the NULL pointer. |
+------------+------------+---------------------------------------------------------------------------------------------+
| 0x1000     | 0x40000000 | ELF sections data (code, initialized and uninitialized data, etc...)                        |
+------------+------------+---------------------------------------------------------------------------------------------+
| 0x40000000 | 0xc0000000 | Allocatable memory/Shared libraries/Stacks/argv, environ                                    |
+------------+------------+---------------------------------------------------------------------------------------------+
| 0xc0000000 | Memory end | Kernel memory                                                                               |
+------------+------------+---------------------------------------------------------------------------------------------+



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
