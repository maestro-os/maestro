```
void *kmalloc(size_t size, int flags);
```

Allocates a block of memory of the specified **size** in bytes.

**flags** is a value containing OR-ed flags. Those can be the following:
- **KMALLOC_BUDDY**: Uses the buddy allocator to get memory instead of the page allocator
