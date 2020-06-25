TTY
===

The TTY (text terminal) is commonly used in UNIX-like environments. By default, it is rendered using the `VGA` text mode.



About VGA text mode
-------------------

The TTY is currently using `VGA` text mode for TTY rendering, however this solution comes with a few downsides as described in the related section.

In the future, the TTY shall be re-implemented with a more modern technology to avoid these problems.



Overview
--------

The TTY implements keyboard input, history up to 128 lines, scrolling and various control sequences.
Line edition is not handled by the TTY as this task is fullfilled by the running process (a shell for example).

The default keyboard input method is by using the PS/2 driver, see `PS2/keyboard`.



Controls
--------

Here is a list of controls for the terminal:

- Ctrl + Q: Resume terminal
- Ctrl + S: Suspend terminal
- Ctrl + W: Erases input
- Shift + PgUp: Scrolls up one page
- Shift + PgDn: Scrolls down one page
TODO



printf
------

An implementation of the printf function is available in the kernel for general and debugging purpose. Though it shouldn't be use outside of the booting process as normal operating environment should be directed by userspace processes.

One can refer to the POSIX description of the printf function for usage.



ANSI escape sequences
---------------------

TODO
