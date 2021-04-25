#!/bin/bash

# TODO doc

cat .config | sed 's/^/--cfg /' | tr '\n' ' '
