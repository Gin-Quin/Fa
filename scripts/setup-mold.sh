#!/usr/bin/env bash
# Optional linker comparison, without root (Debian/Ubuntu packages).
# The normal Rust toolchain already supplies lld on this machine.
set -euo pipefail

install_root="${FA_MOLD_ROOT:-$HOME/.local/mold}"
if [[ -x "$install_root/usr/bin/mold" ]]; then
	"$install_root/usr/bin/mold" --version
	exit 0
fi
if ! command -v apt-get >/dev/null || ! command -v dpkg >/dev/null; then
	echo "apt-get and dpkg are required; install mold for your platform separately." >&2
	exit 1
fi
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
(cd "$scratch" && apt-get download mold)
mkdir -p "$install_root"
for package in "$scratch"/*.deb; do
	dpkg -x "$package" "$install_root"
done
"$install_root/usr/bin/mold" --version
