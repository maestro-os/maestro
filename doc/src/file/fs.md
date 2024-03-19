# Filesystem

A filesystem is a representation of a files hierarchy on a storage device.

The following filesystems are natively supported:
- **ext2**: a common filesystem in UNIX environments. Now obsolete (to be replaced by ext4)



## kernfs

A **kernfs** is a special kind of filesystem that do not store any information on any storage device. Its purpose is to provide a file interface to easily transmit information to the userspace.

Native kernfs kinds include:
- [tmpfs](tmpfs.md): storage for temporary files on RAM
- [procfs](procfs.md): provides information about processes
- [sysfs](sysfs.md): provides information about the system

## Virtual FileSystem

The **VFS** is a filesystem that has no representation on any storage device. Rather, it is built from other filesystems that are assembled together to form the system's files hierarchy.

**Mouting** a filesystem is the action of adding a filesystem to the VFS so that it becomes accessible to users.

The directory on which a filesystem is mounted is called a **mountpoint**.
