#!/bin/bash

echo "Please unmount every partition on the device before proceeding."
read -p "Type device to use as a USB stick (warning: this will erase all data on it): /dev/" device_name
read -p "Type device label (warning: this will erase all data on it): /media/" label

device=/dev/$device_name
media=/media/$label
new_label=MAESTRO

mkfs.vfat -F 32 -n $new_label -I $device &&
	mkdir $media &&
	mount $device $media &&
	grub-install --root-directory=$media --target=i386-pc --no-floppy --recheck --force $device &&
	cp maestro $media/boot/ &&
	cp grub.cfg $media/boot/grub
