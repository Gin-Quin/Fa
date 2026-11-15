//! Typed values produced by compile-time function factories. Function leaves
//! reference the same declaration/lambda/partial templates as runtime values;
//! no backend reconstructs captured types or methods from their layout.

use std::sync::Arc;

use fa_interpreter::Value;
use fa_interpreter::value::MemberKind;

use crate::ids::{DeclKey, ModuleId, NodeId, ParamId, Sym, TypeId};
use crate::provenance::CallableCapabilities;

#[derive(Clone, Debug)]
pub struct ConstantFunction {
	/// Actual function signature, before an annotated function-value coercion.
	pub ty: TypeId,
	pub template: DeclKey,
	/// Source template for native instance waves; JavaScript uses template identity.
	pub module: ModuleId,
	pub node: NodeId,
	/// Includes outer/owner binders, even when absent from the representation.
	pub bindings: Vec<(ParamId, TypeId)>,
	/// Parameter identity to its source owner/order, for native instance waves.
	pub parameters: Vec<(ParamId, NodeId, u16)>,
	/// Declaration defaults in the template's runtime parameter order.
	pub defaults: Vec<bool>,
	pub capabilities: Arc<CallableCapabilities>,
	/// In the template's FnInfo.captures order; a partial has target then
	/// bound values in source order, exactly as at runtime.
	pub captures: Vec<ConstantValue>,
}

#[derive(Clone, Debug)]
pub struct ConstantValue {
	pub ty: TypeId,
	pub kind: ConstantKind,
}

#[derive(Clone, Debug)]
pub enum ConstantKind {
	/// Contains no function value anywhere; emitted by the existing typed
	/// constant lowering, including ordinary composite DATA values.
	Data(Value),
	Function(Arc<ConstantFunction>),
	Array(Vec<ConstantValue>),
	Tuple(Vec<ConstantValue>),
	Set(Vec<ConstantValue>),
	Map(Vec<(ConstantValue, ConstantValue)>),
	Object(Vec<(Sym, ConstantValue)>),
	Member {
		name: Arc<str>,
		kind: MemberKind,
		data: Box<ConstantValue>,
	},
}
