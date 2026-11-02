#!/usr/bin/env bash
# Peak RSS (KiB) and user CPU time of the Fa and Goose executables at the benchmarks' default
# sizes (the Goose program as checked in; Fa without arguments). Needs build.sh first (for Fa).
# Usage: benchmarks/audit/rss.sh [benchmark…]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
goose="${GOOSE:-$HOME/.local/goose/goose}"
export GOOSE_CC="${GOOSE_CC:-$HOME/.local/llvm18/usr/lib/llvm-18/bin/clang}"
export GOOSE_HOME="${GOOSE_HOME:-$here/build/goose-home}"
benchmarks=("$@")
[ ${#benchmarks[@]} -eq 0 ] && mapfile -t benchmarks < <(cut -d' ' -f1 "$here/sizes.txt")
printf '%-13s %10s %10s %6s %8s %8s\n' benchmark fa_rss_kb goose_rss_kb ratio fa_s goose_s
for name in "${benchmarks[@]}"; do
	out="$here/build/$name/default"
	mkdir -p "$out"
	cp "$root/benchmarks/goose/"*.goose "$root/benchmarks/$name/main.goose" "$out/"
	if [ ! -x "$out/goose" ]; then
		"$goose" build -O2 --standalone -o "$out/goose" "$out/main.goose" >/dev/null 2>&1
	fi
	/usr/bin/time -f '%M %U' -o "$out/fa.time" "$here/build/$name/fa/main" >/dev/null
	/usr/bin/time -f '%M %U' -o "$out/goose.time" "$out/goose" >/dev/null
	read -r fa_rss fa_s <"$out/fa.time"
	read -r goose_rss goose_s <"$out/goose.time"
	printf '%-13s %10s %10s %6.2f %8s %8s\n' "$name" "$fa_rss" "$goose_rss" \
		"$(echo "$fa_rss / $goose_rss" | bc -l)" "$fa_s" "$goose_s"
done
