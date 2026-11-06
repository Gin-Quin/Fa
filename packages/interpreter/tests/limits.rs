//! The compiler installs this allocator too. Continuations and prepared
//! execution nodes belong to the same heap budget as computed values.

use std::sync::Arc;

use fa_interpreter::ir::{Expr, FnKey, Function, GlobalKey};
use fa_interpreter::{Error, Host, Interpreter, Limits, Value};

#[global_allocator]
static ALLOCATOR: fa_interpreter::memory::CountingAllocator =
	fa_interpreter::memory::CountingAllocator::SYSTEM;

struct Recursive(Arc<Function>);
impl Host for Recursive {
	fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
		Ok(Arc::clone(&self.0))
	}
	fn global(&mut self, _: GlobalKey) -> Result<Value, Error> {
		unreachable!("no globals")
	}
}

#[test]
fn explicit_call_frames_are_checked_against_the_heap_budget() {
	let mut host = Recursive(Arc::new(Function {
		name: "forever".into(),
		params: Vec::new(),
		captures: Vec::new(),
		slots: 0,
		body: Expr::CallFunction(FnKey(0, 0), Vec::new()),
	}));
	let limits = Limits {
		memory: 64 << 10,
		..Limits::default()
	};
	let result = Interpreter::new(&mut host, limits)
		.evaluate(&Expr::CallFunction(FnKey(0, 0), Vec::new()), 0);
	assert_eq!(result, Err(Error::Memory(64 << 10)));
}
