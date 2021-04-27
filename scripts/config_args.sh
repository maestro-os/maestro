#!/bin/bash

# TODO doc

cat .config | grep -v '="false"$' | sed 's/^/--cfg config_/' | sed 's/="true"$//' | tr '\n' ' '
