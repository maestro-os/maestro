#!/bin/bash

# TODO doc

grep "^$1" .config | sed 's/.*\( |\t\)*=\( |\t\)*\"//' | sed 's/\"$//'
