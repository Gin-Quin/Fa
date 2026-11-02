// String building (Goose port of main.rs): formatting, appending to a growing
// string, zero padding, split, join, length.

import std;
import os;

let BLOCK_SIZE = 1000;
let LABELS: const u8[:][5] = ["alpha", "beta", "gamma", "delta", "epsilon"];

struct Totals { lines: i64, characters: i64, joined_characters: i64 }

fn add(totals: Totals&, text: u8[:]) {
    let parts = split(text, ';');
    totals.lines += parts.len;
    totals.characters += text.len;
    var joined: u8[>..] = [];
    join(joined, parts, "\n");
    totals.joined_characters += joined.len;
}

fn main() {
    let a = args();
    var n = 3000000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    var totals = Totals { 0, 0, 0 };
    var output: u8[>..] = [];
    var last_entry: u8[>..] = [];
    for i in n {
        let label = LABELS[i % 5];
        last_entry.clear();
        format_int(last_entry, i, 10, 8, '0');
        format(last_entry, " ", label, " x", i % 97, " = ", i * 3 + 1);
        format(output, last_entry, ";");
        if i % BLOCK_SIZE == BLOCK_SIZE - 1 {
            add(totals, output);
            output.clear();
        }
    }
    if output.len > 0 { add(totals, output); }
    print("last entry: ", last_entry);
    print("lines: ", totals.lines, ", characters: ", totals.characters,
          ", joined characters: ", totals.joined_characters);
}
