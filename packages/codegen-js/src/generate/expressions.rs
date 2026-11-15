//! Expressions: names, booleans, comparisons, `if` values and the lifting of
//! constructs that need statements (values brief §8, §13, §19).

use fa_analyzer::builtins;
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::symbols::{Resolution, SymbolKind};
use fa_analyzer::syntax::{CompareOp, LogicOp, View};
use fa_analyzer::types::TypeKind;

use crate::generate::{Dest, Generator, Kind, Repr};
use crate::js::{self, Expr, Prec};

impl Generator<'_> {
	/// The JavaScript of an expression, converted for its sink when the
	/// analyzer recorded a conversion or a range check there.
	pub(crate) fn expr(&mut self, n: NodeId) -> Expr {
		let value = self.expr_raw(n);
		self.sink(n, value)
	}

	/// The JavaScript of an expression, without the conversion at its sink.
	pub(crate) fn expr_raw(&mut self, n: NodeId) -> Expr {
		// a value computed at compile time (constants.rs)
		if let Some(value) = self.comptime_value(n) {
			return self.constant_value(n, &value, self.type_of(n));
		}
		let value = self.expr_value(n);
		// reads and writes of reactive places (reactivity.rs)
		self.reactive_expr(n, value)
	}

	/// The JavaScript of an expression, before its reactive instrumentation.
	fn expr_value(&mut self, n: NodeId) -> Expr {
		// `f then g`, `chain[1..]`: a function calling the steps in turn
		if let Some(steps) = self.analysis.compositions.get(&n).cloned() {
			return self.composition(&steps);
		}
		match self.view(n) {
			View::Group(inner) | View::Try(inner) => self.expr(inner),
			View::Error => Expr::constant("undefined", Prec::Primary),
			View::Integer(value) => self.integer_literal(n, value),
			View::Float(value) => self.float_value(n, value),
			View::BigInt(text) => self.big_int_literal(text),
			View::Percentage(value) => self.percentage(n, value),
			View::String(text) => Expr::constant(js::string_literal(text), Prec::Primary),
			View::Template { parts } => self.template(parts),
			View::Boolean(value) => Expr::constant(value.to_string(), Prec::Primary),
			View::None => Expr::constant("undefined", Prec::Primary),
			View::Name(_) => self.name(n),
			View::Access { target, members } => self.member_access(n, target, members),
			View::OptionalAccess { target, members } => self.optional_access(n, target, members),
			View::Object { .. } | View::TypedObject { .. } => self.object_literal(n),
			// a tuple is a JavaScript array (printed by its type's formatter)
			View::Array { items } => match self.collection_literal(n, items) {
				Some(collection) => collection,
				None => self.array_literal(items),
			},
			View::Tuple { items } => self.array_literal(items),
			View::Range {
				start,
				end,
				inclusive,
			} => self.range_value(start, end, inclusive),
			View::Index { target, index } => self.index(n, target, index),
			View::Fallback { operands } => {
				let operands: Vec<NodeId> = operands.iter().collect();
				self.fallback(&operands)
			}
			View::ImplicitMember(_) => match self.analysis.resolution(n) {
				Resolution::Member(nominal, index) => self.nominal_member_value(n, nominal, index),
				_ => self.unsupported(n, "this member"),
			},
			View::Call { callee, arguments } => self.call(n, callee, arguments),
			View::Arithmetic { op, operands } => self.arithmetic(n, op, operands),
			View::Negate(value) => self.negate(n, value),
			View::Equality {
				left,
				right,
				negated,
			} => {
				let values = self.in_order(&[left, right], |g, operand| g.expr(operand));
				self.equality(left, right, &values[0], &values[1], negated)
			}
			View::Comparison {
				operands,
				operators,
			} => {
				let operands: Vec<NodeId> = operands.iter().collect();
				let operators: Vec<CompareOp> = operators.iter().collect();
				self.comparison_chain(&operands, &operators)
			}
			View::Logic { op, operands } => {
				let operands: Vec<NodeId> = operands.iter().collect();
				self.logic(op, &operands)
			}
			View::Is { value, ty, negated } => self.is_expression(value, ty, negated),
			View::IsIn {
				value,
				collection,
				negated,
			} => {
				let values = self.in_order(&[value, collection], |g, node| g.expr(node));
				let set_ty = self.type_of(collection);
				if self.is_fields(set_ty) {
					let test = self.fields_is_in(set_ty, &values[1], &values[0]);
					return if negated {
						js::prefix("!", &test)
					} else {
						test
					};
				}
				// an array: an item `==` to the value; a set or a map: `has`
				let test = match self.types.kind(set_ty) {
					fa_analyzer::types::TypeKind::Collection(
						fa_analyzer::types::CollKind::Array
						| fa_analyzer::types::CollKind::GrowingArray,
						..,
					) => {
						let helper = self.helper("faArrayIncludes");
						js::call(helper, &[values[1].text.clone(), values[0].text.clone()])
					}
					fa_analyzer::types::TypeKind::Collection(..) => Expr::primary(format!(
						"{}.has({})",
						values[1].primary_text(),
						values[0].text
					)),
					_ => {
						let helper = self.helper("faRangeIncludes");
						js::call(helper, &[values[1].text.clone(), values[0].text.clone()])
					}
				};
				if negated {
					js::prefix("!", &test)
				} else {
					test
				}
			}
			View::Not(value) => {
				let value = self.expr(value);
				js::prefix("!", &value)
			}
			View::If {
				condition,
				then_body,
				else_body: Some(else_body),
				compile_time: false,
			} if !self.needs_statements(n) => self.conditional(condition, then_body, else_body),
			View::Lambda { .. } => self.lambda(n),
			View::Watch(_) => self.watch_expr(n),
			View::If { .. }
			| View::Do(_)
			| View::Loop(_)
			| View::Block { .. }
			| View::While { .. }
			| View::WhileLet { .. }
			| View::For { .. }
			| View::Declaration { .. }
			| View::Destructuring { .. }
			| View::Assignment { .. }
			| View::CompoundAssignment { .. }
			| View::Function { .. }
			| View::Assert(_)
			| View::Return(_)
			| View::Break(_)
			| View::Continue
			| View::When { .. }
			| View::Throw(_)
			| View::Catch { .. }
			| View::TypeDecl { .. }
			| View::Use { .. }
			| View::Reuse { .. }
			| View::Provide { .. }
			| View::UpdateFields { .. } => self.lift(n),
			View::Unsupported(kind) => self.unsupported(n, &format!("`{kind}`")),
			// tasks (tasks.rs)
			View::Await(operand) => self.await_expr(n, operand),
			View::Start(operand) => self.start_expr(n, operand),
			_ => {
				let kind = fa_analyzer::syntax::kind_name(self.tree, n);
				self.unsupported(n, &format!("this expression (`{kind}`)"))
			}
		}
	}

	/// Generates a construct that needs statements before the current
	/// statement: its value goes to a temporary (nothing for `Void` and
	/// `Never` constructs).
	pub(crate) fn lift(&mut self, n: NodeId) -> Expr {
		let ty = self.type_of(n);
		if ty == TypeId::VOID || ty == TypeId::NEVER || ty.is_unset() {
			self.statement(n);
			return Expr::constant("undefined", Prec::Primary);
		}
		let temp = self.temp();
		self.out.line(format!("let {temp};"));
		self.into(n, Dest::Assign(temp.clone()));
		Expr::name(temp, true)
	}

	/// Evaluates nodes left to right. When one of them needs statements, the
	/// values computed before it are first stored in temporaries (unless
	/// stable), so that the statements don't run before them.
	pub(crate) fn in_order(
		&mut self,
		nodes: &[NodeId],
		mut f: impl FnMut(&mut Self, NodeId) -> Expr,
	) -> Vec<Expr> {
		let mut values: Vec<Expr> = Vec::with_capacity(nodes.len());
		for &node in nodes {
			let (lines, value) = self.capture(|g| f(g, node));
			if !lines.is_empty() {
				for earlier in values.iter_mut() {
					if !earlier.stable {
						let temp = self.temp();
						self.out.line(format!("const {temp} = {};", earlier.text));
						*earlier = Expr::name(temp, true);
					}
				}
				self.out.append(lines);
			}
			values.push(value);
		}
		values
	}

	/// A value used twice: as is when stable, otherwise assigned to an
	/// inline temporary at its first use. Returns (first use, later uses).
	pub(crate) fn twice(&mut self, value: Expr) -> (Expr, Expr) {
		if value.stable {
			return (value.clone(), value);
		}
		let temp = self.inline_temp();
		(
			Expr::new(format!("({temp} = {})", value.text), Prec::Primary),
			Expr::name(temp, true),
		)
	}

	/// A name in value position.
	fn name(&mut self, n: NodeId) -> Expr {
		match self.analysis.resolution(n) {
			Resolution::Local(symbol) => {
				let name = self.name_of(symbol);
				// a derived value: its node, brought up to date (reactivity.rs)
				if self.analysis.symbols.symbol(symbol).kind == SymbolKind::Derived
					&& !self.aliases.contains_key(&symbol)
				{
					return self.derived_read(symbol, name);
				}
				// an `async` function as a value: both instances (tasks.rs)
				let declaration = self.analysis.symbols.symbol(symbol).decl_node;
				if self.tasks.instances.contains_key(&declaration)
					&& let Some(value) = self.async_function_value(n, declaration)
				{
					return value;
				}
				let stable = self.is_constant(symbol) && !self.aliases.contains_key(&symbol);
				// a value holding a resource moved out: its drop flag cleared
				self.moving_read(n, symbol, Expr::name(name, stable))
			}
			Resolution::Builtin(builtin) => match builtins::get(builtin).path {
				"arguments" => self.use_arguments(),
				path => self.unsupported(n, &format!("`{path}` as a value")),
			},
			// a declaration of another file (modules.rs)
			Resolution::External(..) => match self.external_value(n) {
				Some(value) => value,
				None => self.unsupported(n, "this name of another file"),
			},
			_ => self.unsupported(n, "this name"),
		}
	}

	/// `c ? a : b` for an `if` whose branches are single values.
	fn conditional(&mut self, condition: NodeId, then_body: NodeId, else_body: NodeId) -> Expr {
		let condition = self.condition(condition);
		let indent = self.out.indentation() + 1;
		let (then_lines, yes) = self.capture_at(indent, |g| g.branch_value(then_body));
		let (else_lines, no) = self.capture_at(indent, |g| g.branch_value(else_body));
		if then_lines.is_empty() && else_lines.is_empty() {
			return js::conditional(&condition, &yes, &no);
		}
		// a branch needed statements after all: an `if` statement
		let temp = self.temp();
		self.out.line(format!("let {temp};"));
		self.out.open(format!("if ({})", condition.text));
		self.out.append(then_lines);
		self.out.line(format!("{temp} = {};", yes.text));
		self.out.reopen("else");
		self.out.append(else_lines);
		self.out.line(format!("{temp} = {};", no.text));
		self.out.close("");
		Expr::name(temp, true)
	}

	/// The value of a branch made of one expression (or of an `else if`).
	fn branch_value(&mut self, branch: NodeId) -> Expr {
		match self.view(branch) {
			View::Block { statements } => match statements.first() {
				Some(only) => self.expr(only),
				None => Expr::constant("undefined", Prec::Primary),
			},
			_ => self.expr(branch),
		}
	}

	/// `a and b and c`, `a or b`: `&&` and `||`; an operand that needs
	/// statements is evaluated only when the operands before it ask for it.
	fn logic(&mut self, op: LogicOp, operands: &[NodeId]) -> Expr {
		let (symbol, prec) = match op {
			LogicOp::And => ("&&", Prec::And),
			LogicOp::Or => ("||", Prec::Or),
		};
		let Some((&first, rest)) = operands.split_first() else {
			return Expr::constant("undefined", Prec::Primary);
		};
		let mut result = self.expr(first);
		for &operand in rest {
			let indent = self.out.indentation() + 1;
			let (lines, value) = self.capture_at(indent, |g| g.expr(operand));
			if lines.is_empty() {
				result = js::binary(&result, symbol, &value, prec);
				continue;
			}
			let temp = self.temp();
			self.out.line(format!("let {temp} = {};", result.text));
			match op {
				LogicOp::And => self.out.open(format!("if ({temp})")),
				LogicOp::Or => self.out.open(format!("if (!{temp})")),
			}
			self.out.append(lines);
			self.out.line(format!("{temp} = {};", value.text));
			self.out.close("");
			result = Expr::name(temp, true);
		}
		result
	}

	/// `a == b` (`negated`: `a != b`) on JavaScript values.
	pub(crate) fn equality(
		&mut self,
		left: NodeId,
		right: NodeId,
		left_value: &Expr,
		right_value: &Expr,
		negated: bool,
	) -> Expr {
		let (left_ty, right_ty) = (self.type_of(left), self.type_of(right));
		let operator = if negated { "!==" } else { "===" };
		let (left_repr, right_repr) = (self.repr(left_ty), self.repr(right_ty));
		// a bigint and a number integer (D-69): the number side is converted
		let (left_value, right_value) = match (left_repr, right_repr) {
			(Repr::BigInt, Repr::Number) => {
				(left_value.clone(), self.big_int_of(right, right_value))
			}
			(Repr::Number, Repr::BigInt) => {
				(self.big_int_of(left, left_value), right_value.clone())
			}
			_ => (left_value.clone(), right_value.clone()),
		};
		let comparable = |repr: Repr| {
			matches!(
				repr,
				Repr::Number | Repr::BigInt | Repr::String | Repr::Boolean | Repr::None
			)
		};
		// enumeration members are numbers (or optionals of them: `undefined`)
		let enumeration = |ty: TypeId| {
			self.types.union_members(&ty).iter().all(|&member| {
				matches!(
					self.types.kind(member),
					TypeKind::Enum(..) | TypeKind::EnumMember(..) | TypeKind::None
				)
			})
		};
		if comparable(left_repr)
			|| comparable(right_repr)
			|| enumeration(left_ty)
			|| enumeration(right_ty)
		{
			return js::binary(&left_value, operator, &right_value, Prec::Equality);
		}
		// two values of one object type: its fields only (structural typing)
		let equals = match (left_ty == right_ty)
			.then(|| self.equality_function(left_ty))
			.flatten()
		{
			Some(function) => function,
			None => self.helper("faEquals").to_string(),
		};
		let test = js::call(&equals, &[left_value.text, right_value.text]);
		if negated {
			js::prefix("!", &test)
		} else {
			test
		}
	}

	/// One ordering `a < b` between two values of a node's operands.
	fn order(&mut self, left: NodeId, op: CompareOp, a: &Expr, b: &Expr) -> Expr {
		let symbol = op.symbol();
		if self.kind(self.type_of(left)) == Kind::Text {
			let compare = self.helper("faStringCompare");
			let call = js::call(compare, &[a.text.clone(), b.text.clone()]);
			let zero = Expr::constant("0", Prec::Primary);
			return js::binary(&call, symbol, &zero, Prec::Relational);
		}
		js::binary(a, symbol, b, Prec::Relational)
	}

	/// `a < b <= c`: `a < b && b <= c`, each operand evaluated once and only
	/// when the comparisons before it are true.
	fn comparison_chain(&mut self, operands: &[NodeId], operators: &[CompareOp]) -> Expr {
		let Some((&first, _)) = operands.split_first() else {
			return Expr::constant("undefined", Prec::Primary);
		};
		let mut previous = self.expr(first);
		let mut result: Option<Expr> = None;
		for (index, &op) in operators.iter().enumerate() {
			let operand = operands[index + 1];
			let last = index + 2 == operands.len();
			let indent = self.out.indentation() + usize::from(result.is_some());
			let (lines, value) = self.capture_at(indent, |g| g.expr(operand));
			if lines.is_empty() {
				let (first_use, later) = if last {
					(value.clone(), value)
				} else {
					self.twice(value)
				};
				let pair = self.order(operands[index], op, &previous, &first_use);
				result = Some(match result {
					Some(result) => js::binary(&result, "&&", &pair, Prec::And),
					None => pair,
				});
				previous = later;
				continue;
			}
			match result.take() {
				None => {
					// the first pair: the left operand is evaluated before the statements
					if !previous.stable {
						let temp = self.temp();
						self.out.line(format!("const {temp} = {};", previous.text));
						previous = Expr::name(temp, true);
					}
					self.out.append(lines);
					let (first_use, later) = if last {
						(value.clone(), value)
					} else {
						self.twice(value)
					};
					result = Some(self.order(operands[index], op, &previous, &first_use));
					previous = later;
				}
				Some(so_far) => {
					let temp = self.temp();
					self.out.line(format!("let {temp} = {};", so_far.text));
					self.out.open(format!("if ({temp})"));
					self.out.append(lines);
					let (first_use, later) = if last {
						(value.clone(), value)
					} else {
						self.twice(value)
					};
					let pair = self.order(operands[index], op, &previous, &first_use);
					self.out.line(format!("{temp} = {};", pair.text));
					self.out.close("");
					result = Some(Expr::name(temp, true));
					previous = later;
				}
			}
		}
		result.unwrap_or(previous)
	}

	/// The test of an `assert` comparison over operand values computed already.
	pub(crate) fn comparison_of(&mut self, n: NodeId, values: &[Expr]) -> Expr {
		match self.view(n) {
			View::Equality {
				left,
				right,
				negated,
			} => self.equality(left, right, &values[0], &values[1], negated),
			View::Comparison {
				operands,
				operators,
			} => {
				let left = operands.first().expect("an operand");
				let op = operators.get(0).expect("an operator");
				self.order(left, op, &values[0], &values[1])
			}
			_ => values[0].clone(),
		}
	}
}
