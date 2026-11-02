// Sorting (Goose port of main.rs): an array of records with a three-key
// comparator, then an array of integers. The names are two syllables, so
// they are inline `u8[..8]` strings and a record is plain data.

import std;
import os;

let MODULUS = 1000000007;
let SYLLABLES: const u8[:][16] = ["ka", "lo", "mi", "nu", "pe", "ra", "si", "to",
                                  "va", "ze", "bo", "di", "fu", "ga", "he", "jo"];

// The "minimal standard" Park-Miller generator.
struct Random { state: i64 }

fn next(random: Random&, bound: i64) -> i64 {
    random.state = random.state * 48271 % 2147483647;
    return random.state % bound;
}

struct Record { id: i64, name: u8[..8], score: i64 }

fn main() {
    let a = args();
    var n = 1000000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    var random = Random { 7 };

    var records: Record[>..] = [];
    for id in n {
        var name: u8[..8] = [];
        name.append(SYLLABLES[random.next(16)]);
        name.append(SYLLABLES[random.next(16)]);
        let score = random.next(1000);
        records.push(Record { id, name, score });
    }
    sort(records) { a, b =>
        if a.score != b.score { a.score > b.score }
        else {
            let c = compare(a.name, b.name);
            if c != 0 { c < 0 } else { a.id < b.id }
        }
    };

    var record_checksum = 0;
    for record in records { record_checksum = (record_checksum * 31 + record.id) % MODULUS; }

    var numbers: i64[>..] = [];
    for i in n { numbers.push(random.next(1000000000)); }
    sort(numbers);

    var number_checksum = 0;
    for number in numbers { number_checksum = (number_checksum * 31 + number) % MODULUS; }

    if records.len > 0 {
        let first = records[0];
        let last = records[records.len - 1];
        print("first: ", first.id, " ", first.name, " ", first.score);
        print("last: ", last.id, " ", last.name, " ", last.score);
    }
    print("record checksum: ", record_checksum);
    print("number checksum: ", number_checksum);
}
