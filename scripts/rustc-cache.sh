#!/usr/bin/env bash
# Cargo's RUSTC_WRAPPER: share registry compilation, retain local incrementality.
# Do not normalize worktree paths: env!("CARGO_MANIFEST_DIR") must stay correct.
set -euo pipefail

compiler="$1"
shift
case "${CARGO_MANIFEST_DIR:-}" in
	*/registry/src/*)
		if [[ "${FA_RUST_CACHE:-1}" != 0 ]]; then
			cache="${FA_SCCACHE:-$HOME/.local/bin/sccache}"
			if [[ -x "$cache" ]]; then
				# Local crates bypass sccache entirely; this also permits a caller
				# to explicitly enable Cargo incremental compilation.
				export SCCACHE_ALLOW_INCREMENTAL=1
				exec "$cache" "$compiler" "$@"
			fi
		fi
		;;
esac
exec "$compiler" "$@"
