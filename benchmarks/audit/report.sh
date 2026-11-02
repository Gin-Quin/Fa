#!/usr/bin/env bash
# Writes build/<name>/{fa,goose}/hot.txt (hot.py: cost per function, hottest instruction runs,
# call edges) for every benchmark profiled by profile.sh, and prints the per-function heads.
# Usage: benchmarks/audit/report.sh [benchmark…]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
benchmarks=("$@")
[ ${#benchmarks[@]} -eq 0 ] && mapfile -t benchmarks < <(cut -d' ' -f1 "$here/sizes.txt")
for name in "${benchmarks[@]}"; do
	for side in fa goose; do
		python3 "$here/hot.py" "$here/build/$name/$side/callgrind.out" "$here/build/$name/$side/main.s" \
			--top 25 --calls >"$here/build/$name/$side/hot.txt" 2>&1
	done
	echo "== $name (fa)"
	head -9 "$here/build/$name/fa/hot.txt"
	awk '/^calls \(/ { on = 1; count = 0; next } on && count < 10 { print; count += 1 }' \
		"$here/build/$name/fa/hot.txt"
done
