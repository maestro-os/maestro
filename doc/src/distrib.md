# Distribution

The kernel cannot work by itself. It requires other programs to work as a real operating system.
Such programs can include, but not limited to: a deamon manager, a shell, a package manager, a windowing system, etc...

This chapter describes how the kernel interacts with a distribution.



## Booting

To boot, the kernel requires a bootloader that supports Multiboot2, such as GRUB2. It is required to provide as an argument major and minor numbers of the VFS's root device (See the [**Booting** chapter](./booting.md)).

The list of devices major/minor numbers can be found [here](./device/list.md).

Then, the kernel tries to start the first process, by running the init program. This program is located at `/sbin/init`.

The init process has PID `1` and is running as the superuser (uid: `0`, gid: `0`). This process has the role of initializing the rest of the system. If it ever exits, the kernel shall panic.



## Modules

Not every features are implemented directly in the kernel.
To resolve this problem, a distribution can provide kernel modules and load them with the `init_module` or `finit_module` system call, and unload them with `delete_module`.

To write a kernel module, refer to the [**Module** chapter](./module.md).
