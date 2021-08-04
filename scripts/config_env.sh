#!/bin/sh

# This script takes the configuration of the kernel from the configuration file and to turns it into environment variables

cat .config 2>/dev/null | sed -e 's/^\(.*\)=/\U\1=/' | sed -e 's/^/CONFIG_/' | tr '\n' ' '
