// Word and number counting (Goose port of main.rs): hash maps with string and
// integer keys, insert-or-update, iteration. The words are at most 6 bytes, so
// the string keys are inline `u8[..8]` strings, the std dictionary's form for
// keys that the map owns.

import std;
import os;
import dictionary;

let SYLLABLES: const u8[:][16] = ["ka", "lo", "mi", "nu", "pe", "ra", "si", "to",
                                  "va", "ze", "bo", "di", "fu", "ga", "he", "jo"];

// The "minimal standard" Park-Miller generator.
struct Random { state: i64 }

fn next(random: Random&, bound: i64) -> i64 {
    random.state = random.state * 48271 % 2147483647;
    return random.state % bound;
}

fn main() {
    let a = args();
    var n = 5000000;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { n = v; } }
    var random = Random { 42 };

    var word_counts = dictionary<u8[..8], i64> {};
    for i in n {
        var word: u8[..8] = [];
        let syllables = 1 + random.next(3);
        for s in syllables { word.append(SYLLABLES[random.next(16)]); }
        word_counts.update(word, 0) { it += 1; };
    }

    var number_counts = dictionary<i64, i64> {};
    for i in n {
        number_counts.update(random.next(100000), 0) { it += 1; };
    }

    var best_word: u8[..8] = [];
    var best_count = 0;
    var sum_of_squares = 0;
    word_counts.each() { word, count =>
        sum_of_squares += count * count;
        if count > best_count || (count == best_count && compare(word, best_word) < 0) {
            best_word = word;
            best_count = count;
        }
    };

    var weighted_sum = 0;
    number_counts.each() { number, count => weighted_sum += number * count; };

    print("distinct words: ", word_counts.count, ", sum of squares: ", sum_of_squares);
    print("most frequent: ", best_word, " (", best_count, " times)");
    print("distinct numbers: ", number_counts.count, ", weighted sum: ", weighted_sum);
}
