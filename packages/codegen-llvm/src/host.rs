//! The JavaScript runtime that runs wasm programs.

use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::error::Error;

/// The JavaScript runtime used to run wasm programs.
pub fn javascript_runtime() -> Result<OsString, Error> {
	if let Some(runtime) = env::var_os("FA_JS_RUNTIME") {
		return Ok(runtime);
	}
	let path = env::var_os("PATH").unwrap_or_default();
	for name in ["node", "bun"] {
		if env::split_paths(&path).any(|directory| directory.join(name).is_file()) {
			return Ok(name.into());
		}
	}
	let bun = env::var_os("HOME").map(|home| PathBuf::from(home).join(".bun/bin/bun"));
	match bun {
		Some(bun) if bun.is_file() => Ok(bun.into()),
		_ => Err(Error::Tool {
			command: "node".to_string(),
			message: "neither node nor bun was found (set FA_JS_RUNTIME)".to_string(),
		}),
	}
}
