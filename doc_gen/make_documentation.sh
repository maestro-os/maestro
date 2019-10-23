#/bin/bash

set -C noclobber

proto=`sh get_prototypes.sh`
names=`echo $proto | awk 'print $4'`

IFS=''
for $f in $proto;
do
	unset IFS
	p=TODO
	n=`echo $f | awk '{print $1}'`
	echo "```$p```TODO" > ${n}.md
	IFS='' # TODO Remove?
done
