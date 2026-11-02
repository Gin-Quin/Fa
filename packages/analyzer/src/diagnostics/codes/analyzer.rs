//! Diagnostic codes owned by the analyzer itself (`AN0xx`).
//!
//! The test harness fails a case that emits one of them unless the case
//! expects it.

use crate::diagnostics::Severity::{Error, Warning};

codes! {
	AN001 = "AN001", Error, [], "construct not supported yet by the analyzer";
	AN002 = "AN002", Error, [], "syntax error reported by the parser";
	AN003 = "AN003", Warning, [], "syntax warning reported by the parser";
	AN004 = "AN004", Error, [], "a part of the standard library that doesn't exist on the target";
	AN005 = "AN005", Error, [], "a type without a JSON form given to Json.parse or Json.stringify";
}
