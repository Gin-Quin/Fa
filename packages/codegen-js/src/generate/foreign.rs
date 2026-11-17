//! Foreign code on `bun` (documentation/advanced/foreign-code.md,
//! JavaScript notes): an `@extern` function is a JavaScript function that
//! binds its symbol on first call (`faFfiBind`: `bun:ffi`'s `dlopen`),
//! searching the libraries of `build.link`, then the C library. A call converts its arguments by the passing rules the analyzer
//! recorded (`Analysis::foreign_args`): a string becomes a zero-terminated
//! `Uint8Array`, an array a typed array (copied back after the call when the
//! parameter is `mutates`), a value given by address a one-item typed array
//! (written back to its place), a function a `JSCallback` closed after the
//! call. A function taking or returning a `CStruct` by value goes through
//! a shim compiled by Bun's TinyCC (`bun:ffi`'s `cc`, `faFfiBindStruct`):
//! its arguments are written into memory with the C layout the compiler
//! computes (`c_layout`), and the shim calls the function with them by
//! value, under the platform's C ABI.

use fa_analyzer::analysis::{Argument, ForeignArg};
use fa_analyzer::ids::{NodeId, TypeId};
use fa_analyzer::syntax::View;
use fa_analyzer::types::{CollKind, FloatBits, TypeKind};

use crate::generate::Generator;
use crate::js::{self, Expr, Prec};

/// The C declarations of the shim of a function taking `CStruct`s by value.
#[derive(Default)]
struct CDeclarations {
	/// The `CStruct` types declared: their C name, spec, size and alignment.
	structs: Vec<(TypeId, String, String, u64, u64)>,
	/// Their `typedef`s, each after those of its fields.
	typedefs: Vec<String>,
}

impl Generator<'_> {
	/// The C memory layout of a pointer's target, or opaque Fa data.
	fn pointer_spec(&self, target: TypeId) -> Option<String> {
		match self.types.kind(target) {
			TypeKind::Function(_) => return None,
			TypeKind::Int(id)
				if matches!(
					self.types.int(id).storage,
					fa_analyzer::types::IntStorage::I128 | fa_analyzer::types::IntStorage::U128
				) =>
			{
				return None;
			}
			_ => {}
		}
		self.c_layout(target, &mut CDeclarations::default())
			.map(|(_, _, _, spec)| spec)
	}

	/// `Pointer.allocate(value)`: C-compatible data in memory, other Fa
	/// values rooted behind an opaque C address.
	pub(crate) fn pointer_allocate(&mut self, n: NodeId, arguments: &[Argument]) -> Expr {
		let Some(Argument::Given(value)) = arguments.first() else {
			return self.internal_error(n, "the checked pointer allocation has no value argument");
		};
		let Some(target) = self.types.pointer_target(self.type_of(n)) else {
			return self.internal_error(
				n,
				"the checked pointer allocation has no pointer result type",
			);
		};
		if self
			.types
			.flags(target)
			.contains(fa_analyzer::types::TypeFlags::HAS_PARAM)
		{
			return self.unsupported(n, "this pointer allocation with an unresolved target type");
		}
		let spec = self
			.pointer_spec(target)
			.unwrap_or_else(|| "null".to_string());
		let expression = self.expr(*value);
		let expression = self.copy_stored(*value, expression);
		let helper = self.helper("faPointerAllocate");
		js::call(helper, &[expression.text, spec])
	}

	/// The C type name of a Fa type (`bun:ffi`'s names), or `None` for a type
	/// JavaScript can't give to C.
	pub(crate) fn c_type_name(&self, ty: TypeId) -> Option<&'static str> {
		let types = self.types;
		Some(match types.kind(ty) {
			TypeKind::Void | TypeKind::Never => "void",
			TypeKind::Boolean | TypeKind::BoolLit(_) => "bool",
			TypeKind::Int(id) => {
				use fa_analyzer::types::IntStorage as S;
				match types.int(id).storage {
					S::I8 => "i8",
					S::U8 => "u8",
					S::I16 => "i16",
					S::U16 => "u16",
					S::I32 => "i32",
					S::U32 => "u32",
					S::U64 => "u64",
					_ => "i64",
				}
			}
			TypeKind::Float(FloatBits::F32) => "f32",
			TypeKind::Float(FloatBits::F64) => "f64",
			TypeKind::Pointer(_) | TypeKind::CString => "ptr",
			TypeKind::Union(list)
				if types.pointer_target(ty).is_some()
					|| types
						.list_items(list)
						.iter()
						.any(|&item| types.kind(item) == TypeKind::CString) =>
			{
				"ptr"
			}
			TypeKind::Function(_) => "function",
			_ => return None,
		})
	}

	/// `function name: (…) => R = @extern("symbol")`: a function binding the
	/// symbol on its first call.
	pub(crate) fn extern_declaration(&mut self, n: NodeId, name: &str, symbol: &str) {
		let ty = self
			.analysis
			.symbols
			.symbol_of_decl(n)
			.map_or(TypeId::UNKNOWN, |symbol| {
				self.analysis.symbols.symbol(symbol).ty
			});
		let TypeKind::Function(sig) = self.types.kind(ty) else {
			self.internal_error(
				n,
				"the checked extern declaration has no function signature",
			);
			return;
		};
		let sig = self.types.sig(sig);
		let param_types: Vec<TypeId> = self
			.types
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		// a `CStruct` by value: through a shim (`faFfiBindStruct`)
		if param_types
			.iter()
			.chain([&sig.ret])
			.any(|&ty| self.is_c_struct(ty))
		{
			self.extern_struct_declaration(n, name, symbol, &param_types, sig.ret);
			return;
		}
		let mut args = Vec::new();
		for &param in &param_types {
			match self.c_type_name(param) {
				Some(name) => args.push(js::string_literal(name)),
				None => {
					self.foreign_by_value(n);
					return;
				}
			}
		}
		let Some(returns) = self.c_type_name(sig.ret) else {
			self.foreign_by_value(n);
			return;
		};
		let bind = self.helper("faFfiBind");
		let libraries: Vec<String> = self
			.options
			.libraries
			.iter()
			.map(|library| js::string_literal(library))
			.collect();
		self.out.line(format!("function {name}(...values) {{"));
		self.out.line(format!(
			"\t{name}.c ??= {bind}([{}], {}, [{}], {});",
			libraries.join(", "),
			js::string_literal(symbol),
			args.join(", "),
			js::string_literal(returns)
		));
		self.out.line(format!("\treturn {name}.c(...values);"));
		self.out.line("}");
	}

	/// Whether a type is a `CStruct`.
	fn is_c_struct(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::Object(shape) => self.types.shape(shape).nominal.is_some_and(|nominal| {
				self.types.nominal(nominal).kind == fa_analyzer::types::NominalKind::CStruct
			}),
			_ => false,
		}
	}

	/// The C layout of a type in memory (64-bit platforms: Bun runs on no
	/// other): its size, its alignment, its C type (`int`, `fa_struct0`,
	/// `float[3]`), and its spec for `faCWrite`/`faCRead`: a C type name,
	/// `{ size, fields: [[name, spec, offset, bigint], …] }`, or
	/// `{ array: spec, count, stride, bigint }`. A `CStruct` is declared
	/// once, as a `typedef` of `declarations`.
	fn c_layout(
		&self,
		ty: TypeId,
		declarations: &mut CDeclarations,
	) -> Option<(u64, u64, String, String)> {
		let types = self.types;
		if let TypeKind::CArray(item, count) = types.kind(ty) {
			let (size, align, c, spec) = self.c_layout(item, declarations)?;
			let bigint = self.is_bigint(item);
			return Some((
				size * u64::from(count),
				align,
				format!("{c}[{count}]"),
				format!("{{ array: {spec}, count: {count}, stride: {size}, bigint: {bigint} }}"),
			));
		}
		if self.is_c_struct(ty) {
			if let Some((_, c_name, spec, size, align)) =
				declarations.structs.iter().find(|(other, ..)| *other == ty)
			{
				return Some((*size, *align, c_name.clone(), spec.clone()));
			}
			let fields: Vec<(String, TypeId)> = types
				.fields_of(ty)
				.iter()
				.map(|field| (types.syms.resolve(field.name).to_string(), field.ty))
				.collect();
			let (mut offset, mut align) = (0u64, 1u64);
			let mut members = Vec::new();
			let mut specs = Vec::new();
			for (name, field_type) in fields {
				let (size, field_align, c, spec) = self.c_layout(field_type, declarations)?;
				offset = offset.div_ceil(field_align) * field_align;
				align = align.max(field_align);
				// `c` is `type` or `type[n]`: the name goes before the brackets
				members.push(match c.find('[') {
					Some(bracket) => format!("{} f_{name}{}", &c[..bracket], &c[bracket..]),
					None => format!("{c} f_{name}"),
				});
				let bigint = self.is_bigint(field_type);
				specs.push(format!(
					"[{}, {spec}, {offset}, {bigint}]",
					js::string_literal(&Self::property(&name))
				));
				offset += size;
			}
			let size = offset.div_ceil(align).max(1) * align;
			let c_name = format!("fa_struct{}", declarations.structs.len());
			let spec = format!("{{ size: {size}, fields: [{}] }}", specs.join(", "));
			if members.is_empty() {
				members.push("char f_empty".to_string());
			}
			declarations.typedefs.push(format!(
				"typedef struct {{ {}; }} {c_name};",
				members.join("; ")
			));
			declarations
				.structs
				.push((ty, c_name.clone(), spec.clone(), size, align));
			return Some((size, align, c_name, spec));
		}
		let name = self.c_type_name(ty)?;
		let (size, c) = match name {
			"bool" => (1, "_Bool"),
			"i8" => (1, "signed char"),
			"u8" => (1, "unsigned char"),
			"i16" => (2, "short"),
			"u16" => (2, "unsigned short"),
			"i32" => (4, "int"),
			"u32" => (4, "unsigned int"),
			"f32" => (4, "float"),
			"i64" => (8, "long long"),
			"u64" => (8, "unsigned long long"),
			"f64" => (8, "double"),
			"ptr" | "function" => (8, "void *"),
			_ => return None,
		};
		let spec = if name == "function" { "ptr" } else { name };
		Some((size, size, c.to_string(), js::string_literal(spec)))
	}

	/// Whether values of a type are JavaScript `bigint`s.
	fn is_bigint(&self, ty: TypeId) -> bool {
		match self.types.kind(ty) {
			TypeKind::Int(id) => self.types.int(id).storage.is_js_bigint(),
			_ => false,
		}
	}

	/// An `@extern` function taking or returning a `CStruct` by value: a
	/// shim, compiled by TinyCC on first call, calls it with its arguments
	/// read from memory (`faFfiBindStruct`).
	fn extern_struct_declaration(
		&mut self,
		n: NodeId,
		name: &str,
		symbol: &str,
		params: &[TypeId],
		ret: TypeId,
	) {
		let mut declarations = CDeclarations::default();
		let mut param_c = Vec::new();
		let mut param_specs = Vec::new();
		for &param in params {
			let Some((_, _, c, spec)) = self.c_layout(param, &mut declarations) else {
				self.foreign_by_value(n);
				return;
			};
			param_c.push(c);
			param_specs.push(spec);
		}
		let (ret_c, ret_spec, ret_size) = match self.types.kind(ret) {
			TypeKind::Void | TypeKind::Never => ("void".to_string(), "\"void\"".to_string(), 0),
			_ => match self.c_layout(ret, &mut declarations) {
				Some((size, _, c, spec)) => (c, spec, size),
				None => {
					self.foreign_by_value(n);
					return;
				}
			},
		};
		// the C source: the struct declarations, then the shim
		let mut source = String::new();
		for text in &declarations.typedefs {
			source.push_str(text);
			source.push('\n');
		}
		let shim_params: Vec<String> = (0..params.len())
			.map(|index| format!("void *p{index}"))
			.collect();
		let arguments: Vec<String> = param_c
			.iter()
			.enumerate()
			.map(|(index, c)| format!("*({c} *)p{index}"))
			.collect();
		let call = format!(
			"(({ret_c} (*)({}))f)({})",
			if param_c.is_empty() {
				"void".to_string()
			} else {
				param_c.join(", ")
			},
			arguments.join(", ")
		);
		let mut all_params = vec!["void *f".to_string()];
		all_params.extend(shim_params);
		all_params.push("void *out".to_string());
		let body = if ret_c == "void" {
			format!("{call};")
		} else {
			format!("*({ret_c} *)out = {call};")
		};
		source.push_str(&format!(
			"void fa_shim({}) {{ {body} }}\n",
			all_params.join(", ")
		));
		let bigint = self.is_bigint(ret);
		let bind = self.helper("faFfiBindStruct");
		let libraries: Vec<String> = self
			.options
			.libraries
			.iter()
			.map(|library| js::string_literal(library))
			.collect();
		self.out.line(format!("function {name}(...values) {{"));
		self.out.line(format!(
			"\t{name}.c ??= {bind}([{}], {}, {}, [{}], [{ret_spec}, {ret_size}, {bigint}]);",
			libraries.join(", "),
			js::string_literal(symbol),
			js::string_literal(&source),
			param_specs.join(", "),
		));
		self.out.line(format!("\treturn {name}.c(...values);"));
		self.out.line("}");
	}

	/// A `CStruct` given to C by value: not on JavaScript targets.
	fn foreign_by_value(&mut self, n: NodeId) {
		let target = self.options.target.name();
		self.error(
			&fa_analyzer::diagnostics::codes::advanced::FFI008,
			n,
			format!(
				"a `CStruct` can't be given to C by value on target `{target}`: pass a `Pointer` to it"
			),
		);
	}

	/// A call of an `@extern` function.
	pub(crate) fn foreign_call(
		&mut self,
		n: NodeId,
		callee: NodeId,
		arguments: &[Argument],
	) -> Expr {
		let function = self.expr(callee);
		let TypeKind::Function(sig) = self.types.kind(self.type_of(callee)) else {
			return self.internal_error(n, "the checked C call has no function signature");
		};
		let sig = self.types.sig(sig);
		let params: Vec<TypeId> = self
			.types
			.params(sig.params)
			.iter()
			.map(|param| param.ty)
			.collect();
		// the statements before the call, the values given, and the statements after it
		let mut before: Vec<String> = Vec::new();
		let mut after: Vec<String> = Vec::new();
		let mut values: Vec<String> = Vec::new();
		for (index, argument) in arguments.iter().enumerate() {
			let Argument::Given(value) = argument else {
				return self.internal_error(n, "the checked C call has an unbound argument");
			};
			let value = *value;
			let param = params.get(index).copied().unwrap_or(TypeId::UNKNOWN);
			let temporary = format!("$c{index}");
			match self.analysis.foreign_args.get(&value).copied() {
				Some(ForeignArg::CString) => {
					let text = self.expr(value);
					let helper = self.helper("faCString");
					values.push(js::call(helper, &[text.text]).text);
				}
				Some(ForeignArg::Array { mutated }) => {
					let array = self.expr(value);
					let item = match self.types.kind(self.type_of(value)) {
						TypeKind::Collection(
							CollKind::Array | CollKind::InlineArray | CollKind::GrowingArray,
							item,
							_,
						) => item,
						_ => TypeId::UNKNOWN,
					};
					let Some(c) = self.c_type_name(item).filter(|c| *c != "void") else {
						return self.unsupported(value, "this array given to C");
					};
					let helper = self.helper("faCArray");
					before.push(format!("const {temporary}$items = {};", array.text));
					before.push(format!(
						"const {temporary} = {helper}({temporary}$items, {});",
						js::string_literal(c)
					));
					if mutated {
						let back = self.helper("faCArrayBack");
						after.push(format!(
							"{back}({temporary}$items, {temporary}, {});",
							js::string_literal(c)
						));
					}
					values.push(temporary);
				}
				Some(ForeignArg::Address { mutated }) => {
					let target = self
						.types
						.pointer_target(param)
						.filter(|&target| target != TypeId::VOID)
						.unwrap_or_else(|| self.type_of(value));
					let Some(c) = self.c_type_name(target).filter(|c| *c != "void") else {
						return self.unsupported(value, "the address of this value");
					};
					let lowered = self.expr(value);
					let helper = self.helper("faCBox");
					before.push(format!(
						"const {temporary} = {helper}({}, {});",
						lowered.text,
						js::string_literal(c)
					));
					if mutated && matches!(self.view(value), View::Name(_) | View::Access { .. }) {
						let unbox = self.helper("faCUnbox");
						after.push(format!(
							"{} = {unbox}({temporary}, {});",
							lowered.text,
							js::string_literal(c)
						));
					}
					values.push(temporary);
				}
				Some(ForeignArg::Callback) => {
					let TypeKind::Function(callback) = self.types.kind(param) else {
						return self.internal_error(
							value,
							"the checked C callback argument has no function signature",
						);
					};
					let callback = self.types.sig(callback);
					let mut args = Vec::new();
					for param in self.types.params(callback.params) {
						match self.c_type_name(param.ty) {
							Some(c) => args.push(js::string_literal(c)),
							None => return self.unsupported(value, "this callback"),
						}
					}
					let Some(returns) = self.c_type_name(callback.ret) else {
						return self.unsupported(value, "this callback");
					};
					let function = self.expr(value);
					let helper = self.helper("faCCallback");
					before.push(format!(
						"const {temporary} = {helper}({}, [{}], {});",
						function.text,
						args.join(", "),
						js::string_literal(returns)
					));
					after.push(format!("{temporary}.close();"));
					values.push(format!("{temporary}.pointer"));
				}
				None => {
					let lowered = self.argument_value(value);
					values.push(lowered);
				}
			}
		}
		let call = format!("{}({})", function.primary_text(), values.join(", "));
		if before.is_empty() && after.is_empty() {
			return Expr::new(call, Prec::Primary);
		}
		let mut body = before.join(" ");
		if after.is_empty() {
			body.push_str(&format!(" return {call};"));
		} else {
			body.push_str(&format!(
				" try {{ return {call}; }} finally {{ {} }}",
				after.join(" ")
			));
		}
		Expr::new(format!("(() => {{ {body} }})()"), Prec::Primary)
	}

	/// An argument passed as is.
	fn argument_value(&mut self, value: NodeId) -> String {
		self.expr(value).text
	}

	/// `pointer.read()`, `pointer.write(value)`, `pointer.offset(count)`,
	/// `pointer.toArray(count)`, `text.toString()`.
	pub(crate) fn pointer_method(
		&mut self,
		n: NodeId,
		callee: NodeId,
		path: &str,
		arguments: &[Argument],
	) -> Expr {
		let mut receiver = self.receiver(callee);
		let receiver_type = self.receiver_type(callee);
		let argument = arguments.iter().find_map(|argument| match argument {
			Argument::Given(value) => Some(*value),
			_ => None,
		});
		if path == "CString#toString" {
			let helper = self.helper("faCStringRead");
			return js::call(helper, &[receiver.text]);
		}
		let Some(target) = self.types.pointer_target(receiver_type) else {
			return self.unsupported(n, "this pointer method");
		};
		if self
			.types
			.flags(target)
			.contains(fa_analyzer::types::TypeFlags::HAS_PARAM)
		{
			return self.unsupported(n, "this pointer method with an unresolved target type");
		}
		if argument.is_some() && !receiver.stable {
			let name = self.temp();
			self.out.line(format!("const {name} = {};", receiver.text));
			receiver = Expr::name(name, true);
		}
		let spec = self.pointer_spec(target);
		let opaque = spec.is_none();
		let spec = spec.unwrap_or_else(|| "null".to_string());
		let bigint = self.is_bigint(target).to_string();
		match (path, argument) {
			("Pointer#read", _) => {
				let helper = self.helper("faPointerReadValue");
				let value = js::call(helper, &[receiver.text, spec, bigint]);
				if opaque && self.is_composite(target) {
					self.copy_of(value, target)
				} else {
					value
				}
			}
			("Pointer#write", Some(value)) => {
				let expression = self.expr(value);
				let expression = self.copy_stored(value, expression);
				let drop = self
					.drop_function(target)
					.unwrap_or_else(|| "undefined".to_string());
				let helper = self.helper("faPointerWriteValue");
				js::call(helper, &[receiver.text, spec, expression.text, drop])
			}
			("Pointer#free", _) => {
				let drop = self
					.drop_function(target)
					.unwrap_or_else(|| "undefined".to_string());
				let helper = self.helper("faPointerFree");
				js::call(helper, &[receiver.text, drop])
			}
			("Pointer#offset", Some(count)) if !opaque => {
				let count = self.expr(count);
				let helper = self.helper("faPointerOffset");
				js::call(helper, &[receiver.text, spec, count.text])
			}
			("Pointer#toArray", Some(count)) if !opaque => {
				let count = self.expr(count);
				let helper = self.helper("faPointerToValues");
				js::call(helper, &[receiver.text, spec, bigint, count.text])
			}
			_ => self.unsupported(n, "this pointer method"),
		}
	}

	/// `pointer.value`: a view of its Fa value or its C memory.
	pub(crate) fn pointer_value(&mut self, value: Expr, target: TypeId, member: NodeId) -> Expr {
		if self
			.types
			.flags(target)
			.contains(fa_analyzer::types::TypeFlags::HAS_PARAM)
		{
			return self.unsupported(member, "this pointer view with an unresolved target type");
		}
		let spec = self
			.pointer_spec(target)
			.unwrap_or_else(|| "null".to_string());
		let bigint = self.is_bigint(target).to_string();
		let helper = self.helper("faPointerValue");
		js::call(helper, &[value.text, spec, bigint])
	}

	/// A writable accessor for the whole value a pointer designates.
	pub(crate) fn pointer_cell(&mut self, pointer: Expr, target: TypeId) -> Expr {
		let spec = self
			.pointer_spec(target)
			.unwrap_or_else(|| "null".to_string());
		let bigint = self.is_bigint(target).to_string();
		let helper = self.helper("faPointerCell");
		js::call(helper, &[pointer.text, spec, bigint])
	}
}
