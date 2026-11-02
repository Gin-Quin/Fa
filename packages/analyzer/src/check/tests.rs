//! `test` blocks and `assert` (advanced brief §18).
//!
//! A test is declared in the scope it appears in, as a symbol of kind
//! `Test` named by its quoted name (`"adds two numbers"`, never an
//! identifier), so that two tests with the same name in the same namespace
//! are found by a scope lookup (`TEST002`). Its body is a block analyzed like
//! a function body whose errors are inferred (advanced brief D51): it gets
//! its own `FnCx`, and the flow after the test is the flow before it.

use std::sync::Arc;

use crate::check::{FnCx, Walker};
use crate::diagnostics::codes::advanced::{MOD014, TEST002};
use crate::ids::{NodeId, TypeId};
use crate::symbols::{ScopeKind, Symbol, SymbolKind};
use crate::syntax::{self, TemplatePart, TestModifier, View};

/// Whether a module is a test file (`math.test.fa` is the module `math.test`).
pub fn is_test_module(module: &str) -> bool {
	module.ends_with(".test") || module == "test"
}

impl Walker<'_> {
	/// `test "name" { … }`, `test skip "name" { … }`, `test only "name" { … }`,
	/// `@test "name" { … }`. The modifier only matters to the test runner.
	pub(super) fn check_test(
		&mut self,
		n: NodeId,
		name: NodeId,
		_modifier: Option<TestModifier>,
		body: NodeId,
		compile_time: bool,
	) -> TypeId {
		self.check(name, None);
		self.memo(n, |w| {
			w.declare_test(n, name);
			TypeId::VOID
		});
		// the body: its own function context, and no effect on the flow after it
		let before = self.body.flow.clone();
		// a test body has no value: `return` alone ends it
		self.body.functions.push(FnCx {
			unsupported_at_start: self.unsupported_seen,
			test: true,
			..FnCx::new(n, None, TypeId::VOID, self.body.scope)
		});
		self.check_stmt(body);
		self.body.functions.pop();
		self.body.flow = before;
		// `@test`: run while checking (comptime.rs)
		if compile_time {
			self.run_comptime_test(n, body);
		}
		TypeId::VOID
	}

	/// Declares a test (type part): `MOD014` outside a test file or below
	/// the top level and namespaces, `TEST002` for a name already taken.
	fn declare_test(&mut self, n: NodeId, name: NodeId) {
		let module = self.s.module(self.file).to_string();
		let scope_kind = self.a.symbols.scope(self.body.scope).kind;
		if !is_test_module(&module) {
			let diagnostic = self.diagnostic(
				&MOD014,
				n,
				format!(
					"tests are declared in test files: move it to `{module}.test.fa`, which sees the names of `{module}.fa`"
				),
			);
			self.report(diagnostic);
		} else if !matches!(scope_kind, ScopeKind::Module | ScopeKind::Namespace) {
			let diagnostic = self.diagnostic(
				&MOD014,
				n,
				"a test is declared at the top level of a test file, or in a namespace",
			);
			self.report(diagnostic);
		}
		let Some(text) = self.test_name(name) else {
			return;
		};
		let quoted = format!("{text:?}");
		let sym = self.s.types.syms.intern(&quoted);
		let symbol = Symbol::new(sym, SymbolKind::Test, n, self.span(name));
		let (_, previous) = self.a.symbols.declare(self.body.scope, symbol);
		if let Some(previous) = previous {
			let previous = self.a.symbols.symbol(previous).name_span;
			let diagnostic = self
				.diagnostic(
					&TEST002,
					name,
					format!(
						"there is already a test named {quoted} here: give this one another name"
					),
				)
				.with_related(
					crate::diagnostics::Location {
						file: self.file,
						span: previous,
					},
					"the other test",
				);
			self.report(diagnostic);
		}
	}

	/// The name of a test: the text of its string (an interpolation keeps
	/// its source text).
	fn test_name(&self, name: NodeId) -> Option<String> {
		let tree = Arc::clone(&self.tree);
		match syntax::view(&tree, name) {
			View::String(text) => Some(text.to_string()),
			View::Template { parts } => Some(
				parts
					.iter()
					.map(|part| match part {
						TemplatePart::Text(text) => text.to_string(),
						TemplatePart::Expression(value) => {
							format!("{{{}}}", tree.text(value.index()))
						}
					})
					.collect(),
			),
			_ => None,
		}
	}

	/// `assert condition`: a `Boolean` condition (`E-BOOL-1`, alias
	/// `TEST001`, reported by `check_cond`). A false `assert` stops the
	/// program, or fails the test: the code after it runs only when the
	/// condition is true, so it continues from the true state of the
	/// condition, with its narrowings (values brief D-27).
	pub(super) fn check_assert(&mut self, n: NodeId, condition: NodeId) -> TypeId {
		let (when_true, _) = self.check_cond(condition);
		self.body.flow = when_true;
		self.memo(n, |_| TypeId::VOID)
	}
}

#[cfg(test)]
mod unit_tests {
	use std::sync::Arc;

	use crate::session::Session;

	/// The codes of a test file's diagnostics.
	fn codes_in(module: &str, source: &str) -> Vec<&'static str> {
		let mut session = Session::new();
		let file = session.set_source(module, Arc::from(source));
		session.analyze_file(file);
		session
			.diagnostics(file)
			.iter()
			.map(|d| d.code.id)
			.collect()
	}

	#[test]
	fn tests_check_their_bodies() {
		let source = "test \"adds\" {\n\tlet a = 1\n\tassert true\n\tconsole.log(a)\n}\n\
		              test skip \"later\" { }\ntest only \"focus\" { }\n";
		assert!(codes_in("math.test", source).is_empty());
		assert_eq!(
			codes_in("math.test", "test \"a\" {\n\tconsole.log(missing)\n}\n"),
			["E-DECL-1"]
		);
	}

	#[test]
	fn test_names_are_unique_per_namespace() {
		assert_eq!(
			codes_in("x.test", "test \"a\" { }\ntest \"a\" { }\ntest \"b\" { }\n"),
			["TEST002"]
		);
		// a test name never clashes with a value
		assert!(codes_in("x.test", "let a = 1\ntest \"a\" { }\n").is_empty());
	}

	#[test]
	fn tests_live_at_the_top_level_of_test_files() {
		assert_eq!(codes_in("main", "test \"a\" { }\n"), ["MOD014"]);
		assert_eq!(
			codes_in("x.test", "test \"outer\" {\n\ttest \"inner\" { }\n}\n"),
			["MOD014"]
		);
	}

	#[test]
	fn the_body_of_a_test_does_not_leak_its_names() {
		assert_eq!(
			codes_in(
				"x.test",
				"test \"a\" {\n\tlet inner = 1\n}\ntest \"b\" {\n\tconsole.log(inner)\n}\n"
			),
			["E-DECL-1"]
		);
	}

	#[test]
	fn a_test_symbol_is_recorded() {
		let mut session = Session::new();
		let file = session.set_source("x.test", Arc::from("test \"adds\" { }\n"));
		session.analyze_file(file);
		let symbols = session.analysis(file).symbols().symbols();
		assert_eq!(symbols.len(), 1);
		assert_eq!(session.types().syms.resolve(symbols[0].name), "\"adds\"");
		assert_eq!(symbols[0].kind, crate::symbols::SymbolKind::Test);
	}
}
