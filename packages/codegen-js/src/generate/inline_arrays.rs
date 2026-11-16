//! The extra lifetime work of bounded resource lists. Plain data continues
//! to use ordinary dense array literals and methods.

use fa_analyzer::ids::NodeId;
use fa_analyzer::syntax::NodeList;

use crate::generate::Generator;
use crate::js::Expr;

impl Generator<'_> {
	pub(crate) fn inline_resource_literal(
		&mut self,
		node: NodeId,
		items: NodeList<'_>,
	) -> Option<Expr> {
		let ty = self.type_of(node);
		self.types.inline_array(ty)?;
		let drop = self.drop_function(ty)?;
		let list = self.temp();
		self.out.line(format!("const {list} = [];"));
		self.out.open("try");
		for node in items.iter() {
			let value = self.expr(node);
			let value = self.copy_stored(node, value);
			self.out.line(format!("{list}.push({});", value.text));
		}
		self.out.reopen("catch ($error)");
		self.out.line(format!("{drop}({list});"));
		self.out.line("throw $error;");
		self.out.close("");
		Some(Expr::name(list, true))
	}
}
