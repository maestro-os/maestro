#!/bin/bash

# TODO doc

cat .config | sed 's/^/--cfg config_/' | tr '\n' ' '
