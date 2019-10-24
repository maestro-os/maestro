```
void kfree(void *ptr, int flags);
```

Frees a block of memory that was allocated with **kmalloc**.

See **kmalloc** for **flags**.

If the given **ptr** is not the beginning of an allocated block of memory, the behaviour is undefined.
