//! Hot reloading in `fa develop` (decision D43, architecture/hot-reloading):
//! the program is generated as a development build (`Options::hot`: the
//! modules read each other through a registry, `faHotModule`), and an edit
//! that keeps the shape of the program swaps the changed modules in the
//! running Bun process instead of restarting it.
//!
//! After an edit of the package's own sources without errors, the whole
//! program is generated again and compared with the running one:
//!
//! - the modules whose code changed are swapped when they are all files
//!   that were edited (a change that reaches another module's code is an
//!   interface change: renamed or removed exports, other union tags…), the
//!   edited files declare the same types (their text unchanged: values of
//!   a type whose shape changed may be kept elsewhere), the program has the
//!   same modules (a file added or removed restarts before this), the same
//!   runtime helpers and no worker;
//! - otherwise the program restarts, with the reason.
//!
//! A swap writes the new modules next to the old ones (`greet.hot3.js`) and
//! `$hot.json` (the modules, then the initialization functions to call:
//! every module-level value of a swapped module, the values whose
//! initializers read one of them, and the values new to the program, in
//! initialization order), sends `SIGUSR2` to Bun (`faHotListen`), and waits
//! for `$hot-done.json`. The values of the other modules keep their
//! current values. Unix only; elsewhere, `fa develop` restarts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fa_analyzer::ids::NodeId;
use fa_analyzer::syntax::{self, View};
use fa_codegen_js::ProgramOutput;

use crate::Loaded;

/// The program running in Bun, as generated.
pub(crate) struct Running {
	/// Its output folder (`main.js`).
	directory: PathBuf,
	/// Its modules: path → code.
	modules: BTreeMap<String, String>,
	/// Its initialization functions, in order: (module path, function).
	inits: Vec<(String, String)>,
	/// The number of swaps.
	version: u32,
}

/// What a swap does.
pub(crate) struct Plan {
	/// The paths of the modules swapped.
	pub changed: Vec<String>,
	/// The initialization functions to call, in order.
	inits: Vec<(String, String)>,
	/// The modules of the new program.
	modules: BTreeMap<String, String>,
	/// The initialization functions of the new program.
	all_inits: Vec<(String, String)>,
}

/// Whether hot reloading is possible here (the swap is signaled).
pub(crate) fn available() -> bool {
	cfg!(unix)
}

/// The initialization functions of a program, in order.
fn inits_of(loaded: &Loaded) -> Vec<(String, String)> {
	let session = &loaded.inner.session;
	loaded
		.inner
		.program
		.init_order
		.iter()
		.map(|&(file, decl)| {
			(
				fa_codegen_js::js_path(session.module(file)),
				format!("$init{}", decl.index()),
			)
		})
		.collect()
}

/// The texts of the type declarations of a source, sorted.
pub(crate) fn type_declarations(text: &str) -> Vec<String> {
	let tree = fa_parser::parse(text);
	let mut found: Vec<String> = (0..tree.nodes.len())
		.filter(|&index| {
			matches!(
				syntax::view(&tree, NodeId::new(index)),
				View::TypeDecl { .. } | View::ExportType { .. }
			)
		})
		.map(|index| {
			let span = tree.spans[index];
			let end = span.end.min(tree.source.len());
			tree.source[span.start.min(end)..end].to_string()
		})
		.collect();
	found.sort();
	found
}

impl Running {
	/// The program just started from `main` (its folder holds the modules).
	pub(crate) fn new(main: &Path, output: &ProgramOutput, loaded: &Loaded) -> Self {
		Running {
			directory: main.parent().map(Path::to_path_buf).unwrap_or_default(),
			modules: output.modules.iter().cloned().collect(),
			inits: inits_of(loaded),
			version: 0,
		}
	}

	/// What swapping to a new program does, or why it restarts instead.
	/// `edited`: the modules edited (`library/greet`), with their text
	/// before the edit.
	pub(crate) fn plan(
		&self,
		loaded: &Loaded,
		output: &ProgramOutput,
		edited: &[(String, String)],
	) -> Result<Plan, String> {
		for (module, before) in edited {
			let file = loaded.inner.session.loaded_module(module);
			let after = file.map(|file| loaded.inner.session.text(file).to_string());
			if after.is_some_and(|after| type_declarations(&after) != type_declarations(before)) {
				return Err(format!("a type of `{module}.fa` changed"));
			}
		}
		let modules: BTreeMap<String, String> = output.modules.iter().cloned().collect();
		if modules.keys().ne(self.modules.keys()) {
			return Err("the modules of the program changed".to_string());
		}
		if modules.contains_key(fa_codegen_js::WORKER_MODULE) {
			return Err("the program uses workers".to_string());
		}
		if modules.get("$runtime.js") != self.modules.get("$runtime.js") {
			return Err("the program uses other helpers of the runtime".to_string());
		}
		let edited_paths: Vec<String> = edited
			.iter()
			.map(|(module, _)| fa_codegen_js::js_path(module))
			.collect();
		let mut changed = Vec::new();
		for (path, code) in &modules {
			if path == "main.js" || path == "$runtime.js" || self.modules.get(path) == Some(code) {
				continue;
			}
			if !edited_paths.contains(path) {
				let module = path.strip_suffix(".js").unwrap_or(path);
				return Err(format!("the change reaches `{module}.fa`"));
			}
			changed.push(path.clone());
		}
		// the values to initialize again: of the swapped modules, reading
		// one of them, or new
		let all_inits = inits_of(loaded);
		let reads = loaded
			.inner
			.session
			.initializer_reads(&loaded.inner.program);
		let mut selected = vec![false; all_inits.len()];
		for (index, init) in all_inits.iter().enumerate() {
			selected[index] = changed.contains(&init.0)
				|| !self.inits.contains(init)
				|| reads
					.get(index)
					.is_some_and(|reads| reads.iter().any(|&read| selected[read]));
		}
		let inits = all_inits
			.iter()
			.zip(&selected)
			.filter(|(_, selected)| **selected)
			.map(|(init, _)| init.clone())
			.collect();
		Ok(Plan {
			changed,
			inits,
			modules,
			all_inits,
		})
	}

	/// Swaps the changed modules in the running program (`pid`): `Err` with
	/// the reason when it failed (the program is then restarted).
	pub(crate) fn swap(&mut self, plan: Plan, pid: u32) -> Result<(), String> {
		if plan.changed.is_empty() && plan.inits.is_empty() {
			self.modules = plan.modules;
			return Ok(());
		}
		let version = self.version + 1;
		let write = |path: &str, text: &str| {
			fs::write(self.directory.join(path), text)
				.map_err(|error| format!("can't write `{path}`: {error}"))
		};
		let mut listed = Vec::new();
		for path in &plan.changed {
			let file = format!(
				"{}.hot{version}.js",
				path.strip_suffix(".js").unwrap_or(path)
			);
			write(&file, &plan.modules[path])?;
			listed.push(format!(
				"{{ \"path\": {}, \"file\": {} }}",
				json_string(path),
				json_string(&file)
			));
		}
		let inits: Vec<String> = plan
			.inits
			.iter()
			.map(|(path, name)| format!("[{}, {}]", json_string(path), json_string(name)))
			.collect();
		let done = self.directory.join("$hot-done.json");
		let _ = fs::remove_file(&done);
		write(
			"$hot.json",
			&format!(
				"{{ \"version\": {version}, \"modules\": [{}], \"inits\": [{}] }}\n",
				listed.join(", "),
				inits.join(", ")
			),
		)?;
		let signaled = std::process::Command::new("kill")
			.arg("-USR2")
			.arg(pid.to_string())
			.status()
			.is_ok_and(|status| status.success());
		if !signaled {
			return Err("the program can't be signaled".to_string());
		}
		let start = Instant::now();
		let answer = loop {
			if let Ok(text) = fs::read_to_string(&done)
				&& text.contains(&format!("\"version\":{version},"))
			{
				break text;
			}
			if start.elapsed() > Duration::from_secs(10) {
				return Err("the program didn't answer".to_string());
			}
			std::thread::sleep(Duration::from_millis(10));
		};
		self.version = version;
		if !answer.contains("\"ok\":true") {
			let error = answer
				.split_once("\"error\":\"")
				.map_or("", |(_, rest)| rest.trim_end_matches("\"}"));
			return Err(format!("the swap failed: {error}"));
		}
		self.modules = plan.modules;
		self.inits = plan.all_inits;
		Ok(())
	}
}

/// A JSON string.
fn json_string(text: &str) -> String {
	let mut out = String::from("\"");
	for c in text.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
			c => out.push(c),
		}
	}
	out.push('"');
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn type_declarations_are_compared_by_their_text() {
		let before = "type Cow = { name: String }\nexport function = (): Integer => 1\n";
		let body = "type Cow = { name: String }\nexport function = (): Integer => 2\n";
		let shape =
			"type Cow = { name: String, age: Integer }\nexport function = (): Integer => 1\n";
		assert_eq!(type_declarations(before), type_declarations(body));
		assert_ne!(type_declarations(before), type_declarations(shape));
		assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
	}
}
