// Binary trees: the Goose suite's bintrees.goose (bench/goose/bintrees.goose in
// github.com/aardappel/goose), with the output and the size handling of main.rs.
// Every tree is a local pool of 8-byte nodes linked by 4-byte relative
// references, built by a recursive function into the pool it is handed, and
// freed by resetting one pointer when the function returns.

import std;
import os;

let MINDEPTH = 4;

struct TNode { l: TNode&<u32>?, r: TNode&<u32>? }

recursive fn make(pool: TNode[>..]&, depth: i64) -> TNode& {
    if depth == 0 { return pool.push(TNode {}); }
    let l: TNode& = make(pool, depth - 1);
    let r: TNode& = make(pool, depth - 1);
    return pool.push(TNode { l: l, r: r });
}

recursive fn check(n: TNode&) -> i64 {
    let l = n.l;
    guard l else { return 1; }
    let r = n.r;
    guard r else { return 1; }
    return 1 + check(l) + check(r);
}

// Build, check, discard: the pool is a local, so the discard is free.
fn tree_check(depth: i64) -> i64 {
    var pool: TNode[>..] = [];
    let root: TNode& = make(pool, depth);
    return check(root);
}

fn main() {
    let a = args();
    var size = 17;
    if a.len > 1 { let v, ok = parse_int(a[1]); if ok { size = v; } }
    let max_depth = max(size, MINDEPTH + 2);
    let stretch_depth = max_depth + 1;
    print("stretch tree of depth ", stretch_depth, "\t check: ", tree_check(stretch_depth));
    var long: TNode[>..] = [];
    let lroot: TNode& = make(long, max_depth);
    var d = MINDEPTH;
    while d <= max_depth {
        let iterations = 1 << (max_depth - d + MINDEPTH);
        var sum = 0;
        for i in iterations { sum += tree_check(d); }
        print(iterations, "\t trees of depth ", d, "\t check: ", sum);
        d += 2;
    }
    print("long lived tree of depth ", max_depth, "\t check: ", check(lroot));
}
