#!/usr/bin/env bash
# The async benchmarks (design/async.md §6), single thread: Fa natively
# against Tokio `current_thread` and Go with GOMAXPROCS=1. Builds the three
# sides, runs each program `RUNS` times (default 3), and prints the best wall
# time and the peak resident memory of each.
#
#   benchmarks/async/run.sh [spawn] [ping-pong]
#
# Needs LLVM_SYS_181_PREFIX (the Fa compiler), cargo, and Go in ~/.local/go
# (or `go` on the PATH). Builds go to benchmarks/async/build/ (git-ignored).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/benchmarks/async"
BUILD="$HERE/build"
RUNS="${RUNS:-3}"
GO="${GO:-$HOME/.local/go/bin/go}"
command -v "$GO" >/dev/null || GO=go
NAMES=("$@")
[ ${#NAMES[@]} -eq 0 ] && NAMES=(spawn ping-pong)
mkdir -p "$BUILD"

echo "building…" >&2
for name in "${NAMES[@]}"; do
	(cd "$ROOT" && cargo run -q --release -p fa-codegen-llvm --example build -- \
		"$HERE/fa/$name.fa" --output="$BUILD" >/dev/null)
done
cargo build -q --release --manifest-path "$HERE/Cargo.toml"
(cd "$HERE/go" && GOTOOLCHAIN=local "$GO" build -o "$BUILD/go-spawn" . &&
	GOTOOLCHAIN=local "$GO" build -o "$BUILD/go-ping-pong" ./pingpong)

# best of $RUNS: "seconds kilobytes"
measure() {
	local best="" memory=""
	for _ in $(seq "$RUNS"); do
		local line
		line=$( { /usr/bin/time -f "%e %M" "$@" >/dev/null; } 2>&1 | tail -1)
		local seconds=${line% *}
		if [ -z "$best" ] || awk "BEGIN { exit !($seconds < $best) }"; then
			best=$seconds
			memory=${line#* }
		fi
	done
	echo "$best $memory"
}

printf "%-10s %-22s %10s %10s\n" benchmark program seconds "peak MB"
for name in "${NAMES[@]}"; do
	case "$name" in
	spawn) tokio=("$HERE/target/release/fa-async-benchmarks" 1000000 current) ;;
	ping-pong) tokio=("$HERE/target/release/ping_pong" 10000000 current) ;;
	esac
	for side in fa tokio go; do
		case "$side" in
		fa) command=("$BUILD/$name") ;;
		tokio) command=("${tokio[@]}") ;;
		go)
			if [ "$name" = spawn ]; then
				command=(env GOMAXPROCS=1 "$BUILD/go-spawn" 1000000)
			else
				command=(env GOMAXPROCS=1 "$BUILD/go-ping-pong" 10000000)
			fi
			;;
		esac
		read -r seconds kilobytes < <(measure "${command[@]}")
		printf "%-10s %-22s %10s %10.1f\n" "$name" "$side" "$seconds" "$(awk "BEGIN { print $kilobytes / 1024 }")"
	done
done
