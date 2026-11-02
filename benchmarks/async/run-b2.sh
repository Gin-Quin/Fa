#!/usr/bin/env bash
# The B2 async benchmarks (design/async.md §6): `timers`, and the servers
# `echo`, `fairness` and `starve` under the load generator (load/main.go).
# Builds the Go and Tokio sides into benchmarks/async/build/, runs each
# configuration once and prints one markdown table per benchmark.
#
#   benchmarks/async/run-b2.sh [timers] [echo] [fairness] [starve]
#
# Configurations: fa, tokio current and go GOMAXPROCS=1 pinned on core 0;
# the references tokio multi and go default on cores 0-3; the load generator
# on cores 4-5. The Fa servers are read from $FA_BIN_DIR (binaries `timers`,
# `echo`, `fairness`, `starve`, taking the arguments of the Go ones); a
# missing one prints `n/a`.
#
# Environment: DURATION (load window, default 10s), CONNECTIONS (echo,
# default 10000), FLOOD_CONNECTIONS (starve, default 5000), TICKS (starve,
# default 3000), TIMERS_N (default 1000000), PORT (default 7878),
# STARVE_TIMEOUT (seconds, default 120), FA_BIN_DIR.
#
# Every non-Go process runs under `ulimit -v 8000000`. Go binaries (servers
# and load generator) run uncapped: older Go runtimes reserved huge virtual
# ranges and failed under such a cap. go1.27 does start under it (checked:
# `timers 1000` runs under `ulimit -v 1000000`), but its 1M-goroutine
# `timers` needs several GB of stacks, so no cap keeps the comparison clean.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/benchmarks/async"
BUILD="$HERE/build"
LOGS="$BUILD/logs"
GO="${GO:-$HOME/.local/go/bin/go}"
command -v "$GO" >/dev/null || GO=go
DURATION="${DURATION:-10s}"
CONNECTIONS="${CONNECTIONS:-10000}"
FLOOD_CONNECTIONS="${FLOOD_CONNECTIONS:-5000}"
TICKS="${TICKS:-3000}"
TIMERS_N="${TIMERS_N:-1000000}"
PORT="${PORT:-7878}"
STARVE_TIMEOUT="${STARVE_TIMEOUT:-120}"
FA_BIN_DIR="${FA_BIN_DIR:-}"
NAMES=("$@")
[ ${#NAMES[@]} -eq 0 ] && NAMES=(timers echo fairness starve)
CONFIGS=(fa "tokio current" "go GOMAXPROCS=1" "tokio multi" "go default")
mkdir -p "$BUILD" "$LOGS"
# 10 000 connections need as many descriptors on each side
ulimit -n "$(ulimit -Hn)"

echo "building…" >&2
(ulimit -v 8000000 && cargo build -q --release --manifest-path "$HERE/Cargo.toml")
for name in timers echo fairness starve; do
	cp "$HERE/target/release/$name" "$BUILD/tokio-$name"
	(cd "$HERE/go" && GOTOOLCHAIN=local "$GO" build -o "$BUILD/go-$name" "./$name")
done
(cd "$HERE/load" && GOTOOLCHAIN=local "$GO" build -o "$BUILD/load" .)

# command CONFIG NAME ARGUMENTS…: sets `command` (the program, pinned and
# capped as its configuration says), or returns 1 when it is missing
command_for() {
	local config=$1 name=$2
	shift 2
	case "$config" in
	fa)
		[ -n "$FA_BIN_DIR" ] && [ -x "$FA_BIN_DIR/$name" ] || return 1
		command=(bash -c 'ulimit -v 8000000 && exec "$@"' - taskset -c 0 "$FA_BIN_DIR/$name" "$@")
		;;
	"tokio current")
		command=(bash -c 'ulimit -v 8000000 && exec "$@"' - taskset -c 0 "$BUILD/tokio-$name" "$@" current)
		;;
	"tokio multi")
		command=(bash -c 'ulimit -v 8000000 && exec "$@"' - taskset -c 0-3 "$BUILD/tokio-$name" "$@" multi)
		;;
	"go GOMAXPROCS=1") command=(env GOMAXPROCS=1 taskset -c 0 "$BUILD/go-$name" "$@") ;;
	"go default") command=(taskset -c 0-3 "$BUILD/go-$name" "$@") ;;
	esac
}

# value KEY FILE: the value of `KEY=…` in the last line of FILE, or n/a
value() {
	local found
	found=$(tail -1 "$2" 2>/dev/null | grep -o "\b$1=[^ ]*" | cut -d= -f2) || true
	echo "${found:-n/a}"
}

wait_port() {
	local pid=$1
	for _ in $(seq 200); do
		kill -0 "$pid" 2>/dev/null || return 1
		(exec 3<>"/dev/tcp/127.0.0.1/$PORT") 2>/dev/null && return 0
		sleep 0.05
	done
	return 1
}

load() {
	taskset -c 4-5 "$BUILD/load" "$@" -addr "127.0.0.1:$PORT" 2>>"$LOGS/load.err"
}

# serve CONFIG NAME OUTPUT ARGUMENTS… -- LOAD ARGUMENTS…: runs the server,
# the load against it once the port accepts, then stops the server
serve() {
	local config=$1 name=$2 output=$3
	shift 3
	local server=()
	while [ "$1" != -- ]; do
		server+=("$1")
		shift
	done
	shift
	: >"$output"
	command_for "$config" "$name" "${server[@]}" || return 0
	"${command[@]}" >"$LOGS/server.out" 2>"$LOGS/server.err" &
	local pid=$!
	if wait_port "$pid"; then
		load "$@" >"$output" || true
	fi
	kill "$pid" 2>/dev/null || true
	wait "$pid" 2>/dev/null || true
}

timers() {
	echo
	echo "### timers (n = $TIMERS_N)"
	echo
	echo "| config | total ms | lateness p99 µs | lateness max µs | peak MB |"
	echo "| --- | ---: | ---: | ---: | ---: |"
	for config in "${CONFIGS[@]}"; do
		local output="$LOGS/timers.out" megabytes=n/a
		: >"$output"
		if command_for "$config" timers "$TIMERS_N"; then
			/usr/bin/time -o "$LOGS/timers.time" -f "%M" "${command[@]}" >"$output" 2>"$LOGS/server.err" || true
			megabytes=$(awk '{ printf "%.1f", $1 / 1024 }' <(tail -1 "$LOGS/timers.time"))
		fi
		echo "| $config | $(value total_ms "$output") | $(value lateness_p99_us "$output") | $(value lateness_max_us "$output") | $megabytes |"
	done
}

echo_() {
	echo
	echo "### echo ($CONNECTIONS connections, 64 B, $DURATION)"
	echo
	echo "| config | requests/s | p50 µs | p99 µs | p99.9 µs | errors |"
	echo "| --- | ---: | ---: | ---: | ---: | ---: |"
	for config in "${CONFIGS[@]}"; do
		local output="$LOGS/echo.out"
		serve "$config" echo "$output" "$PORT" -- echo -duration "$DURATION" -connections "$CONNECTIONS" -size 64
		echo "| $config | $(value rps "$output") | $(value p50_us "$output") | $(value p99_us "$output") | $(value p999_us "$output") | $(value errors "$output") |"
	done
}

fairness() {
	echo
	echo "### fairness (64 short clients, 4 long, $DURATION)"
	echo
	echo "| config | long | short p50 µs | short p99 µs | short p99.9 µs | short requests | long requests | errors |"
	echo "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"
	# `auto` (Fa only): the busy loop is in the async function, whose loop
	# back-edges yield once the turn lasted 2 ms (design/async.md A2)
	for variant in yield spin auto; do
		for config in "${CONFIGS[@]}"; do
			[ "$variant" = auto ] && [ "$config" != fa ] && continue
			local output="$LOGS/fairness.out"
			serve "$config" fairness "$output" "$PORT" "$variant" -- fairness -duration "$DURATION" -short 64 -long 4 -size 64
			echo "| $config | $variant | $(value p50_us "$output") | $(value p99_us "$output") | $(value p999_us "$output") | $(value short_requests "$output") | $(value long_requests "$output") | $(value errors "$output") |"
		done
	done
}

# the server ends by itself after its ticks; the flood stops with it
starve() {
	echo
	echo "### starve ($TICKS ticks, flood of $FLOOD_CONNECTIONS connections)"
	echo
	echo "| config | lateness p50 µs | lateness p99 µs | lateness max µs | connections | ttfb p50 µs | ttfb p99 µs | ttfb max µs | old p99 µs | unanswered | errors |"
	echo "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"
	for config in "${CONFIGS[@]}"; do
		local server="$LOGS/starve.out" flood="$LOGS/flood.out"
		: >"$server"
		: >"$flood"
		if command_for "$config" starve "$PORT" "$TICKS"; then
			timeout "$STARVE_TIMEOUT" "${command[@]}" >"$server" 2>"$LOGS/server.err" &
			local pid=$!
			if wait_port "$pid"; then
				# not through `load`: $! must be the generator itself
				taskset -c 4-5 "$BUILD/load" flood -addr "127.0.0.1:$PORT" -duration "${STARVE_TIMEOUT}s" \
					-connections "$FLOOD_CONNECTIONS" >"$flood" 2>>"$LOGS/load.err" &
				local flooding=$!
				wait "$pid" 2>/dev/null || true
				kill -TERM "$flooding" 2>/dev/null || true
				wait "$flooding" 2>/dev/null || true
			else
				kill "$pid" 2>/dev/null || true
				wait "$pid" 2>/dev/null || true
			fi
		fi
		echo "| $config | $(value lateness_p50_us "$server") | $(value lateness_p99_us "$server") | $(value lateness_max_us "$server") | $(value connections "$flood") | $(value ttfb_p50_us "$flood") | $(value ttfb_p99_us "$flood") | $(value ttfb_max_us "$flood") | $(value old_p99_us "$flood") | $(value unanswered "$flood") | $(value errors "$flood") |"
	done
}

for name in "${NAMES[@]}"; do
	case "$name" in
	timers) timers ;;
	echo) echo_ ;;
	fairness) fairness ;;
	starve) starve ;;
	*) echo "unknown benchmark $name" >&2 && exit 2 ;;
	esac
done
