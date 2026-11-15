use std::sync::Arc;

use ethnum::I256;
use fa_interpreter::ir::{Arg, Expr, FnKey, Function, GlobalKey, Param};
use fa_interpreter::{Error, Host, Interpreter, Limits, Value};

struct Functions {
	function: Arc<Function>,
	reads: Vec<u32>,
}

impl Host for Functions {
	fn function(&mut self, _: FnKey) -> Result<Arc<Function>, Error> {
		Ok(Arc::clone(&self.function))
	}
	fn global(&mut self, key: GlobalKey) -> Result<Value, Error> {
		self.reads.push(key.1);
		Ok(match key.1 {
			0 => Value::Function(Arc::new(fa_interpreter::value::Closure {
				function: FnKey(0, 0),
				captured: Vec::new(),
				bound: Vec::new(),
				partials: Vec::new(),
			})),
			1 => Value::Integer(I256::from(20)),
			_ => Value::Integer(I256::from(10)),
		})
	}
}

fn functions() -> Functions {
	Functions {
		function: Arc::new(Function {
			name: "three".into(),
			params: (0..3)
				.map(|slot| Param {
					slot,
					default: (slot == 2).then_some(Expr::Local(0)),
				})
				.collect(),
			captures: Vec::new(),
			slots: 3,
			body: Expr::Tuple(vec![Expr::Local(0), Expr::Local(1), Expr::Local(2)]),
		}),
		reads: Vec::new(),
	}
}

#[test]
fn bound_arguments_are_evaluated_once_in_source_order() {
	let mut host = functions();
	let partial = Expr::Partial {
		site: FnKey(0, 10),
		function: Expr::Global(GlobalKey(0, 0)).boxed(),
		bound: vec![
			(1, Expr::Global(GlobalKey(0, 1))),
			(0, Expr::Global(GlobalKey(0, 2))),
		],
	};
	let mut interpreter = Interpreter::new(&mut host, Limits::default());
	let value = interpreter.evaluate(&partial, 0).unwrap();
	let expected = Value::Tuple(Arc::new(vec![
		Value::Integer(10.into()),
		Value::Integer(20.into()),
		Value::Integer(30.into()),
	]));
	assert_eq!(
		interpreter
			.call_value(&value, vec![Value::Integer(30.into())])
			.unwrap(),
		expected
	);
	assert_eq!(
		interpreter
			.call_value(&value, vec![Value::Integer(30.into())])
			.unwrap(),
		expected
	);
	assert_eq!(host.reads, [0, 1, 2]);
}

#[test]
fn unbound_defaults_see_the_bound_earlier_parameters() {
	let mut host = functions();
	let expr = Expr::Call(
		Expr::Partial {
			site: FnKey(0, 10),
			function: Expr::Function(FnKey(0, 0)).boxed(),
			bound: vec![
				(1, Expr::Const(Value::Integer(20.into()))),
				(0, Expr::Const(Value::Integer(10.into()))),
			],
		}
		.boxed(),
		vec![Arg::Default],
	);
	let result = Interpreter::new(&mut host, Limits::default())
		.evaluate(&expr, 0)
		.unwrap();
	assert_eq!(
		result,
		Value::Tuple(Arc::new(vec![
			Value::Integer(10.into()),
			Value::Integer(20.into()),
			Value::Integer(10.into())
		]))
	);
}

#[test]
fn another_with_binds_the_remaining_parameter_positions() {
	let mut host = functions();
	let inner = Expr::Partial {
		site: FnKey(0, 10),
		function: Expr::Function(FnKey(0, 0)).boxed(),
		bound: vec![(1, Expr::Const(Value::Integer(20.into())))],
	};
	let outer = Expr::Partial {
		site: FnKey(0, 11),
		function: inner.boxed(),
		bound: vec![(1, Expr::Const(Value::Integer(30.into())))],
	};
	let value = Interpreter::new(&mut host, Limits::default())
		.evaluate(&outer, 0)
		.unwrap();
	let Value::Function(value) = value else {
		panic!("a partial function");
	};
	assert_eq!(
		value
			.partials
			.iter()
			.map(|partial| partial.site)
			.collect::<Vec<_>>(),
		[FnKey(0, 10), FnKey(0, 11)]
	);
	assert_eq!(value.partials[0].arguments, [Value::Integer(20.into())]);
	assert_eq!(value.partials[1].arguments, [Value::Integer(30.into())]);
	let expr = Expr::Call(
		outer.boxed(),
		vec![Arg::Value(Expr::Const(Value::Integer(10.into())))],
	);
	let result = Interpreter::new(&mut host, Limits::default())
		.evaluate(&expr, 0)
		.unwrap();
	assert_eq!(
		result,
		Value::Tuple(Arc::new(vec![
			Value::Integer(10.into()),
			Value::Integer(20.into()),
			Value::Integer(30.into())
		]))
	);
}
