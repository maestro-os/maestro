#!/bin/bash

cd ..

for file in `find src/ -type f -name '*.h'`;
do
	ctags -x --c-kinds=p $file;
done
