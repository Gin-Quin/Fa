#!/usr/bin/env bash
# Run one of the three package fixtures with leak checking at O0/O3.
# FA_REVIEW_PACKAGE=per-unit|per-unit-delete|per-unit-generic
# Invoke under taskset -c 0-4; serialize this with other Cargo commands.
set -euo pipefail
REVIEW_DIRECTORY="$(cd "$(dirname "$0")" && pwd)"
REVIEW_ROOT="$(cd "$REVIEW_DIRECTORY/../../../.." && pwd)"
REVIEW_SCRATCH="${FA_REVIEW_OUT:-/tmp/review-collections-package}"
mkdir -p "$REVIEW_SCRATCH/runner"
python3 - "$REVIEW_ROOT" "$REVIEW_DIRECTORY" "$REVIEW_SCRATCH/runner/Cargo.toml" <<'PY'
import json
from pathlib import Path
import sys
root, here, manifest = map(Path, sys.argv[1:])
quote = lambda path: json.dumps(str(path))
manifest.write_text(f'''[package]
name = "fa-review-collections"
version = "0.0.0"
edition = "2024"
[workspace]
[[bin]]
name = "fa-review-collections"
path = {quote(here / "run_package.rs")}
[dependencies]
fa-analyzer = {{ path = {quote(root / "packages/analyzer")} }}
fa-codegen-llvm = {{ path = {quote(root / "packages/codegen-llvm")} }}
''')
PY
export CARGO_BUILD_JOBS=2
export LLVM_SYS_181_PREFIX="${LLVM_SYS_181_PREFIX:-$HOME/.local/llvm18/usr/lib/llvm-18}"
export CC="${CC:-$HOME/.local/llvm18/usr/bin/clang-18}"
export PATH="$HOME/.bun/bin:$HOME/.local/bin:$PATH"
export CARGO_TARGET_DIR="$REVIEW_ROOT/target"
cargo run --manifest-path "$REVIEW_SCRATCH/runner/Cargo.toml" -- \
	"$REVIEW_DIRECTORY" "$REVIEW_SCRATCH/output"
