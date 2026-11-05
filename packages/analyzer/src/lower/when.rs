//! The decision list of a `when` (values brief §17.2), shared by the backends.
//!
//! A `when` evaluates its subject once, then tests its arms in order and runs
//! the first that matches. [`decision_list`] turns its arms into that order of
//! tests, from the syntax and the analysis: the alternatives of a pattern
//! (`A | B`) are flattened, literals are decoded, ranges keep their bound
//! nodes (any integer expression), and type patterns (`Integer`, `Shape.Circle`,
//! `ApiError`) give the type the analysis recorded on them (`TYPE_EXPR`).
//!
//! In a program without errors every `when` is exhaustive (statements
//! included): a backend may test every arm but the last one and run the last
//! one by default (an `unreachable` default when it has tests), or use
//! [`DecisionList::has_else`] to know whether the default is an `else` arm.
//!
//! ```text
//! when shape is {                    Decision { tests: [IsType(Shape.Circle)], binding, value }
//!     Shape.Circle as { radius } => …
//!     1 | 2 => …                     Decision { tests: [Equals(1), Equals(2)], … }
//!     3..=9 => …                     Decision { tests: [InRange { 3, 9, inclusive }], … }
//!     else => …                      Decision { tests: [], … }   (else: always matches)
//! }
//! ```

use ethnum::I256;
use smallvec::SmallVec;

use fa_parser::TypedSyntaxTree;

use crate::analysis::{Analysis, NodeFlags};
use crate::ids::{NodeId, TypeId};
use crate::syntax::{self, View};

/// A literal of a pattern.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Literal<'t> {
	/// An integer (negative literals included).
	Integer(I256),
	/// A float.
	Float(f64),
	/// A string or a character (escapes decoded).
	String(&'t str),
	/// `true` or `false`.
	Boolean(bool),
	/// `none`.
	None,
}

/// One way for the subject to match an arm.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Test<'t> {
	/// The subject equals a literal.
	Equals(Literal<'t>),
	/// The subject is an integer in a range: `start <= subject` and
	/// `subject < end` (`<=` when `inclusive`); a missing bound is open.
	/// The bounds are expression nodes, evaluated after the subject.
	InRange {
		start: Option<NodeId>,
		end: Option<NodeId>,
		inclusive: bool,
	},
	/// The subject's value is of a type: a member of a named union, error
	/// type or enumeration (test the tag), a whole named union or error type,
	/// or a member type of an anonymous union (`Integer`, `String`, `None`).
	IsType(TypeId),
}

/// One arm of a `when`, in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision<'t> {
	/// The arm matches when any of these tests passes; none for `else`.
	pub tests: SmallVec<[Test<'t>; 2]>,
	/// The pattern bound with `as` (a name or a destructuring pattern), if any.
	pub binding: Option<NodeId>,
	/// The value of the arm: a block or an expression.
	pub value: NodeId,
}

impl Decision<'_> {
	/// Whether the arm is `else`: it always matches.
	pub fn is_else(&self) -> bool {
		self.tests.is_empty()
	}
}

/// The decisions of a `when`, in source order.
#[derive(Clone, Debug, PartialEq)]
pub struct DecisionList<'t> {
	/// The subject, evaluated once before the tests.
	pub subject: NodeId,
	/// Its type (before narrowing).
	pub subject_type: TypeId,
	/// The arms.
	pub decisions: Vec<Decision<'t>>,
}

impl DecisionList<'_> {
	/// Whether the last arm is `else`.
	pub fn has_else(&self) -> bool {
		self.decisions.last().is_some_and(Decision::is_else)
	}
}

/// Why a `when` has no decision list: the program has errors there, which
/// the analysis reported.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LowerError {
	/// The node is not a `when`.
	NotAWhen,
	/// A pattern that is neither a literal, a range nor a type (an error).
	InvalidPattern(NodeId),
}

/// The decision list of the `when` node `n`.
pub fn decision_list<'t>(
	tree: &'t TypedSyntaxTree,
	analysis: &Analysis,
	n: NodeId,
) -> Result<DecisionList<'t>, LowerError> {
	let View::When { subject, arms } = syntax::view(tree, n) else {
		return Err(LowerError::NotAWhen);
	};
	let mut decisions = Vec::with_capacity(arms.len());
	for arm in arms.iter() {
		let mut tests = SmallVec::new();
		if let Some(pattern) = arm.pattern {
			add_tests(tree, analysis, pattern, &mut tests)?;
		}
		decisions.push(Decision {
			tests,
			binding: arm.binding,
			value: arm.value,
		});
	}
	Ok(DecisionList {
		subject,
		subject_type: analysis.type_of(subject),
		decisions,
	})
}

/// The tests of a pattern, alternatives flattened in order.
fn add_tests<'t>(
	tree: &'t TypedSyntaxTree,
	analysis: &Analysis,
	pattern: NodeId,
	tests: &mut SmallVec<[Test<'t>; 2]>,
) -> Result<(), LowerError> {
	// a set of flags: equal to the pattern's bits (check/fields.rs)
	if let Some(fa_interpreter::Value::Integer(bits)) = analysis.constant(pattern)
		&& analysis.fields_patterns.contains(&pattern)
	{
		tests.push(Test::Equals(Literal::Integer(*bits)));
		return Ok(());
	}
	// a type pattern, as the analysis resolved it
	if analysis.has_flags(pattern, NodeFlags::TYPE_EXPR) {
		let ty = analysis.type_of(pattern);
		if ty.is_unset() || ty == TypeId::UNKNOWN {
			return Err(LowerError::InvalidPattern(pattern));
		}
		tests.push(Test::IsType(ty));
		return Ok(());
	}
	let test = match syntax::view(tree, pattern) {
		View::Union { operands } => {
			for operand in operands.iter() {
				add_tests(tree, analysis, operand, tests)?;
			}
			return Ok(());
		}
		View::Group(inner) => return add_tests(tree, analysis, inner, tests),
		View::Integer(value) => Test::Equals(Literal::Integer(value)),
		View::Float(value) => Test::Equals(Literal::Float(value)),
		View::Negate(inner) => match syntax::view(tree, inner) {
			View::Integer(value) => Test::Equals(Literal::Integer(-value)),
			View::Float(value) => Test::Equals(Literal::Float(-value)),
			_ => return Err(LowerError::InvalidPattern(pattern)),
		},
		View::String(text) => Test::Equals(Literal::String(text)),
		View::Boolean(value) => Test::Equals(Literal::Boolean(value)),
		View::None => Test::Equals(Literal::None),
		View::Range {
			start,
			end,
			inclusive,
		} => Test::InRange {
			start,
			end,
			inclusive,
		},
		_ => return Err(LowerError::InvalidPattern(pattern)),
	};
	tests.push(test);
	Ok(())
}

/// A name bound by the `as` of an arm.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArmBinding {
	/// The field it reads (`as { radius }`, `as { radius as r }`), `None`
	/// for `as name` (the subject, or the data of a member that holds no
	/// object).
	pub field: Option<fa_parser::Name>,
	/// Its symbol.
	pub symbol: crate::ids::SymbolId,
}

/// The names bound by the `as` of an arm ([`Decision::binding`]), each
/// with the field it reads, paired by name (a field the analysis didn't
/// bind, an error already reported, is left out).
pub fn arm_bindings(
	tree: &TypedSyntaxTree,
	analysis: &Analysis,
	binding: NodeId,
) -> Vec<ArmBinding> {
	let symbols = &analysis.symbols;
	let Some(first) = symbols.symbol_of_decl(binding) else {
		return Vec::new();
	};
	match syntax::view(tree, binding) {
		View::Name(_) => vec![ArmBinding {
			field: None,
			symbol: first,
		}],
		View::ObjectPattern { fields, .. } => {
			// the symbols of the pattern are declared together, in order
			let declared: Vec<crate::ids::SymbolId> = (first.index()..symbols.symbols().len())
				.map(crate::ids::SymbolId::new)
				.take_while(|&symbol| symbols.symbol(symbol).decl_node == binding)
				.collect();
			fields
				.iter()
				.filter_map(|field| {
					let bound = syntax::name_span(field.alias.unwrap_or(field.name));
					let symbol = declared
						.iter()
						.copied()
						.find(|&symbol| symbols.symbol(symbol).name_span == bound)?;
					Some(ArmBinding {
						field: Some(field.name),
						symbol,
					})
				})
				.collect()
		}
		_ => Vec::new(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::ids::FileId;

	/// The tree of `source`, an empty analysis, and the first `when` node.
	fn when_of(source: &str) -> (TypedSyntaxTree, Analysis, NodeId) {
		let tree = fa_parser::parse(source);
		let analysis = Analysis::new(FileId(0), tree.nodes.len());
		let when = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| matches!(syntax::view(&tree, node), View::When { .. }))
			.expect("a `when`");
		(tree, analysis, when)
	}

	#[test]
	fn literals_alternatives_ranges_and_else() {
		let (tree, analysis, when) = when_of(
			"when code is {\n\t1 | -2 => \"small\"\n\t3..=9 => \"digit\"\n\t10.. => \"big\"\n\telse => \"negative\"\n}\n",
		);
		let list = decision_list(&tree, &analysis, when).unwrap();
		assert_eq!(tree.text(list.subject.index()), "code");
		assert_eq!(list.decisions.len(), 4);
		assert_eq!(
			list.decisions[0].tests.as_slice(),
			[
				Test::Equals(Literal::Integer(I256::new(1))),
				Test::Equals(Literal::Integer(I256::new(-2)))
			]
		);
		let Test::InRange {
			start: Some(start),
			end: Some(end),
			inclusive: true,
		} = list.decisions[1].tests[0]
		else {
			panic!("not a closed range: {:?}", list.decisions[1].tests);
		};
		assert_eq!(
			(tree.text(start.index()), tree.text(end.index())),
			("3", "9")
		);
		assert!(matches!(
			list.decisions[2].tests[0],
			Test::InRange {
				start: Some(_),
				end: None,
				..
			}
		));
		assert!(list.decisions[3].is_else());
		assert!(list.has_else());
		assert_eq!(tree.text(list.decisions[3].value.index()), "\"negative\"");
	}

	#[test]
	fn strings_booleans_none_and_floats() {
		let (tree, analysis, when) =
			when_of("when x is {\n\t\"a\" | \"b\" => 1\n\ttrue => 2\n\tnone => 3\n\t1.5 => 4\n}\n");
		let list = decision_list(&tree, &analysis, when).unwrap();
		assert_eq!(
			list.decisions[0].tests.as_slice(),
			[
				Test::Equals(Literal::String("a")),
				Test::Equals(Literal::String("b"))
			]
		);
		assert_eq!(
			list.decisions[1].tests[0],
			Test::Equals(Literal::Boolean(true))
		);
		assert_eq!(list.decisions[2].tests[0], Test::Equals(Literal::None));
		assert_eq!(
			list.decisions[3].tests[0],
			Test::Equals(Literal::Float(1.5))
		);
		assert!(!list.has_else());
	}

	#[test]
	fn type_patterns_come_from_the_analysis() {
		let (tree, mut analysis, when) =
			when_of("when value is {\n\tInteger as n => n\n\tString | none => 0\n}\n");
		let integer = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| tree.text(node.index()) == "Integer")
			.unwrap();
		// unanalyzed: a name is not a pattern by itself
		assert_eq!(
			decision_list(&tree, &analysis, when),
			Err(LowerError::InvalidPattern(integer))
		);
		for (name, ty) in [("Integer", TypeId::INTEGER), ("String", TypeId::STRING)] {
			let node = (0..tree.nodes.len())
				.map(NodeId::new)
				.find(|&node| tree.text(node.index()) == name)
				.unwrap();
			analysis.flags[node.index()] |= NodeFlags::TYPE_EXPR;
			analysis.types[node.index()] = ty;
		}
		let list = decision_list(&tree, &analysis, when).unwrap();
		assert_eq!(list.decisions[0].tests[0], Test::IsType(TypeId::INTEGER));
		assert_eq!(tree.text(list.decisions[0].binding.unwrap().index()), "n");
		assert_eq!(
			list.decisions[1].tests.as_slice(),
			[Test::IsType(TypeId::STRING), Test::Equals(Literal::None)]
		);
	}

	#[test]
	fn not_a_when() {
		let tree = fa_parser::parse("let a = 1\n");
		let analysis = Analysis::new(FileId(0), tree.nodes.len());
		assert_eq!(
			decision_list(&tree, &analysis, NodeId::new(tree.root)),
			Err(LowerError::NotAWhen)
		);
	}

	#[test]
	fn arm_bindings_pair_fields_with_symbols_by_name() {
		let source = concat!(
			"type Shape = Union {\n\tCircle = { radius: Float, label: String }\n\tPoint\n}\n",
			"function describe = (shape: Shape) => when shape is {\n",
			"\tShape.Circle as { label as name, radius } => \"{name} {radius}\"\n",
			"\tShape.Point as point => \"point\"\n}\n",
			"console.log(describe(Shape.Circle { radius = 1, label = \"c\" }))\n",
		);
		let (session, file) = crate::analyze_source(source);
		let tree = session.tree(file);
		let analysis = session.analysis(file);
		let when = (0..tree.nodes.len())
			.map(NodeId::new)
			.find(|&node| matches!(syntax::view(tree, node), View::When { .. }))
			.expect("a `when`");
		let list = decision_list(tree, analysis, when).unwrap();
		let circle = arm_bindings(tree, analysis, list.decisions[0].binding.unwrap());
		let pairs: Vec<(String, String)> = circle
			.iter()
			.map(|binding| {
				let field = tree.name(binding.field.unwrap()).to_string();
				let symbol = analysis.symbols.symbol(binding.symbol).name;
				(field, session.types().syms.resolve(symbol).to_string())
			})
			.collect();
		assert_eq!(
			pairs,
			[
				("label".to_string(), "name".to_string()),
				("radius".to_string(), "radius".to_string())
			]
		);
		let point = arm_bindings(tree, analysis, list.decisions[1].binding.unwrap());
		assert_eq!(point.len(), 1);
		assert!(point[0].field.is_none());
	}
}
