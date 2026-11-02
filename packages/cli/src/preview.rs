//! `fa preview`: runs the output of `fa build` (a small Bun server for
//! `web`).

use super::*;

/// `fa preview [<package>] [--target=<target>]` (advanced brief §20): runs
/// the output of `fa build`, for the target named, else for the first
/// configured target that runs here (`native`, then `bun`, `node`;
/// `web` is served on `develop.port`).
pub(crate) fn preview(arguments: &[OsString]) -> i32 {
	let mut target = None;
	let mut paths = Vec::new();
	for argument in arguments {
		let text = argument.to_string_lossy();
		if let Some(value) = text.strip_prefix("--target=") {
			if !["web", "bun", "node", "native", "wasm"].contains(&value) {
				eprintln!(
					"error[CLI002]: unknown target `{value}`: the targets are `bun`, `node`, `web`, `native` and `wasm`"
				);
				return USAGE;
			}
			target = Some(value.to_string());
		} else if text.starts_with("--") {
			eprintln!("fa preview: unknown option `{text}`");
			return USAGE;
		} else {
			paths.push(argument.clone());
		}
	}
	let path = match optional_path(&paths, "preview") {
		Ok(path) => path,
		Err(code) => return code,
	};
	on_selection(
		path.as_deref(),
		Implements::Application,
		"preview",
		|path| preview_one(path, target.as_deref()),
	)
}

/// The runtime that runs the output of a target here, if any.
pub(crate) fn preview_command(target: &str, directory: &Path, port: u16) -> Option<Command> {
	let on_path = |name: &str| -> Option<PathBuf> {
		let path = std::env::var_os("PATH")?;
		std::env::split_paths(&path)
			.map(|folder| folder.join(name))
			.find(|candidate| candidate.is_file())
	};
	match target {
		"native" => {
			let program = directory.join("main");
			program.is_file().then(|| Command::new(program))
		}
		"wasm" => {
			let loader = directory.join("main.mjs");
			let runtime = fa_codegen_llvm::javascript_runtime().ok()?;
			loader.is_file().then(|| {
				let mut command = Command::new(runtime);
				command.arg(loader);
				command
			})
		}
		"bun" | "node" => {
			let main = directory.join("main.js");
			if !main.is_file() {
				return None;
			}
			let runtime = match target {
				"bun" => find_bun()?,
				other => on_path(other)?,
			};
			let mut command = Command::new(runtime);
			command.arg(main);
			Some(command)
		}
		"web" => {
			let main = directory.join("main.js");
			if !main.is_file() {
				return None;
			}
			let bun = find_bun()?;
			let server = format!(
				"const root = {root:?};\nconst page = '<!doctype html><meta charset=\"utf-8\"><script type=\"module\" src=\"/main.js\"></script>';\nBun.serve({{ port: {port}, async fetch(request) {{\n\tconst path = decodeURIComponent(new URL(request.url).pathname);\n\tif (path === '/' || path === '/index.html') {{\n\t\tconst index = Bun.file(root + '/index.html');\n\t\treturn (await index.exists()) ? new Response(index) : new Response(page, {{ headers: {{ 'content-type': 'text/html' }} }});\n\t}}\n\tif (path.includes('..')) return new Response('not found', {{ status: 404 }});\n\tconst file = Bun.file(root + path);\n\treturn (await file.exists()) ? new Response(file) : new Response('not found', {{ status: 404 }});\n}} }});\nconsole.error('fa preview: serving ' + root + ' on http://localhost:{port}');\n",
				root = directory.display().to_string()
			);
			let script = cache_directory().join(format!("preview-{port}.mjs"));
			fs::create_dir_all(script.parent()?).ok()?;
			fs::write(&script, server).ok()?;
			let mut command = Command::new(bun);
			command.arg(script);
			Some(command)
		}
		_ => None,
	}
}

/// `fa preview` of one package.
pub(crate) fn preview_one(path: Option<&Path>, target: Option<&str>) -> i32 {
	let project = match project::locate(path) {
		Ok(project) => project,
		Err(message) => {
			crate::color::eprintln_message(&message);
			return 1;
		}
	};
	if !project.package || project.entry.is_some() {
		eprintln!(
			"fa preview: previews the application of a package; run a file with `fa run <file.fa>`"
		);
		return USAGE;
	}
	let configuration_for = |target: &str| configuration_for(&project.base, target);
	let candidates: Vec<String> = match target {
		Some(target) => vec![target.to_string()],
		None => {
			let configured = configuration_for("bun").targets;
			["native", "bun", "node", "wasm", "web"]
				.iter()
				.filter(|target| configured.iter().any(|other| other == *target))
				.map(|target| target.to_string())
				.collect()
		}
	};
	let mut looked = Vec::new();
	for target in &candidates {
		let configuration = configuration_for(target);
		let directory = project.base.join(&configuration.build.output).join(target);
		looked.push(dependencies::shown_path(&directory));
		let Some(mut command) = preview_command(target, &directory, configuration.develop.port)
		else {
			continue;
		};
		eprintln!("fa preview: running the `{target}` build");
		let _ = std::io::stdout().flush();
		return match command.status() {
			Ok(status) => exit_code(status),
			Err(error) => {
				eprintln!("fa preview: can't start the program: {error}");
				1
			}
		};
	}
	eprintln!(
		"error[CLI006]: nothing to preview: no build that runs here in {}\n  help: build it first with `fa build{}`",
		looked
			.iter()
			.map(|path| format!("`{path}`"))
			.collect::<Vec<_>>()
			.join(", "),
		target.map_or(String::new(), |target| format!(" --target={target}"))
	);
	1
}
