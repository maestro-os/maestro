if grub-file --is-x86-multiboot2 kernel; then
	echo "OK :D"
else
	echo "KO :("
fi
