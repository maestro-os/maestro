#!/bin/bash

echo "Running selftests..."
# TODO

echo "Selftests output:"
cat serial.log
grep 'No more tests to run' -- serial.log >/dev/null 2>&1
