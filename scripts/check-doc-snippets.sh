#!/usr/bin/env bash
# Parse every ```fa block of `documentation/` and `README.md` with fa-parser, and report the
# blocks that don't parse (the examples of syntax errors listed in the test must report one).
#
#   scripts/check-doc-snippets.sh           every block, plus truncated and garbled versions
#   scripts/check-doc-snippets.sh --full    also every truncation and many more garbled versions
#
# Set FA_DOCUMENTATION to check another documentation folder.
set -euo pipefail
cd "$(dirname "$0")/.."

arguments=(--nocapture)
if [[ "${1:-}" == "--full" ]]; then
	arguments+=(--include-ignored)
fi

cargo test --release -p fa-parser --test doc_snippets -- "${arguments[@]}"
