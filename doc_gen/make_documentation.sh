#!/bin/bash

set -C noclobber

IFS=''
proto=`sh get_prototypes.sh | grep -v -f ignored`
names=`echo $proto | awk '{print $4}'`

echo $proto | while read f;
do
{
	n=`echo $f | awk '{print $1}'`
	p='TODO' # TODO Must be the entire prototype (currently, only the first line is in the variable)
	echo "\`\`\`
$p
\`\`\`

TODO" >"${n}.md" 2>/dev/null
} 2>/dev/null
done
