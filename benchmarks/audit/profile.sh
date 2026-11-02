#!/usr/bin/env bash
# Runs the Fa and Goose executables built by build.sh under callgrind (instructions per function,
# inclusive and exclusive) and under /usr/bin/time (peak RSS), at the size of sizes.txt, and prints
# one summary line per benchmark. Outputs in benchmarks/audit/build/<name>/{fa,goose}/.
# Usage: benchmarks/audit/profile.sh [benchmark…]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
valgrind="${VALGRIND:-$HOME/.local/bin/valgrind}"
annotate="${CALLGRIND_ANNOTATE:-$HOME/.local/valgrind/usr/bin/callgrind_annotate}"
benchmarks=("$@")
[ ${#benchmarks[@]} -eq 0 ] && mapfile -t benchmarks < <(cut -d' ' -f1 "$here/sizes.txt")
printf '%-13s %14s %14s %6s %10s %10s %6s\n' benchmark fa_instr goose_instr ratio fa_rss_kb goose_rss_kb ratio
for name in "${benchmarks[@]}"; do
	size="$(grep "^$name " "$here/sizes.txt" | cut -d' ' -f2)"
	out="$here/build/$name"
	fa_exe="$(find "$out/fa" -maxdepth 3 -type f -perm -u+x ! -name '*.ll' | head -1)"
	goose_args=()
	grep -q BENCH_N "$out/goose/main.goose" || goose_args=("$size")
	declare -A instr rss
	for side in fa goose; do
		if [ "$side" = fa ]; then cmd=("$fa_exe" "$size"); else cmd=("$out/goose/main" "${goose_args[@]}"); fi
		/usr/bin/time -f '%M' -o "$out/$side/rss.txt" "${cmd[@]}" >"$out/$side/stdout.txt"
		"$valgrind" --tool=callgrind --dump-instr=yes --callgrind-out-file="$out/$side/callgrind.out" "${cmd[@]}" \
			>/dev/null 2>"$out/$side/callgrind.log"
		"$annotate" "$out/$side/callgrind.out" >"$out/$side/self.txt" 2>/dev/null
		"$annotate" --inclusive=yes "$out/$side/callgrind.out" >"$out/$side/inclusive.txt" 2>/dev/null
		instr[$side]="$(grep -m1 'PROGRAM TOTALS' "$out/$side/self.txt" | awk '{gsub(",","",$1); print $1}')"
		rss[$side]="$(tail -1 "$out/$side/rss.txt")"
	done
	cmp -s "$out/fa/stdout.txt" "$out/goose/stdout.txt" || echo "$name: outputs differ" >&2
	printf '%-13s %14s %14s %6.2f %10s %10s %6.2f\n' "$name" "${instr[fa]}" "${instr[goose]}" \
		"$(echo "${instr[fa]} / ${instr[goose]}" | bc -l)" "${rss[fa]}" "${rss[goose]}" \
		"$(echo "${rss[fa]} / ${rss[goose]}" | bc -l)"
done
