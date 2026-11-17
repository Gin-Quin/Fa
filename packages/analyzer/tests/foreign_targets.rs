//! Foreign allocation has the same target restrictions as C calls.

use std::sync::Arc;

use fa_analyzer::Session;

#[test]
fn pointer_allocation_and_free_require_a_c_interface() {
	for source in [
		"let pointer = Pointer.allocate(41)",
		"function release = (pointer: Pointer<Integer>) => pointer.free()",
	] {
		for target in ["bun", "native", "wasm", "node", "web"] {
			let mut session = Session::new();
			session.set_target(target);
			let file = session.set_source("main.foreign", Arc::from(source));
			session.analyze_file(file);
			let diagnostics = session.diagnostics(file);
			let errors: Vec<_> = diagnostics
				.iter()
				.filter(|d| d.code.id == "FFI008")
				.collect();
			if matches!(target, "node" | "web") {
				assert_eq!(errors.len(), 1, "{target}: {diagnostics:#?}");
				assert!(errors[0].message.contains("@if target"));
			} else {
				assert!(
					diagnostics
						.iter()
						.all(|d| d.severity != fa_analyzer::diagnostics::Severity::Error),
					"{target}: {diagnostics:#?}"
				);
			}
		}
	}
}
