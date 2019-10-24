```
void *krealloc(void *ptr, size_t size, int flags);
```

If **ptr** is **NULL**, calls **kmalloc** and returns the allocated block.

If **size** equals `0`, calls **kfree**.

Else, the function tries to change the size of the given block of memory to the given **size**.

See **kmalloc** for **flags**.

If the given **ptr** is not the beginning of an allocated block of memory, the behaviour is undefined.
