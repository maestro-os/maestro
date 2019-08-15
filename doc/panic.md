# Kernel panic

Sometimes, the kernel might be forced to stop because of an unexpected internal problem. Then it will trigger a kernel panic and halt the system to prevent further modifications to the system as the kernel may now be unsafe to use.

The kernel panic screen shows the following informations:
- The reason for the kernel panic (might be unknown)
- An associated error code (useful when reporting a bug)

If the kernel is in debug mode, it will also print the file and line in the kernel's source where the panic has been triggered.
