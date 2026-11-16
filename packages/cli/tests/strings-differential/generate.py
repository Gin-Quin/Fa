#!/usr/bin/env python3
"""Deterministic Fa/JavaScript differential corpus (no compiler in the oracle).

Run: python3 generate.py OUTPUT_DIRECTORY [BUN]
The regular CLI program test invokes this generator. See README.md for coverage.
"""

import json
from pathlib import Path
import random
import subprocess
import sys


SEED = 0xFA1616


def literal(text):
    """Fa literals cannot spell surrogates; obtain those by cutting a scalar."""
    parts = []
    current = ""
    for char in text:
        code = ord(char)
        if 0xD800 <= code <= 0xDFFF:
            if current:
                parts.append(quoted(current))
                current = ""
            scalar = 0x10000 + ((code - 0xD800) << 10) if code < 0xDC00 else 0x10000 + code - 0xDC00
            parts.append(f'"\\u{{{scalar:X}}}"[{0 if code < 0xDC00 else 1}]')
        else:
            current += char
    if current or not parts:
        parts.append(quoted(current))
    return " + ".join(parts)


def quoted(text):
    result = '"'
    for char in text:
        result += {"\"": '\\"', "\\": "\\\\", "{": "{{", "}": "}}"}.get(char, f"\\u{{{ord(char):X}}}" if ord(char) < 32 or ord(char) > 126 else char)
    return result + '"'


PRELUDE = r'''
function units = (text: String): String => {
    mutable result: Array<String> = []
    for point in text {
        for index in 0..point.length { result.push(point.codeUnitAt(index).toString() catch _ => "unexpected-index-error") }
    }
    result.join(",")
}
function strings = (values: Array<String>): String => {
    mutable result: Array<String> = []
    for value in values { result.push("[" + units(value) + "]") }
    result.join(",")
}
function probe = (text: String, needle: String): String throws IndexError => {
    mutable rows: Array<String> = []
    rows.push("{text.length}:{text.isEmpty()}:{text.isWellFormed()}:{text.utf8Length()}")
    rows.push(units(text.toString()))
    for index in [-text.length - 1, -text.length, -1, 0, 1, text.length - 1, text.length] {
        let item = text.get(index)
        rows.push(if item is none { "none" } else { units(item) })
        rows.push(text.codeUnitAt(index).toString() catch _ => "error")
        rows.push(units(text[index]) catch _ => "error")
    }
    for start in [0, text.length // 2, text.length] {
        for end in [0, text.length // 2, text.length] {
            rows.push(units(try text[start..end]))
        }
    }
    rows.push(units(text[..]))
    rows.push(units(try text[-text.length..]))
    rows.push(units(try text[..-text.length]))
    if text.length > 0 {
        rows.push(units(text[0..=-1]))
        rows.push(units(text[-1..]))
    }
    for range in [-text.length-1..0, 0..text.length+1, text.length..text.length, -1..0, -text.length..=-text.length, 0..=text.length, text.length..=0, 0..=-1] {
        rows.push(units(text[range]) catch _ => "error")
    }
    mutable points: Array<String> = []
    for point, index in text { points.push("{index}:[{units(point)}]") }
    rows.push(points.join(","))
    rows.push("{text.codePoints()}")
    rows.push(text.codePoints().length.toString())
    rows.push(strings(text.characters()))
    rows.push("{text == needle}:{text != needle}:{text < needle}:{text <= needle}:{text > needle}:{text >= needle}")
    rows.push(units(text + needle))
    mutable appended = text
    appended += needle
    rows.push(units(appended))
    mutable doubled = text
    doubled += doubled
    rows.push(units(doubled))
    rows.push("{text.includes(needle)}:{text.startsWith(needle)}:{text.endsWith(needle)}:{text.indexOf(needle) ?? -1}:{text.lastIndexOf(needle) ?? -1}")
    rows.push(strings(text.split(needle)))
    rows.push(units(text.split(needle).join(needle)))
    rows.push(units(text.replace(needle, "$&")))
    rows.push(units(text.replaceAll(needle, "$&")))
    rows.push(units(text.replace(needle, needle)))
    rows.push(units(text.replaceAll(needle, needle)))
    for width in [-1, 0, text.length, text.length + 1, text.length + 3] {
        rows.push(units(text.padStart(width, needle)))
        rows.push(units(text.padEnd(width, needle)))
    }
    rows.push(units(text.repeat(0)))
    rows.push(units(text.repeat(2)))
    rows.push(units(text.trim()))
    rows.push(units(text.trimStart()))
    rows.push(units(text.trimEnd()))
    rows.push(units(text.toUpperCase()))
    rows.push(units(text.toLowerCase()))
    rows.push(units("<{text}>"))
    let bytes = text.toUtf8()
    rows.push("{bytes.length}:{bytes.isEmpty()}:{bytes.toString()}:{bytes == Bytes.from(bytes.toArray())}")
    rows.push(units(String.fromUtf8(bytes) catch _ => "UNEXPECTED"))
    rows.push(units(String.fromUtf8Lossy(bytes)))
    mutable encoded: Array<Byte> = []
    for byte in text.toUtf8() { encoded.push(byte) }
    rows.push("{encoded}")
    for index in [-bytes.length - 1, -1, 0, bytes.length] {
        let byte = bytes.get(index)
        rows.push(if byte is none { "999" } else { byte.toString() })
        rows.push(bytes[index].toString() catch _ => "error")
    }
    rows.push(bytes[..].toString())
    rows.push((try bytes[bytes.length..0]).toString())
    rows.push((try bytes[-bytes.length..]).toString())
    if bytes.length > 0 { rows.push(bytes[0..=-1].toString()) }
    rows.join("|")
}
'''

REPRESENTATIONS = r'''
type Stored = { text: String }
type Payload = Union { Leaf = { text: String }, Branch = { left: Payload, right: Payload } }
function ownerProbe = (text: String, needle: String): String throws IndexError => try probe(String.from(text, capacity = 8192), needle)
function viewProbe = (text: String, needle: String): String throws IndexError => {
    let source = "^" + text + "!"
    use view = try source[1..-1]
    try probe(view, needle)
}
function splitProbe = (text: String, needle: String): String throws IndexError => {
    mutable result = ""
    for part in text.split("\u{10FFFF}\u{10FFFF}") { result = try probe(part, needle) }
    result
}
function fieldProbe = (text: String, needle: String): String throws IndexError => {
    let object = Stored { text = text }
    try probe(object.text, needle)
}
function arrayProbe = (text: String, needle: String): String throws IndexError => {
    let items = [text]
    try probe(items[0], needle)
}
function poolProbe = (text: String, needle: String): String throws IndexError => {
    mutable pool: Pool<Stored> = []
    use item = pool.add(Stored { text = text })
    try probe(item.text, needle)
}
function payloadProbe = (node: Payload, needle: String): String throws IndexError => when node is {
    Payload.Leaf as leaf => try probe(leaf.text, needle)
    Payload.Branch as branch => try payloadProbe(branch.left, needle)
}
function compactProbe = (text: String, needle: String): String throws IndexError => {
    let tree = Payload.Branch { left = Payload.Leaf { text = text }, right = Payload.Leaf { text = text } }
    try payloadProbe(tree, needle)
}
function frameProbe = (text: String, needle: String): String throws IndexError => {
    mutable built = ""
    for unit in text.split("") { built += unit }
    try probe(built, needle)
}
function reservedProbe = (text: String, needle: String): String throws IndexError => {
    mutable built = String.from("", capacity = 8192)
    for unit in text.split("") { built += unit }
    try probe(built, needle)
}
'''

ORACLE = r'''
const data = JSON.parse(await Bun.stdin.text());
const enc = new TextEncoder();
const decoder = new TextDecoder("utf-8", {ignoreBOM:true});
const fatal = new TextDecoder("utf-8", {ignoreBOM:true, fatal:true});
const segmenter = new Intl.Segmenter("und", {granularity:"grapheme"});
const units = s => Array.from({length:s.length}, (_,i) => s.charCodeAt(i)).join(",");
const strings = a => a.map(s => "["+units(s)+"]").join(",");
const array = a => "[" + Array.from(a).join(", ") + "]";
function probe(text, needle) {
    const rows = [];
    rows.push(`${text.length}:${text.length===0}:${text.isWellFormed()}:${enc.encode(text).length}`);
    rows.push(units(text));
    for (let index of [-text.length-1,-text.length,-1,0,1,text.length-1,text.length]) {
        const i = index < 0 ? text.length+index : index;
        const valid = i>=0 && i<text.length;
        rows.push(valid ? units(text[i]) : "none");
        rows.push(valid ? String(text.charCodeAt(i)) : "error");
        rows.push(valid ? units(text[i]) : "error");
    }
    for (const start of [0,Math.floor(text.length/2),text.length])
        for (const end of [0,Math.floor(text.length/2),text.length]) rows.push(units(text.slice(start,end)));
    rows.push(units(text), units(text), "");
    if (text.length>0) rows.push(units(text),units(text.slice(-1)));
    for(const [a,b,inclusive] of [[-text.length-1,0,false],[0,text.length+1,false],[text.length,text.length,false],[-1,0,false],[-text.length,-text.length,true],[0,text.length,true],[text.length,0,true],[0,-1,true]]) {
        const valid=a>=-text.length&&a<=text.length&&b>=-text.length&&b<=text.length-(inclusive?1:0);
        const start=a<0?a+text.length:a;
        const end=(b<0?b+text.length:b)+(inclusive?1:0);
        rows.push(valid?units(text.slice(start,Math.max(start,end))):"error");
    }
    let index=0; const points=[];
    for (const point of text) { points.push(`${index}:[${units(point)}]`); index+=point.length; }
    rows.push(points.join(","), array(Array.from(text,s=>s.codePointAt(0))), String(Array.from(text).length));
    rows.push(strings(Array.from(segmenter.segment(text),x=>x.segment)));
    rows.push(`${text===needle}:${text!==needle}:${text<needle}:${text<=needle}:${text>needle}:${text>=needle}`);
    rows.push(units(text+needle),units(text+needle),units(text+text));
    rows.push(`${text.includes(needle)}:${text.startsWith(needle)}:${text.endsWith(needle)}:${text.indexOf(needle)}:${text.lastIndexOf(needle)}`);
    rows.push(strings(text.split(needle)),units(text.split(needle).join(needle)));
    rows.push(units(text.replace(needle,()=>"$&")),units(text.replaceAll(needle,()=>"$&")));
    rows.push(units(text.replace(needle,()=>needle)),units(text.replaceAll(needle,()=>needle)));
    for (const width of [-1,0,text.length,text.length+1,text.length+3]) rows.push(units(text.padStart(Math.max(width,0),needle)),units(text.padEnd(Math.max(width,0),needle)));
    rows.push(units(text.repeat(0)),units(text.repeat(2)),units(text.trim()),units(text.trimStart()),units(text.trimEnd()),units(text.toUpperCase()),units(text.toLowerCase()),units(`<${text}>`));
    const bytes=enc.encode(text);
    rows.push(`${bytes.length}:${bytes.length===0}:${array(bytes)}:true`,units(fatal.decode(bytes)),units(decoder.decode(bytes)),array(bytes));
    for (const index of [-bytes.length-1,-1,0,bytes.length]) {
        const i=index<0?bytes.length+index:index;
        const valid=i>=0&&i<bytes.length;
        rows.push(valid?String(bytes[i]):"999",valid?String(bytes[i]):"error");
    }
    rows.push(array(bytes),"[]",array(bytes));
    if (bytes.length>0) rows.push(array(bytes));
    return rows.join("|");
}
for (const [text,needle] of data) console.log(probe(text,needle));
'''

STORAGE = r'''
type Box = { text: String }
type Tree = Union { Leaf = { text: String }, Branch = { left: Tree, right: Tree } }
function shape = (text: String): String => "{text.length}:{text.utf8Length()}:{text.isWellFormed()}:[{units(text)}]"
function treeShape = (tree: Tree): String => when tree is {
    Tree.Leaf as leaf => shape(leaf.text)
    Tree.Branch as branch => treeShape(branch.left) + ";" + treeShape(branch.right)
}
function storage = (text: String): String throws IndexError => {
    mutable rows: Array<String> = []
    rows.push(shape(text))
    let owner = String.from(text, capacity = 8192)
    rows.push(shape(owner))
    let container = "^" + text + "!"
    use view = try container[1..-1]
    rows.push(shape(view))
    for part in text.split("\u{10FFFF}\u{10FFFF}") { rows.push(shape(part)) }
    let box = Box { text = text }
    rows.push(shape(box.text))
    let array = [text]
    rows.push(shape(array[0]))
    mutable pool: Pool<Box> = []
    use item = pool.add(Box { text = text })
    rows.push(shape(item.text))
    let tree = Tree.Branch { left = Tree.Leaf { text = text }, right = Tree.Leaf { text = text } }
    rows.push(treeShape(tree))
    mutable packed: Array<Tree> = []
    packed.push(Tree.Leaf { text = text })
    for node in packed { rows.push(treeShape(node)) }
    mutable frame = ""
    mutable reserved = String.from("", capacity = 8192)
    reserved.reserve(text.utf8Length())
    mutable lengths: Array<String> = []
    for unit in text.split("") {
        frame += unit
        reserved += unit
        lengths.push("{frame.length}:{reserved.length}")
        if frame.length > 64 {
            lengths.push(frame.codeUnitAt(63).toString())
            lengths.push((try reserved.codeUnitAt(64)).toString())
        }
    }
    rows.push(shape(frame))
    rows.push(shape(reserved))
    rows.push(lengths.join(","))
    let map: Map<String, Integer> = [(text, 1), (view, 2)]
    let set: Set<String> = [text, view, owner]
    rows.push("{map.size}:{map.get(owner) ?? 0}:{set.size}:{set.includes(owner)}")
    mutable sorted = [text, "\u{FFFF}", "\u{10000}", "", "a", "😀"[0], "😀"[1]]
    rows.push(strings(sorted.sort()))
    rows.join("|")
}
'''


def write_cases(output, name, prelude, cases, references, function, shard_size=12, compile_time=True):
    for shard in range(0, len(cases), shard_size):
        source = prelude
        for index in range(shard, min(shard + shard_size, len(cases))):
            args = ", ".join(cases[index])
            if compile_time:
                source += f'\nstatic result{index} = @{function}({args})\n'
                source += f'console.log("{index} static " + result{index})\n-- prints: {index} static {references[index]}\n'
            runtime_function = ["probe","ownerProbe","viewProbe","splitProbe","fieldProbe","arrayProbe","poolProbe","compactProbe","frameProbe","reservedProbe"][index % 10] if function == "probe" else function
            source += f'console.log("{index} runtime " + (try {runtime_function}({args})))\n-- prints: {index} runtime {references[index]}\n'
        (output / f"{name}-{shard // shard_size:02}.fa").write_text(source)


def byte_cases(output, bun):
    invalid = [[0x80], [0xBF], [0xC0,0xAF], [0xC1,0xBF], [0xC2], [0xDF],
               [0xE0,0x80,0xAF], [0xED,0xA0,0x80], [0xED,0xBF,0xBF],
               [0xE1], [0xE2,0x82], [0xF0], [0xF0,0x9F], [0xF0,0x9F,0x92],
               [0xF0,0x80,0x80,0xAF], [0xF4,0x90,0x80,0x80], [0xF5,0x80,0x80,0x80],
               [0xF8,0x88,0x80,0x80,0x80], [0xFC], [0xFE], [0xFF], [0xE1,0x80,0x41], [0xC2,0x41], [0xF0,0x9F,0x41], [0xF0,0x9F,0x92,0x41]]
    inputs = [[0x61] * prefix + value for prefix in [0, 1, 15, 16, 31, 32] for value in invalid]
    inputs += [[], [0], [0xEF,0xBB,0xBF], [0xF4,0x8F,0xBF,0xBF], [0xED,0x9F,0xBF], [0xE0,0xA0,0x80]]
    oracle = r'''
const inputs=JSON.parse(await Bun.stdin.text());
const decoder=new TextDecoder("utf-8",{ignoreBOM:true});
const units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i)).join(",");
for(const bytes of inputs) console.log(units(decoder.decode(new Uint8Array(bytes))));
'''
    lossy = subprocess.run([bun,"-e",oracle], input=json.dumps(inputs),text=True,capture_output=True,check=True).stdout.splitlines()
    references = []
    for value, text in zip(inputs, lossy):
        try:
            value_text = bytes(value).decode("utf-8")
            strict = ",".join(str(int.from_bytes(value_text.encode("utf-16-le")[i:i+2],"little")) for i in range(0,len(value_text.encode("utf-16-le")),2))
        except UnicodeDecodeError as error:
            strict = f"error:{error.start}"
        references.append(text + "|" + strict)
    prelude = PRELUDE.split("function probe",1)[0] + r'''
function decode = (bytes: Bytes): String => {
    let lossy = units(String.fromUtf8Lossy(bytes))
    let strict = units(String.fromUtf8(bytes)) catch error => "error:{error.position}"
    lossy + "|" + strict
}
'''
    write_cases(output,"decode",prelude,[("Bytes.from(["+",".join(map(str,value))+"])",) for value in inputs],references,"decode",32)
    buffers=inputs[::6]+[[0x9F,0x98], [0xF0,0x9F,0x98], list(range(256))]
    buffer_prelude=PRELUDE.split("function probe",1)[0]+r'''
function buffer = (data: Bytes): String throws IndexError => {
    mutable original = copy data
    let previous = copy original
    mutable use view = original[..]
    if view.length > 0 { view[0] = 255 }
    let changed = original.toString()
    reuse view = try original[original.length..0]
    let empty = view.toString()
    mutable rows = [changed, empty, previous.toString(), "{previous == data}", Bytes.filled(data.length, 17).toString(), Bytes.filled(0).toString()]
    rows.push("{original == previous}:{Bytes.filled(data.length, 17) == data}:{Bytes.filled(data.length + 1, 17) == data}:{data.isEmpty()}:{data.toArray().length}")
    for range in [-data.length-1..0, 0..data.length+1, data.length..data.length, -1..0, -data.length..=-data.length, 0..=data.length, data.length..=0, 0..=-1] {
        rows.push(data[range].toString() catch _ => "error")
    }
    rows.join("|")
}
'''
    buffer_reference=[]
    for data in buffers:
        changed=data.copy()
        if changed:changed[0]=255
        array=lambda value:"["+", ".join(map(str,value))+"]"
        rows=[array(changed),"[]",array(data),"true",array([17]*len(data)),"[]"]
        rows.append(f"{str(changed==data).lower()}:{str([17]*len(data)==data).lower()}:false:{str(not data).lower()}:{len(data)}")
        n=len(data)
        for a,b,inclusive in [(-n-1,0,False),(0,n+1,False),(n,n,False),(-1,0,False),(-n,-n,True),(0,n,True),(n,0,True),(0,-1,True)]:
            valid=-n<=a<=n and -n<=b<=n-int(inclusive)
            start=a+n if a<0 else a
            end=(b+n if b<0 else b)+int(inclusive)
            rows.append(array(data[start:max(start,end)]) if valid else "error")
        buffer_reference.append("|".join(rows))
    write_cases(output,"bytes-storage",buffer_prelude,[("Bytes.from(["+",".join(map(str,data))+"])",) for data in buffers],buffer_reference,"buffer",32)
    return len(inputs)


def json_cases(output,bun):
    values=subjects()
    oracle=r'''
const values=JSON.parse(await Bun.stdin.text());
const units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i)).join(",");
for(const value of values) { const json=JSON.stringify(value); console.log(units(json)+"|"+units(JSON.parse(json))+"|"+units(JSON.parse(json))); }
'''
    reference=subprocess.run([bun,"-e",oracle],input=json.dumps(values,ensure_ascii=True),text=True,capture_output=True,check=True).stdout.splitlines()
    prelude='-- skip-target: native-freestanding\nuse { Json, JsonError } = std.json\n'+PRELUDE.split("function probe",1)[0]+r'''
function jsonProbe = (text: String): String throws JsonError => {
    let json = Json.stringify(text)
    let parsed: String = try Json.parse(json)
    let parsedBytes: String = try Json.parse(json.toUtf8())
    units(json) + "|" + units(parsed) + "|" + units(parsedBytes)
}
function parsed = (json: String): String throws JsonError => {
    let text: String = try Json.parse(json)
    units(text)
}
'''
    write_cases(output,"json-runtime",prelude,[(literal(text),) for text in values],reference,"jsonProbe",32,False)
    # Keep this separately compilable: the known missing CT serializer must
    # not prevent the other independent operations from being exercised.
    write_cases(output,"json-comptime",prelude,[(literal(text),) for text in values[:16]],reference[:16],"jsonProbe",32)
    source=prelude
    for index,text in enumerate(values):
        document=json.dumps(text,ensure_ascii=True,separators=(",",":"))
        encoded=text.encode("utf-16-le",errors="surrogatepass")
        want=",".join(str(int.from_bytes(encoded[i:i+2],"little")) for i in range(0,len(encoded),2))
        source+=f'\nstatic parsed{index} = @parsed({literal(document)})\nconsole.log(parsed{index})\n-- prints: {want}\n'
    (output/"json-parse.fa").write_text(source)
    union=prelude+r'''
function unionParse = (input: String | Bytes): String throws JsonError => {
    let value: String = try Json.parse(input)
    units(value)
}
static fromText = @unionParse("\"é\"")
static fromBytes = @unionParse("\"😀\"".toUtf8())
console.log(fromText, fromBytes) -- prints: 233 55357,56832
console.log(try unionParse("\"é\""), try unionParse("\"😀\"".toUtf8())) -- prints: 233 55357,56832
'''
    (output/"json-union.fa").write_text(union)


def edge_cases(output,bun):
    cases=[]
    values=[]
    for prefix in [0,8,19,20,21,22,23,60,61,62,63,64,65,127,4095]:
        source="a"*prefix+"😀é日😀z"
        for start,end in [(prefix,prefix+1),(prefix+1,prefix+2),(prefix+1,prefix+5)]:
            encoded=source.encode("utf-16-le")
            text=encoded[start*2:end*2].decode("utf-16-le",errors="surrogatepass")
            needle="\ude00" if start%2 else "\ud83d"
            cases.append((literal(source),str(start),str(end),literal(needle)))
            values.append((text,needle))
    references=subprocess.run([bun,"-e",ORACLE],input=json.dumps(values,ensure_ascii=True),text=True,capture_output=True,check=True).stdout.splitlines()
    references=[value+"|1:2:1:true" for value in references]
    prelude=PRELUDE+r'''
function cutProbe = (source: String, start: Integer, end: Integer, needle: String): String throws IndexError => {
    use part = try source[start..end]
    let owner = String.from(part)
    let map: Map<String, Integer> = [(part, 1), (owner, 2)]
    let set: Set<String> = [part, owner]
    (try probe(part, needle)) + "|{map.size}:{map.get(part) ?? 0}:{set.size}:{set.includes(part)}"
}
'''
    write_cases(output,"cut-views",prelude,cases,references,"cutProbe",32)
    return len(cases)


def subjects():
    values = ["", "a", "abc", "aaa", ",a,,", "é", "日", "😀", "é日😀a", "\ufffd", "\ufeff", "\0", "e\u0301", "👩🏽\u200d💻", "🇫🇷", "ΟΣ", "AΣ\u0301", "ßİıI", "\ufeff \t\r\na\u00a0\u3000", "\x85\u200ba\x85\u200b"]
    values += ["a" * n + "😀é日" for n in [8, 9, 10, 19, 20, 21, 22, 23, 24, 60, 61, 62, 63, 64, 65, 126, 127, 128]]
    values += ["é" * n + "😀z" for n in [10, 11, 30, 31, 32, 62, 63, 64, 65]]
    values += ["\ud83d", "\ude00", "\ud83dabc", "abc\ude00", "a\ud83db", "a\ude00b", "\ude00\ud83d", "\ud83d\ud83d", "\ude00\ude00"]
    values += ["\u007f", "\u0080", "\u07ff", "\u0800", "\uffff", "\U00010000", "\U0010ffff", "\ud800", "\udbff", "\udc00", "\udfff"]
    whitespace="".join(map(chr,[*range(9,14),32,160,5760,*range(8192,8203),8232,8233,8239,8287,12288,65279]))
    values += [whitespace+"x"+whitespace, "a" * 4097, "é日😀" * 700]
    return values


def main():
    output = Path(sys.argv[1])
    output.mkdir(parents=True, exist_ok=True)
    bun = sys.argv[2] if len(sys.argv) > 2 else str(Path.home() / ".bun/bin/bun")
    rng = random.Random(SEED)
    needles = ["", "a", "aa", ",", "é", "日", "😀", "\ud83d", "\ude00", "\ude00\ud83d", "\uffff", "\0", "$&", "longer-than-most-inputs"]
    cases = []
    for index, text in enumerate(subjects()):
        # Every subject meets empty, surrogate-edge, and sampled ordinary needles.
        for needle in ["", "\ud83d", "\ude00", needles[rng.randrange(len(needles))]]:
            cases.append((text, needle))
    # The decided edges that the sampled needles may miss (points 1 to 4).
    cases += [("aaa", "aa"), ("aaaa", "aa"), ("", ","), ("", ""), ("x", "😀"), ("😀", ""),
              ("a,b,,c", ","), ("abc", "abcd"), ("ab", "longer-than-most-inputs"), ("😀😀", "\ude00\ud83d")]
    reference = subprocess.run([bun, "-e", ORACLE], input=json.dumps(cases, ensure_ascii=True), text=True, capture_output=True, check=True).stdout.splitlines()
    write_cases(output,"semantic",PRELUDE+REPRESENTATIONS,[(literal(text),literal(needle)) for text,needle in cases],reference,"probe",32)
    storage_oracle = r'''
const subjects=JSON.parse(await Bun.stdin.text());
const enc=new TextEncoder();
const units=s=>Array.from({length:s.length},(_,i)=>s.charCodeAt(i)).join(",");
const shape=s=>`${s.length}:${enc.encode(s).length}:${s.isWellFormed()}:[${units(s)}]`;
for(const text of subjects) {
    const value=shape(text); const rows=Array(7).fill(value);
    rows.push(value+";"+value,value,value,value);
    const lengths=[];
    for(let n=1;n<=text.length;n++) {
        lengths.push(`${n}:${n}`);
        if(n>64) lengths.push(String(text.charCodeAt(63)),String(text.charCodeAt(64)));
    }
    rows.push(lengths.join(","),"1:2:1:true");
    rows.push([text,"\uffff","\u{10000}","","a","😀"[0],"😀"[1]].sort().map(s=>"["+units(s)+"]").join(","));
    console.log(rows.join("|"));
}
'''
    storage_reference = subprocess.run([bun,"-e",storage_oracle],input=json.dumps(subjects(),ensure_ascii=True),text=True,capture_output=True,check=True).stdout.splitlines()
    prelude = PRELUDE.split("function probe",1)[0] + STORAGE
    prelude += r'''
use { sleep } = std.time
async function crossing = (text: String): String throws IndexError => {
    let held = String.from(text, capacity = 8192)
    await sleep(0)
    try storage(held)
}
'''
    write_cases(output,"storage",prelude,[(literal(text),) for text in subjects()],storage_reference,"storage",12,False)
    for shard in range(0,len(subjects()),12):
        path=output / f"storage-{shard//12:02}.fa"
        with path.open("a") as source:
            for index in range(shard,min(shard+12,len(subjects()))):
                source.write(f'console.log("{index} await " + (try await crossing({literal(subjects()[index])})))\n-- prints: {index} await {storage_reference[index]}\n')
    decoded = byte_cases(output,bun)
    json_cases(output,bun)
    cuts=edge_cases(output,bun)
    (output / "manifest.json").write_text(json.dumps({"seed": SEED, "subjects": len(subjects()), "pairs": len(cases), "decoders":decoded,"cut_views":cuts,"oracle": "Bun direct JavaScript"}, indent=2) + "\n")


if __name__ == "__main__":
    main()
