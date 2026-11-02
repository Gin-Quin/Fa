#!/usr/bin/env bash
# Builds the test of the event loop (`executor_test.c`, which includes
# `fa_runtime.c`) with the C library and AddressSanitizer, and runs it.
#
#   packages/codegen-llvm/runtime/test/run.sh [output directory]
#
# The compiler is `$CC`, else `cc`; it must support `-fsanitize=address`.

set -euo pipefail

DIRECTORY="$(cd "$(dirname "$0")" && pwd)"
OUTPUT="${1:-${TMPDIR:-/tmp}}"
mkdir -p "$OUTPUT"
"${CC:-cc}" -std=gnu11 -O1 -g -fsanitize=address,undefined -fno-sanitize=nonnull-attribute -fno-sanitize-recover=undefined -fno-omit-frame-pointer \
	-fno-builtin -Wno-builtin-declaration-mismatch -Wno-attributes \
	-pthread "$DIRECTORY/executor_test.c" -o "$OUTPUT/fa_executor_test"
"$OUTPUT/fa_executor_test"
