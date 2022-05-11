Building a distribution
***********************

The kernel cannot work by itself. It requires some other programs around to work as an operating system.

Booting
-------

To boot, the kernel requires a bootloader that supports Multiboot2, such as GRUB2. It is required to provide as an argument major and minor numbers of the VFS's root device (See: `Booting <booting>_`).

The list of devices major/minor numbers can be found `here <device/list>_`.

Then, the kernel tries to start the first process, by running the init binary. this binary is located at `/sbin/init`.

The kernel will check all these pathes in the same order. A command line argument can be used to specify the init binary path, overriding the previous ones.

The init process has PID `1` and is running as the superuser. This process has the role of initializing the rest of the system. If it ever exits, the kernel shall panic.
