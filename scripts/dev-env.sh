# Source this file from any working directory: source /path/to/Fa/scripts/dev-env.sh
# Each worktree keeps its own target/; only the registry compiler cache is shared.
fa_dev_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="$HOME/.local/bin:$HOME/.bun/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RUSTC_WRAPPER="$fa_dev_root/scripts/rustc-cache.sh"
export SCCACHE_DIR="${SCCACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/fa/sccache}"
export SCCACHE_CACHE_SIZE="${SCCACHE_CACHE_SIZE:-4G}"
if [[ -z "${LLVM_SYS_181_PREFIX:-}" ]] && [[ -x "$HOME/.local/llvm18/usr/lib/llvm-18/bin/llvm-config" ]]; then
	export LLVM_SYS_181_PREFIX="$HOME/.local/llvm18/usr/lib/llvm-18"
fi
unset fa_dev_root
