#!/bin/sh

# This script takes the configuration of the kernel from the configuration file and to turns it into command line arguments to pass to the compiler

cat .config 2>/dev/null | grep -v '="false"$' | sed 's/^/--cfg config_/' | sed 's/="true"$//' | tr '\n' ' '
