//! Statements, blocks and control flow (values brief §5, §17, §18, §19).
//!
//! A construct whose value is used is generated *into a destination*
//! ([`Dest`]): the value of its last expression is returned, assigned to a
//! name or dropped, so that `let x = if c { … } else { … }` becomes
//! `let x; if (c) { …; x = 1; } else { …; x = 2; }` and a function body
//! ending with an `if` returns from each branch, without temporaries.

use fa_analyzer::ids::{NodeId, SymbolId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{BindingKind, View};

use crate::generate::{Dest, Generator, Kind, LoopCx};
use crate::js::{self, Expr, Prec};

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
		for &statement in rest {
			self.statement(statement);
		}
		self.into(last, dest);
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
			View::Do(body) => {
				self.out.open("");
				self.block_into(body, dest);
				self.out.close("");
			}
			View::Loop(body) => self.loop_into(body, dest),
			View::While { condition, body } => self.while_statement(condition, body),
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
			} => self.assignment(target, value),
			View::CompoundAssignment { op, target, value } => {
				self.compound_assignment(n, op, target, value)
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
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
	}

	/// `for i in a..b { … }`: a counted `for` loop, the end evaluated once.
	fn for_statement(
		&mut self,
		n: NodeId,
		pattern: Option<NodeId>,
		has_index: bool,
		iterable: NodeId,
		body: NodeId,
	) {
		let iterable = self.strip_groups(iterable);
		let View::Range {
			start: Some(start),
			end,
			inclusive,
		} = self.view(iterable)
		else {
			self.for_items(n, pattern, has_index, iterable, body);
			return;
		};
		if has_index {
			self.unsupported(n, "an index in a `for` loop over a range");
			return;
		}
		let bounds: Vec<NodeId> = std::iter::once(start).chain(end).collect();
		let values = self.in_order(&bounds, |g, bound| g.expr(bound));
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
		let written = variable_symbol.is_some_and(|symbol| self.is_written(body, symbol));
		if text {
			let characters = self.helper("faCharacters");
			let items = if has_index {
				let temp = self.temp();
				self.out
					.line(format!("const {temp} = {characters}({});", collection.text));
				Expr::name(temp, true)
			} else {
				js::call(characters, std::slice::from_ref(&collection.text))
			};
			self.loop_over(
				items,
				variable_symbol,
				destructured,
				index_symbol,
				written || has_index,
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
		self.block_into(body, Dest::Discard);
		self.fcx.loops.pop();
		self.out.close("");
		self.pop_scope();
	}

	/// Whether a statement assigns a symbol as a whole (`x = …`, `x += …`).
	fn is_written(&self, body: NodeId, symbol: SymbolId) -> bool {
		let span = self.tree.spans[body.index()];
		(0..self.tree.nodes.len()).any(|index| {
			let node = NodeId::new(index);
			let inside = self.tree.spans[index];
			if inside.start < span.start || inside.end > span.end {
				return false;
			}
			let target = match self.view(node) {
				View::Assignment { target, .. } | View::CompoundAssignment { target, .. } => target,
				_ => return false,
			};
			let target = self.strip_groups(target);
			matches!(self.view(target), View::Name(_))
				&& self.analysis.resolution(target) == Resolution::Local(symbol)
		})
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
		if module_value {
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
			self.out.line(format!(
				"{helper}({}, {}, {});",
				values[0].text, values[1].text, stored.text
			));
			return;
		}
		let Some(place) = self.place(target_node, !self.needs_statements(value)) else {
			self.unsupported(target, "this assignment target");
			return;
		};
		if self.needs_statements(value) {
			self.into(value, Dest::Assign(place.text.clone()));
			self.copy_assigned(value, &place.text, true);
			return;
		}
		let value_node = value;
		let value = self.expr(value);
		let value = match self.copy_into_local(target_node, value_node, &value) {
			Some(reused) => reused,
			None => self.copy_into(value_node, value, true),
		};
		self.out.line(format!("{} = {};", place.text, value.text));
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
