# Keyboard handling

Keyboard inputs are handled using the PS/2 controller for retro compatibility.

Each key is associated to one or several keycodes. When a key is pressed or released, the IDT `0x21` is fired and the keycode is read by the kernel.
