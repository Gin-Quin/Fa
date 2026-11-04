#!/usr/bin/env bash
# Builds the Fa (native, release, with its optimized IR) and Goose (-O2, standalone, clang 18)
# executables of the Goose-suite benchmarks for the minimalism audit (design/minimalism-audit.md),
# both at the audit size of sizes.txt (the Goose program's `// BENCH_N` line is rewritten).
# Usage: benchmarks/audit/build.sh [benchmark…]   (outputs in benchmarks/audit/build/<name>/)
# Needs a release `fa` (FA, default target/release/fa), Goose (GOOSE, default ~/.local/goose/goose)
# and clang 18 (GOOSE_CC, default the one of scripts/setup-llvm.sh).
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
fa="${FA:-$root/target/release/fa}"
goose="${GOOSE:-$HOME/.local/goose/goose}"
export GOOSE_CC="${GOOSE_CC:-$HOME/.local/llvm18/usr/lib/llvm-18/bin/clang}"
export GOOSE_HOME="${GOOSE_HOME:-$here/build/goose-home}"
benchmarks=("$@")
[ ${#benchmarks[@]} -eq 0 ] && mapfile -t benchmarks < <(cut -d' ' -f1 "$here/sizes.txt")
for name in "${benchmarks[@]}"; do
	size="$(grep "^$name " "$here/sizes.txt" | cut -d' ' -f2)"
	out="$here/build/$name"
	rm -rf "$out"
	mkdir -p "$out/fa" "$out/goose"
	# a `-- BENCH_N` constant of the Fa port takes the audit size, as Goose's line below
	sed -E "s/^(let\s+\w+\s*=\s*)[0-9_]+(\s*--\s*BENCH_N.*)$/\1$size\2/" \
		"$root/benchmarks/$name/main.fa" >"$out/fa/main.fa"
	if ! "$fa" build "$out/fa/main.fa" --target=native --emit-ir \
		--output="$out/fa" >"$out/fa/build.log" 2>&1; then
		echo "$name: fa build failed"
		tail -5 "$out/fa/build.log"
	fi
	cp "$root/benchmarks/goose/"*.goose "$out/goose/"
	sed -E "s/^(\s*let\s+\w+\s*=\s*)[0-9]+(\s*;.*BENCH_N.*)$/\1$size\2/" \
		"$root/benchmarks/$name/main.goose" >"$out/goose/main.goose"
	if ! "$goose" build -O2 --standalone -v -o "$out/goose/main" "$out/goose/main.goose" \
		>"$out/goose/build.log" 2>&1; then
		echo "$name: goose build failed"
		tail -5 "$out/goose/build.log"
	fi
	# the C that Goose generated, named in the clang command that -v printed
	generated="$(grep -o '[^ ]*/main\.c' "$out/goose/build.log" | tail -1 || true)"
	if [ -n "$generated" ]; then cp "$generated" "$out/goose/main.c"; fi
	objdump -d --no-show-raw-insn "$out/goose/main" >"$out/goose/main.s" 2>/dev/null || true
	exe="$(find "$out/fa" -maxdepth 3 -type f -perm -u+x ! -name '*.ll' | head -1)"
	if [ -n "$exe" ]; then objdump -d --no-show-raw-insn "$exe" >"$out/fa/main.s" 2>/dev/null || true; fi
	echo "$name (size $size): fa=$exe goose=$out/goose/main"
done
