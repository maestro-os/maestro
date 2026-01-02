# Synchronization primitives

## Spin

`Spin` is simply a spinlock, the most basic synchronization primitive.
A spinlock is acquired by **atomically** setting its value.
Other contexts trying to lock it in the meantime will loop until the value is clear (by unlocking it).

A spinlock does not have context ordering, nor make a process sleep. For this, use a **Mutex**.

## RwLock

`RwLock` is a read-write lock (also called "pushlock"). It allows locking a resource like a spinlock, except it can accept several readers OR a single writer at once (contrary to the spinlock which accepts only one reader or writer at once).

This is useful to reduce contention when a resource is read often, but rarely written.

## Wait queues

The `WaitQueue` makes a process wait on a resource by putting it in an interruptible sleep state. Processes are ordered in the queue in a FIFO fashion.

A process can wait on a resource until a given condition is fulfilled.

## Mutex

A `Mutex` uses both a `Spin` and a `WaitQueue`. It allows locking a resources, putting processes waiting on it to sleep.

Unlocking the mutex wakes the next process in queue.