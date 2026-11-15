//! Statically generated binary operations. A factory contains typed functions,
//! not a schema or source to compile at run time. Recursive edges are queued
//! on a worklist, so a long value never consumes the JavaScript call stack.

use std::collections::{BTreeSet, HashMap};

use fa_analyzer::ids::{NominalId, TypeId};
use fa_analyzer::types::{CollKind, FieldFlags, FloatBits, SyncKind, TypeKind};

use super::{Generator, Repr};
use crate::js;

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
struct Node(TypeId, bool);

#[derive(Clone)]
pub(super) struct Factory {
	name: String,
	params: Vec<TypeId>,
}

struct Graph {
	nodes: Vec<Node>,
	ids: HashMap<Node, usize>,
	params: Vec<TypeId>,
}

impl Graph {
	fn write(&self, node: Node) -> String {
		format!("$w{}", self.ids[&node])
	}
	fn read(&self, node: Node) -> String {
		format!("$r{}", self.ids[&node])
	}
	fn child(&self, ty: TypeId) -> Node {
		Node(ty, self.nodes[0].1)
	}
	fn param(&self, param: TypeId) -> String {
		format!(
			"$p{}",
			self.params
				.iter()
				.position(|&known| known == param)
				.expect("a graph parameter")
		)
	}
}

impl Generator<'_> {
	/// A codec using the representation of this type, including host-reference
	/// categories. Its descriptor identity is supplied by the actual instance.
	pub(crate) fn codec_for_type(&mut self, ty: TypeId) -> Option<String> {
		if let Some(value) = self.hidden_codec_type(ty) {
			return Some(value);
		}
		self.make_codec(Node(ty, false))
	}

	/// DATA stored under a synchronization owner, including unique resources.
	/// The surrounding Mutex/Channel owns those values; no Resource reference
	/// is created or shared by this codec.
	pub(crate) fn payload_codec_for_type(&mut self, ty: TypeId) -> Option<String> {
		self.make_codec(Node(ty, true))
	}

	fn make_codec(&mut self, root: Node) -> Option<String> {
		if let Some(factory) = self.codecs.get(&(root.0, root.1)).cloned() {
			return self.codec_application(&factory);
		}
		let graph = self.codec_graph(root)?;
		let codec = self.helper("faCodec");
		let static_codec = self.helper("faStaticCodec");
		let mut body = String::new();
		for &node in &graph.nodes {
			let (write, read, extra) = self.codec_steps(&graph, node, codec)?;
			body.push_str(&format!(
				"function {}($o, $v, $s) {{\n{write}\n}}\n",
				graph.write(node)
			));
			body.push_str(&format!(
				"function {}($i, $t, $k, $s) {{\n{read}\n}}\n",
				graph.read(node)
			));
			body.push_str(&extra);
		}
		body.push_str(&format!(
			"return {static_codec}({}, {});",
			graph.write(root),
			graph.read(root)
		));
		let names: Vec<_> = (0..graph.params.len())
			.map(|index| format!("$p{index}"))
			.collect();
		let code = if names.is_empty() {
			format!("(() => {{\n{body}\n}})()")
		} else {
			format!("({}) => {{\n{body}\n}}", names.join(", "))
		};
		let name = self.constant("codec", code);
		let factory = Factory {
			name,
			params: graph.params,
		};
		self.codecs.insert((root.0, root.1), factory.clone());
		self.codec_application(&factory)
	}

	fn hidden_codec_type(&self, ty: TypeId) -> Option<String> {
		let expected = self.types.stable_hash(ty);
		self.hidden_names.iter().find_map(|(hidden, value)| {
			let super::generics::Hidden::Codec(actual) = hidden else {
				return None;
			};
			(self.types.stable_hash(*actual) == expected).then(|| value.clone())
		})
	}

	fn codec_application(&self, factory: &Factory) -> Option<String> {
		if factory.params.is_empty() {
			return Some(factory.name.clone());
		}
		let arguments: Option<Vec<_>> = factory
			.params
			.iter()
			.map(|&param| {
				self.hidden_codec_type(param)
					.map(|codec| format!("{{codec: {codec}}}"))
			})
			.collect();
		Some(format!("{}({})", factory.name, arguments?.join(", ")))
	}

	fn codec_graph(&self, root: Node) -> Option<Graph> {
		let mut graph = Graph {
			nodes: vec![root],
			ids: HashMap::from([(root, 0)]),
			params: Vec::new(),
		};
		let mut params = BTreeSet::new();
		let mut index = 0;
		while index < graph.nodes.len() {
			let node = graph.nodes[index];
			index += 1;
			let kind = self.types.kind(node.0);
			if matches!(kind, TypeKind::Param(_))
				|| matches!(kind, TypeKind::Union(_))
					&& self
						.types
						.flags(node.0)
						.contains(fa_analyzer::types::TypeFlags::HAS_PARAM)
			{
				// The caller supplies this union after normalization. No recipe
				// is interpreted here, and substitution may collapse it entirely.
				params.insert(node.0);
				continue;
			}
			for child in self.codec_children(node)? {
				if graph.ids.contains_key(&child) {
					continue;
				}
				graph.ids.insert(child, graph.nodes.len());
				graph.nodes.push(child);
			}
		}
		graph.params.extend(params);
		Some(graph)
	}

	fn codec_fields(&self, ty: TypeId) -> Option<Vec<(String, TypeId)>> {
		let mut fields = Vec::new();
		for field in self.types.fields_of(ty) {
			if field.flags.contains(FieldFlags::RELATION) {
				return None;
			}
			if !field.flags.contains(FieldFlags::METHOD) {
				fields.push((self.types.syms.resolve(field.name).to_string(), field.ty));
			}
		}
		fields.sort_by(|left, right| left.0.cmp(&right.0));
		Some(fields)
	}

	fn codec_members(&self, nominal: NominalId, mask: fa_analyzer::ids::MaskId) -> Vec<u16> {
		let count = self
			.types
			.sym_list(self.types.nominal(nominal).members)
			.len() as u16;
		(0..count)
			.filter(|&index| self.types.mask_has(mask, index))
			.collect()
	}

	fn codec_children(&self, Node(ty, payload): Node) -> Option<Vec<Node>> {
		if self.types.is_resource(ty) && !payload {
			return None;
		}
		let children = match self.types.kind(ty) {
			TypeKind::None
			| TypeKind::Void
			| TypeKind::Boolean
			| TypeKind::BoolLit(_)
			| TypeKind::Int(_)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::BigInt
			| TypeKind::BigIntLit(_)
			| TypeKind::String
			| TypeKind::StringLit(_)
			| TypeKind::Character
			| TypeKind::Percentage
			| TypeKind::Fields(_)
			| TypeKind::Enum(..)
			| TypeKind::EnumMember(..)
			| TypeKind::Range
			| TypeKind::Function(_)
			| TypeKind::Subscription
			| TypeKind::Sync(..) => Vec::new(),
			TypeKind::Param(param) if !self.types.param(param).value => Vec::new(),
			TypeKind::Rec(_) => vec![self.types.rec_target(ty)?],
			TypeKind::Object(_) => self
				.codec_fields(ty)?
				.into_iter()
				.map(|(_, ty)| ty)
				.collect(),
			TypeKind::Tuple(list) | TypeKind::Union(list) => self.types.list_items(list).to_vec(),
			TypeKind::NamedUnion(nominal, mask) => self
				.codec_members(nominal, mask)
				.into_iter()
				.map(|index| self.types.payload(nominal, index))
				.filter(|ty| !ty.is_unset())
				.map(|ty| (payload || !self.types.is_resource(ty)).then_some(ty))
				.collect::<Option<Vec<_>>>()?,
			TypeKind::Member(nominal, index) => {
				let ty = self.types.payload(nominal, index);
				if !ty.is_unset() && self.types.is_resource(ty) && !payload {
					return None;
				}
				if ty.is_unset() { Vec::new() } else { vec![ty] }
			}
			TypeKind::Collection(
				CollKind::Array
				| CollKind::GrowingArray
				| CollKind::OrderedColumnar
				| CollKind::Set
				| CollKind::GrowingSet,
				item,
				_,
			) => vec![item],
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, item, key) => {
				vec![key, item]
			}
			_ => return None,
		};
		Some(children.into_iter().map(|ty| Node(ty, payload)).collect())
	}

	fn codec_steps(
		&mut self,
		graph: &Graph,
		node: Node,
		codec: &str,
	) -> Option<(String, String, String)> {
		let Node(ty, payload) = node;
		let scalar = |write: &str, read: &str| {
			(
				format!("{codec}.{write}($o, $v);"),
				format!("$t[$k] = {codec}.{read}($i);"),
				String::new(),
			)
		};
		let literal = |value: String| (String::new(), format!("$t[$k] = {value};"), String::new());
		let reference = |name: &str| {
			(
				format!(
					"{codec}.writeReference($o, {}, $v);",
					js::string_literal(name)
				),
				format!(
					"$t[$k] = {codec}.readReference($i, {});",
					js::string_literal(name)
				),
				String::new(),
			)
		};
		if self.types.is_resource(ty) && !payload {
			return None;
		}
		if graph.params.contains(&ty) {
			let parameter = graph.param(ty);
			return Some((
				format!("$s.push({parameter}.codec.$write, $v);"),
				format!("$s.push({parameter}.codec.$read, $t, $k);"),
				String::new(),
			));
		}
		if !matches!(self.types.kind(ty), TypeKind::FloatLit(..))
			&& let Some(value) = self.literal_of(ty)
		{
			return Some(literal(value.text));
		}
		Some(match self.types.kind(ty) {
			TypeKind::None | TypeKind::Void => literal("undefined".into()),
			TypeKind::Boolean => (
				format!("{codec}.writeUint8($o, $v ? 1 : 0);"),
				format!(
					"const $v = {codec}.readUint8($i); if ($v > 1) throw new TypeError(\"invalid Fa Boolean\"); $t[$k] = $v !== 0;"
				),
				String::new(),
			),
			TypeKind::Int(id) => {
				let int = self.types.int(id);
				let mut result = if int.storage.is_js_bigint() {
					(
						format!("{codec}.writeVarBigInt($o, $v, Number.MAX_SAFE_INTEGER);"),
						format!("const $v = {codec}.readVarBigInt($i, Number.MAX_SAFE_INTEGER);"),
						String::new(),
					)
				} else if int.min >= 0i128 && int.max <= 255i128 {
					(
						format!("{codec}.writeUint8($o, $v);"),
						format!("const $v = {codec}.readUint8($i);"),
						String::new(),
					)
				} else {
					(
						format!("{codec}.writeVarInt($o, $v);"),
						format!("const $v = {codec}.readVarInt($i);"),
						String::new(),
					)
				};
				let suffix = if int.storage.is_js_bigint() { "n" } else { "" };
				result.1.push_str(&format!(" if ($v < {}{suffix} || $v > {}{suffix}) throw new TypeError(\"invalid Fa integer\"); $t[$k] = $v;", int.min, int.max));
				result
			}
			TypeKind::SymInt(_) => scalar("writeVarInt", "readVarInt"),
			TypeKind::Float(FloatBits::F32) | TypeKind::FloatLit(FloatBits::F32, _) => {
				scalar("writeFloat32", "readFloat32")
			}
			TypeKind::Float(_) | TypeKind::FloatLit(..) | TypeKind::Percentage => {
				scalar("writeFloat64", "readFloat64")
			}
			TypeKind::BigInt => (
				format!("{codec}.writeVarBigInt($o, $v, Number.MAX_SAFE_INTEGER);"),
				format!("$t[$k] = {codec}.readVarBigInt($i, Number.MAX_SAFE_INTEGER);"),
				String::new(),
			),
			TypeKind::String | TypeKind::Character => scalar("writeVarString", "readVarString"),
			TypeKind::Fields(_) if self.types.fields_width(ty) > 32 => (
				format!("{codec}.writeVarBigInt($o, $v, Number.MAX_SAFE_INTEGER);"),
				format!("$t[$k] = {codec}.readVarBigInt($i, Number.MAX_SAFE_INTEGER);"),
				String::new(),
			),
			TypeKind::Fields(_) | TypeKind::Sync(SyncKind::Ordering, _) => {
				scalar("writeVarUint", "readVarUint")
			}
			TypeKind::Function(_) => reference("fa.function"),
			TypeKind::Subscription => reference("fa.sync:Subscription"),
			TypeKind::Sync(kind, _) => reference(&format!("fa.sync:{}", kind.name())),

			TypeKind::Rec(_) => {
				let child = graph.child(self.types.rec_target(ty)?);
				(
					format!("$s.push({}, $v);", graph.write(child)),
					format!("$s.push({}, $t, $k);", graph.read(child)),
					String::new(),
				)
			}
			TypeKind::Object(_) => {
				let fields = self.codec_fields(ty)?;
				self.codec_object_steps(graph, fields, "{}".into())
			}
			TypeKind::Tuple(list) => {
				let items = self.types.list_items(list);
				let mut write = String::new();
				let mut read = format!("const $v = $t[$k] = new Array({});\n", items.len());
				for (index, &child) in items.iter().enumerate().rev() {
					write.push_str(&format!(
						"$s.push({}, $v[{index}]);\n",
						graph.write(graph.child(child))
					));
					read.push_str(&format!(
						"$s.push({}, $v, {index});\n",
						graph.read(graph.child(child))
					));
				}
				(write, read, String::new())
			}
			TypeKind::Collection(
				CollKind::Array | CollKind::GrowingArray | CollKind::OrderedColumnar,
				item,
				_,
			) => {
				let child = graph.child(item);
				let length = self.helper("faWireLength");
				// This fast path has the exact format of the typed element steps.
				// The decoder owns an Array, never a view into a reusable packet.
				if self
					.types
					.as_int(item)
					.is_some_and(|int| int.min == 0i128 && int.max == 255i128)
				{
					(
						format!("{codec}.writeVarUint8Array($o, Uint8Array.from($v));"),
						format!("$t[$k] = Array.from({codec}.readVarUint8Array($i));"),
						String::new(),
					)
				} else {
					(
						format!(
							"{codec}.writeVarUint($o, $v.length); for (let $n = $v.length - 1; $n >= 0; $n--) $s.push({}, $v[$n]);",
							graph.write(child)
						),
						format!(
							"const $n = {length}($i); const $v = $t[$k] = new Array($n); for (let $j = $n - 1; $j >= 0; $j--) $s.push({}, $v, $j);",
							graph.read(child)
						),
						String::new(),
					)
				}
			}
			TypeKind::Collection(CollKind::Map | CollKind::GrowingMap, item, key) => {
				self.codec_map_steps(graph, node, key, item, codec)
			}
			TypeKind::Collection(CollKind::Set | CollKind::GrowingSet, item, _) => {
				self.codec_set_steps(graph, node, item, codec)
			}
			TypeKind::Union(list) => {
				let mut members = self.types.list_items(list).to_vec();
				members.sort_by_cached_key(|&ty| {
					fa_analyzer::stable_names::stable_type_name(self.types, ty)
				});
				let mut write = String::new();
				let mut read = format!("switch ({codec}.readVarUint($i)) {{\n");
				let mut tests = std::collections::HashSet::new();
				for (index, member) in members.into_iter().enumerate() {
					let test = self.codec_test(ty, member)?;
					if !tests.insert(test.clone()) {
						return None;
					}
					write.push_str(&format!(
						"if ({test}) {{ {codec}.writeVarUint($o, {index}); $s.push({}, $v); return; }}\n",
						graph.write(graph.child(member))
					));
					read.push_str(&format!(
						"case {index}: $s.push({}, $t, $k); return;\n",
						graph.read(graph.child(member))
					));
				}
				write.push_str("throw new TypeError(\"invalid Fa union\");");
				read.push_str("default: throw new TypeError(\"invalid Fa union tag\"); }");
				(write, read, String::new())
			}
			TypeKind::Enum(nominal, mask) => {
				let members = self.codec_members(nominal, mask);
				self.codec_enum_steps(nominal, &members, codec)
			}
			TypeKind::EnumMember(nominal, index) => literal(self.enum_value(nominal, index).text),
			TypeKind::NamedUnion(nominal, mask) => {
				let mut write = String::new();
				let mut read = format!("switch ({codec}.readVarUint($i)) {{\n");
				let mut extra = String::new();
				for index in self.codec_members(nominal, mask) {
					let (w, r, e) = self.codec_member_steps(graph, node, nominal, index)?;
					let test = self.codec_member_test(nominal, index);
					write.push_str(&format!(
						"if ({test}) {{ {codec}.writeVarUint($o, {index}); {w} return; }}\n"
					));
					read.push_str(&format!("case {index}: {{ {r} return; }}\n"));
					extra.push_str(&e);
				}
				write.push_str("throw new TypeError(\"invalid Fa named union\");");
				read.push_str("default: throw new TypeError(\"invalid Fa named-union tag\"); }");
				(write, read, extra)
			}
			TypeKind::Member(nominal, index) => {
				self.codec_member_steps(graph, node, nominal, index)?
			}
			TypeKind::Range => {
				let class = self.helper("FaRange");
				(
					format!(
						"{codec}.writeUint8($o, ($v.start !== undefined ? 1 : 0) | ($v.end !== undefined ? 2 : 0) | ($v.includesEnd ? 4 : 0)); if ($v.start !== undefined) {codec}.writeVarInt($o, $v.start); if ($v.end !== undefined) {codec}.writeVarInt($o, $v.end);"
					),
					format!(
						"const $f = {codec}.readUint8($i); if ($f > 7) throw new TypeError(\"invalid Fa range\"); $t[$k] = new {class}(($f & 1) ? {codec}.readVarInt($i) : undefined, ($f & 2) ? {codec}.readVarInt($i) : undefined, ($f & 4) !== 0);"
					),
					String::new(),
				)
			}
			_ => return None,
		})
	}

	fn codec_object_steps(
		&self,
		graph: &Graph,
		fields: Vec<(String, TypeId)>,
		constructor: String,
	) -> (String, String, String) {
		let mut write = String::new();
		let mut read = format!("const $v = $t[$k] = {constructor};\n");
		for (name, _) in &fields {
			read.push_str(&format!("Object.defineProperty($v, {}, {{ value: undefined, writable: true, enumerable: true, configurable: true }});\n", js::string_literal(&Self::property(name))));
		}
		for (name, ty) in fields.into_iter().rev() {
			let property = js::string_literal(&Self::property(&name));
			write.push_str(&format!(
				"$s.push({}, $v[{property}]);\n",
				graph.write(graph.child(ty))
			));
			read.push_str(&format!(
				"$s.push({}, $v, {property});\n",
				graph.read(graph.child(ty))
			));
		}
		(write, read, String::new())
	}

	fn codec_map_steps(
		&mut self,
		graph: &Graph,
		node: Node,
		key: TypeId,
		item: TypeId,
		codec: &str,
	) -> (String, String, String) {
		let class = if self.is_primitive(key) {
			"Map".to_string()
		} else {
			self.helper("FaHashMap").to_string()
		};
		let length = self.helper("faWireLength");
		let finish = format!("{}$finish", graph.read(node));
		let extra = format!(
			"function {finish}($i, $v, $k, $s) {{ for (const $e of $v.entries) {{ if ($v.map.has($e[0])) throw new TypeError(\"duplicate Fa map key\"); $v.map.set($e[0], $e[1]); }} }}\n"
		);
		let write = format!(
			"{codec}.writeVarUint($o, $v.size); const $e = Array.from($v.entries()); for (let $j = $e.length - 1; $j >= 0; $j--) {{ $s.push({}, $e[$j][1], {}, $e[$j][0]); }}",
			graph.write(graph.child(item)),
			graph.write(graph.child(key))
		);
		let read = format!(
			"const $n = {length}($i); const $v = $t[$k] = new {class}(); const $e = new Array($n); $s.push({finish}, {{ map: $v, entries: $e }}, 0); for (let $j = $n - 1; $j >= 0; $j--) {{ $e[$j] = new Array(2); $s.push({}, $e[$j], 1, {}, $e[$j], 0); }}",
			graph.read(graph.child(item)),
			graph.read(graph.child(key))
		);
		(write, read, extra)
	}

	fn codec_set_steps(
		&mut self,
		graph: &Graph,
		node: Node,
		item: TypeId,
		codec: &str,
	) -> (String, String, String) {
		let class = if self.is_primitive(item) {
			"Set".to_string()
		} else {
			self.helper("FaSet").to_string()
		};
		let length = self.helper("faWireLength");
		let finish = format!("{}$finish", graph.read(node));
		let extra = format!(
			"function {finish}($i, $v, $k, $s) {{ for (const $e of $v.entries) {{ if ($v.set.has($e)) throw new TypeError(\"duplicate Fa set key\"); $v.set.add($e); }} }}\n"
		);
		let write = format!(
			"{codec}.writeVarUint($o, $v.size); const $e = Array.from($v); for (let $j = $e.length - 1; $j >= 0; $j--) $s.push({}, $e[$j]);",
			graph.write(graph.child(item))
		);
		let read = format!(
			"const $v = $t[$k] = new {class}(); const $n = {length}($i); const $e = new Array($n); $s.push({finish}, {{ set: $v, entries: $e }}, 0); for (let $j = $n - 1; $j >= 0; $j--) $s.push({}, $e, $j);",
			graph.read(graph.child(item))
		);
		(write, read, extra)
	}

	fn codec_enum_steps(
		&mut self,
		nominal: NominalId,
		members: &[u16],
		codec: &str,
	) -> (String, String, String) {
		let mut write = String::new();
		let mut read = format!("switch ({codec}.readVarUint($i)) {{\n");
		for &index in members {
			let value = self.enum_value(nominal, index);
			write.push_str(&format!(
				"if ($v === {}) {{ {codec}.writeVarUint($o, {index}); return; }}\n",
				value.text
			));
			read.push_str(&format!("case {index}: $t[$k] = {}; return;\n", value.text));
		}
		write.push_str("throw new TypeError(\"invalid Fa enum value\");");
		read.push_str("default: throw new TypeError(\"invalid Fa enum tag\"); }");
		(write, read, String::new())
	}

	fn codec_member_test(&mut self, nominal: NominalId, index: u16) -> String {
		if self.is_errors(nominal) {
			let class = self.error_class(nominal, index);
			return format!(
				"$v?.[Symbol.for(\"fa.error\")] === {class}.prototype[Symbol.for(\"fa.error\")]"
			);
		}
		let payload = self.types.payload(nominal, index);
		if let Some(literal) = self.literal_of(payload).filter(|_| !payload.is_unset()) {
			return format!("$v === {}", literal.text);
		}
		format!(
			"$v?.$t === {}",
			js::string_literal(&self.member_qualified_name(nominal, index))
		)
	}

	fn codec_member_steps(
		&mut self,
		graph: &Graph,
		node: Node,
		nominal: NominalId,
		index: u16,
	) -> Option<(String, String, String)> {
		let payload = self.types.payload(nominal, index);
		let error = self.is_errors(nominal);
		let class = error.then(|| self.error_class(nominal, index));
		if payload.is_unset() {
			let value = if let Some(class) = class {
				format!("new {class}()")
			} else {
				self.nominal_member_value(fa_analyzer::ids::NodeId::new(0), nominal, index)
					.text
			};
			return Some((String::new(), format!("$t[$k] = {value};"), String::new()));
		}
		if !error && let Some(value) = self.literal_of(payload) {
			return Some((
				String::new(),
				format!("$t[$k] = {};", value.text),
				String::new(),
			));
		}
		if matches!(self.types.kind(payload), TypeKind::Object(_)) {
			let fields = self.codec_fields(payload)?;
			let constructor = match class {
				Some(class) => format!("Object.create({class}.prototype)"),
				None => format!(
					"{{ $t: {} }}",
					js::string_literal(&self.member_qualified_name(nominal, index))
				),
			};
			return Some(self.codec_object_steps(graph, fields, constructor));
		}
		let finish = format!("{}$member{index}", graph.read(node));
		let constructor = match class {
			Some(class) => format!("new {class}($v.value)"),
			None => format!(
				"{{ $t: {}, $v: $v.value }}",
				js::string_literal(&self.member_qualified_name(nominal, index))
			),
		};
		let extra =
			format!("function {finish}($i, $v, $k, $s) {{ $v.target[$v.key] = {constructor}; }}\n");
		Some((
			format!("$s.push({}, $v.$v);", graph.write(graph.child(payload))),
			format!(
				"const $v = {{ target: $t, key: $k, value: undefined }}; $s.push({finish}, $v, 0, {}, $v, \"value\");",
				graph.read(graph.child(payload))
			),
			extra,
		))
	}

	fn codec_test(&mut self, union: TypeId, member: TypeId) -> Option<String> {
		if let Some(value) = self.literal_of(member) {
			return Some(format!("$v === {}", value.text));
		}
		Some(match self.types.kind(member) {
			TypeKind::Rec(_) => {
				return self.codec_test(union, self.types.rec_target(member)?);
			}
			TypeKind::Param(_) => return None,
			TypeKind::None => "$v === undefined".into(),
			TypeKind::Member(nominal, index) => self.codec_member_test(nominal, index),
			TypeKind::NamedUnion(nominal, mask) => self
				.codec_members(nominal, mask)
				.into_iter()
				.map(|index| self.codec_member_test(nominal, index))
				.collect::<Vec<_>>()
				.join(" || "),
			TypeKind::Object(_) => {
				let mut tests = vec![
					"typeof $v === \"object\" && $v !== null && !Array.isArray($v)".to_string(),
				];
				for (name, ty) in self.codec_fields(member)? {
					if let Some(literal) = self.literal_of(ty) {
						tests.push(format!(
							"$v[{}] === {}",
							js::string_literal(&Self::property(&name)),
							literal.text
						));
					}
				}
				// Object alternatives need a literal discriminator; a mere kind
				// test could select the wrong layout and lose data.
				if tests.len() == 1
					&& self
						.types
						.union_members(&union)
						.iter()
						.filter(|&&ty| matches!(self.types.kind(ty), TypeKind::Object(_)))
						.count() > 1
				{
					return None;
				}
				tests.join(" && ")
			}
			TypeKind::Tuple(_)
			| TypeKind::Collection(
				CollKind::Array | CollKind::GrowingArray | CollKind::OrderedColumnar,
				..,
			) => "Array.isArray($v)".into(),
			TypeKind::Int(id) if self.repr(member) == Repr::BigInt => {
				let int = self.types.int(id);
				format!(
					"typeof $v === \"bigint\" && $v >= {}n && $v <= {}n",
					int.min, int.max
				)
			}
			TypeKind::Enum(nominal, mask) => self
				.codec_members(nominal, mask)
				.into_iter()
				.map(|index| format!("$v === {}", self.enum_value(nominal, index).text))
				.collect::<Vec<_>>()
				.join(" || "),
			TypeKind::Int(id) => {
				let int = self.types.int(id);
				format!(
					"Number.isInteger($v) && !Object.is($v, -0) && $v >= {} && $v <= {}",
					int.min, int.max
				)
			}
			TypeKind::Float(FloatBits::F32) => {
				"typeof $v === \"number\" && Object.is(Math.fround($v), $v)".into()
			}
			TypeKind::EnumMember(..)
			| TypeKind::Float(_)
			| TypeKind::FloatLit(..)
			| TypeKind::Percentage => "typeof $v === \"number\"".into(),
			_ => match self.repr(member) {
				Repr::BigInt => "typeof $v === \"bigint\"".into(),
				Repr::String => "typeof $v === \"string\"".into(),
				Repr::Boolean => "typeof $v === \"boolean\"".into(),
				_ => return None,
			},
		})
	}
}

#[cfg(test)]
mod tests {
	use std::sync::atomic::{AtomicU64, Ordering};

	use crate::{Options, Target};

	static NEXT: AtomicU64 = AtomicU64::new(0);

	fn generated(definitions: &str) -> String {
		let source = format!(
			"function carry = <Item>(_value: Item): Void => {{}}\n{definitions}\nfunction request = (value: Payload): Void => carry(value)\n"
		);
		let (mut session, file) = fa_analyzer::analyze_source(&source);
		session.set_target("bun");
		session.analyze_file(file);
		let analysis = session.analysis(file);
		let mut requests = Vec::new();
		for wanted in [
			"arrayWitness",
			"byteWitness",
			"optionalArrayWitness",
			"optionalByteWitness",
			"optionalWitness",
			"request",
		] {
			let Some(symbol) = analysis
				.symbols
				.symbols()
				.iter()
				.find(|symbol| session.types().syms.resolve(symbol.name) == wanted)
			else {
				continue;
			};
			let info = analysis
				.function(symbol.decl_node)
				.expect("a test function");
			requests.push(analysis.symbols.symbol(info.params[0].symbol).ty);
		}
		let options = Options::new(Target::Bun, "wire.fa");
		let mut generator = super::Generator::new(&session, file, &options, None);
		for ty in requests {
			if let Some(graph) = generator.codec_graph(super::Node(ty, false)) {
				for (index, parameter) in graph.params.into_iter().enumerate() {
					generator.hidden_names.insert(
						super::super::generics::Hidden::Codec(parameter),
						format!("$test$type{index}"),
					);
				}
			}
			let codec = generator.codec_for_type(ty);
			if !session
				.types()
				.flags(ty)
				.contains(fa_analyzer::types::TypeFlags::HAS_PARAM)
			{
				assert!(codec.is_some(), "the concrete test type needs a codec");
			}
		}
		let program = generator.program();
		assert!(generator.errors.is_empty(), "{:?}", generator.errors);
		assert!(program.contains("const codec ="), "no codec was generated");
		let runtime = crate::runtime::collect_helpers_inline(generator.helpers);
		format!("{runtime}\n{program}")
	}

	fn run(program: &str, checks: &str) {
		if std::process::Command::new("bun")
			.arg("--version")
			.output()
			.is_err()
		{
			return;
		}
		let path = std::env::temp_dir().join(format!(
			"fa-generated-wire-{}-{}.mjs",
			std::process::id(),
			NEXT.fetch_add(1, Ordering::Relaxed)
		));
		std::fs::write(
			&path,
			format!("{program}\n{checks}\nconsole.log('fa-codec-test-ok');\n"),
		)
		.unwrap();
		let output = std::process::Command::new("bun")
			.arg(&path)
			.output()
			.unwrap();
		let _ = std::fs::remove_file(path);
		assert!(
			output.status.success(),
			"{}\n{}",
			String::from_utf8_lossy(&output.stdout),
			String::from_utf8_lossy(&output.stderr)
		);
		assert!(String::from_utf8_lossy(&output.stdout).contains("fa-codec-test-ok"));
	}

	#[test]
	fn ordinary_generic_calls_do_not_request_binary_operations() {
		let source =
			"function identity = <Item>(value: Item): Item => value\nconsole.log(identity(7))\n";
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let output =
			crate::generate(&mut session, file, &Options::new(Target::Bun, "plain.fa")).unwrap();
		assert!(!output.helpers.contains(&"faCodec"));
		assert!(!output.helpers.contains(&"faStaticCodec"));
		run(&output.code, "");
	}

	#[test]
	fn direct_wire_capabilities_keep_regular_argument_positions() {
		let source = "use { Mutex } = std.concurrency\nfunction holder = <Item>(value: Item): Mutex<Item | none> => Mutex<Item | none> { value = copy value }\nfunction forward = <Element>(value: Element): Mutex<Element | none> => holder(value)\nmutable cell = forward(7)\nassert (cell.tryLock()?.value ?? 0) == 7\n";
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let output =
			crate::generate(&mut session, file, &Options::new(Target::Bun, "direct.fa")).unwrap();
		assert!(output.helpers.contains(&"faStaticCodec"));
		run(&output.code, "");
	}

	#[test]
	fn unbound_indirect_wire_abis_are_rejected_explicitly() {
		let source = "use { Mutex } = std.concurrency\nfunction holder = <Item>(value: Item): Mutex<Item | none> => Mutex<Item | none> { value = copy value }\nfunction apply = (read: <Element>(value: Element) => Mutex<Element | none>): Integer => { mutable _cell = read(7)\n0 }\nconsole.log(apply(holder))\n";
		let (mut session, file) = fa_analyzer::analyze_source(source);
		let outcome = crate::generate(
			&mut session,
			file,
			&Options::new(Target::Bun, "indirect.fa"),
		);
		assert!(outcome.is_err(), "an unbound wire ABI must be rejected");
		let errors = outcome.unwrap_err();
		assert!(
			errors
				.diagnostics
				.iter()
				.any(|error| error.code.id == "AN001"
					&& error.message.contains("binary transport capabilities")),
			"{errors:?}"
		);
	}

	#[test]
	fn unique_resources_have_only_protected_data_codecs() {
		let source = "type Token = { value: Int32, function destroy = (self) => console.log(self.value) }\ntype Payload = { token: Token, tokens: Array<Token> }\nfunction request = (value: Payload): Void => {}\n";
		let mut session = fa_analyzer::Session::new();
		let file = session.set_source("wire.foreign", source.into());
		session.set_target("bun");
		session.analyze_file(file);
		let errors: Vec<_> = session
			.diagnostics(file)
			.iter()
			.filter(|diagnostic| diagnostic.severity == fa_analyzer::Severity::Error)
			.collect();
		assert!(errors.is_empty(), "{errors:?}");
		let analysis = session.analysis(file);
		let symbol = analysis
			.symbols
			.symbols()
			.iter()
			.find(|symbol| session.types().syms.resolve(symbol.name) == "request")
			.unwrap();
		let info = analysis.function(symbol.decl_node).unwrap();
		let ty = analysis.symbols.symbol(info.params[0].symbol).ty;
		let options = Options::new(Target::Bun, "wire.foreign.fa");
		let mut generator = super::Generator::new(&session, file, &options, None);
		assert!(generator.codec_for_type(ty).is_none());
		let codec = generator
			.payload_codec_for_type(ty)
			.expect("protected DATA codec");
		let program = generator.program();
		assert!(generator.errors.is_empty(), "{:?}", generator.errors);
		assert!(!program.contains("fa.resource"));
		let runtime = crate::runtime::collect_helpers_inline(generator.helpers);
		run(
			&format!("{runtime}\n{program}"),
			&format!(
				"const copied = {codec}.decode({codec}.encode({{token: {{value: 7}}, tokens: [{{value: 9}}]}})); if (copied.token.value !== 7 || copied.tokens[0].value !== 9) throw new Error('protected resource data changed');"
			),
		);
	}

	#[test]
	fn recursive_object_wire_operations_do_not_use_the_host_stack() {
		let program = generated("type Payload = { value: Int32, next: Payload | none }");
		run(
			&program,
			r#"
let value;
for (let index = 0; index < 16000; index++) value = { value: index, next: value };
let copied = codec.decode(codec.encode(value));
for (let index = 15999; index >= 0; index--) {
 if (copied.value !== index) throw new Error("recursive payload changed");
 copied = copied.next;
}
if (copied !== undefined) throw new Error("recursive tail changed");
"#,
		);
	}

	#[test]
	fn recursive_named_union_arrays_are_encoded_by_typed_steps() {
		let program = generated(
			"type Payload = Union { Leaf = { value: Integer }, Branch = { children: Array<Payload> } }",
		);
		run(
			&program,
			r#"
let value = { $t: "Payload.Leaf", value: 7 };
for (let index = 0; index < 12000; index++) value = { $t: "Payload.Branch", children: [value] };
let copied = codec.decode(codec.encode(value));
for (let index = 0; index < 12000; index++) {
 if (copied.$t !== "Payload.Branch" || copied.children.length !== 1) throw new Error("union payload changed");
 copied = copied.children[0];
}
if (copied.$t !== "Payload.Leaf" || copied.value !== 7) throw new Error("union leaf changed");
"#,
		);
	}

	#[test]
	fn decoded_byte_arrays_own_their_data_after_packet_reuse() {
		let program = generated("type Payload = Array<UInt8>");
		run(
			&program,
			r#"
const bytes = codec.encode([1, 2, 255]);
const value = codec.decode(bytes);
bytes.fill(0);
if (value.join(",") !== "1,2,255") throw new Error("decoded bytes alias the packet");
"#,
		);
	}

	#[test]
	fn typed_enums_errors_and_structural_discriminators_round_trip() {
		let program = generated(
			r#"
type Color = Enum { Red = 7, Blue = 42 }
type Failure = Errors { Bad = { message: String, code: UInt8 }, Empty }
type Payload = { color: Color, error: Failure, value: { kind: "number", number: Int32 } | { kind: "text", text: String } }
"#,
		);
		run(
			&program,
			r#"
for (const value of [{ kind: "number", number: -3 }, { kind: "text", text: "hello" }]) {
 const copied = codec.decode(codec.encode({ color: 42, error: new Failure$Bad("problem", 9), value }));
 if (copied.color !== 42 || !(copied.error instanceof Failure$Bad) || copied.error.message !== "problem" || copied.error.code !== 9) throw new Error("nominal value changed");
 if (copied.value.kind !== value.kind || copied.value.number !== value.number || copied.value.text !== value.text) throw new Error("discriminated value changed");
}
"#,
		);
	}

	#[test]
	fn generic_array_factories_have_the_concrete_wire_format() {
		let program = generated(
			r#"
function arrayWitness = <Element>(value: Array<Element>): Void => carry(value)
function byteWitness = (value: UInt8): Void => carry(value)
type Payload = Array<UInt8>
"#,
		);
		run(
			&program,
			r#"
const generic = codec({ codec: codec$1 });
const value = [0, 1, 127, 128, 255];
const a = generic.encode(value);
const b = codec$2.encode(value);
if (a.length !== b.length || a.some((byte, index) => byte !== b[index])) throw new Error("generic wire format differs");
if (codec$2.decode(a).join(",") !== value.join(",") || generic.decode(b).join(",") !== value.join(",")) throw new Error("generic cross-decode differs");
"#,
		);
	}

	#[test]
	fn composite_optional_array_capabilities_use_the_concrete_wire_format() {
		let program = generated(
			r#"
function optionalArrayWitness = <Element>(value: Array<Element | none>): Void => carry(value)
function optionalByteWitness = (value: UInt8 | none): Void => carry(value)
type Payload = Array<UInt8 | none>
"#,
		);
		run(
			&program,
			r#"
const generic = codec({ codec: codec$1 });
const values = [undefined, 0, 255, undefined];
const concrete = codec$2.encode(values);
const erased = generic.encode(values);
if (concrete.length !== erased.length || concrete.some((byte, index) => byte !== erased[index])) throw new Error("normalized union wire differs");
const copied = generic.decode(concrete);
if (copied.length !== 4 || copied[0] !== undefined || copied[1] !== 0 || copied[2] !== 255 || copied[3] !== undefined) throw new Error("normalized union payload changed");
"#,
		);
	}

	#[test]
	fn concrete_numeric_unions_preserve_fractions_and_wide_integers() {
		let program = generated("type Payload = { number: Float, wide: Int64 | Float }");
		run(
			&program,
			r#"
for (const number of [1.5, -7, -0, Infinity, NaN]) {
 const value = { number, wide: 1n << 60n };
 const copied = codec.decode(codec.encode(value));
 if (!Object.is(copied.number, number) || copied.wide !== value.wide) throw new Error("numeric union changed representation or value");
}
"#,
		);
	}

	#[test]
	fn unresolved_union_wire_does_not_block_ordinary_generic_calls() {
		let program = generated(
			r#"
function optionalWitness = <Element>(value: Element | none): Void => carry(value)
type Payload = UInt8
"#,
		);
		run(
			&program,
			r#"
optionalWitness({ ownsTask: false, hasCollection: false, codec }, 7);
if (codec.decode(codec.encode(42)) !== 42) throw new Error("concrete codec changed");
"#,
		);
	}

	#[test]
	fn separately_generated_modules_exchange_the_same_wire_format_in_a_worker() {
		let first = generated(
			r#"
type Payload = { title: String, values: Array<Integer>, pairs: Map<String, Integer>, flags: Set<String>, big: BigInt, float: Float }
"#,
		);
		let second = generated(
			r#"
type Noise = { values: Boolean, float: Character }
type Payload = { float: Float, big: BigInt, flags: Set<String>, pairs: Map<String, Integer>, values: Array<Integer>, title: String }
"#,
		);
		assert!(!first.contains("new Function"));
		assert!(!first.contains("readSchema"));
		let path = std::env::temp_dir().join(format!(
			"fa-wire-worker-{}-{}.mjs",
			std::process::id(),
			NEXT.fetch_add(1, Ordering::Relaxed)
		));
		std::fs::write(&path, format!("{second}\nglobalThis.onmessage = (event) => {{ const value = codec.decode(event.data); value.values.push(9); globalThis.postMessage(codec.encode(value)); }};\n")).unwrap();
		let checks = format!(
			r#"
const worker = new Worker({});
const original = {{ title: "a\u0000🙂", values: [-1, 2, 3], pairs: new Map([["x", 4]]), flags: new Set(["a", "b"]), big: 1n << 1800n, float: -0 }};
try {{
 const bytes = await new Promise((resolve, reject) => {{
  const timeout = setTimeout(() => reject(new Error("wire worker timed out")), 5000);
  worker.onmessage = event => {{ clearTimeout(timeout); resolve(event.data); }};
  worker.onerror = event => {{ clearTimeout(timeout); reject(event.error ?? event.message); }};
  worker.postMessage(codec.encode(original));
 }});
 const value = codec.decode(bytes);
 if (value.title !== original.title || value.values.join(",") !== "-1,2,3,9" || value.pairs.get("x") !== 4 || !value.flags.has("b") || value.big !== original.big || !Object.is(value.float, -0)) throw new Error("cross-module worker wire differs");
}} finally {{ await worker.terminate(); }}
"#,
			crate::js::string_literal(&path.to_string_lossy())
		);
		run(&first, &checks);
		let _ = std::fs::remove_file(path);
	}
}
