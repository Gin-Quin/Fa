#!/usr/bin/env bash
# Compiles the tier 1 runtime (`fa_runtime.c`) to LLVM IR with clang 18, once
# for x86-64, AArch64 and wasm32, and writes the IR that
# `src/runtime.rs` links into every program (checked in, so that building the
# compiler doesn't need clang).
#
#   FA_LLVM_WITH_CLANG=1 scripts/setup-llvm.sh   # installs clang 18
#   packages/codegen-llvm/runtime/generate.sh
#
# The target triple, the data layout and the per-function CPU attributes are
# removed: the program module sets its own.

set -euo pipefail

DIRECTORY="$(cd "$(dirname "$0")" && pwd)"
CLANG="${CLANG:-${LLVM_SYS_181_PREFIX:-$HOME/.local/llvm18/usr/lib/llvm-18}/bin/clang}"
FLAGS=(-Os -std=c11 -ffreestanding -fno-builtin -nostdlib -fno-stack-protector
	-fno-unwind-tables -fno-asynchronous-unwind-tables -fno-exceptions
	-fno-ident -Wall -Wextra -Werror -S -emit-llvm)

# the case tables, from the Unicode tables of Bun (or Node)
if command -v bun >/dev/null || [ -x "$HOME/.bun/bin/bun" ]; then
	"$(command -v bun || echo "$HOME/.bun/bin/bun")" "$DIRECTORY/case_tables.js"
fi

compile() {
	local target="$1" output="$2"
	shift 2
	"$CLANG" "${FLAGS[@]}" --target="$target" "$@" "$DIRECTORY/fa_runtime.c" -o "$output.tmp"
	sed -E \
		-e '/^target (datalayout|triple)/d' \
		-e 's/ "target-cpu"="[^"]*"//g' \
		-e 's/ "target-features"="[^"]*"//g' \
		-e 's/ "tune-cpu"="[^"]*"//g' \
		-e 's/ "no-builtins"//g' -e 's/ nobuiltin//g' \
		-e '/^; ModuleID/d' \
		-e 's/^source_filename = .*/source_filename = "fa_runtime.c"/' \
		"$output.tmp" >"$output"
	rm "$output.tmp"
}

compile x86_64-unknown-linux-gnu "$DIRECTORY/fa_runtime.64.ll" -fPIC
compile aarch64-unknown-linux-gnu "$DIRECTORY/fa_runtime.aarch64.ll" -fPIC
compile wasm32-unknown-unknown "$DIRECTORY/fa_runtime.wasm32.ll" \
	-mbulk-memory -mmutable-globals -msign-ext -mnontrapping-fptoint
echo "wrote the x86-64, AArch64 and wasm32 runtimes"
