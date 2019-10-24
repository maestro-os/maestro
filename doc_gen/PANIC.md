```
# define PANIC(reason, code) kernel_panic_(reason, code, __FILE__, __LINE__)
```

Triggers a kernel panic with the specified **reason** string and **code**.
