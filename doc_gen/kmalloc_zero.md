```
void *kmalloc_zero(size_t size, int flags);
```

Calls **kmalloc** passing the exact same parameters and applies **bzero** to the allocated block of memory.
