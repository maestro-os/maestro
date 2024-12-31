# Script

A script can be run directly by the kernel by specifying its interpreter use a **shebang**.

The syntax of a shebang is the following:

```
#!interpreter-path [optional-arg]
```

The shebang is placed at the top of the script file.

Description:
- `interpreter-path` is the path to the interpreter program. The interpreter can itself be a script, up to 4 recursions
- `optional-arg` is an optional argument to be appended to the interpreter
