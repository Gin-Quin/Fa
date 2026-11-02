#!/usr/bin/env python3
"""Prints the runs of a hot.txt report that start at the given addresses.
Usage: show.py <hot.txt> <0xaddress>…"""

import sys

path, wanted = sys.argv[1], set(sys.argv[2:])
printing = False
with open(path) as file:
    for line in file:
        if line.startswith("== "):
            printing = line.split()[-1] in wanted
        elif not line.strip():
            printing = False
        if printing:
            print(line, end="")
