TTY
***

The TTY (text terminal) is commonly used in UNIX-like environments. By default, it is rendered using the `VGA` text mode.

For process API references, check the documentation of module `tty`.



About VGA text mode
===================

The TTY is currently using `VGA` text mode for TTY rendering, however this solution comes with a few downsides as described in the related section.

In the future, the TTY shall be re-implemented with modern technology to avoid these problems.



Overview
========

The TTY implements keyboard input, history up to 128 lines, scrolling and various control sequences.
Line edition is not handled by the TTY as this task is fullfilled by the running process (a shell for example).

The default keyboard input method is by using the PS/2 driver, see `PS2/keyboard`.



Controls
--------

Here is a list of controls for the terminal:

- Shift + PgUp: Scrolls up one page
- Shift + PgDn: Scrolls down one page
TODO



ANSI escape sequences
---------------------

The ANSI escape sequences are characters sequences that allow to control the terminal. See: `ANSI Escape Code <https://en.wikipedia.org/wiki/ANSI_escape_code>_`
