#!/bin/bash

grep '^ [^\*]' -rn src/
grep ' $' -rn src/
grep '	$' -rn src/
