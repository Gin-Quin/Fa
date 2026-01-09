use crate::nodes::{ExtractSymbol, ExtractSymbolKind, ExtractionKind, IdentifierReference, Node};
use crate::parsing::parse::parse;
use crate::parsing::parse_single_statement::parse_single_statement;
use crate::typed_syntax_tree::TypedSyntaxTree;

fn collect_identifiers(tree: &TypedSyntaxTree) -> Vec<(usize, &'static str, IdentifierReference)> {
	let mut identifiers = Vec::new();
	for (index, node) in tree.nodes.iter().enumerate() {
		if let Node::Identifier { name, reference } = node {
			identifiers.push((index, *name, *reference));
		}
	}
	identifiers
}

fn find_function_index(tree: &TypedSyntaxTree, name: &str) -> usize {
	tree.nodes
		.iter()
		.enumerate()
		.find_map(|(index, node)| match node {
			Node::Function {
				name: node_name, ..
			} if *node_name == name => Some(index),
			_ => None,
		})
		.expect("Expected function declaration")
}

fn find_let_index(tree: &TypedSyntaxTree, name: &str) -> usize {
	tree.nodes
		.iter()
		.enumerate()
		.find_map(|(index, node)| match node {
			Node::Let {
				name: node_name, ..
			} if *node_name == name => Some(index),
			_ => None,
		})
		.expect("Expected let declaration")
}

fn find_parameter_declaration(tree: &TypedSyntaxTree, name: &str) -> usize {
	for (index, node) in tree.nodes.iter().enumerate() {
		if let Node::Identifier {
			name: node_name,
			reference: IdentifierReference::Declaration(target),
		} = node
		{
			if *node_name == name && *target == index {
				return index;
			}
		}
	}
	panic!("Expected parameter declaration");
}

fn find_extract_node(tree: &TypedSyntaxTree) -> &Node {
	tree.nodes
		.iter()
		.find(|node| matches!(node, Node::Extract { .. }))
		.expect("Expected extract node")
}

fn find_extract_index(tree: &TypedSyntaxTree) -> usize {
	tree.nodes
		.iter()
		.enumerate()
		.find_map(|(index, node)| matches!(node, Node::Extract { .. }).then_some(index))
		.expect("Expected extract node")
}

fn assert_extract_symbol(symbols: &[ExtractSymbol], name: &str, kind: ExtractSymbolKind) {
	let symbol = symbols
		.iter()
		.find(|symbol| symbol.name == name)
		.expect("Missing extract symbol");
	assert_eq!(symbol.kind, kind);
}

#[test]
fn tracks_external_symbols() {
	let tree = parse_single_statement("foo");
	let identifiers = collect_identifiers(&tree);
	assert_eq!(identifiers.len(), 1);
	let (index, name, reference) = identifiers[0];
	assert_eq!(name, "foo");
	assert_eq!(reference, IdentifierReference::External);
	let references = tree.external_symbols.get("foo").expect("missing external");
	assert_eq!(references.len(), 1);
	assert_eq!(references[0], index);
}

#[test]
fn ignores_member_access_names() {
	let tree = parse_single_statement("foo.bar.baz");
	let identifiers = collect_identifiers(&tree);
	assert_eq!(identifiers.len(), 3);
	let foo = identifiers
		.iter()
		.find(|(_, name, _)| *name == "foo")
		.expect("missing foo");
	let bar = identifiers
		.iter()
		.find(|(_, name, _)| *name == "bar")
		.expect("missing bar");
	let baz = identifiers
		.iter()
		.find(|(_, name, _)| *name == "baz")
		.expect("missing baz");
	assert_eq!(foo.2, IdentifierReference::External);
	assert_eq!(bar.2, IdentifierReference::External);
	assert_eq!(baz.2, IdentifierReference::External);
	assert!(tree.external_symbols.contains_key("foo"));
	assert!(!tree.external_symbols.contains_key("bar"));
	assert!(!tree.external_symbols.contains_key("baz"));
}

#[test]
fn resolves_function_hoisting_same_scope() {
	let tree = parse("foo\nfunction foo = () => 1");
	let function_index = find_function_index(&tree, "foo");
	let identifiers = collect_identifiers(&tree);
	let usage = identifiers
		.iter()
		.find(|(_, name, _)| *name == "foo")
		.expect("missing foo usage");
	assert_eq!(usage.2, IdentifierReference::Declaration(function_index));
	assert!(!tree.external_symbols.contains_key("foo"));
}

#[test]
fn resolves_outer_function_for_inner_use() {
	let tree = parse("do { foo }\nfunction foo = () => 1");
	let function_index = find_function_index(&tree, "foo");
	let identifiers = collect_identifiers(&tree);
	let usage = identifiers
		.iter()
		.find(|(_, name, _)| *name == "foo")
		.expect("missing foo usage");
	assert_eq!(usage.2, IdentifierReference::Declaration(function_index));
	assert!(!tree.external_symbols.contains_key("foo"));
}

#[test]
fn does_not_hoist_inner_function_outward() {
	let tree = parse("foo\ndo { function foo = () => 1\nbar }");
	let identifiers = collect_identifiers(&tree);
	let usage = identifiers
		.iter()
		.find(|(_, name, _)| *name == "foo")
		.expect("missing foo usage");
	assert_eq!(usage.2, IdentifierReference::External);
	assert!(tree.external_symbols.contains_key("foo"));
}

#[test]
fn resolves_function_parameters() {
	let tree = parse_single_statement("(a, b) => a + b");
	let a_declaration = find_parameter_declaration(&tree, "a");
	let b_declaration = find_parameter_declaration(&tree, "b");
	let identifiers = collect_identifiers(&tree);
	let a_usages: Vec<usize> = identifiers
		.iter()
		.filter(|(index, name, reference)| {
			*name == "a"
				&& *index != a_declaration
				&& *reference == IdentifierReference::Declaration(a_declaration)
		})
		.map(|(index, ..)| *index)
		.collect();
	let b_usages: Vec<usize> = identifiers
		.iter()
		.filter(|(index, name, reference)| {
			*name == "b"
				&& *index != b_declaration
				&& *reference == IdentifierReference::Declaration(b_declaration)
		})
		.map(|(index, ..)| *index)
		.collect();
	assert_eq!(a_usages.len(), 1);
	assert_eq!(b_usages.len(), 1);
	assert!(!tree.external_symbols.contains_key("a"));
	assert!(!tree.external_symbols.contains_key("b"));
}

#[test]
fn block_scopes_shadow_and_pop() {
	let tree = parse("do { let a = 1\n a }\n a");
	let let_index = find_let_index(&tree, "a");
	let identifiers = collect_identifiers(&tree);
	let inner_usage = identifiers
		.iter()
		.find(|(_, name, reference)| {
			*name == "a" && *reference == IdentifierReference::Declaration(let_index)
		})
		.expect("missing inner usage");
	let outer_usage = identifiers
		.iter()
		.find(|(index, name, reference)| {
			*name == "a" && *index != inner_usage.0 && *reference == IdentifierReference::External
		})
		.expect("missing outer usage");
	assert_eq!(inner_usage.2, IdentifierReference::Declaration(let_index));
	assert_eq!(outer_usage.2, IdentifierReference::External);
	assert!(tree.external_symbols.contains_key("a"));
}

#[test]
fn namespace_scopes_do_not_leak() {
	let tree = parse("namespace Foo = { let a = 1\n a }\n a");
	let let_index = find_let_index(&tree, "a");
	let identifiers = collect_identifiers(&tree);
	let inner_usage = identifiers
		.iter()
		.find(|(_, name, reference)| {
			*name == "a" && *reference == IdentifierReference::Declaration(let_index)
		})
		.expect("missing inner usage");
	let outer_usage = identifiers
		.iter()
		.find(|(index, name, reference)| {
			*name == "a" && *index != inner_usage.0 && *reference == IdentifierReference::External
		})
		.expect("missing outer usage");
	assert_eq!(inner_usage.2, IdentifierReference::Declaration(let_index));
	assert_eq!(outer_usage.2, IdentifierReference::External);
	assert!(tree.external_symbols.contains_key("a"));
}

#[test]
fn extract_symbols_with_outer_use_members() {
	let tree = parse_single_statement("object >> use { a, b = 4 }");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::Member);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "a", ExtractSymbolKind::Alias);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Alias);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_inner_use_let_members() {
	let tree = parse_single_statement("object >> { a, use b, let c = 1 }");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::Member);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Alias);
			assert_extract_symbol(symbols, "c", ExtractSymbolKind::Copy);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_index_members() {
	let tree = parse_single_statement("array >> use [a, b = 4]");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::Index);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "a", ExtractSymbolKind::Alias);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Alias);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_tuple_members() {
	let tree = parse_single_statement("tuple >> (a, let b)");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::TupleMember);
			assert_eq!(symbols.len(), 1);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Copy);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_identifier_outer_use() {
	let tree = parse_single_statement("values >> use item");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::TupleMember);
			assert_eq!(symbols.len(), 1);
			assert_extract_symbol(symbols, "item", ExtractSymbolKind::Alias);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_identifier_no_use() {
	let tree = parse_single_statement("values >> item");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::TupleMember);
			assert!(symbols.is_empty());
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_tuple_outer_use() {
	let tree = parse_single_statement("values >> use (a, b)");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::TupleMember);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "a", ExtractSymbolKind::Alias);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Alias);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_tuple_outer_let() {
	let tree = parse_single_statement("values >> let (a, b)");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::TupleMember);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "a", ExtractSymbolKind::Copy);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Copy);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_members_no_use() {
	let tree = parse_single_statement("object >> { a, b = 4 }");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::Member);
			assert!(symbols.is_empty());
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
fn extract_symbols_with_index_outer_let() {
	let tree = parse_single_statement("array >> let [a, b = 4]");
	let extract = find_extract_node(&tree);
	match extract {
		Node::Extract {
			symbols,
			extraction_kind,
			..
		} => {
			assert_eq!(*extraction_kind, ExtractionKind::Index);
			assert_eq!(symbols.len(), 2);
			assert_extract_symbol(symbols, "a", ExtractSymbolKind::Copy);
			assert_extract_symbol(symbols, "b", ExtractSymbolKind::Copy);
		}
		_ => panic!("Expected extract node"),
	}
}

#[test]
#[should_panic(expected = "Unexpected `use` in extract members")]
fn extract_rejects_nested_use_with_outer_use() {
	parse_single_statement("object >> use { use a }");
}

#[test]
#[should_panic(expected = "Invalid extract member")]
fn extract_rejects_invalid_member_expression() {
	parse_single_statement("object >> { a + b }");
}

#[test]
#[should_panic(expected = "Invalid extract group expression")]
fn extract_rejects_invalid_tuple_expression() {
	parse_single_statement("tuple >> (a + b)");
}

#[test]
fn extract_symbols_declare_in_scope() {
	let tree = parse("values >> use item\nitem");
	let extract_index = find_extract_index(&tree);
	let identifiers = collect_identifiers(&tree);
	let usage = identifiers
		.iter()
		.find(|(_, name, reference)| {
			*name == "item" && *reference == IdentifierReference::Declaration(extract_index)
		})
		.expect("missing item usage");
	assert_eq!(usage.2, IdentifierReference::Declaration(extract_index));
}

#[test]
fn extract_without_symbols_does_not_declare() {
	let tree = parse("values >> item\nitem");
	let identifiers = collect_identifiers(&tree);
	let usage = identifiers
		.iter()
		.filter(|(_, name, _)| *name == "item")
		.max_by_key(|(index, ..)| *index)
		.expect("missing item usage");
	assert_eq!(usage.2, IdentifierReference::External);
}
