//! Configuration files (advanced brief §16, decision D37): `package.fa`,
//! `workspace.fa` and `dependencies.fa`.
//!
//! Until the compile-time interpreter exists, a configuration file exports a
//! **literal**: strings, numbers, booleans, `none`, arrays, objects, and names
//! of `let` literals of the same file. Anything else is `CFG007`. The value is
//! checked against the configuration types of `architecture/configuration`
//! (`PackageConfiguration`, `BuildOptions`…): an unknown field is `STR002`
//! (with the closest field name), a value of the wrong type `E-TYPE-1`, a
//! missing `name` in `package.fa` `STR001`.
//!
//! Only the fields **written** are kept ([`ConfigValue`]); [`merge`] combines
//! the workspace's and the package's configurations for a target (§16.4), and
//! [`PackageConfiguration::from_value`] applies the defaults last (D32).

use std::sync::Arc;

use ethnum::I256;
use rustc_hash::FxHashMap;

use fa_parser::TypedSyntaxTree;

use crate::diagnostics::codes::advanced::CFG007;
use crate::diagnostics::codes::values::E_TYPE_1;
use crate::diagnostics::{Code, Diagnostic, Location, SourceSpan, codes};
use crate::ids::{FileId, NodeId};
use crate::syntax::{self, TemplatePart, View};

/// A configuration value: only what is written.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
	/// `none`.
	None,
	/// A boolean.
	Boolean(bool),
	/// An integer.
	Integer(i128),
	/// A float.
	Float(f64),
	/// A string.
	String(String),
	/// An array.
	Array(Vec<ConfigValue>),
	/// An object: its fields in the order written.
	Object(Vec<(String, ConfigValue)>),
}

impl ConfigValue {
	/// A field of an object.
	pub fn field(&self, name: &str) -> Option<&ConfigValue> {
		match self {
			ConfigValue::Object(fields) => fields
				.iter()
				.find(|(field, _)| field == name)
				.map(|(_, value)| value),
			_ => None,
		}
	}

	/// The text of a string.
	pub fn as_str(&self) -> Option<&str> {
		match self {
			ConfigValue::String(text) => Some(text),
			_ => None,
		}
	}

	/// A boolean.
	pub fn as_bool(&self) -> Option<bool> {
		match self {
			ConfigValue::Boolean(value) => Some(*value),
			_ => None,
		}
	}

	/// The strings of an array of strings.
	pub fn as_strings(&self) -> Option<Vec<String>> {
		match self {
			ConfigValue::Array(items) => items
				.iter()
				.map(|item| item.as_str().map(str::to_string))
				.collect(),
			_ => None,
		}
	}
}

/// The type of a configuration value (the configuration types of
/// `architecture/configuration`).
#[derive(Copy, Clone, Debug)]
enum Schema {
	String,
	Boolean,
	/// An integer within bounds.
	Integer(i128, i128),
	/// `"web" | "bun" | …`
	Target,
	Array(&'static Schema),
	/// An object type: its name and fields (all optional except `name` of a package).
	Object(&'static str, &'static [(&'static str, Schema)]),
	/// `DependencySource`: a string, or `{ source, generate }`.
	Dependency,
}

/// The targets (`Target`).
pub const TARGETS: &[&str] = &["web", "bun", "node", "deno", "native", "wasm", "rust"];

const STRINGS: Schema = Schema::Array(&Schema::String);
const TARGET_LIST: Schema = Schema::Array(&Schema::Target);

const C_LIBRARY: &[(&str, Schema)] = &[
	("header", Schema::String),
	("prefix", Schema::String),
	("api", Schema::String),
];
const BUILD: &[(&str, Schema)] = &[
	("optimize", Schema::Boolean),
	("overflowChecks", Schema::Boolean),
	("sourceMaps", Schema::Boolean),
	("output", Schema::String),
	("minify", Schema::Boolean),
	("link", STRINGS),
	("libraryPaths", STRINGS),
	("cLibrary", Schema::Object("CLibraryOptions", C_LIBRARY)),
];
const DEVELOP: &[(&str, Schema)] = &[
	("port", Schema::Integer(1, 65535)),
	("open", Schema::Boolean),
	("hotReload", Schema::Boolean),
];
const TEST: &[(&str, Schema)] = &[
	("include", STRINGS),
	("timeout", Schema::Integer(1, i64::MAX as i128)),
];
const OVERRIDE: &[(&str, Schema)] = &[
	("entry", Schema::String),
	("build", Schema::Object("BuildOptions", BUILD)),
	("develop", Schema::Object("DevelopOptions", DEVELOP)),
	("test", Schema::Object("TestOptions", TEST)),
];
const OVERRIDES: &[(&str, Schema)] = &[
	("web", Schema::Object("TargetOverride", OVERRIDE)),
	("bun", Schema::Object("TargetOverride", OVERRIDE)),
	("node", Schema::Object("TargetOverride", OVERRIDE)),
	("deno", Schema::Object("TargetOverride", OVERRIDE)),
	("native", Schema::Object("TargetOverride", OVERRIDE)),
	("wasm", Schema::Object("TargetOverride", OVERRIDE)),
	("rust", Schema::Object("TargetOverride", OVERRIDE)),
];
const PACKAGE: &[(&str, Schema)] = &[
	("name", Schema::String),
	("version", Schema::String),
	("targets", TARGET_LIST),
	("entry", Schema::String),
	("description", Schema::String),
	("license", Schema::String),
	("repository", Schema::String),
	("authors", STRINGS),
	("build", Schema::Object("BuildOptions", BUILD)),
	("develop", Schema::Object("DevelopOptions", DEVELOP)),
	("test", Schema::Object("TestOptions", TEST)),
	("overrides", Schema::Object("TargetOverrides", OVERRIDES)),
];

/// Which configuration file.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ConfigurationKind {
	/// `package.fa`: a `PackageConfiguration`.
	Package,
	/// `workspace.fa`: a `WorkspaceConfiguration` (the same fields, all optional).
	Workspace,
	/// `dependencies.fa`: an object of `DependencySource`s.
	Dependencies,
}

/// Evaluates a configuration file: its exported literal, checked against
/// its configuration type. The diagnostics are on `file`; the value is
/// `None` when the export is missing or not a literal.
pub fn evaluate(
	file: FileId,
	tree: &TypedSyntaxTree,
	kind: ConfigurationKind,
) -> (Option<ConfigValue>, Vec<Diagnostic>) {
	let mut evaluator = Evaluator {
		file,
		tree,
		lets: FxHashMap::default(),
		diagnostics: Vec::new(),
	};
	let root = NodeId::new(tree.root);
	let View::Module { statements } = syntax::view(tree, root) else {
		return (None, Vec::new());
	};
	let mut export = None;
	for statement in statements.iter() {
		match syntax::view(tree, statement) {
			View::Declaration {
				kind: syntax::BindingKind::Let,
				name,
				value: Some(value),
				..
			} => {
				evaluator.lets.insert(tree.name(name).to_string(), value);
			}
			View::ExportValue {
				mutable: false,
				value,
				..
			} if export.is_none() => export = Some(value),
			_ => {}
		}
	}
	let Some(export) = export else {
		let diagnostic = evaluator.diagnostic(
			codes::lookup("MOD005").expect("a registered code"),
			SourceSpan::new(0, 0),
			"a configuration file exports its configuration, ex: `export = { name = \"farm\" }`",
		);
		return (None, vec![diagnostic]);
	};
	let Some(value) = evaluator.value(export, 0) else {
		return (None, evaluator.diagnostics);
	};
	evaluator.check_export(export, value, kind)
}

/// Whether the diagnostics of [`evaluate`] say that the export is computed
/// (`CFG007`): the compile-time interpreter evaluates it instead.
pub fn is_computed(diagnostics: &[Diagnostic]) -> bool {
	diagnostics
		.iter()
		.any(|diagnostic| std::ptr::eq(diagnostic.code, &CFG007))
}

/// The export node of a configuration file.
pub fn export_node(tree: &TypedSyntaxTree) -> Option<NodeId> {
	let View::Module { statements } = syntax::view(tree, NodeId::new(tree.root)) else {
		return None;
	};
	statements.iter().find(|&statement| {
		matches!(
			syntax::view(tree, statement),
			View::ExportValue { mutable: false, .. }
		)
	})
}

/// Checks the value a configuration file's export computed (on the
/// compile-time interpreter) against its configuration type.
pub fn check_computed(
	file: FileId,
	tree: &TypedSyntaxTree,
	kind: ConfigurationKind,
	value: ConfigValue,
) -> (Option<ConfigValue>, Vec<Diagnostic>) {
	let evaluator = Evaluator {
		file,
		tree,
		lets: FxHashMap::default(),
		diagnostics: Vec::new(),
	};
	let Some(export) = export_node(tree) else {
		return (None, Vec::new());
	};
	evaluator.check_export(export, value, kind)
}

/// A configuration value as a compile-time value (`configuration`).
pub fn to_value(value: &ConfigValue) -> fa_interpreter::Value {
	use fa_interpreter::Value;
	match value {
		ConfigValue::None => Value::None,
		ConfigValue::Boolean(value) => Value::Boolean(*value),
		ConfigValue::Integer(value) => Value::Integer(I256::from(*value)),
		ConfigValue::Float(value) => Value::Float(*value),
		ConfigValue::String(text) => Value::string(text),
		ConfigValue::Array(items) => Value::array(items.iter().map(to_value).collect()),
		ConfigValue::Object(fields) => Value::object(
			fields
				.iter()
				.map(|(name, value)| (name.as_str(), to_value(value)))
				.collect(),
		),
	}
}

/// A compile-time value as a configuration value (`None` for a value a
/// configuration can't hold: a function, a map…).
pub fn from_value(value: &fa_interpreter::Value) -> Option<ConfigValue> {
	use fa_interpreter::Value;
	Some(match value {
		Value::None => ConfigValue::None,
		Value::Boolean(value) => ConfigValue::Boolean(*value),
		Value::Integer(value) => ConfigValue::Integer(i256_to_i128(*value)),
		Value::Float(value) => ConfigValue::Float(*value),
		Value::String(text) => ConfigValue::String(text.to_string()),
		Value::Array(items) => {
			ConfigValue::Array(items.iter().map(from_value).collect::<Option<_>>()?)
		}
		Value::Object(object) => ConfigValue::Object(
			object
				.fields
				.iter()
				// an omitted optional field stays absent (D32)
				.filter(|(_, value)| !matches!(value, Value::None))
				.map(|(name, value)| Some((name.to_string(), from_value(value)?)))
				.collect::<Option<_>>()?,
		),
		_ => return None,
	})
}

/// The defaults of `PackageConfiguration` (`architecture/configuration`), as
/// written fields: the value of `configuration` merges the written ones over them.
pub fn defaults() -> ConfigValue {
	let strings = |items: &[&str]| {
		ConfigValue::Array(
			items
				.iter()
				.map(|item| ConfigValue::String(item.to_string()))
				.collect(),
		)
	};
	let object = |fields: Vec<(&str, ConfigValue)>| {
		ConfigValue::Object(
			fields
				.into_iter()
				.map(|(name, value)| (name.to_string(), value))
				.collect(),
		)
	};
	object(vec![
		("name", ConfigValue::String(String::new())),
		("targets", strings(&["web", "bun", "native"])),
		("entry", ConfigValue::String("application/main.fa".into())),
		("authors", strings(&[])),
		(
			"build",
			object(vec![
				("optimize", ConfigValue::Boolean(true)),
				("sourceMaps", ConfigValue::Boolean(false)),
				("output", ConfigValue::String("build".into())),
				("minify", ConfigValue::Boolean(true)),
				("link", strings(&[])),
				("libraryPaths", strings(&[])),
			]),
		),
		(
			"develop",
			object(vec![
				("port", ConfigValue::Integer(3000)),
				("open", ConfigValue::Boolean(false)),
				("hotReload", ConfigValue::Boolean(true)),
			]),
		),
		(
			"test",
			object(vec![
				("include", strings(&["**/*.test.fa"])),
				("timeout", ConfigValue::Integer(5000)),
			]),
		),
		("overrides", object(Vec::new())),
	])
}

/// Evaluates the configuration file of a text (tests, tools).
pub fn evaluate_source(
	text: &str,
	kind: ConfigurationKind,
) -> (Option<ConfigValue>, Vec<Diagnostic>) {
	let tree = fa_parser::parse_shared(Arc::from(text));
	evaluate(FileId::new(0), &tree, kind)
}

/// The literal a Fa text exports, with no configuration type: the files
/// `fa` generates or reads as data (`dependencies.lock`, the `manifest.fa`
/// of a remote folder). `None` when the export is missing or computed, or
/// the text has a syntax error.
pub fn evaluate_literal(text: &str) -> Option<ConfigValue> {
	let tree = fa_parser::parse_shared(Arc::from(text));
	if tree
		.diagnostics
		.iter()
		.any(|diagnostic| diagnostic.severity == fa_parser::Severity::Error)
	{
		return None;
	}
	let mut evaluator = Evaluator {
		file: FileId::new(0),
		tree: &tree,
		lets: FxHashMap::default(),
		diagnostics: Vec::new(),
	};
	let export = export_node(&tree)?;
	let value = match syntax::view(&tree, export) {
		View::ExportValue { value, .. } => value,
		_ => return None,
	};
	let value = evaluator.value(value, 0)?;
	evaluator.diagnostics.is_empty().then_some(value)
}

struct Evaluator<'t> {
	file: FileId,
	tree: &'t TypedSyntaxTree,
	/// The `let` declarations of the file, by name: their values.
	lets: FxHashMap<String, NodeId>,
	diagnostics: Vec<Diagnostic>,
}

impl Evaluator<'_> {
	/// Checks the value of the export against the configuration type.
	fn check_export(
		mut self,
		export: NodeId,
		value: ConfigValue,
		kind: ConfigurationKind,
	) -> (Option<ConfigValue>, Vec<Diagnostic>) {
		let schema = match kind {
			ConfigurationKind::Package | ConfigurationKind::Workspace => Schema::Object(
				if kind == ConfigurationKind::Package {
					"PackageConfiguration"
				} else {
					"WorkspaceConfiguration"
				},
				PACKAGE,
			),
			ConfigurationKind::Dependencies => Schema::Dependency,
		};
		if kind == ConfigurationKind::Dependencies {
			self.check_dependencies(export, &value);
		} else {
			self.check(export, &value, schema);
			if kind == ConfigurationKind::Package && value.field("name").is_none() {
				let span = syntax::span(self.tree, export);
				let diagnostic = self.diagnostic(
					codes::lookup("STR001").expect("a registered code"),
					span,
					"a package is named: add its `name`, ex: `name = \"farm\"`",
				);
				self.diagnostics.push(diagnostic);
			}
		}
		let ok = self
			.diagnostics
			.iter()
			.all(|diagnostic| diagnostic.severity != crate::Severity::Error);
		(ok.then_some(value), self.diagnostics)
	}

	fn diagnostic(&self, code: Code, span: SourceSpan, message: impl Into<String>) -> Diagnostic {
		Diagnostic::new(
			code,
			Location {
				file: self.file,
				span,
			},
			message,
		)
	}

	/// `CFG007` on a node.
	fn computed(&mut self, node: NodeId) {
		let span = syntax::span(self.tree, node);
		let diagnostic = self.diagnostic(
			&CFG007,
			span,
			"computed configuration needs the compile-time interpreter (not available yet): write a literal",
		);
		self.diagnostics.push(diagnostic);
	}

	/// The literal value of a node.
	fn value(&mut self, node: NodeId, depth: u32) -> Option<ConfigValue> {
		if depth > 64 {
			self.computed(node);
			return None;
		}
		let tree = self.tree;
		Some(match syntax::view(tree, node) {
			View::None => ConfigValue::None,
			View::Boolean(value) => ConfigValue::Boolean(value),
			View::Integer(value) => ConfigValue::Integer(i256_to_i128(value)),
			View::Float(value) => ConfigValue::Float(value),
			View::String(text) => ConfigValue::String(text.to_string()),
			View::Template { parts } => {
				// a template of literal parts only
				let mut text = String::new();
				for part in parts.iter() {
					match part {
						TemplatePart::Text(literal) => text.push_str(literal),
						TemplatePart::Expression(_) => {
							self.computed(node);
							return None;
						}
					}
				}
				ConfigValue::String(text)
			}
			View::Group(inner) => return self.value(inner, depth + 1),
			View::Negate(inner) => match self.value(inner, depth + 1)? {
				ConfigValue::Integer(value) => ConfigValue::Integer(-value),
				ConfigValue::Float(value) => ConfigValue::Float(-value),
				_ => {
					self.computed(node);
					return None;
				}
			},
			View::Array { items } => {
				let mut values = Vec::new();
				for item in items.iter() {
					values.push(self.value(item, depth + 1)?);
				}
				ConfigValue::Array(values)
			}
			View::Object { items } => {
				let mut fields: Vec<(String, ConfigValue)> = Vec::new();
				for item in items.iter() {
					let (name, value) = match syntax::view(tree, item) {
						View::Assignment {
							target,
							annotation: None,
							value: Some(value),
						} => match syntax::view(tree, target) {
							View::Name(name) => (tree.name(name).to_string(), value),
							_ => {
								self.computed(item);
								return None;
							}
						},
						// `{ license }`: the `let` of that name
						View::Name(name) => (tree.name(name).to_string(), item),
						_ => {
							self.computed(item);
							return None;
						}
					};
					let value = self.value(value, depth + 1)?;
					fields.retain(|(field, _)| *field != name);
					fields.push((name, value));
				}
				ConfigValue::Object(fields)
			}
			View::Name(name) => {
				let text = tree.name(name);
				match self.lets.get(text).copied() {
					Some(value) => return self.value(value, depth + 1),
					None => {
						self.computed(node);
						return None;
					}
				}
			}
			_ => {
				self.computed(node);
				return None;
			}
		})
	}

	/// The node of the value of field `name` in an object literal (for the
	/// location of a diagnostic), following `let` names.
	fn field_node(&self, object: NodeId, name: &str) -> Option<(NodeId, NodeId)> {
		let tree = self.tree;
		let object = self.literal_node(object);
		let View::Object { items } = syntax::view(tree, object) else {
			return None;
		};
		items
			.iter()
			.rev()
			.find_map(|item| {
				match syntax::view(tree, item) {
			View::Assignment {
				target,
				value: Some(value),
				..
			} => matches!(syntax::view(tree, target), View::Name(field) if tree.name(field) == name)
				.then_some((target, value)),
			View::Name(field) if tree.name(field) == name => Some((item, item)),
			_ => None,
		}
			})
	}

	/// The literal a node stands for (through groups and `let` names).
	fn literal_node(&self, node: NodeId) -> NodeId {
		let mut node = node;
		for _ in 0..64 {
			match syntax::view(self.tree, node) {
				View::Group(inner) => node = inner,
				View::Name(name) => match self.lets.get(self.tree.name(name)) {
					Some(&value) => node = value,
					None => return node,
				},
				_ => return node,
			}
		}
		node
	}

	/// The item nodes of an array literal.
	fn item_nodes(&self, node: NodeId) -> Vec<NodeId> {
		match syntax::view(self.tree, self.literal_node(node)) {
			View::Array { items } => items.iter().collect(),
			_ => Vec::new(),
		}
	}

	/// A type error on a node.
	fn mismatch(&mut self, node: NodeId, expected: &str) {
		let span = syntax::span(self.tree, node);
		let diagnostic = self.diagnostic(
			&E_TYPE_1,
			span,
			format!("this value is not accepted where a `{expected}` is expected"),
		);
		self.diagnostics.push(diagnostic);
	}

	/// Checks a value against its schema.
	fn check(&mut self, node: NodeId, value: &ConfigValue, schema: Schema) {
		match (schema, value) {
			(Schema::String, ConfigValue::String(_))
			| (Schema::Boolean, ConfigValue::Boolean(_)) => {}
			(Schema::Integer(min, max), ConfigValue::Integer(number)) => {
				if *number < min || *number > max {
					self.mismatch(node, &format!("Integer(min = {min}, max = {max})"));
				}
			}
			(Schema::Target, ConfigValue::String(target)) => {
				if !TARGETS.contains(&target.as_str()) {
					let span = syntax::span(self.tree, node);
					let diagnostic = self.diagnostic(
						&E_TYPE_1,
						span,
						format!(
							"`{target}` is not a target: the targets are {}",
							TARGETS
								.iter()
								.map(|target| format!("`{target}`"))
								.collect::<Vec<_>>()
								.join(", ")
						),
					);
					self.diagnostics.push(diagnostic);
				}
			}
			(Schema::Array(item), ConfigValue::Array(items)) => {
				let nodes = self.item_nodes(node);
				for (index, value) in items.iter().enumerate() {
					let node = nodes.get(index).copied().unwrap_or(node);
					self.check(node, value, *item);
				}
			}
			(Schema::Object(type_name, fields), ConfigValue::Object(values)) => {
				for (name, value) in values {
					let (name_node, value_node) =
						self.field_node(node, name).unwrap_or((node, node));
					match fields.iter().find(|(field, _)| field == name) {
						Some((_, schema)) => self.check(value_node, value, *schema),
						None => {
							let suggestion = closest(name, fields.iter().map(|(field, _)| *field))
								.map(|field| format!(": did you mean `{field}`?"))
								.unwrap_or_default();
							let span = syntax::span(self.tree, name_node);
							let diagnostic = self.diagnostic(
								codes::lookup("STR002").expect("a registered code"),
								span,
								format!("`{name}` is not a field of `{type_name}`{suggestion}"),
							);
							self.diagnostics.push(diagnostic);
						}
					}
				}
			}
			(Schema::Dependency, ConfigValue::String(_) | ConfigValue::Object(_)) => {}
			(schema, _) => {
				let expected = match schema {
					Schema::String => "String".to_string(),
					Schema::Boolean => "Boolean".to_string(),
					Schema::Integer(..) => "Integer".to_string(),
					Schema::Target => "Target".to_string(),
					Schema::Array(_) => "Array".to_string(),
					Schema::Object(name, _) => name.to_string(),
					Schema::Dependency => "DependencySource".to_string(),
				};
				self.mismatch(node, &expected);
			}
		}
	}

	/// `dependencies.fa`: an object of `DependencySource`s.
	fn check_dependencies(&mut self, node: NodeId, value: &ConfigValue) {
		let ConfigValue::Object(fields) = value else {
			self.mismatch(node, "{ name = DependencySource }");
			return;
		};
		for (name, source) in fields {
			let (_, value_node) = self.field_node(node, name).unwrap_or((node, node));
			self.check(value_node, source, Schema::Dependency);
		}
	}
}

/// An `I256` literal as an `i128` (saturating: a configuration never needs more).
fn i256_to_i128(value: I256) -> i128 {
	if value > I256::from(i128::MAX) {
		i128::MAX
	} else if value < I256::from(i128::MIN) {
		i128::MIN
	} else {
		value.as_i128()
	}
}

/// The closest name to a misspelled one (edit distance ≤ 2, or a prefix).
fn closest<'a>(name: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
	candidates
		.map(|candidate| (distance(name, candidate), candidate))
		.filter(|&(distance, candidate)| {
			distance <= 2 || candidate.starts_with(name) || name.starts_with(candidate)
		})
		.min_by_key(|&(distance, _)| distance)
		.map(|(_, candidate)| candidate)
}

/// The edit distance between two names.
fn distance(a: &str, b: &str) -> usize {
	let b: Vec<char> = b.chars().collect();
	let mut row: Vec<usize> = (0..=b.len()).collect();
	for (i, ca) in a.chars().enumerate() {
		let mut previous = row[0];
		row[0] = i + 1;
		for (j, &cb) in b.iter().enumerate() {
			let current = row[j + 1];
			row[j + 1] = (previous + usize::from(ca != cb))
				.min(row[j] + 1)
				.min(current + 1);
			previous = current;
		}
	}
	row[b.len()]
}

/// `a <- b` (§16.4): objects merge field by field at every depth; any other
/// value of `b` (arrays included) replaces `a`'s.
pub fn merge_values(a: &ConfigValue, b: &ConfigValue) -> ConfigValue {
	match (a, b) {
		(ConfigValue::Object(base), ConfigValue::Object(over)) => {
			let mut fields = base.clone();
			for (name, value) in over {
				match fields.iter_mut().find(|(field, _)| field == name) {
					Some((_, existing)) => *existing = merge_values(existing, value),
					None => fields.push((name.clone(), value.clone())),
				}
			}
			ConfigValue::Object(fields)
		}
		(_, b) => b.clone(),
	}
}

/// The configuration of a package for a target (§16.4): the workspace's,
/// then the package's, then their `overrides` of the target (`entry`,
/// `build`, `develop`, `test`). The defaults are applied by
/// [`PackageConfiguration::from_value`].
pub fn merge(workspace: Option<&ConfigValue>, package: &ConfigValue, target: &str) -> ConfigValue {
	let empty = ConfigValue::Object(Vec::new());
	let without_overrides = |value: &ConfigValue| match value {
		ConfigValue::Object(fields) => ConfigValue::Object(
			fields
				.iter()
				.filter(|(name, _)| name != "overrides")
				.cloned()
				.collect(),
		),
		other => other.clone(),
	};
	let mut merged = without_overrides(workspace.unwrap_or(&empty));
	merged = merge_values(&merged, &without_overrides(package));
	for source in [workspace, Some(package)].into_iter().flatten() {
		if let Some(over) = source
			.field("overrides")
			.and_then(|overrides| overrides.field(target))
		{
			merged = merge_values(&merged, over);
		}
	}
	merged
}

/// `BuildOptions` with their defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct BuildOptions {
	/// Optimize for speed.
	pub optimize: bool,
	/// Check the possible overflows at runtime (`None`: the command's default).
	pub overflow_checks: Option<bool>,
	/// Emit source maps.
	pub source_maps: bool,
	/// The output folder.
	pub output: String,
	/// Minify the JavaScript and WebAssembly outputs.
	pub minify: bool,
	/// The C libraries to link.
	pub link: Vec<String>,
	/// Extra folders where the C libraries are searched.
	pub library_paths: Vec<String>,
}

/// `TestOptions` with their defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct TestOptions {
	/// The globs of the test files, relative to the package.
	pub include: Vec<String>,
	/// The timeout of each test, in milliseconds.
	pub timeout: u64,
}

/// A `PackageConfiguration` merged for a target, defaults applied.
#[derive(Clone, Debug, PartialEq)]
pub struct PackageConfiguration {
	/// The name of the package.
	pub name: String,
	/// Its version.
	pub version: Option<String>,
	/// The targets it is built for.
	pub targets: Vec<String>,
	/// The entry file of the application.
	pub entry: String,
	/// Its description.
	pub description: Option<String>,
	/// Its license.
	pub license: Option<String>,
	/// Where its source lives.
	pub repository: Option<String>,
	/// Its authors.
	pub authors: Vec<String>,
	/// The options of `fa build`.
	pub build: BuildOptions,
	/// The options of `fa test`.
	pub test: TestOptions,
	/// The options of `fa develop`.
	pub develop: DevelopOptions,
}

/// `DevelopOptions` with their defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct DevelopOptions {
	/// The port of the development server (`web`).
	pub port: u16,
	/// Open a browser on start.
	pub open: bool,
	/// Reload the changed modules instead of restarting.
	pub hot_reload: bool,
}

impl PackageConfiguration {
	/// The configuration of a merged value, with the defaults of the
	/// fields not written.
	pub fn from_value(value: &ConfigValue) -> Self {
		let string = |value: &ConfigValue, name: &str| {
			value
				.field(name)
				.and_then(ConfigValue::as_str)
				.map(str::to_string)
		};
		let empty = ConfigValue::Object(Vec::new());
		let build = value.field("build").unwrap_or(&empty);
		let test = value.field("test").unwrap_or(&empty);
		let develop = value.field("develop").unwrap_or(&empty);
		let flag = |object: &ConfigValue, name: &str, default: bool| {
			object
				.field(name)
				.and_then(ConfigValue::as_bool)
				.unwrap_or(default)
		};
		let strings = |object: &ConfigValue, name: &str| {
			object
				.field(name)
				.and_then(ConfigValue::as_strings)
				.unwrap_or_default()
		};
		Self {
			name: string(value, "name").unwrap_or_default(),
			version: string(value, "version"),
			targets: value
				.field("targets")
				.and_then(ConfigValue::as_strings)
				.unwrap_or_else(|| vec!["web".into(), "bun".into(), "native".into()]),
			entry: string(value, "entry").unwrap_or_else(|| "application/main.fa".into()),
			description: string(value, "description"),
			license: string(value, "license"),
			repository: string(value, "repository"),
			authors: strings(value, "authors"),
			build: BuildOptions {
				optimize: flag(build, "optimize", true),
				overflow_checks: build.field("overflowChecks").and_then(ConfigValue::as_bool),
				source_maps: flag(build, "sourceMaps", false),
				output: string(build, "output").unwrap_or_else(|| "build".into()),
				minify: flag(build, "minify", true),
				link: strings(build, "link"),
				library_paths: strings(build, "libraryPaths"),
			},
			test: TestOptions {
				include: test
					.field("include")
					.and_then(ConfigValue::as_strings)
					.unwrap_or_else(|| vec!["**/*.test.fa".into()]),
				timeout: match test.field("timeout") {
					Some(ConfigValue::Integer(timeout)) if *timeout > 0 => *timeout as u64,
					_ => 5000,
				},
			},
			develop: DevelopOptions {
				port: match develop.field("port") {
					Some(ConfigValue::Integer(port)) if (1..=65535).contains(port) => *port as u16,
					_ => 3000,
				},
				open: flag(develop, "open", false),
				hot_reload: flag(develop, "hotReload", true),
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn codes_of(text: &str, kind: ConfigurationKind) -> Vec<&'static str> {
		evaluate_source(text, kind)
			.1
			.iter()
			.map(|diagnostic| diagnostic.code.id)
			.collect()
	}

	#[test]
	fn a_literal_configuration_is_evaluated() {
		let (value, diagnostics) = evaluate_source(
			"let version = \"2.1.0\"\nexport = {\n\tname = \"farm\"\n\tversion = version\n\ttargets = [\"web\", \"native\"]\n\tbuild = { overflowChecks = true, output = \"out\" }\n\ttest = { timeout = 100 }\n}\n",
			ConfigurationKind::Package,
		);
		assert!(diagnostics.is_empty(), "{diagnostics:?}");
		let configuration = PackageConfiguration::from_value(&value.unwrap());
		assert_eq!(configuration.name, "farm");
		assert_eq!(configuration.version.as_deref(), Some("2.1.0"));
		assert_eq!(configuration.targets, ["web", "native"]);
		assert_eq!(configuration.entry, "application/main.fa");
		assert_eq!(configuration.build.overflow_checks, Some(true));
		assert_eq!(configuration.build.output, "out");
		assert!(configuration.build.optimize);
		assert_eq!(configuration.test.timeout, 100);
		assert_eq!(configuration.test.include, ["**/*.test.fa"]);
		assert_eq!(configuration.develop.port, 3000);
		assert!(configuration.develop.hot_reload);
	}

	#[test]
	fn a_literal_is_read_without_a_configuration_type() {
		let value = evaluate_literal(
			"-- dependencies.lock: generated by `fa`, do not edit\nexport = [\n\t{\n\t\tsource = \"github:farmers/farm\"\n\t\tversion = \"4.2.1\"\n\t}\n]\n",
		)
		.unwrap();
		let ConfigValue::Array(items) = value else {
			panic!("an array");
		};
		assert_eq!(
			items[0].field("source").and_then(ConfigValue::as_str),
			Some("github:farmers/farm")
		);
		assert_eq!(evaluate_literal("export = f()\n"), None);
		assert_eq!(evaluate_literal("export = [\n"), None);
	}

	#[test]
	fn configuration_errors() {
		assert_eq!(
			codes_of(
				"export = { name = \"farm\", target = [\"web\"] }\n",
				ConfigurationKind::Package
			),
			["E-OBJ-1"]
		);
		let (_, diagnostics) = evaluate_source(
			"export = { name = \"farm\", target = [\"web\"] }\n",
			ConfigurationKind::Package,
		);
		assert!(diagnostics[0].message.contains("did you mean `targets`"));
		assert_eq!(
			codes_of(
				"function f = () => [\"web\"]\nexport = { name = \"farm\", targets = f() }\n",
				ConfigurationKind::Package
			),
			["CFG007"]
		);
		assert_eq!(
			codes_of("export = { version = \"1\" }\n", ConfigurationKind::Package),
			["STR001"]
		);
		assert!(
			codes_of(
				"export = { version = \"1\" }\n",
				ConfigurationKind::Workspace
			)
			.is_empty()
		);
		assert_eq!(
			codes_of(
				"export = { name = \"x\", targets = [\"mars\"], build = { minify = 1 } }\n",
				ConfigurationKind::Package
			),
			["E-TYPE-1", "E-TYPE-1"]
		);
		assert!(
			codes_of(
				"export = { farm = \"github:farmers/farm@4\" }\n",
				ConfigurationKind::Dependencies
			)
			.is_empty()
		);
	}

	#[test]
	fn configurations_merge_for_a_target() {
		let (workspace, _) = evaluate_source(
			"export = { name = \"myWorkspace\", version = \"1.0.0\", license = \"MIT\", targets = [\"web\", \"native\"], build = { minify = false } }\n",
			ConfigurationKind::Workspace,
		);
		let (package, _) = evaluate_source(
			"export = { name = \"tool\", targets = [\"native\"], build = { output = \"out\" }, overrides = { native = { build = { optimize = false } } } }\n",
			ConfigurationKind::Package,
		);
		let merged = merge(workspace.as_ref(), &package.unwrap(), "native");
		let configuration = PackageConfiguration::from_value(&merged);
		assert_eq!(configuration.name, "tool");
		assert_eq!(configuration.version.as_deref(), Some("1.0.0"));
		assert_eq!(configuration.license.as_deref(), Some("MIT"));
		assert_eq!(configuration.targets, ["native"]);
		assert_eq!(configuration.build.output, "out");
		assert!(!configuration.build.minify);
		assert!(!configuration.build.optimize, "the override of the target");
		assert!(!configuration.build.source_maps);
	}
}
