//! Statements, blocks and control flow (values brief §5, §17, §18, §19).
//!
//! A construct whose value is used is generated *into a destination*
//! ([`Dest`]): the value of its last expression is returned, assigned to a
//! name or dropped, so that `let x = if c { … } else { … }` becomes
//! `let x; if (c) { …; x = 1; } else { …; x = 2; }` and a function body
//! ending with an `if` returns from each branch, without temporaries.

use fa_analyzer::analysis::NodeFlags;
use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};

use crate::generate::{Dest, Generator, Kind, LoopCx};
use crate::js::{Expr, Prec};

impl Generator<'_> {
	/// A list of statements; the value of the last one goes to `dest`.
	pub(crate) fn statements(&mut self, statements: &[NodeId], dest: Dest) {
		let statements: Vec<NodeId> = statements
			.iter()
			.copied()
			.filter(|&statement| !matches!(self.view(statement), View::Test { .. }))
			.collect();
		let Some((&last, rest)) = statements.split_last() else {
			return;
		};
		let mut skip = false;
		for (index, &statement) in rest.iter().enumerate() {
			if std::mem::take(&mut skip) {
				continue;
			}
			// a byte array and its fill loop (`typed_arrays.rs`); the loop
			// is never the value of the block
			if let Some(&next) = statements.get(index + 1)
				&& (index + 2 < statements.len() || dest == Dest::Discard)
				&& let Some(fill) = self.byte_fill(statement, next)
			{
				self.byte_array_declaration(fill);
				if self.flushes_after(statement) || self.flushes_after(next) {
					self.flush();
				}
				if index + 2 == statements.len() {
					return;
				}
				skip = true;
				continue;
			}
			// a `watch` block disposed at the end of the block (reactivity.rs)
			if self.scoped_watch(statement) {
				self.scoped_watch_statements(statement, &statements[index + 1..], dest);
				return;
			}
			self.guarded(|g| g.statement(statement));
			if self.flushes_after(statement) {
				self.flush();
			}
		}
		if self.scoped_watch(last) {
			self.scoped_watch_statements(last, &[], dest);
			return;
		}
		if !self.flushes_after(last) {
			self.guarded(|g| g.into(last, dest));
			return;
		}
		// the pending `watch` blocks run after the last statement too
		if dest == Dest::Return {
			self.out.open("try");
			self.into(last, dest);
			let helper = self.helper("faFlush");
			self.out.close(&format!("finally {{ {helper}(); }}"));
		} else {
			self.into(last, dest);
			self.flush();
		}
	}

	/// The statements of a block (without its braces), in a new scope.
	pub(crate) fn block_into(&mut self, block: NodeId, dest: Dest) {
		self.push_scope();
		match self.view(block) {
			View::Block { statements } => {
				let statements: Vec<NodeId> = statements.iter().collect();
				self.statements(&statements, dest);
			}
			_ => self.into(block, dest),
		}
		self.pop_scope();
	}

	/// A statement: its value, if any, is dropped.
	pub(crate) fn statement(&mut self, n: NodeId) {
		self.into(n, Dest::Discard);
	}

	/// A statement or an expression whose value goes to `dest`.
	pub(crate) fn into(&mut self, n: NodeId, dest: Dest) {
		match self.view(n) {
			View::Group(inner) | View::Try(inner) => self.into(inner, dest),
			View::Error => {}
			View::Block { .. } => {
				self.out.open("");
				self.block_into(n, dest);
				self.out.close("");
			}
			View::If {
				condition,
				then_body,
				else_body,
				compile_time: false,
			} => {
				let condition = self.condition(condition);
				self.if_chain(condition, then_body, else_body, dest);
			}
			// compile-time constructs (constants.rs)
			View::If {
				then_body,
				else_body,
				compile_time: true,
				..
			} => self.comptime_if_into(n, then_body, else_body, dest),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel: None,
				compile_time: true,
			} => self.comptime_for(n, pattern, index.is_some(), iterable, body),
			View::Declaration {
				kind: BindingKind::Static,
				..
			} => {}
			View::Do(body) => {
				self.out.open("");
				self.block_into(body, dest);
				self.out.close("");
			}
			View::Loop(body) => self.loop_into(body, dest),
			View::While { condition, body } => self.while_statement(condition, body),
			View::WhileLet {
				pattern,
				value,
				body,
			} => self.while_let_statement(pattern, value, body),
			View::For {
				pattern,
				index,
				iterable,
				body,
				parallel: None,
				compile_time: false,
			} => self.for_statement(n, pattern, index.is_some(), iterable, body),
			View::Break(value) => self.break_statement(value),
			View::Continue => self.out.line("continue;"),
			View::Return(value) => match value {
				Some(value) => self.into(value, Dest::Return),
				None => self.out.line("return;"),
			},
			View::Declaration {
				kind,
				annotation: _,
				value,
				..
			} => self.declaration(n, kind, value),
			// `use { a } = utils`: a name alias, resolved by the analyzer (modules.rs)
			View::Use { .. } | View::Destructuring { .. } if self.is_name_alias(n) => {}
			// references (references.rs)
			View::Use { value, mutable, .. } => self.use_declaration(n, value, mutable),
			View::Reuse { value, .. } => self.reuse_statement(n, value),
			View::Provide { ty, value } => self.provide_statement(n, ty, value),
			View::Destructuring {
				kind,
				pattern,
				value,
			} => {
				if matches!(kind, BindingKind::Let | BindingKind::Mutable) {
					self.destructuring(pattern, value, kind == BindingKind::Mutable);
				} else {
					self.unsupported(n, &format!("`{}` with a pattern", kind.keyword()));
				}
			}
			View::Assignment {
				target,
				annotation: None,
				value: Some(value),
			} => {
				// a reactive place notifies its observers (reactivity.rs)
				self.reactive_assignment(n, target, |g| {
					// a relation side: the two-sided update (relations.rs)
					if !g.relation_assignment(n, target, value) {
						g.assignment(target, value);
					}
				});
			}
			View::CompoundAssignment { op, target, value } => {
				self.reactive_assignment(n, target, |g| {
					g.compound_assignment(n, op, target, value)
				});
			}
			View::Function { .. } | View::ExportFunction { .. } => self.function_declaration(n),
			// a file's export (modules.rs)
			View::ExportValue { mutable, value, .. } => {
				let kind = if mutable {
					BindingKind::Mutable
				} else {
					BindingKind::Let
				};
				self.declaration(n, kind, Some(value))
			}
			View::ExportType { .. } => self.type_declaration(n),
			View::Namespace { .. } | View::ExportNamespace(_) => {}
			View::Assert(condition) => self.assert_statement(n, condition),
			View::Test { .. } => {}
			View::When { .. } => self.when_into(n, dest),
			View::Throw(value) => self.throw_statement(value),
			View::Catch {
				expression,
				handler,
				..
			} => self.catch_into(n, expression, handler, dest),
			View::TypeDecl { .. } => self.type_declaration(n),
			View::UpdateFields { target, value } => self.update_fields(target, value),
			_ => {
				let value = self.expr(n);
				self.finish(n, value, &dest);
			}
		}
	}

	/// Sends the value of an expression (the node `n`) to its destination;
	/// a returned value is fresh ([`Generator::copy_returned`]).
	pub(crate) fn finish(&mut self, n: NodeId, value: Expr, dest: &Dest) {
		let ty = self.type_of(n);
		match dest {
			Dest::Discard => {
				let fresh = !matches!(
					self.view(self.strip_groups(n)),
					View::Name(_) | View::Access { .. } | View::Index { .. }
				);
				if fresh
					&& matches!(
						self.types.kind(self.without_none(ty)),
						fa_analyzer::types::TypeKind::Sync(fa_analyzer::types::SyncKind::Guard, _)
					) {
					let unlock = self.helper("faUnlock");
					self.out.line(format!("{unlock}({});", value.text));
					return;
				}
				// a new value holding a resource, discarded: dropped (resources.rs)
				if fresh && let Some(drop) = self.drop_function(ty) {
					self.out.line(format!("{drop}({});", value.text));
					return;
				}
				if !value.pure {
					let text = if value.text.starts_with('{') || value.text.starts_with("function")
					{
						format!("({})", value.text)
					} else {
						value.text
					};
					self.out.line(format!("{text};"));
				}
			}
			Dest::Return => {
				if ty == TypeId::VOID {
					if !value.pure {
						self.out.line(format!("{};", value.text));
					}
				} else {
					let value = self.copy_returned(n, value);
					self.out.line(format!("return {};", value.text));
				}
			}
			Dest::Assign(name) => self.out.line(format!("{name} = {};", value.text)),
		}
	}

	/// The condition of an `if` or a loop: its JavaScript expression.
	pub(crate) fn condition(&mut self, condition: NodeId) -> Expr {
		self.expr(condition)
	}

	/// `if (condition) { … } else if … else { … }` with each branch into `dest`.
	fn if_chain(
		&mut self,
		condition: Expr,
		then_body: NodeId,
		else_body: Option<NodeId>,
		dest: Dest,
	) {
		self.out.open(format!("if ({})", condition.text));
		self.block_into(then_body, dest.clone());
		let Some(else_body) = else_body else {
			self.out.close("");
			return;
		};
		if let View::If {
			condition: next,
			then_body: next_then,
			else_body: next_else,
			compile_time: false,
		} = self.view(else_body)
		{
			let indent = self.out.indentation();
			let (lines, next_condition) = self.capture_at(indent, |g| g.condition(next));
			if lines.is_empty() {
				self.out
					.reopen(&format!("else if ({})", next_condition.text));
				self.block_into(next_then, dest.clone());
				return self.else_chain(next_else, dest);
			}
			self.out.reopen("else");
			self.out.append(lines);
			self.if_chain(next_condition, next_then, next_else, dest);
			self.out.close("");
			return;
		}
		self.out.reopen("else");
		self.block_into(else_body, dest);
		self.out.close("");
	}

	/// The rest of an `else if` chain after its first branch.
	fn else_chain(&mut self, else_body: Option<NodeId>, dest: Dest) {
		let Some(else_body) = else_body else {
			self.out.close("");
			return;
		};
		if let View::If {
			condition,
			then_body,
			else_body: next_else,
			compile_time: false,
		} = self.view(else_body)
		{
			let indent = self.out.indentation();
			let (lines, condition) = self.capture_at(indent, |g| g.condition(condition));
			if lines.is_empty() {
				self.out.reopen(&format!("else if ({})", condition.text));
				self.block_into(then_body, dest.clone());
				return self.else_chain(next_else, dest);
			}
			self.out.reopen("else");
			self.out.append(lines);
			self.if_chain(condition, then_body, next_else, dest);
			self.out.close("");
			return;
		}
		self.out.reopen("else");
		self.block_into(else_body, dest);
		self.out.close("");
	}

	/// `loop { … }`: `while (true)`, its `break` values going to `dest`.
	fn loop_into(&mut self, body: NodeId, dest: Dest) {
		self.out.open("while (true)");
		self.fcx.loops.push(LoopCx { dest });
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
	}

	/// `while condition { … }`. A condition that needs statements is
	/// evaluated at the start of each iteration: `while (true) { …; if
	/// (!condition) break; … }`.
	fn while_statement(&mut self, condition: NodeId, body: NodeId) {
		let indent = self.out.indentation();
		let (lines, condition) = self.capture_at(indent + 1, |g| g.condition(condition));
		if lines.is_empty() {
			self.out.open(format!("while ({})", condition.text));
		} else {
			self.out.open("while (true)");
			self.out.append(lines);
			let negated = crate::js::prefix("!", &condition);
			self.out.line(format!("if ({}) break;", negated.text));
		}
		self.fcx.loops.push(LoopCx {
			dest: Dest::Discard,
		});
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
	}

	/// `while let name = value { … }`: the value read before each round, the
	/// loop left at the first `none` (`undefined`).
	fn while_let_statement(&mut self, pattern: NodeId, value: NodeId, body: NodeId) {
		self.out.open("while (true)");
		let expr = self.expr(value);
		let value = self.copy_stored(value, expr);
		self.push_scope();
		match self.analysis.symbols.symbol_of_decl(pattern) {
			Some(symbol) => {
				let name = self.declare(symbol);
				self.out.line(format!("const {name} = {};", value.text));
				self.out.line(format!("if ({name} === undefined) break;"));
			}
			None => self
				.out
				.line(format!("if ({} === undefined) break;", value.text)),
		}
		self.fcx.loops.push(LoopCx {
			dest: Dest::Discard,
		});
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.pop_scope();
		self.out.close("");
	}

	/// `for i in a..b { … }`: a counted `for` loop, the end evaluated once.
	pub(crate) fn for_statement(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) {
		let iterable = self.strip_groups(iterable);
		let (values, end, inclusive) = match self.view(iterable) {
			View::Range {
				start: Some(start),
				end,
				inclusive,
			} => {
				let bounds: Vec<NodeId> = std::iter::once(start).chain(end).collect();
				let values = self.in_order(&bounds, |g, bound| g.expr(bound));
				(values, end.map(|_| ()), inclusive)
			}
			// a compile-time range (`static span = 1..5`): its bounds
			_ => match self.constant_range(iterable) {
				Some((Some(start), end, inclusive)) if !has_index => {
					let constant = |value: ethnum::I256| {
						Expr::constant(value.to_string(), crate::js::Prec::Primary)
					};
					let values: Vec<Expr> = std::iter::once(constant(start))
						.chain(end.map(constant))
						.collect();
					(values, end.map(|_| ()), inclusive)
				}
				_ => {
					self.for_items(n, pattern, has_index, iterable, body);
					return;
				}
			},
		};
		if has_index {
			self.unsupported(n, "an index in a `for` loop over a range");
			return;
		}
		self.push_scope();
		let variable = match pattern.map(|pattern| (pattern, self.view(pattern))) {
			Some((pattern, View::Name(_))) => match self.analysis.symbols.symbol_of_decl(pattern) {
				Some(symbol) => self.declare(symbol),
				None => self.temp(),
			},
			Some((pattern, _)) => {
				self.unsupported(pattern, "a pattern in a `for` loop over a range");
				self.temp()
			}
			None => self.temp(),
		};
		let start = &values[0];
		let header = match values.get(1) {
			Some(end) => {
				let operator = if inclusive { "<=" } else { "<" };
				if end.stable {
					format!(
						"for (let {variable} = {}; {variable} {operator} {}; {variable}++)",
						start.text,
						end.at(crate::js::Prec::Additive)
					)
				} else {
					let limit = self.temp();
					format!(
						"for (let {variable} = {}, {limit} = {}; {variable} {operator} {limit}; {variable}++)",
						start.text, end.text
					)
				}
			}
			None => format!("for (let {variable} = {}; ; {variable}++)", start.text),
		};
		self.out.open(header);
		if end.is_none() {
			self.helper("faRangeOverflow");
			let location = self.location(iterable);
			self.out.line(format!(
				"if ({variable} > 9007199254740991) faRangeOverflow({location});"
			));
		}
		self.fcx.loops.push(LoopCx {
			dest: Dest::Discard,
		});
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
		self.pop_scope();
	}

	/// `for item, index in items { … }` over an array or a string. An array
	/// is walked with a counted loop when the body writes its items through
	/// the loop variable (`score += 1` writes `scores[$t0]`) or reads the
	/// index; with `for…of` otherwise.
	fn for_items(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) {
		if self.type_of(iterable) == TypeId::RANGE {
			self.for_range_value(n, pattern, iterable, body);
			return;
		}
		// `for item in array.values()`: the loop over the array, whose
		// elements the variable designates (written in place)
		let iterable = self.values_receiver(iterable).unwrap_or(iterable);
		// `for (key, value) in map`: the entries of the map
		if let Some(pattern) = pattern
			&& let View::TuplePattern { items } = self.view(pattern)
			&& matches!(
				self.types.kind(self.type_of(iterable)),
				fa_analyzer::types::TypeKind::Collection(
					fa_analyzer::types::CollKind::Map | fa_analyzer::types::CollKind::GrowingMap,
					..
				)
			) {
			let collection = self.expr(iterable);
			// a value written through the loop variable: `map.set(key, …)`
			let symbols: Vec<Option<SymbolId>> = items
				.iter()
				.map(|item| self.analysis.symbols.symbol_of_decl(item))
				.collect();
			let written = symbols
				.get(1)
				.copied()
				.flatten()
				.filter(|&symbol| self.is_written(body, symbol));
			let collection = if written.is_some() && !collection.stable {
				let temp = self.temp();
				self.out
					.line(format!("const {temp} = {};", collection.text));
				Expr::name(temp, true)
			} else {
				collection
			};
			self.push_scope();
			let mut names: Vec<String> = symbols
				.iter()
				.map(|symbol| match symbol {
					Some(symbol) => self.declare(*symbol),
					None => self.temp(),
				})
				.collect();
			if let Some(value) = written {
				let map = collection.primary_text();
				let key = names[0].clone();
				self.alias(value, format!("{map}.get({key})"));
				self.map_aliases.insert(value, (map, key));
				names.truncate(1);
			}
			self.out.open(format!(
				"for (const [{}] of {})",
				names.join(", "),
				collection.text
			));
			self.fcx.loops.push(LoopCx {
				dest: Dest::Discard,
			});
			self.coop_point();
			self.block_into(body, Dest::Discard);
			self.fcx.loops.pop();
			self.out.close("");
			self.pop_scope();
			return;
		}
		let utf8 = self.has(self.strip_groups(iterable), NodeFlags::UTF8_ITERATION);
		let collection = self.expr(iterable);
		let text = self.kind(self.type_of(iterable)) == Kind::Text;
		self.push_scope();
		let mut destructured = None;
		let variable_symbol = match pattern.map(|pattern| (pattern, self.view(pattern))) {
			Some((pattern, View::Name(_))) => self.analysis.symbols.symbol_of_decl(pattern),
			Some((pattern, _)) if self.analysis.pattern(pattern).is_some() => {
				destructured = Some(pattern);
				None
			}
			Some((pattern, _)) => {
				self.unsupported(pattern, "a pattern in a `for` loop");
				None
			}
			None => None,
		};
		let index_symbol = if has_index {
			self.analysis
				.symbols
				.symbols()
				.iter()
				.enumerate()
				.find(|(_, symbol)| {
					symbol.decl_node == n
						&& matches!(symbol.kind, SymbolKind::LoopVariable { copy: true })
				})
				.map(|(index, _)| SymbolId::new(index))
		} else {
			None
		};
		let index_symbol = index_symbol.filter(|symbol| self.reads_symbol(body, *symbol));
		let has_index = index_symbol.is_some();
		let mut written = variable_symbol.is_some_and(|symbol| self.is_written(body, symbol));
		// a pool or a columnar (a `Set` of objects): `for…of`, the variable
		// is the element, written in place (references.rs)
		if let fa_analyzer::types::TypeKind::Collection(kind, ..) =
			self.types.kind(self.type_of(iterable))
			&& crate::generate::collections::is_pool(kind)
		{
			if written && let Some(symbol) = variable_symbol {
				self.references
					.insert(symbol, crate::generate::references::Reference::Object);
			}
			written = false;
		}
		if text && has_index {
			let items = self.keep(collection);
			let position = index_symbol
				.map(|symbol| self.declare(symbol))
				.unwrap_or_else(|| self.temp());
			let code = self.temp();
			self.out.open(format!("for (let {position} = 0, {code}; {position} < {}.length; {position} += {code} > 0xffff ? 2 : 1)", items.primary_text()));
			self.out.line(format!(
				"{code} = {}.codePointAt({position});",
				items.primary_text()
			));
			if let Some(symbol) = variable_symbol {
				let name = self.declare(symbol);
				self.out
					.line(format!("const {name} = String.fromCodePoint({code});"));
			}
			self.fcx.loops.push(LoopCx {
				dest: Dest::Discard,
			});
			self.coop_point();
			self.block_into(body, Dest::Discard);
			self.fcx.loops.pop();
			self.out.close("");
		} else if utf8 {
			// TextEncoder's bulk encoding and an indexed byte loop are much
			// cheaper on JS engines than resuming a generator for every byte.
			let items = self.keep(collection);
			self.loop_over(
				items,
				variable_symbol,
				destructured,
				index_symbol,
				true,
				body,
			);
		} else if text {
			self.loop_over(
				collection,
				variable_symbol,
				destructured,
				index_symbol,
				false,
				body,
			);
		} else {
			// a loop doesn't change the structure of its array (E-LOOP-4): a name is read again
			let name = matches!(self.view(iterable), View::Name(_));
			let items = if collection.stable || name || !(written || has_index) {
				collection
			} else {
				let temp = self.temp();
				self.out
					.line(format!("const {temp} = {};", collection.text));
				Expr::name(temp, true)
			};
			self.loop_over(
				items,
				variable_symbol,
				destructured,
				index_symbol,
				written || has_index,
				body,
			);
		}
		self.pop_scope();
	}

	/// The array of `array.values()`.
	fn values_receiver(&self, n: NodeId) -> Option<NodeId> {
		let View::Call { callee, arguments } = self.view(self.strip_groups(n)) else {
			return None;
		};
		if !arguments.is_empty() {
			return None;
		}
		let View::Access { target, members } = self.view(callee) else {
			return None;
		};
		let member = members.first().filter(|_| members.len() == 1)?;
		match self.analysis.resolution(member) {
			Resolution::Builtin(builtin)
				if fa_analyzer::builtins::get(builtin).path == "Array#values" =>
			{
				Some(target)
			}
			_ => None,
		}
	}

	/// The loop of [`Generator::for_items`] over a JavaScript array.
	#[allow(clippy::too_many_arguments)]
	fn loop_over(
		&mut self,
		items: Expr,
		variable: Option<SymbolId>,
		pattern: Option<NodeId>,
		index: Option<SymbolId>,
		counted: bool,
		body: NodeId,
	) {
		if counted {
			let position = match index {
				Some(symbol) => self.declare(symbol),
				None => self.temp(),
			};
			self.out.open(format!(
				"for (let {position} = 0; {position} < {}.length; {position}++)",
				items.primary_text()
			));
			if let Some(symbol) = variable {
				let item = format!("{}[{position}]", items.primary_text());
				if self.is_written(body, symbol) {
					self.alias(symbol, item);
				} else {
					let name = self.declare(symbol);
					self.out.line(format!("const {name} = {item};"));
				}
			}
			if let Some(pattern) = pattern {
				let item = Expr::new(
					format!("{}[{position}]", items.primary_text()),
					Prec::Primary,
				);
				self.bind_pattern(pattern, item, false, false);
			}
		} else {
			let name = match variable {
				Some(symbol) => self.declare(symbol),
				None => self.temp(),
			};
			self.out
				.open(format!("for (const {name} of {})", items.text));
			if let Some(pattern) = pattern {
				self.bind_pattern(pattern, Expr::name(name, true), false, false);
			}
		}
		self.fcx.loops.push(LoopCx {
			dest: Dest::Discard,
		});
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
	}

	/// `for i in range` over a `Range` value (not a literal): from its start
	/// (a missing start stops the program) by one, to its end.
	fn for_range_value(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		iterable: NodeId,
		body: NodeId,
	) {
		let range = self.expr(iterable);
		let range = if range.stable {
			range
		} else {
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", range.text));
			Expr::name(temp, true)
		};
		let location = self.location(iterable);
		let values = self.helper("faRangeValues");
		self.push_scope();
		let variable =
			match pattern.and_then(|pattern| self.analysis.symbols.symbol_of_decl(pattern)) {
				Some(symbol) => self.declare(symbol),
				None => self.temp(),
			};
		let _ = n;
		self.out.open(format!(
			"for (const {variable} of {values}({}, {location}))",
			range.text
		));
		self.fcx.loops.push(LoopCx {
			dest: Dest::Discard,
		});
		self.coop_point();
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
		self.pop_scope();
	}

	fn reads_symbol(&self, body: NodeId, symbol: SymbolId) -> bool {
		let mut pending = vec![body];
		while let Some(node) = pending.pop() {
			if self.analysis.resolution(node) == Resolution::Local(symbol) {
				return true;
			}
			pending.extend(fa_analyzer::syntax::children(self.tree, node));
		}
		false
	}

	/// Whether a statement assigns a symbol as a whole (`x = …`, `x += …`).
	fn is_written(&self, body: NodeId, symbol: SymbolId) -> bool {
		// the nodes of the body only
		let mut pending = vec![body];
		while let Some(node) = pending.pop() {
			if let View::Assignment { target, .. } | View::CompoundAssignment { target, .. } =
				self.view(node)
			{
				let target = self.strip_groups(target);
				if matches!(self.view(target), View::Name(_))
					&& self.analysis.resolution(target) == Resolution::Local(symbol)
				{
					return true;
				}
			}
			pending.extend(fa_analyzer::syntax::children(self.tree, node));
		}
		false
	}

	/// `break` or `break value`.
	fn break_statement(&mut self, value: Option<NodeId>) {
		let dest = self
			.fcx
			.loops
			.last()
			.map_or(Dest::Discard, |cx| cx.dest.clone());
		if let Some(value) = value {
			self.into(value, dest.clone());
		}
		if dest != Dest::Return || value.is_none() {
			self.out.line("break;");
		}
	}

	/// `let name = value`, `mutable name = value`.
	fn declaration(&mut self, n: NodeId, kind: BindingKind, value: Option<NodeId>) {
		// reactivity.rs: a `reactive` value is a `mutable` one with a source
		let kind = match (kind, value) {
			(BindingKind::Derived, Some(value)) => return self.derived_declaration(n, value),
			(BindingKind::Reactive, _) => {
				if let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) {
					self.declare_value_source(symbol);
				}
				BindingKind::Mutable
			}
			(kind, _) => kind,
		};
		if !matches!(kind, BindingKind::Let | BindingKind::Mutable) {
			self.unsupported(n, &format!("`{}`", kind.keyword()));
			return;
		}
		let Some(symbol) = self.analysis.symbols.symbol_of_decl(n) else {
			return;
		};
		let module_value = self.is_module_value(symbol);
		let name = self.declare(symbol);
		let Some(value) = value else {
			if !module_value {
				self.out.line(format!("let {name};"));
			}
			return;
		};
		if self.is_control(value) && self.needs_statements(value) {
			if !module_value {
				self.out.line(format!("let {name};"));
			}
			self.into(value, Dest::Assign(name.clone()));
			self.copy_assigned(value, &name, kind == BindingKind::Mutable);
			return;
		}
		let value_node = value;
		let value = self.expr(value);
		let value = self.copy_into(value_node, value, kind == BindingKind::Mutable);
		// a literal read by functions: a module-level `const` (one less
		// context slot to load in hot functions)
		if module_value && kind == BindingKind::Let && self.is_literal(value_node) && value.pure {
			self.module_constant(name, value.text);
			return;
		}
		if module_value || self.guard_binding(&name) {
			self.out.line(format!("{name} = {};", value.text));
		} else {
			let keyword = if kind == BindingKind::Let {
				"const"
			} else {
				"let"
			};
			self.out.line(format!("{keyword} {name} = {};", value.text));
		}
	}

	/// Whether a node is a literal (or a negated one).
	fn is_literal(&self, n: NodeId) -> bool {
		match self.view(self.strip_groups(n)) {
			View::Integer(_)
			| View::Float(_)
			| View::BigInt(_)
			| View::String(_)
			| View::Boolean(_)
			| View::Percentage(_) => true,
			View::Negate(inner) => self.is_literal(inner),
			_ => false,
		}
	}

	/// `target = value`: a name, a field or an item.
	fn assignment(&mut self, target: NodeId, value: NodeId) {
		let target_node = self.strip_groups(target);
		// a place a reference may designate: written in place (D60, references.rs)
		if self.writes_in_place(target_node) {
			self.assign_in_place(target_node, value);
			return;
		}
		// `map[key] = value`: `map.set(key, value)`
		if let View::Index {
			target: container,
			index,
		} = self.view(target_node)
			&& matches!(
				self.types.kind(self.type_of(container)),
				fa_analyzer::types::TypeKind::Collection(
					fa_analyzer::types::CollKind::Map | fa_analyzer::types::CollKind::GrowingMap,
					..
				)
			) {
			let values = self.in_order(&[container, index, value], |g, node| g.expr(node));
			let stored = self.copy_stored(value, values[2].clone());
			self.out.line(format!(
				"{}.set({}, {});",
				values[0].primary_text(),
				values[1].text,
				stored.text
			));
			return;
		}
		if let View::Index { index, .. } = self.view(target_node)
			&& self.index_needs_normalizing(target_node, index)
			&& !self.needs_statements(value)
		{
			// `faIndexSet(items, index, value)`: the place, then the value, then the check
			let View::Index {
				target: container,
				index,
			} = self.view(target_node)
			else {
				return;
			};
			let values = self.in_order(&[container, index, value], |g, node| g.expr(node));
			let stored = self.copy_stored(value, values[2].clone());
			let helper = self.helper("faIndexSet");
			let drop = self
				.drop_function(self.type_of(target_node))
				.map(|drop| format!(", {drop}"))
				.unwrap_or_default();
			self.out.line(format!(
				"{helper}({}, {}, {}{drop});",
				values[0].text, values[1].text, stored.text
			));
			return;
		}
		let Some(place) = self.place(target_node, !self.needs_statements(value)) else {
			self.unsupported(target, "this assignment target");
			return;
		};
		if self.needs_statements(value) {
			if place.map.is_some() {
				// a value of a map: computed into a temporary, then `set`
				let temp = self.temp();
				self.out.line(format!("let {temp};"));
				self.into(value, Dest::Assign(temp.clone()));
				self.copy_assigned(value, &temp, true);
				self.out.line(place.write(&temp));
				return;
			}
			self.into(value, Dest::Assign(place.text.clone()));
			self.copy_assigned(value, &place.text, true);
			return;
		}
		let value_node = value;
		let saved_move = self.fcx.moving_replaced;
		self.fcx.moving_replaced = self.replaced_root_read(target_node, value);
		let value = self.expr(value);
		let value = match self.copy_into_local(target_node, value_node, &value) {
			Some(reused) => reused,
			None => self.copy_into(value_node, value, true),
		};
		self.fcx.moving_replaced = saved_move;
		// a value holding a resource: the old one dropped (resources.rs)
		if place.map.is_none()
			&& let Some((drop, flag)) = self.drop_replaced(target_node)
		{
			let temp = self.temp();
			self.out.line(format!("const {temp} = {};", value.text));
			match &flag {
				Some(flag) => self
					.out
					.line(format!("if ({flag}) {drop}({});", place.text)),
				None => self.out.line(format!("{drop}({});", place.text)),
			}
			self.out.line(place.write(&temp));
			if let Some(flag) = flag {
				self.out.line(format!("{flag} = true;"));
			}
			return;
		}
		self.out.line(place.write(&value.text));
	}

	/// `assert condition`: stops with the source of the condition, and the
	/// operands of a top-level comparison.
	fn assert_statement(&mut self, n: NodeId, condition: NodeId) {
		let inner = self.strip_groups(condition);
		let source = crate::js::string_literal(self.text(condition));
		let location = self.location(n);
		self.helper("faAssertFailed");
		let operands = match self.view(inner) {
			View::Equality { left, right, .. } => Some((left, right)),
			View::Comparison { operands, .. } if operands.len() == 2 => {
				Some((operands.get(0).unwrap(), operands.get(1).unwrap()))
			}
			_ => None,
		};
		let Some((left, right)) = operands else {
			let condition = self.condition(condition);
			let negated = crate::js::prefix("!", &condition);
			self.out.line(format!(
				"if ({}) faAssertFailed({source}, {location});",
				negated.text
			));
			return;
		};
		// the operands are evaluated once: kept in temporaries for the message
		let values = self.in_order(&[left, right], |g, operand| g.expr(operand));
		let mut kept = Vec::new();
		let mut shown = Vec::new();
		for (value, operand) in values.into_iter().zip([left, right]) {
			if value.stable {
				shown.push(value.clone());
				kept.push(value);
			} else {
				let temp = self.inline_temp();
				kept.push(Expr::new(
					format!("{temp} = {}", value.text),
					crate::js::Prec::Assign,
				));
				shown.push(Expr::name(temp, true));
			}
			let _ = operand;
		}
		let test = self.comparison_of(inner, &kept);
		let negated = crate::js::prefix("!", &test);
		let left_text = self.format_nested(&shown[0], self.type_of(left));
		let right_text = self.format_nested(&shown[1], self.type_of(right));
		self.out.line(format!(
			"if ({}) faAssertFailed({source}, {location}, `left: ${{{left_text}}}, right: ${{{right_text}}}`);",
			negated.text
		));
	}

	/// Whether generating a value needs statements: an `if` with blocks, a
	/// `do`, a `loop`, a `return`… (a syntactic test: it only chooses
	/// between `let x; if (…) { x = … }` and `const x = c ? a : b`).
	pub(crate) fn needs_statements(&self, n: NodeId) -> bool {
		match self.view(n) {
			View::Group(inner) | View::Try(inner) => self.needs_statements(inner),
			View::If {
				condition,
				then_body,
				else_body,
				..
			} => {
				let Some(else_body) = else_body else {
					return true;
				};
				self.needs_statements(condition)
					|| !self.is_single_value(then_body)
					|| !self.is_single_value(else_body)
			}
			View::Block { .. }
			| View::Do(_)
			| View::Loop(_)
			| View::While { .. }
			| View::WhileLet { .. }
			| View::For { .. }
			| View::Return(_)
			| View::Break(_)
			| View::Continue
			| View::Throw(_)
			| View::When { .. }
			| View::Catch { .. }
			| View::Declaration { .. }
			| View::Destructuring { .. }
			| View::Assignment { .. }
			| View::CompoundAssignment { .. }
			| View::Function { .. }
			| View::Assert(_)
			| View::TypeDecl { .. }
			| View::Use { .. }
			| View::Reuse { .. }
			| View::Provide { .. }
			| View::UpdateFields { .. } => true,
			View::Lambda { .. } => false,
			// the `name = value` items of literals and named arguments are not statements
			View::Object { items } | View::TypedObject { members: items, .. } => {
				items.iter().any(|item| self.item_needs_statements(item))
			}
			View::Call { callee, arguments } => {
				self.needs_statements(callee)
					|| arguments
						.iter()
						.any(|argument| self.item_needs_statements(argument))
			}
			_ => fa_analyzer::syntax::children(self.tree, n)
				.any(|child| self.needs_statements(child)),
		}
	}

	/// Whether a node is a construct whose value can go straight to a
	/// destination (`let x; if (c) { x = 1 } else { x = 2 }`).
	fn is_control(&self, n: NodeId) -> bool {
		matches!(
			self.view(self.strip_groups(n)),
			View::If { .. }
				| View::Do(_)
				| View::Loop(_)
				| View::When { .. }
				| View::Catch { .. }
				| View::Block { .. }
		)
	}

	/// [`Generator::needs_statements`] for an item of a literal or an
	/// argument: the value of `name = value`.
	fn item_needs_statements(&self, item: NodeId) -> bool {
		match self.view(item) {
			View::Assignment {
				value: Some(value), ..
			} => self.needs_statements(value),
			_ => self.needs_statements(item),
		}
	}

	/// Whether a branch is one expression that needs no statement.
	fn is_single_value(&self, branch: NodeId) -> bool {
		match self.view(branch) {
			View::Block { statements } => {
				statements.len() == 1
					&& statements
						.first()
						.is_some_and(|only| !self.needs_statements(only))
			}
			View::If { .. } => !self.needs_statements(branch),
			_ => false,
		}
	}
}
