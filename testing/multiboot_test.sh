#!/bin/bash

# This file is meant to check that the kernel image is a valid Multiboot-compilant kernel.

grub-file --is-x86-multiboot maestro && echo "Multiboot:	OK :D" || echo "Multiboot:	KO :("
grub-file --is-x86-multiboot2 maestro && echo "Multiboot 2:	OK :D" || echo "Multiboot 2:	KO :("

grub-file --is-x86-multiboot maestro || grub-file --is-x86-multiboot2 maestro
