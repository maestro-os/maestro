VGA
===

VGA (Video Graphics Array) was first introduced with the IBM PS/2 line of computers in 1987.
Nowdays this standard is deprecated but still supported in IBM PCs and is a good start for any rendering system.



Text mode
---------

The text mode allows to easily write text on the screen but it comes with a few downsides, including:

- Resolution is locked to 80x25 characters
- Limited to ASCII characters
- Limited colors
- Not available if not booting using a Legacy BIOS



The buffer for this mode is located at address 0xb8000 on the physical memory.
As this region of memory is in DMA, special considerations for paging are necessary: Write-Through caching should be enabled to ensure direct writting to the main memory, so that the result shows directly on screen instead of waiting for the CPU to swap the cache.

TODO: Description of every characters
TODO: Description of cursor register
