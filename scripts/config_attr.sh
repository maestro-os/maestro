#!/bin/sh

# This script returns the value of the configuration field with the given name from the configuration file

grep "^${1}=" .config 2>/dev/null | sed 's/.*\( |\t\)*=\( |\t\)*\"//' | sed 's/\"$//'
