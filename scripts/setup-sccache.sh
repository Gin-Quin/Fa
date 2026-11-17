#!/usr/bin/env bash
# Install the pinned Linux x86-64 sccache binary without root.
# Usage: scripts/setup-sccache.sh; source scripts/dev-env.sh
set -euo pipefail

version=0.19.0
checksum=aa73c7270c997894038c0d8f0d297865f57569e1687883322bc431713d4bb9dd
install_dir="${FA_TOOLS_BIN:-$HOME/.local/bin}"
if [[ "$(uname -s)-$(uname -m)" != Linux-x86_64 ]]; then
	echo "Install sccache for your platform from https://github.com/mozilla/sccache/releases" >&2
	exit 1
fi
if [[ -x "$install_dir/sccache" ]] && [[ "$("$install_dir/sccache" --version)" == "sccache $version" ]]; then
	echo "sccache $version is already installed in $install_dir"
	exit 0
fi
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
archive="sccache-v$version-x86_64-unknown-linux-musl"
curl --fail --location --silent --show-error 	"https://github.com/mozilla/sccache/releases/download/v$version/$archive.tar.gz" 	-o "$scratch/sccache.tar.gz"
printf '%s  %s\n' "$checksum" "$scratch/sccache.tar.gz" | sha256sum --check --status
tar -xzf "$scratch/sccache.tar.gz" -C "$scratch"
mkdir -p "$install_dir"
install -m 755 "$scratch/$archive/sccache" "$install_dir/sccache.new"
mv "$install_dir/sccache.new" "$install_dir/sccache"
"$install_dir/sccache" --version
