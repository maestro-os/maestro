if grub-file --is-x86-multiboot crumbleos; then
	echo "Multiboot:	OK :D"
else
	echo "Multiboot:	KO :("
fi

if grub-file --is-x86-multiboot2 crumbleos; then
	echo "Multiboot 2:	OK :D"
else
	echo "Multiboot 2:	KO :("
fi
