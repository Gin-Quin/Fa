#!/usr/bin/env python3
"""Prints the hottest instructions of an executable from a callgrind profile taken with
--dump-instr=yes (compare.py --mode profile), interleaved with their disassembly (objdump), grouped into runs of
consecutive hot instructions, so that the inlined loops of `main` can be read with their cost.

Usage: hot.py <callgrind.out> <main.s> [--top N] [--min-share 0.002] [--calls]
  --calls also prints the call edges (caller address -> callee, calls, inclusive cost).
"""

import os
import re
import sys
from collections import defaultdict


def parse_callgrind(path, executable, event="Ir", addresses=None):
    """Self cost per instruction address of the executable, and its call edges
    (address, caller, callee, calls, inclusive cost). Positions are `instr line`."""
    self_cost = defaultdict(int)
    calls = []
    names = {}
    objects = {}
    current_fn = None
    in_executable = False
    anonymous = False
    unknown_cost = defaultdict(int)
    unknown_ir = defaultdict(int)
    unknown_calls = []
    pending_call = None
    last_address = 0
    event_index = 0

    def resolve(table, rest):
        match = re.match(r"\((\d+)\)(?: (.*))?", rest)
        if not match:
            return rest
        if match.group(2):
            table[match.group(1)] = match.group(2)
        return table.get(match.group(1), rest)

    with open(path) as file:
        for line in file:
            line = line.rstrip("\n")
            if not line:
                continue
            if line.startswith("events: "):
                event_index = line.split()[1:].index(event)
                continue
            if line.startswith("ob="):
                object_name = resolve(objects, line[3:])
                in_executable = object_name.endswith(executable)
                anonymous = object_name == "???"
                continue
            if line.startswith("cob="):
                resolve(objects, line[4:])
                continue
            if line.startswith("fn="):
                current_fn = resolve(names, line[3:])
                continue
            if line.startswith("cfn="):
                pending_call = [resolve(names, line[4:]), 0]
                continue
            if line.startswith("calls="):
                pending_call[1] = int(line.split("=")[1].split()[0])
                continue
            if not (line[0] in "+-*" or line.startswith("0x")):
                continue
            fields = line.split()
            token = fields[0]
            if token.startswith("0x"):
                address = int(token, 16)
            elif token[0] in "+-":
                delta = int(token[1:], 0)
                address = last_address + delta if token[0] == "+" else last_address - delta
            else:
                address = last_address
            last_address = address
            cost = int(fields[2 + event_index]) if len(fields) > 2 + event_index else 0
            if pending_call is not None:
                if in_executable or anonymous:
                    destination = calls if in_executable else unknown_calls
                    destination.append((address, current_fn, pending_call[0], pending_call[1], cost))
                pending_call = None
            elif in_executable:
                self_cost[address] += cost
            elif anonymous:
                unknown_cost[address] += cost
                unknown_ir[address] += int(fields[2]) if len(fields) > 2 else 0
    # Valgrind can discard the main ELF's object/symbol names while retaining
    # absolute PCs. Recover only an anonymous object whose instruction-weighted
    # PCs overwhelmingly match the saved executable's disassembly at one base.
    if addresses and unknown_ir:
        candidates = (0, 0x108000, 0x555555554000, 0x400000)
        scores = {offset: sum(cost for address, cost in unknown_ir.items()
                              if address - offset in addresses) for offset in candidates}
        offset = max(scores, key=scores.get)
        if scores[offset] >= .90 * sum(unknown_ir.values()):
            for address, cost in unknown_cost.items():
                if address - offset in addresses:
                    self_cost[address] += cost
            calls.extend(call for call in unknown_calls if call[0] - offset in addresses)
    return self_cost, calls


def parse_objdump(path):
    instructions = {}
    function_of = {}
    current = None
    with open(path) as file:
        for line in file:
            header = re.match(r"^([0-9a-f]+) <(.*)>:", line)
            if header:
                current = header.group(2)
                continue
            match = re.match(r"^\s+([0-9a-f]+):\s+(.*)$", line)
            if match:
                address = int(match.group(1), 16)
                instructions[address] = match.group(2).strip()
                function_of[address] = current
    return instructions, function_of


def main():
    arguments = sys.argv[1:]
    top = 40
    min_share = 0.002
    show_calls = False
    event = "Ir"
    if "--event" in arguments:
        event = arguments[arguments.index("--event") + 1]
    if "--top" in arguments:
        top = int(arguments[arguments.index("--top") + 1])
    if "--min-share" in arguments:
        min_share = float(arguments[arguments.index("--min-share") + 1])
    if "--calls" in arguments:
        show_calls = True
    profile, disassembly = arguments[0], arguments[1]
    executable = os.path.abspath(disassembly)[:-2]  # main.s -> main
    instructions, function_of = parse_objdump(disassembly)
    self_cost, calls = parse_callgrind(profile, executable, event, set(instructions))
    total = sum(self_cost.values())
    # with PIE executables callgrind reports load addresses: find the offset that matches
    offset = 0
    if instructions and self_cost:
        hottest = max(self_cost, key=self_cost.get)
        if hottest not in instructions:
            low = min(instructions)
            for candidate in (0x555555554000, 0x108000, 0x400000):
                if hottest - candidate in instructions:
                    offset = candidate
                    break
            else:
                offset = (hottest & ~0xFFF) - (low & ~0xFFF) if hottest > low else 0
    print(f"total self cost {total:,} ({event})")
    if not total:
        return
    by_function = defaultdict(int)
    for address, cost in self_cost.items():
        by_function[function_of.get(address - offset, "?")] += cost
    for name, cost in sorted(by_function.items(), key=lambda item: -item[1])[:12]:
        print(f"  {cost / total:6.1%} {cost:>14,} {name}")
    # runs of consecutive instructions, each with at least min_share of the total
    addresses = sorted(instructions)
    runs = []
    run = []
    for address in addresses:
        cost = self_cost.get(address + offset, 0)
        if cost >= total * min_share / 20:
            run.append(address)
        elif run:
            runs.append(run)
            run = []
    if run:
        runs.append(run)
    runs = [r for r in runs if sum(self_cost.get(a + offset, 0) for a in r) >= total * min_share]
    runs.sort(key=lambda r: -sum(self_cost.get(a + offset, 0) for a in r))
    for r in runs[:top]:
        cost = sum(self_cost.get(a + offset, 0) for a in r)
        print(f"\n== {cost / total:6.2%} {cost:,} in {function_of.get(r[0])} at {r[0]:#x}")
        for address in r:
            print(f"{self_cost.get(address + offset, 0):>12,}  {address:#8x}  {instructions[address]}")
    if show_calls:
        edges = defaultdict(lambda: [0, 0])
        for address, caller, callee, count, cost in calls:
            edges[(caller, callee)][0] += count
            edges[(caller, callee)][1] += cost
        print("\ncalls (caller -> callee: count, inclusive cost)")
        def named(function):
            match = re.match(r"^0x([0-9a-fA-F]+)(?:'\d+)?$", function or "")
            return function_of.get(int(match[1], 16) - offset, function) if match else function

        for (caller, callee), (count, cost) in sorted(edges.items(), key=lambda item: -item[1][1])[:25]:
            caller, callee = named(caller), named(callee)
            print(f"  {caller} -> {callee}: {count:,} calls, {cost:,} ({cost / total:.1%})")


if __name__ == "__main__":
    main()
