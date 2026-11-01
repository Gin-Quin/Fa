#!/usr/bin/env bash
# Installs LLVM 18 and lld into a user directory, without root, for the
# `fa-codegen-llvm` crate (see design/llvm-backend-plan.md).
#
# Debian and Ubuntu only: the packages are fetched with `apt-get download` and
# unpacked with `dpkg -x`, so nothing is installed system-wide.
#
#   scripts/setup-llvm.sh            # installs into $HOME/.local/llvm18
#   FA_LLVM_ROOT=/opt/llvm scripts/setup-llvm.sh
#
# At the end, it prints the environment variable to export.

set -euo pipefail

ROOT="${FA_LLVM_ROOT:-$HOME/.local/llvm18}"
PREFIX="$ROOT/usr/lib/llvm-18"
PACKAGES=(libllvm18 llvm-18 llvm-18-dev llvm-18-runtime libpolly-18-dev lld-18 liblld-18)

if [ -x "$PREFIX/bin/llvm-config" ] && [ -x "$PREFIX/bin/wasm-ld" ]; then
	echo "LLVM $("$PREFIX/bin/llvm-config" --version) is already installed in $PREFIX"
else
	if ! command -v apt-get >/dev/null || ! command -v dpkg >/dev/null; then
		echo "error: apt-get and dpkg are needed (Debian or Ubuntu)." >&2
		echo "Elsewhere, install LLVM 18 with lld and set LLVM_SYS_181_PREFIX to its prefix." >&2
		exit 1
	fi
	DOWNLOADS="$(mktemp -d)"
	trap 'rm -rf "$DOWNLOADS"' EXIT
	echo "Downloading ${PACKAGES[*]}"
	(cd "$DOWNLOADS" && apt-get download "${PACKAGES[@]}")
	mkdir -p "$ROOT"
	for deb in "$DOWNLOADS"/*.deb; do
		echo "Extracting $(basename "$deb")"
		dpkg -x "$deb" "$ROOT"
	done
	"$PREFIX/bin/llvm-config" --version >/dev/null
	echo "LLVM $("$PREFIX/bin/llvm-config" --version) installed in $PREFIX"
fi

cat <<EOF

Add this to your shell profile (fa-codegen-llvm builds and tests only when it is set):

	export LLVM_SYS_181_PREFIX="$PREFIX"
EOF
