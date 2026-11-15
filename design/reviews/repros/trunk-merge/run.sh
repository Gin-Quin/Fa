#!/usr/bin/env bash
# Runs a single-file repro on native, wasm and native-freestanding, at -O0 and
# -O3, with the leak check, and on Bun; compares stdout with its `-- prints:`.
# usage: run.sh file.fa [targets...]
set -u
root="$(git -C "$(dirname "$0")" rev-parse --show-toplevel)"
build="$root/target/debug/examples/build"
fa="$root/target/debug/fa"
file="$1"; shift
targets="${*:-native wasm native-freestanding bun}"
expected="$(grep -oE -- '-- prints: .*' "$file" | sed 's/^-- prints: //')"
out=/tmp/trm/out-$$; mkdir -p "$out"
status=0
if ! "$fa" check "$file" >"$out/check" 2>&1; then cat "$out/check"; rm -rf "$out"; exit 2; fi
for target in $targets; do
	if [ "$target" = bun ]; then levels=bun; else levels="-O0 -O3"; fi
	for level in $levels; do
		if [ "$target" = bun ]; then
			actual="$(PATH="$HOME/.bun/bin:$PATH" timeout 120 "$fa" run "$file" 2>"$out/err")"; code=$?
		else
			actual="$(ulimit -v 2000000; timeout 300 "$build" "$file" --target=$target $level --leak-check --output="$out/$target$level" --run 2>"$out/err")"; code=$?
		fi
		if [ "$actual" = "$expected" ] && [ $code = 0 ]; then
			echo "ok   $target $level"
		else
			status=1
			echo "FAIL $target $level (exit $code)"
			diff <(echo "$expected") <(echo "$actual") | head -20
			head -c 1500 "$out/err"
		fi
	done
done
rm -rf "$out"
exit $status
