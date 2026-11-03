#!/usr/bin/env python3
"""Count static LLVM operations per function (not executed instructions).

Usage: python3 benchmarks/audit/ir_counts.py path/to/main.ll > counts.json
Use Callgrind and compare.py --profile for dynamic machine instruction counts.
"""

from collections import Counter
import json
from pathlib import Path
import re
import sys


OPERATIONS = set("""ret br switch indirectbr invoke callbr resume catchswitch catchret cleanupret
unreachable fneg add fadd sub fsub mul fmul udiv sdiv fdiv urem srem frem shl lshr ashr and or xor
extractelement insertelement shufflevector extractvalue insertvalue alloca load store fence cmpxchg
atomicrmw getelementptr trunc zext sext fptrunc fpext fptoui fptosi uitofp sitofp ptrtoint inttoptr
bitcast addrspacecast icmp fcmp phi select freeze call va_arg landingpad catchpad cleanuppad""".split())


def counts(path):
    functions = {}
    current = None
    for line in Path(path).read_text().splitlines():
        if line.startswith("define "):
            current = re.search(r'@("[^"]+"|[^ (]+)\(', line)[1]
            functions[current] = Counter()
        elif line == "}":
            current = None
        elif current is not None:
            instruction = re.sub(r"^\s*%[^=]+ = ", "", line).strip()
            instruction = re.sub(r"^(?:tail|musttail|notail) ", "", instruction)
            operation = instruction.split(" ", 1)[0]
            if operation in OPERATIONS:
                functions[current][operation] += 1
    total = sum(functions.values(), Counter())
    return {"total": dict(total), "functions": functions}


if __name__ == "__main__":
    print(json.dumps(counts(sys.argv[1]), indent=2))
