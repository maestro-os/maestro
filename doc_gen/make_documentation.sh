#!/bin/bash

set -C noclobber

IFS=''
proto=`sh get_prototypes.sh | grep -v -f ignored`
names=`echo $proto | awk '{print $4}'`

echo $proto | while read f;
do
{
	unset IFS
	n="`echo $f | awk '{print $1}'`"
	p=`echo \`echo $f | awk '{$1=$2=$3=$4=""; print $0}'\` | xargs echo`
	echo "\`\`\`
$p
\`\`\`

TODO" >"${n}.md" 2>/dev/null
} 2>/dev/null
done
