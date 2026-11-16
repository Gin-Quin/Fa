#!/usr/bin/env python3
"""Deterministic InlineArray matrix. Rust's inline_array_matrix test runs it.

Every item/position pair executes the whole operation sequence, except unique
copy/key cells, which are emitted separately as diagnostic tests. No random seed,
expected-output literals or compiler invocation lives in the generator: @run is
the interpreter oracle; the harness checks Bun against it before checking LLVM.
"""
import argparse
import re
from pathlib import Path


def kinds():
    for sign in ('Int', 'UInt'):
        for width in (8, 16, 32, 64, 128):
            yield f'{sign.lower()}{width}', f'{sign}{width}', '7', '', False
    yield 'integer', 'Integer', '7', '', False
    for width in (32, 64):
        yield f'float{width}', f'Float{width}', '1.5', '', False
    yield 'boolean', 'Boolean', 'true', '', False
    yield 'short_string', 'String', '"short"', '', False
    yield 'long_string', 'String', '"long ".repeat(12)', '', False
    yield 'enum', 'Color', 'Color.Red', 'type Color = Enum { Red, Blue }', False
    yield 'object', 'Data', 'Data { value = "object" }', 'type Data = { value: String }', False
    yield 'structure', 'Data', 'Data { value = ["owned"] }', 'type Data = { value: Array<String> }', False
    yield 'tuple', '(String, Int16)', '("tuple", 7)', '', False
    yield 'optional', 'String?', '"optional"', '', False
    yield 'anonymous_union', 'String | Integer', '"union"', '', False
    yield 'anonymous_union_owned', 'String | { value: Array<String> }', '{ value = ["owned"] }', '', False
    yield 'named_union', 'Value', 'Value.Data { text = "payload", number = 7 }', 'type Value = Union { Empty, Data = { text: String, number: Int16 } }', False
    yield 'named_union_owned', 'Value', 'Value.Data { value = ["owned"] }', 'type Value = Union { Empty, Data = { value: Array<String> } }', False
    yield 'array', 'Array<String>', '["array"]', '', False
    yield 'nested', 'InlineArray<String, 2>', '["nested"]', '', False
    yield 'unique', 'Resource', 'Resource { name = "unique" }', 'type Resource = { name: String, function destroy = (self) => { assert self.name == "unique"\nconsole.log("drop unique") } }', True
    yield 'unique_owned', 'Resource', 'Resource { values = ["long ".repeat(12)] }', 'type Resource = { values: InlineArray<String, 2>, function destroy = (self) mutates self => { assert self.values.length == 1\nself.values.clear()\nconsole.log("drop unique") } }', True


CORE = '''
type Item = TYPE
type List = InlineArray<Item, 3>
type Holder = { items: List = [] }
function seed = (second: Boolean = false): Item => if second { OTHER } else { SEED }
function read = (items: List): String => "{items}"
function exercise = <capacity: UInt32>(items: InlineArray<Item, capacity>): String
    throws CapacityError | IndexError mutates items => {
    mutable trace = "{items}"
    try items.push(seed())
    try items.push(seed(true))
    try items.push(seed())
    mutable full = false
    items.push(seed()) catch _error => { full = true }
    assert full
    trace += "|{items}"
    trace += "|{try items[0]}"
    try items[0] = seed(true)
    for item in items { trace += "|{item}" }
    for item in items { item = seed(true) }
    trace += "|{items}"
    trace += "|{items.pop()}"
    try items.delete(0)
    trace += "|{try items.take(0)}"
    assert items.length == 0
    try items.push(seed())
    items.clear()
    assert items.length == 0
    try items.push(seed())
    COPY
    trace + "|{items}"
}
'''


def positions(unique):
    yield 'local', 'mutable items: List = [seed()]\nitems.clear()\nlet trace = try exercise<3>(items)\n"{trace}|{items}"'
    yield 'object_field', 'mutable owner: Holder = {}\nlet trace = try exercise<3>(owner.items)\n"{trace}|{owner.items}"'
    for name, collection in [('pool_field', 'Pool<Holder>'), ('array_field', 'Array<Holder>')]:
        yield name, f'mutable owners: {collection} = [{{}}]\nmutable trace = ""\nfor owner in owners {{ trace = try exercise<3>(owner.items) }}\n"{{trace}}|{{owners}}"'
    yield 'parameter_read', 'mutable items: List = []\nlet trace = try exercise<3>(items)\n"{trace}|{read(items)}"'
    yield 'parameter_mutated', 'mutable items: List = []\nlet callback = exercise<3>\nlet trace = try callback(items)\n"{trace}|{items}"'
    yield 'result', 'let items = try build()\n"{items}"'
    yield 'capture', 'mutable items: List = []\nmutable trace = ""\ntry [0].map((_index) => { trace = try exercise<3>(items)\n0 })\n"{trace}|{items}"'
    yield 'collection_callback', 'mutable rows: Array<List> = [[]]\nlet traces = try rows.map((items) => { try exercise<3>(items) })\n"{try traces[0]}|{rows}"'
    yield 'generic_parameter', 'mutable items: List = []\nlet trace = try generic<Item>(items, exercise<3>)\n"{trace}|{items}"'
    yield 'await', 'mutable items: List = []\nlet trace = try exercise<3>(items)\nAWAIT\n"{trace}|{items}"'
    yield 'async_parameter', 'mutable items: List = []\nlet callback = async_exercise<3>\nlet trace = try await callback(items, true)\n"{trace}|{items}"'
    if not unique:
        yield 'map_key', 'let key = try build()\nlet map: Map<List, Integer> = [(copy key, 9)]\n"{map.get(key)}|{key}"'
    yield 'map_value', 'mutable map: Map<String, Holder> = [("key", {})]\nlet trace = try map.map((_key, owner) => { try exercise<3>(owner.items) })\n"{trace}|{map}"'


def generate_kind(directory, name, ty, seed, declarations, unique, synchronous=False):
    directory.mkdir(parents=True, exist_ok=True)
    other = ('9' if name.startswith(('int', 'uint')) else
             '2.5' if name.startswith('float') else
             'false' if name == 'boolean' else
             'Color.Blue' if name == 'enum' else
             'none' if name == 'optional' else
             '7' if name == 'anonymous_union' else
             '"other"' if name == 'anonymous_union_owned' else
             'Value.Empty' if name.startswith('named_union') else
             seed.replace('short', 'other').replace('long ', 'other ').replace('object', 'other').replace('owned', 'other').replace('tuple', 'other').replace('array', 'other').replace('nested', 'other'))
    copy = '' if unique else 'mutable duplicate = copy items\n    assert duplicate == items\n    try duplicate[0] = seed(true)\n    assert duplicate != items\n    let moved = duplicate\n    assert moved != items'
    source = declarations + '\n' + CORE.replace('TYPE', ty).replace('SEED', seed).replace('OTHER', other).replace('COPY', copy)
    source += '''
function build = (): List throws CapacityError | IndexError => {
mutable items: List = []
try exercise<3>(items)
items
}
function generic = <T>(items: InlineArray<T, 3>, callback: (items: InlineArray<T, 3>) => String throws CapacityError | IndexError mutates items): String
throws CapacityError | IndexError mutates items => { try callback(items) }
async function async_exercise = <capacity: UInt32>(items: InlineArray<Item, capacity>, pause: Boolean = true): String throws CapacityError | IndexError mutates items => {
if pause { await Task.yield() }
    try exercise<capacity>(items)
}
'''
    if synchronous:
        source = re.sub(r"async function async_exercise = .*?\n}\n", "", source, flags=re.S)
    # One file per kind bounds peak compiler memory. Each row has its own
    # compile-time oracle, so a stale mutable place cannot be masked by a
    # later reinitialization. Await is the same row plus one suspension.
    for position, body in positions(unique):
        if synchronous and position not in ("result", "parameter_mutated", "collection_callback"):
            continue
        function = f'row_{position}'
        # The reference executes the identical operation sequence on a
        # local, then constructs the enclosing value. It need not emulate
        # an async scheduler or the backend's physical parameter places.
        suffix = {
            'pool_field': '{[Holder { items = items }]}',
            'array_field': '{[Holder { items = items }]}',
            'collection_callback': '{[items]}',
        }.get(position, '{items}')
        oracle = f'mutable items: List = []\nlet trace = try exercise<3>(items)\n"{{trace}}|{suffix}"'
        if position in ('result', 'map_key'):
            oracle = body
        if position == 'map_value':
            oracle = 'mutable items: List = []\nlet trace = try exercise<3>(items)\nlet traces: Map<String, String> = [("key", trace)]\nlet map: Map<String, Holder> = [("key", { items = items })]\n"{traces}|{map}"'
        source += f'\nfunction oracle_{position} = (): String throws CapacityError | IndexError => {{\n{oracle}\n}}\n'
        prefix = 'async ' if position == 'async_parameter' else ''
        source += f'\n{prefix}function {function} = (): String throws CapacityError | IndexError => {{\n{body.replace("AWAIT", "")}\n}}\n'
        source += f'static expected_{position} = @oracle_{position}()\n'
        source += f'console.log("{position}", expected_{position})\n'
        if position == 'await':
            source += f'async function suspended = (): String throws CapacityError | IndexError => {{\n{body.replace("AWAIT", "await Task.yield()")}\n}}\n'
            source += 'console.log("await", try await suspended())\n'
        else:
            await_call = 'await ' if prefix else ''
            source += f'console.log("{position}", try {await_call}{function}())\n'
    if unique:
        # Ten Resource constructions per exercise, including the rejected
        # push; local also constructs and clears its literal before exercise.
        rows = 3 if synchronous else len(list(positions(unique)))
        source += f'\n-- destroys: {rows * 10 + (0 if synchronous else 1)}\n'
    (directory / f'{"sync_" if synchronous else ""}{name}.foreign.fa').write_text(source)
    if synchronous:
        return
    negative = declarations + f'\ntype Item = {ty}\nmutable nodes: GrowingPool<Item> = []\nlet invalid: InlineArray<Item in nodes, 3> = []\n-- error: REF010\n'
    (directory / f'{name}.reference.reject.fa').write_text(negative)
    if unique:
        negative = declarations + f'\nlet items: InlineArray<{ty}, 3> = [{seed}]\nlet invalid = copy items\n-- error: MEM003\n'
        (directory / f'{name}.copy.reject.foreign.fa').write_text(negative)
        negative = declarations + f'\nfunction invalid = (items: InlineArray<{ty}, 3>): InlineArray<{ty}, 3> => items\n-- error: MEM002\n'
        (directory / f'{name}.borrowed_move.reject.foreign.fa').write_text(negative)
        for label, expression, code in [('equality', 'assert items == items', 'E-CMP-4'), ('key', f'let invalid: Map<InlineArray<{ty}, 3>, Integer> = []', 'CL020')]:
            negative = declarations + f'\nlet items: InlineArray<{ty}, 3> = [{seed}]\n{expression}\n-- error: {code}\n'
            (directory / f'{name}.{label}.reject.foreign.fa').write_text(negative)


def generate(directory):
    for kind in kinds():
        generate_kind(directory, *kind)
        if kind[0] in ('integer', 'long_string', 'object', 'structure', 'named_union', 'named_union_owned', 'anonymous_union_owned', 'nested', 'unique', 'unique_owned'):
            generate_kind(directory, *kind, synchronous=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    generate(parser.parse_args().directory)
