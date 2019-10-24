```
extern void spin_lock(spinlock_t *spinlock);
```

Tries to hook the specified **spinlock**.
If already hooked, the task waits until the spinlock is freed.

This function uses the `xchg` instruction to prevent race conditions.
