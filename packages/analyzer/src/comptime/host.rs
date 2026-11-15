//! Evaluation outside of a walk: the export of a computed configuration
//! file (advanced brief §16.3, replacing `CFG007` of D37), once every file
//! it uses is analyzed.

use std::sync::Arc;

use fa_interpreter::ir::{FnKey, Function, GlobalKey};
use fa_interpreter::{Error, Host, Interpreter, Limits, Value};

use crate::comptime::GlobalState;
use crate::comptime::lower::Lowerer;
use crate::ids::{FileId, NodeId};
use crate::session::Session;
use crate::syntax::{self, View};

/// The interpreter's host over a session whose files are analyzed.
pub struct SessionHost<'s> {
	/// The session.
	pub session: &'s mut Session,
}

impl SessionHost<'_> {
	fn lower(&self, key: FnKey) -> Option<Function> {
		let instance = self
			.session
			.comptime
			.instances
			.lock()
			.expect("the compile-time instance registry")
			.get(key)
			.cloned();
		let source = instance.as_ref().map_or(key, |instance| instance.source);
		let context = instance.map_or_else(super::instances::Context::default, |instance| {
			instance.context
		});
		let (file, node) = (FileId(source.0), NodeId(source.1));
		let source = self.session.source_file(file);
		if !source.done {
			return None;
		}
		let tree = Arc::clone(&source.tree);
		Lowerer::new(self.session, file, &tree, self.session.analysis(file))
			.with_instance(context)
			.function(node)
	}

	/// Evaluates the value node `n` of `file` (module-level values allowed).
	pub fn evaluate(&mut self, file: FileId, n: NodeId) -> Result<Value, Error> {
		let tree = Arc::clone(&self.session.source_file(file).tree);
		let (expr, slots) = Lowerer::new(self.session, file, &tree, self.session.analysis(file))
			.with_globals()
			.compile_time_expression(n)
			.map_err(|not_known| Error::Unsupported(format!("`{}`", not_known.name)))?;
		Interpreter::new(self, Limits::default()).evaluate(&expr, slots)
	}
}

impl Host for SessionHost<'_> {
	fn function(&mut self, key: FnKey) -> Result<Arc<Function>, Error> {
		if let Some(function) = self.session.comptime.functions.get(&key) {
			return Ok(Arc::clone(function));
		}
		let function = self
			.lower(key)
			.ok_or_else(|| Error::Unsupported("this function".into()))?;
		let function = Arc::new(function);
		self.session
			.comptime
			.functions
			.insert(key, Arc::clone(&function));
		Ok(function)
	}

	fn instantiate(&mut self, function: FnKey, application: FnKey) -> Result<FnKey, Error> {
		super::instances::instantiate(self.session, None, function, application)
	}

	fn bind_partial(&mut self, function: FnKey, site: FnKey) -> Result<FnKey, Error> {
		super::instances::bind_partial(self.session, None, function, site)
	}

	fn global(&mut self, key: GlobalKey) -> Result<Value, Error> {
		match self.session.comptime.globals.get(&key) {
			Some(GlobalState::Done(result)) => return result.clone(),
			Some(GlobalState::InProgress) => {
				return Err(Error::Stop("a module-level value depends on itself".into()));
			}
			None => {}
		}
		self.session
			.comptime
			.globals
			.insert(key, GlobalState::InProgress);
		let (file, node) = (FileId(key.0), NodeId(key.1));
		let tree = Arc::clone(&self.session.source_file(file).tree);
		let result = match syntax::view(&tree, node) {
			View::Declaration {
				value: Some(value), ..
			}
			| View::ExportValue { value, .. }
			| View::Function { value, .. }
			| View::ExportFunction { value, .. } => self.evaluate(file, value),
			_ => Err(Error::Unsupported("this module-level value".into())),
		};
		self.session
			.comptime
			.globals
			.insert(key, GlobalState::Done(result.clone()));
		result
	}
}

/// Evaluates the export of a configuration file on the interpreter: its
/// value, or the error and the node to report it at.
pub fn evaluate_export(session: &mut Session, file: FileId) -> Result<Value, Box<(NodeId, Error)>> {
	let tree = Arc::clone(&session.source_file(file).tree);
	let Some(export) = crate::configuration::export_node(&tree) else {
		return Err(Box::new((
			NodeId::new(tree.root),
			Error::Unsupported("a missing export".into()),
		)));
	};
	let View::ExportValue { value, .. } = syntax::view(&tree, export) else {
		return Err(Box::new((export, Error::Unsupported("this export".into()))));
	};
	fa_interpreter::on_large_stack(|| SessionHost { session }.evaluate(file, value))
		.map_err(|error| Box::new((value, error)))
}

/// A source of `dependencies.fa`, evaluated (architecture/dependencies.md,
/// `DependencySource`).
#[derive(Clone, Debug)]
pub enum DependencySource {
	/// A source written as a string.
	Source(String),
	/// `{ source, generate }`: the source is fetched, then `generate` turns
	/// its content into the Fa source of the dependency.
	Generated { source: String, generate: Value },
}

/// The sources of an analyzed `dependencies.fa` file, by key, its export
/// evaluated on the interpreter; `Err` holds the message of what is not a
/// `DependencySource`.
pub fn dependency_sources(
	session: &mut Session,
	file: FileId,
) -> Result<Vec<(String, DependencySource)>, String> {
	let value = evaluate_export(session, file).map_err(|failure| failure.1.message())?;
	let Value::Object(object) = value else {
		return Err(
			"`dependencies.fa` exports an object: `export = { farm = \"github:farmers/farm@4\" }`"
				.to_string(),
		);
	};
	let mut sources = Vec::new();
	for (key, value) in &object.fields {
		let source = match value {
			Value::String(text) => DependencySource::Source(text.to_string()),
			Value::Object(fields) => match (fields.get("source"), fields.get("generate")) {
				(Some(Value::String(source)), Some(generate @ Value::Function(_))) => {
					DependencySource::Generated {
						source: source.to_string(),
						generate: generate.clone(),
					}
				}
				(Some(Value::String(source)), None) => DependencySource::Source(source.to_string()),
				_ => {
					return Err(format!(
						"the source of `{key}` is a string, or `{{ source = \"https://…\", generate = (content: String): String => … }}`"
					));
				}
			},
			_ => {
				return Err(format!(
					"the source of `{key}` is a string, ex: `\"github:owner/repository@4\"`"
				));
			}
		};
		sources.push((key.to_string(), source));
	}
	Ok(sources)
}

/// Runs the `generate` function of a `DependencySource` on the content of
/// its source, on the interpreter (pure, bounded like any compile-time
/// evaluation): the Fa source of the dependency, or the message of its
/// failure.
pub fn run_generate(
	session: &mut Session,
	generate: &Value,
	content: &str,
) -> Result<String, String> {
	let result = fa_interpreter::on_large_stack(|| {
		let mut host = SessionHost { session };
		Interpreter::new(&mut host, Limits::default())
			.call_value(generate, vec![Value::string(content)])
	});
	match result {
		Ok(Value::String(text)) => Ok(text.to_string()),
		Ok(other) => Err(format!("it returned `{}`, not a `String`", other.to_text())),
		Err(error) => Err(error.message()),
	}
}

#[cfg(test)]
mod tests {
	use crate::CodebaseResolver;
	use crate::configuration::ConfigValue;
	use crate::session::Session;

	fn configuration(package: &str) -> (Option<ConfigValue>, Vec<String>) {
		let files = vec![
			("package.fa".to_string(), package.to_string()),
			(
				"application/main.fa".to_string(),
				"export function = () => {}\n".to_string(),
			),
		];
		let resolver = CodebaseResolver::from_texts(&files, Some("application/main.fa"));
		let mut session = Session::new();
		session.set_resolver(Box::new(resolver));
		let file = session.file_of_module("package").unwrap();
		session.analyze_file(file);
		let codes = session
			.diagnostics(file)
			.iter()
			.map(|diagnostic| diagnostic.code.id.to_string())
			.collect();
		(session.source_file(file).configuration.clone(), codes)
	}

	#[test]
	fn computed_configurations_run_on_the_interpreter() {
		let (value, codes) = configuration(
			"let version = \"2.1.0\"\nfunction withServerTargets = (targets: Array<Target>): Array<Target> => [...targets, \"bun\", \"node\"]\nexport = {\n\tname = \"farm\"\n\tversion = version\n\tdescription = \"Farm utilities, version {version}\"\n\ttargets = withServerTargets([\"web\"])\n}\n",
		);
		assert!(codes.is_empty(), "{codes:?}");
		let value = value.expect("a configuration");
		assert_eq!(
			value.field("targets").and_then(ConfigValue::as_strings),
			Some(vec![
				"web".to_string(),
				"bun".to_string(),
				"node".to_string()
			])
		);
		assert_eq!(
			value.field("description").and_then(ConfigValue::as_str),
			Some("Farm utilities, version 2.1.0")
		);
		// still checked against `PackageConfiguration`
		let (_, codes) = configuration(
			"function name = (): String => \"farm\"\nexport = { name = name(), port = 1 }\n",
		);
		assert_eq!(codes, ["E-OBJ-1"]);
	}
}
