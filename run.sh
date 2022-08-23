#!/bin/sh

# abort on errors
set -e

# run all
./run_one.sh 18
./run_one.sh 16
./run_one.sh 8

# Combine output
python combine.py
