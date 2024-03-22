#!/bin/bash

# This script links partitions from the partitions table on the disk `qemu_disk` to device files in order to mount them

export IFS='
'

SECTOR_SIZE=512

for p in $(fdisk -l qemu_disk | grep '^qemu_disk'); do
	NAME=$(echo $p | awk '{print $1}')
	START=$(echo $p | awk '{print $2}')
	END=$(echo $p | awk '{print $3}')
	SIZE=$(($END - $START))

	DEV=$(losetup -o $(($START * $SECTOR_SIZE)) --sizelimit $(($SIZE * $SECTOR_SIZE)) --sector-size $SECTOR_SIZE --show -f qemu_disk)
	echo "$NAME linked as $DEV (offset: $(($START * $SECTOR_SIZE)); size: $(($SIZE * $SECTOR_SIZE)))"
done
