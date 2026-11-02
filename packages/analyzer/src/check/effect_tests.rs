//! Unit tests of effects, references and closures (E2): diagnostics and the
//! backend-facing side tables (`FnSig::effects`, `FnInfo::captures`,
//! `Analysis::escapes`).

use crate::analyze_source;
use crate::effects::path_text;
use crate::ids::{FileId, NodeId};
use crate::session::Session;
use crate::types::TypeKind;

/// The codes emitted for `source`, in order.
fn codes(source: &str) -> Vec<&'static str> {
	let (session, file) = analyze_source(source);
	session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| diagnostic.code.id)
		.collect()
}

/// The messages emitted for `source`.
fn messages(source: &str) -> Vec<String> {
	let (session, file) = analyze_source(source);
	session
		.diagnostics(file)
		.iter()
		.map(|diagnostic| format!("{} {}", diagnostic.code.id, diagnostic.message))
		.collect()
}

/// The declaration node of the function named `name`.
fn function_node(session: &Session, file: FileId, name: &str) -> NodeId {
	let analysis = session.analysis(file);
	let symbol = analysis
		.symbols()
		.symbols()
		.iter()
		.find(|symbol| session.types().syms.resolve(symbol.name) == name)
		.unwrap_or_else(|| panic!("no symbol `{name}`"));
	symbol.decl_node
}

/// The effects of a function's signature, printed (`mutates a.b`).
fn effects_of(source: &str, name: &str) -> Vec<String> {
	let (session, file) = analyze_source(source);
	let declaration = function_node(&session, file, name);
	let analysis = session.analysis(file);
	let info = analysis.function(declaration).expect("a function");
	let types = session.types();
	let TypeKind::Function(sig) = types.kind(info.ty) else {
		panic!("not a function type");
	};
	let (mutates, invalidates) = types.effects(types.sig(sig).effects);
	let name_of = |index: u16| -> Option<String> {
		let param = info.params.get(index as usize)?;
		let symbol = analysis.symbols().symbol(param.symbol);
		Some(types.syms.resolve(symbol.name).to_string())
	};
	mutates
		.iter()
		.map(|&path| format!("mutates {}", path_text(types, path, &name_of)))
		.chain(
			invalidates
				.iter()
				.map(|&path| format!("invalidates {}", path_text(types, path, &name_of))),
		)
		.collect()
}

#[test]
fn single_expressions_infer_their_effects() {
	assert_eq!(
		effects_of(
			"function add = (names: Array(String)) => names.push(\"x\")\n",
			"add"
		),
		["mutates names"]
	);
	assert_eq!(
		effects_of(
			"type P = { score: Integer }\nfunction reset = (p: P) => p.score = 0\n",
			"reset"
		),
		["mutates p.score"],
		"any body not starting with `{{` is a single expression"
	);
	assert_eq!(
		effects_of(
			"function last = (items: Array(Integer)) => items.pop()\n",
			"last"
		),
		["mutates items.length", "invalidates items[..]"]
	);
	// a call substitutes the callee's paths
	assert_eq!(
		effects_of(
			"function add = (names: Array(String)) => names.push(\"x\")\nfunction twice = (all: Array(Array(String))) => add(all[0])\n",
			"twice"
		),
		["mutates all[0]"]
	);
}

#[test]
fn declared_effects_are_normalized() {
	assert_eq!(
		effects_of(
			"type P = { a: Integer, b: Integer }\nfunction f = (p: P, q: P) mutates q.a, p, p.b => {\n\tp.a = 1\n\tq.a = 2\n}\n",
			"f"
		),
		["mutates p", "mutates q.a"]
	);
}

#[test]
fn block_bodies_declare_their_effects() {
	let source = "type P = { a: Integer, b: Integer }\nfunction f = (p: P) mutates p.a => {\n\tp.a = 1\n\tp.b = 2\n}\nfunction g = (p: P) => {\n\tp.b = 2\n}\nfunction h = (items: Array(Integer)) => {\n\titems.push(1)\n}\n";
	assert_eq!(codes(source), ["EFF004", "EFF003", "EFF001"]);
	let found = messages(source);
	assert!(found[0].contains("only `p.a` is declared"), "{found:?}");
	assert!(found[1].contains("add `mutates p`"), "{found:?}");
	assert!(found[2].contains("add `mutates items`"), "{found:?}");
}

#[test]
fn mutated_arguments_must_be_mutable() {
	let source = "function add = (names: Array(String)) => names.push(\"x\")\nlet fixed = [\"a\"]\nadd(fixed)\nmutable open = [\"a\"]\nadd(open)\nfor name in [\"b\"] {\n\tadd([name])\n}\n";
	assert_eq!(codes(source), ["EFF007"]);
}

#[test]
fn iteration_and_aliasing() {
	assert_eq!(
		codes("mutable xs = [1, 2]\nfor x in xs {\n\txs.push(0)\n}\n"),
		["REF012"]
	);
	assert_eq!(
		codes("mutable xs = [1, 2]\nfor x, i in xs {\n\txs[i] = x + 1\n}\n"),
		Vec::<&str>::new()
	);
	assert_eq!(codes("mutable xs = [1, 2]\nxs.push(xs[0])\n"), ["REF013"]);
	assert_eq!(
		codes("mutable xs = [1, 2]\nxs.push(xs.length)\n"),
		Vec::<&str>::new()
	);
}

#[test]
fn references_follow_the_flow() {
	// invalidated by a push, valid again once declared again
	assert_eq!(
		codes(
			"mutable xs = [1, 2]\nuse first = xs[0]\nxs.push(3)\nconsole.log(first)\nuse first = xs[0]\nconsole.log(first)\n"
		),
		["REF001"]
	);
	// maybe invalidated after a branch
	assert_eq!(
		codes(
			"mutable xs = [1, 2]\nlet go = xs.length > 1\nuse first = xs[0]\nif go {\n\txs.push(3)\n}\nconsole.log(first)\n"
		),
		["REF002"]
	);
	// `pop` invalidates the last element only
	assert_eq!(
		codes("mutable xs = [1, 2, 3]\nuse first = xs[0]\nxs.pop()\nconsole.log(first)\n"),
		Vec::<&str>::new()
	);
	// declaring a valid reference again
	assert_eq!(
		codes("mutable xs = [1]\nuse a = xs[0]\nuse a = xs[0]\n"),
		["REF003"]
	);
	// `use` needs a place
	assert_eq!(codes("use n = 5\n"), ["REF004"]);
	// `reuse` moves a `mutable use` only
	assert_eq!(
		codes("mutable xs = [1, 2]\nuse a = xs[0]\nreuse a = xs[1]\n"),
		["REF005"]
	);
	// a write through a `use` of an immutable place
	assert_eq!(
		codes("type P = { n: Integer }\nlet ps = [P { n = 1 }]\nuse p = ps[0]\np.n = 2\n"),
		["REF015"]
	);
}

#[test]
fn closures_record_captures_and_escapes() {
	let source = "function main = () => {\n\tmutable count = 0\n\tlet step = 2\n\tfunction bump = () => {\n\t\tcount += step\n\t}\n\tbump()\n\tmutable callbacks: Array(() => Void) = []\n\tcallbacks.push(() => console.log(step))\n}\n";
	let (session, file) = analyze_source(source);
	assert!(
		session.diagnostics(file).is_empty(),
		"{:?}",
		session.diagnostics(file)
	);
	let analysis = session.analysis(file);
	let bump = function_node(&session, file, "bump");
	let captured: Vec<&str> = analysis
		.function(bump)
		.expect("bump")
		.captures
		.iter()
		.map(|&symbol| {
			session
				.types()
				.syms
				.resolve(analysis.symbols().symbol(symbol).name)
		})
		.collect();
	assert_eq!(captured, ["count", "step"]);
	assert_eq!(analysis.escapes.len(), 1, "the lambda given to `push`");
	// an escaping closure can't write what it captures
	assert_eq!(
		codes(
			"function main = () => {\n\tmutable n = 0\n\tmutable fs: Array(() => Void) = []\n\tfs.push(() => {\n\t\tn += 1\n\t})\n}\n"
		),
		["REF021"]
	);
	// a local function's effects happen at its calls
	assert_eq!(
		codes(
			"function main = () => {\n\tmutable xs = [1]\n\tuse first = xs[0]\n\tfunction grow = () => xs.push(2)\n\tgrow()\n\tconsole.log(first)\n}\n"
		),
		["REF001"]
	);
}

/// Whether the read of `name` on line `line` (1-based) is a move.
fn moved(source: &str, line: usize, name: &str) -> bool {
	let (session, file) = analyze_source(source);
	let tree = session.tree(file);
	let analysis = session.analysis(file);
	let start: usize = source
		.lines()
		.take(line - 1)
		.map(|text| text.len() + 1)
		.sum();
	let end = start + source.lines().nth(line - 1).map_or(0, str::len);
	(0..tree.nodes.len()).any(|node| {
		let span = tree.spans[node];
		span.start >= start
			&& span.end <= end
			&& tree.text(node) == name
			&& analysis.has_flags(NodeId::new(node), crate::analysis::NodeFlags::MOVE)
	})
}

#[test]
fn last_uses_that_copy_are_moves() {
	let source = "let a = [1, 2]\nlet b = a\nconsole.log(b)\n";
	assert!(moved(source, 2, "a"));
	// used after the copy: a copy
	let source = "let a = [1, 2]\nlet b = a\nconsole.log(a, b)\n";
	assert!(!moved(source, 2, "a"));
	// inside a loop the declaration is not in: read again by the next iteration
	let source = "let a = [1, 2]\nfor i in 0..3 {\n\tlet b = a\n}\n";
	assert!(!moved(source, 3, "a"));
	// a primitive is never moved, a returned local is
	let source = "function f = (): Array(Integer) => {\n\tmutable items = [1]\n\titems.push(2)\n\titems\n}\nlet n = 1\nlet m = n\n";
	assert!(moved(source, 4, "items"));
	assert!(!moved(source, 7, "n"));
	// pushed into a collection: moved; a local function reading it later keeps it
	let source = "function main = () => {\n\tlet item = [1]\n\tmutable all: Array(Array(Integer)) = []\n\tall.push(item)\n}\n";
	assert!(moved(source, 4, "item"));
	let source = "function main = () => {\n\tlet item = [1]\n\tfunction show = () => console.log(item)\n\tlet copy = item\n\tshow()\n}\n";
	assert!(!moved(source, 4, "item"));
}
