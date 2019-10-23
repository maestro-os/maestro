#!/bin/bash

for f in `ls -1 *.md`;
do
	if [ "`tail -n 1 $f`" = "TODO" ];
	then
		rm $f
	fi
done
