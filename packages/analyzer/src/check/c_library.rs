//! The API file of a package built as a C library (`build.cLibrary`,
//! `crate::c_library`): checked once its walk is over and its interface
//! published. `FFI013` on a JavaScript target, `FFI014` for an export that
//! isn't a namespace or a function that C can't call, `REQ006` for a
//! `requires`. `FFI012` (a module-level `mutable` value) is the program's
//! check (`Session::worker_diagnostics`): it follows the calls into every
//! file.

use crate::c_library;
use crate::check::Walker;
use crate::diagnostics::codes::advanced::{FFI013, FFI014, REQ006};
use crate::ids::NodeId;
use crate::types::TypeKind;

impl Walker<'_> {
	/// The checks of the API file, when this file is it.
	pub(super) fn finish_c_library(&mut self, root: NodeId) {
		if !self.in_foreign_file() {
			return;
		}
		let Some((options, targets)) =
			c_library::configured_with_targets(self.s.comptime.configuration.as_ref())
		else {
			return;
		};
		if options.api_module() != self.s.module(self.file) {
			return;
		}
		// the checks depend on the target and on the configuration
		self.a.reads_target = true;
		let interface = self.s.source_file(self.file).interface.clone();
		let export = interface.export.map_or(root, |export| export.decl);
		let target = self.s.target().to_string();
		if !targets.contains(&target) && target != "native-freestanding" {
			// a target the package isn't built for
			return;
		}
		if matches!(target.as_str(), "web" | "bun" | "node") {
			let message = format!(
				"`build.cLibrary` builds a C library, which only the `native` target can build, not `{}`: set it in `overrides.native.build` of `package.fa`, or build with `--target=native`",
				self.s.target()
			);
			let diagnostic = self.diagnostic(&FFI013, export, message);
			self.report(diagnostic);
			return;
		}
		let Some(functions) = c_library::api_functions(&self.s.types, &interface, &options.prefix)
		else {
			let diagnostic = self.diagnostic(
				&FFI014,
				export,
				"the API of a C library (`build.cLibrary.api`) exports a namespace of functions: `export namespace = { function create = … }`",
			);
			self.report(diagnostic);
			return;
		};
		for function in functions {
			let TypeKind::Function(sig) = self.s.types.kind(function.ty) else {
				continue;
			};
			let sig = self.s.types.sig(sig);
			let at = function.decl;
			let name = function.name.clone();
			if !self.s.types.requirements(sig.requires).is_empty() {
				let message = format!(
					"`{name}` is a function of the C library: it can't declare `requires`, since C has nothing to provide; pass the values as parameters"
				);
				let diagnostic = self.diagnostic(&REQ006, at, message);
				self.report(diagnostic);
			}
			if !self.s.types.list_items(sig.generics).is_empty() {
				let message = format!(
					"`{name}` is a function of the C library: it can't be generic, since C calls one version of it"
				);
				let diagnostic = self.diagnostic(&FFI014, at, message);
				self.report(diagnostic);
				continue;
			}
			let names = interface.parameters.get(&function.decl);
			let params: Vec<(String, crate::ids::TypeId)> = self
				.s
				.types
				.params(sig.params)
				.iter()
				.enumerate()
				.map(|(position, param)| {
					let name = names.and_then(|names| names.get(position)).map_or_else(
						|| format!("{}", position + 1),
						|parameter| self.s.types.syms.resolve(parameter.name).to_string(),
					);
					(name, param.ty)
				})
				.collect();
			for (param, ty) in params {
				if c_library::boundary_type(&self.s.types, ty, false).is_none() {
					let message = format!(
						"the parameter `{param}` of `{name}` is a `{}`, which C can't give: a function of the C library takes C types (numbers of a fixed width, `Boolean`, `Pointer(T)`, `CString`, a `CStruct`); pass a Fa value behind a `Pointer`",
						self.display(ty)
					);
					let diagnostic = self.diagnostic(&FFI014, at, message);
					self.report(diagnostic);
				}
			}
			let result = c_library::boundary_type(&self.s.types, sig.ret, true);
			if result.is_none() && sig.ret != crate::ids::TypeId::NEVER {
				let message = format!(
					"`{name}` returns a `{}`, which C can't receive: a function of the C library returns a C type (a number of a fixed width, `Boolean`, `Pointer(T)`, `CString`, a `CStruct`) or nothing; return a Fa value behind a `Pointer` (`Pointer.allocate(value)`)",
					self.display(sig.ret)
				);
				let diagnostic = self.diagnostic(&FFI014, at, message);
				self.report(diagnostic);
			}
			if c_library::error_type(&self.s.types, sig.throws).is_err() {
				let message = format!(
					"`{name}` throws `{}`: a function of the C library throws the members of one `Errors` type, whose position is the status it returns",
					self.display(sig.throws)
				);
				let diagnostic = self.diagnostic(&FFI014, at, message);
				self.report(diagnostic);
			}
		}
	}
}
