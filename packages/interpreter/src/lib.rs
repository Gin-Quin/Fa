//! The compile-time interpreter of Fa (advanced brief §12.6).
//!
//! It runs analyzed code while compiling: `static` values, `@` calls, `@if`
//! conditions and `@for` iterables, `@test` blocks and computed
//! configuration files. It does not run programs: `fa run` compiles to Bun
//! (advanced brief D6).
//!
//! The crate knows nothing of the analyzer: the analyzer *lowers* the checked
//! tree into the [`ir`] (every name resolved, every call bound, every
//! arithmetic operation carrying the range of its result under the target,
//! decision D49) and asks the [`Interpreter`] to evaluate it, answering its
//! questions through the [`Host`] trait (the lowered function of a key, the
//! value of a module-level value). So `fa-analyzer` depends on this crate,
//! and the [`Value`]s it computes are what the backends embed as constants.
//!
//! - Values are copied on write ([`value`]); evaluation is deterministic:
//!   there is no clock, randomness, environment or I/O here. The analyzer
//!   reads the build inputs (`@readFile`, `@readDirectory`) and gives the
//!   results as values.
//! - Bounds: 100 million evaluated expressions per evaluation (D26), a depth
//!   of nested calls ([`Limits`]).
//! - Code with an outside effect is lowered to [`ir::Expr::Impure`]: the
//!   analyzer rejects a callee that can reach it (`META002`) before running
//!   it.

pub mod builtins;
pub mod eval;
pub mod ir;
pub mod json;
pub mod memory;
pub mod types;
pub mod value;

pub use eval::{Error, Host, Interpreter, Limits};
pub use value::Value;

/// The stack of a thread an evaluation runs on: deep recursion in Fa code
/// is deep recursion of the evaluator (and of a debug build's large frames).
/// Only the pages touched are committed.
pub const STACK_SIZE: usize = 1 << 30;

std::thread_local! {
	/// The depth of nested calls the stack of the current thread holds
	/// (set on the evaluation threads; `Interpreter::new` lowers
	/// [`Limits::depth`] to it).
	pub(crate) static STACK_DEPTH: std::cell::Cell<u32> = const { std::cell::Cell::new(u32::MAX) };
	/// Whether this thread is an evaluation thread (a nested evaluation
	/// runs in place).
	static ON_LARGE_STACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The smallest stack an evaluation thread falls back to.
const MINIMUM_STACK_SIZE: usize = 16 << 20;

/// The stack an evaluation thread is given at least, when the address
/// space is limited and shared by several of them.
const SHARED_STACK_SIZE: usize = 256 << 20;

/// An evaluation to run, its lifetime erased ([`on_large_stack`] waits for
/// it to finish).
type Job = Box<dyn FnOnce() + Send + 'static>;

/// The evaluation threads (design/parallel-pipeline.md, P8): started on
/// demand, up to one per core, then reused, so that the analyses running
/// in parallel (`fa_analyzer::pipeline`) evaluate in parallel too, and no
/// evaluation pays for a thread.
struct Pool {
	state: std::sync::Mutex<PoolState>,
	/// Signaled when an evaluation is queued.
	queued: std::sync::Condvar,
}

struct PoolState {
	jobs: std::collections::VecDeque<Job>,
	/// The threads started.
	threads: usize,
	/// The threads waiting for an evaluation.
	idle: usize,
	/// The most threads: one per core, fewer when the address space is
	/// limited (they share a quarter of it), or when one couldn't start.
	most: usize,
	/// The stack of every thread (the first thread may lower it, when such
	/// a stack can't be reserved: then every thread has the same depth).
	stack: usize,
}

impl Pool {
	fn get() -> &'static Pool {
		static POOL: std::sync::OnceLock<Pool> = std::sync::OnceLock::new();
		POOL.get_or_init(|| {
			let cores = std::thread::available_parallelism().map_or(1, usize::from);
			let (most, stack) = match memory::address_space_limit() {
				None => (cores, STACK_SIZE),
				// the stacks take a quarter of the address space, together
				Some(limit) => {
					let budget = limit / 4;
					let most = (budget / SHARED_STACK_SIZE).clamp(1, cores);
					(most, (budget / most).clamp(MINIMUM_STACK_SIZE, STACK_SIZE))
				}
			};
			Pool {
				state: std::sync::Mutex::new(PoolState {
					jobs: std::collections::VecDeque::new(),
					threads: 0,
					idle: 0,
					most,
					stack,
				}),
				queued: std::sync::Condvar::new(),
			}
		})
	}

	fn lock(&self) -> std::sync::MutexGuard<'_, PoolState> {
		self.state
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	/// Queues an evaluation, and starts a thread for it when every thread
	/// is busy and there may be one more.
	fn submit(&'static self, job: Job) {
		let mut state = self.lock();
		state.jobs.push_back(job);
		if state.jobs.len() > state.idle && state.threads < state.most {
			self.start(&mut state);
		}
		drop(state);
		self.queued.notify_one();
	}

	/// Starts a thread (the pool's lock held). The first thread lowers the
	/// stack of all of them until one can be reserved; when a later one
	/// can't start, the pool keeps the threads it has.
	fn start(&'static self, state: &mut PoolState) {
		loop {
			let stack = state.stack;
			let depth = (Limits::default().depth as usize * stack / STACK_SIZE) as u32;
			let started = std::thread::Builder::new()
				.name("fa-interpreter".into())
				.stack_size(stack)
				.spawn(move || self.work(depth));
			match started {
				Ok(_) => {
					state.threads += 1;
					return;
				}
				Err(_) if state.threads == 0 && stack > MINIMUM_STACK_SIZE => {
					state.stack = (stack / 4).max(MINIMUM_STACK_SIZE);
				}
				Err(error) if state.threads == 0 => {
					panic!("the evaluation thread can't be started: {error}")
				}
				Err(_) => {
					state.most = state.threads;
					return;
				}
			}
		}
	}

	/// The loop of an evaluation thread: runs the evaluations queued.
	fn work(&self, depth: u32) {
		STACK_DEPTH.set(depth);
		ON_LARGE_STACK.set(true);
		let mut state = self.lock();
		loop {
			match state.jobs.pop_front() {
				Some(job) => {
					drop(state);
					job();
					state = self.lock();
				}
				None => {
					state.idle += 1;
					state = self
						.queued
						.wait(state)
						.unwrap_or_else(|poisoned| poisoned.into_inner());
					state.idle -= 1;
				}
			}
		}
	}
}

/// Runs `f` on an evaluation thread, whose stack is [`STACK_SIZE`], so that
/// the depth of nested calls is bounded by [`Limits::depth`] and not by the
/// caller's stack. A panic of `f` is resumed in the caller.
///
/// The evaluation threads are a pool, started on demand up to one per
/// core and reused: evaluations of parallel analyses run in parallel. When
/// the address space is limited (`ulimit -v`), the threads share a quarter
/// of it (fewer threads, smaller stacks), or when such a stack can't be
/// reserved, a smaller one is used; the depth of nested calls is lowered in
/// proportion. Called from an evaluation thread, `f` runs in place.
pub fn on_large_stack<R: Send>(f: impl FnOnce() -> R + Send) -> R {
	if ON_LARGE_STACK.get() {
		return f();
	}
	let (sender, receiver) = std::sync::mpsc::sync_channel(1);
	let job = move || {
		let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
		let _ = sender.send(result);
	};
	let job: Box<dyn FnOnce() + Send + '_> = Box::new(job);
	// SAFETY: the job borrows from this frame (`f` and its result); this
	// function returns only once the job sent its result, after which it
	// touches nothing borrowed (`f` was consumed; the channel is shared
	// and outlives both ends). The receive below can't fail before then:
	// the job catches the panics of `f`, and a thread never drops a job
	// without running it.
	let job: Job = unsafe { std::mem::transmute::<Box<dyn FnOnce() + Send + '_>, Job>(job) };
	Pool::get().submit(job);
	match receiver.recv() {
		Ok(Ok(result)) => result,
		Ok(Err(panic)) => std::panic::resume_unwind(panic),
		Err(_) => unreachable!("an evaluation thread sends its result"),
	}
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use ethnum::I256;
	use rustc_hash_free::Functions;

	use super::*;
	use crate::ir::{
		Arg, ArithOp, CompareOp, Expr, FnKey, Function, Item, Num, Param, Part, Place, Step,
	};

	/// A host over a table of functions (no globals).
	mod rustc_hash_free {
		use std::collections::HashMap;
		use std::sync::Arc;

		use crate::ir::{FnKey, Function, GlobalKey};
		use crate::{Error, Host, Value};

		#[derive(Default)]
		pub struct Functions(pub HashMap<FnKey, Arc<Function>>);

		impl Host for Functions {
			fn function(&mut self, key: FnKey) -> Result<Arc<Function>, Error> {
				self.0
					.get(&key)
					.cloned()
					.ok_or_else(|| Error::Unsupported("an unknown function".into()))
			}

			fn global(&mut self, _key: GlobalKey) -> Result<Value, Error> {
				Err(Error::Unsupported("globals".into()))
			}
		}
	}

	const INTEGER: Num = Num::Int {
		min: I256::from_words(-1, -(1i128 << 63)),
		max: I256::from_words(0, (1i128 << 63) - 1),
	};

	fn js_integer() -> Num {
		let max = I256::from((1i64 << 53) - 1);
		Num::Int { min: -max, max }
	}

	fn int(value: i64) -> Expr {
		Expr::Const(Value::int(value))
	}

	fn local(slot: u32) -> Box<Expr> {
		Box::new(Expr::Local(slot))
	}

	fn run(host: &mut Functions, expr: &Expr, slots: u32) -> Result<Value, Error> {
		Interpreter::new(host, Limits::default()).evaluate(expr, slots)
	}

	/// `function add = (x, y) => x + y`.
	fn add() -> Function {
		Function {
			name: "add".into(),
			params: vec![
				Param {
					slot: 0,
					default: None,
				},
				Param {
					slot: 1,
					default: Some(int(10)),
				},
			],
			captures: Vec::new(),
			slots: 2,
			body: Expr::Arith(ArithOp::Add, INTEGER, local(0), local(1)),
		}
	}

	#[test]
	fn calls_bind_arguments_and_defaults() {
		let mut host = Functions::default();
		host.0.insert(FnKey(0, 1), Arc::new(add()));
		let call = Expr::CallFunction(FnKey(0, 1), vec![Arg::Value(int(2)), Arg::Value(int(4))]);
		assert_eq!(run(&mut host, &call, 0), Ok(Value::int(6)));
		let call = Expr::CallFunction(FnKey(0, 1), vec![Arg::Value(int(2)), Arg::Default]);
		assert_eq!(run(&mut host, &call, 0), Ok(Value::int(12)));
	}

	#[test]
	fn arithmetic_follows_the_target() {
		let mut host = Functions::default();
		let big = Expr::Const(Value::Integer(I256::from(1i64 << 53)));
		// fine on native, an overflow on JavaScript (D49)
		let native = Expr::Arith(ArithOp::Add, INTEGER, big.clone().boxed(), int(1).boxed());
		assert!(run(&mut host, &native, 0).is_ok());
		let js = Expr::Arith(ArithOp::Add, js_integer(), big.boxed(), int(1).boxed());
		assert!(matches!(run(&mut host, &js, 0), Err(Error::Overflow(_))));
		let divide = Expr::Arith(
			ArithOp::IntegerDivide,
			INTEGER,
			int(-7).boxed(),
			int(2).boxed(),
		);
		assert_eq!(run(&mut host, &divide, 0), Ok(Value::int(-4)));
		let remainder = Expr::Arith(ArithOp::Remainder, INTEGER, int(-7).boxed(), int(2).boxed());
		assert_eq!(run(&mut host, &remainder, 0), Ok(Value::int(1)));
		let zero = Expr::Arith(
			ArithOp::IntegerDivide,
			INTEGER,
			int(1).boxed(),
			int(0).boxed(),
		);
		assert!(matches!(run(&mut host, &zero, 0), Err(Error::Thrown(_))));
		let float = Expr::Arith(ArithOp::Divide, INTEGER, int(1).boxed(), int(4).boxed());
		assert_eq!(run(&mut host, &float, 0), Ok(Value::Float(0.25)));
	}

	#[test]
	fn loops_mutations_and_templates() {
		let mut host = Functions::default();
		// mutable result = ""; for i in 0..3 { result += "ab" }; mutable list = []; list.push(…)
		let program = Expr::Block(vec![
			Expr::Let(0, Expr::Const(Value::string("")).boxed()),
			Expr::Let(2, Expr::Array(Vec::new()).boxed()),
			Expr::For {
				item: Some(1),
				index: None,
				iterable: Expr::Range(Some(int(0).boxed()), Some(int(3).boxed()), false).boxed(),
				body: Expr::Block(vec![
					Expr::Assign(
						Place {
							slot: 0,
							steps: Vec::new(),
						},
						Expr::Template(vec![
							Part::Value(Expr::Local(0)),
							Part::Text(Arc::from("ab")),
						])
						.boxed(),
					),
					Expr::Mutate(
						"Array#push",
						Place {
							slot: 2,
							steps: Vec::new(),
						},
						vec![Expr::Local(1)],
					),
				])
				.boxed(),
			},
			Expr::Tuple(vec![Expr::Local(0), Expr::Local(2)]),
		]);
		assert_eq!(
			run(&mut host, &program, 3).map(|value| value.to_text()),
			Ok("(\"ababab\", [0, 1, 2])".to_string())
		);
	}

	#[test]
	fn copies_are_independent() {
		let mut host = Functions::default();
		// let a = { x = 1 }; mutable b = a; b.x = 2; (a.x, b.x)
		let program = Expr::Block(vec![
			Expr::Let(
				0,
				Expr::Const(Value::object(vec![("x", Value::int(1))])).boxed(),
			),
			Expr::Let(1, local(0)),
			Expr::Assign(
				Place {
					slot: 1,
					steps: vec![Step::Field(Arc::from("x"))],
				},
				int(2).boxed(),
			),
			Expr::Tuple(vec![
				Expr::Field(local(0), Arc::from("x"), false),
				Expr::Field(local(1), Arc::from("x"), false),
			]),
		]);
		assert_eq!(
			run(&mut host, &program, 2).map(|value| value.to_text()),
			Ok("(1, 2)".to_string())
		);
	}

	#[test]
	fn closures_and_callbacks() {
		let mut host = Functions::default();
		// (n) => n * factor, with `factor` captured
		host.0.insert(
			FnKey(0, 2),
			Arc::new(Function {
				name: "<lambda>".into(),
				params: vec![Param {
					slot: 0,
					default: None,
				}],
				captures: vec![1],
				slots: 2,
				body: Expr::Arith(ArithOp::Multiply, INTEGER, local(0), local(1)),
			}),
		);
		let program = Expr::Block(vec![
			Expr::Let(0, int(3).boxed()),
			Expr::Builtin(
				"Array#map",
				vec![
					Expr::Array(vec![Item::One(int(1)), Item::One(int(2))]),
					Expr::Closure {
						function: FnKey(0, 2),
						captures: vec![0],
					},
				],
			),
		]);
		assert_eq!(
			run(&mut host, &program, 1).map(|value| value.to_text()),
			Ok("[3, 6]".to_string())
		);
	}

	#[test]
	fn the_budget_and_the_depth_are_bounded() {
		let mut host = Functions::default();
		let forever = Expr::Loop(Expr::Block(Vec::new()).boxed());
		let limits = Limits {
			steps: 10_000,
			depth: 50,
			..Limits::default()
		};
		let result = Interpreter::new(&mut host, limits).evaluate(&forever, 0);
		assert_eq!(result, Err(Error::Steps(10_000)));
		// function loop = (n) => loop(n + 1)
		host.0.insert(
			FnKey(0, 3),
			Arc::new(Function {
				name: "loop".into(),
				params: vec![Param {
					slot: 0,
					default: None,
				}],
				captures: Vec::new(),
				slots: 1,
				body: Expr::CallFunction(
					FnKey(0, 3),
					vec![Arg::Value(Expr::Arith(
						ArithOp::Add,
						INTEGER,
						local(0),
						int(1).boxed(),
					))],
				),
			}),
		);
		let call = Expr::CallFunction(FnKey(0, 3), vec![Arg::Value(int(1))]);
		let result = Interpreter::new(&mut host, limits).evaluate(&call, 0);
		assert_eq!(result, Err(Error::Depth(50)));
		// the default depth fits in the stack of the evaluation thread (debug)
		let result =
			on_large_stack(|| Interpreter::new(&mut host, Limits::default()).evaluate(&call, 0));
		assert_eq!(result, Err(Error::Depth(Limits::default().depth)));
	}

	#[test]
	fn the_memory_is_bounded() {
		let mut host = Functions::default();
		let text = Place {
			slot: 0,
			steps: Vec::new(),
		};
		// mutable text = "ab"; loop { text += text }
		let doubling = Expr::Block(vec![
			Expr::Let(0, Expr::Const(Value::string("ab")).boxed()),
			Expr::Loop(
				Expr::Assign(
					text.clone(),
					Expr::Template(vec![
						Part::Value(Expr::Read(text.clone())),
						Part::Value(Expr::Read(text)),
					])
					.boxed(),
				)
				.boxed(),
			),
		]);
		let limits = Limits {
			memory: 1 << 20,
			..Limits::default()
		};
		let result = Interpreter::new(&mut host, limits).evaluate(&doubling, 1);
		assert_eq!(result, Err(Error::Memory(1 << 20)));
		// the items of a range are not built by a loop, but are by a spread
		let range = |end: i64| Expr::Range(Some(int(0).boxed()), Some(int(end).boxed()), false);
		let looped = Expr::For {
			item: Some(0),
			index: None,
			iterable: range(1 << 40).boxed(),
			body: Expr::Break(None).boxed(),
		};
		assert_eq!(run(&mut host, &looped, 1), Ok(Value::None));
		let spread = Expr::Array(vec![Item::Spread(range(1 << 40))]);
		assert!(matches!(run(&mut host, &spread, 0), Err(Error::Memory(_))));
	}

	#[test]
	fn a_collection_is_changed_in_place() {
		let mut host = Functions::default();
		let items = Place {
			slot: 0,
			steps: Vec::new(),
		};
		// mutable items = []; for index in 0..200_000 { items.push(index) }; items.length
		// (a copy per push would take minutes)
		let program = Expr::Block(vec![
			Expr::Let(0, Expr::Array(Vec::new()).boxed()),
			Expr::For {
				item: Some(1),
				index: None,
				iterable: Expr::Range(Some(int(0).boxed()), Some(int(200_000).boxed()), false)
					.boxed(),
				body: Expr::Mutate("Array#push", items.clone(), vec![Expr::Local(1)]).boxed(),
			},
			Expr::Builtin("Array#length", vec![Expr::Read(items)]),
		]);
		assert_eq!(run(&mut host, &program, 2), Ok(Value::int(200_000)));
	}

	#[test]
	fn errors_are_thrown_and_caught() {
		let mut host = Functions::default();
		let parse = Expr::Builtin("Integer.parse", vec![Expr::Const(Value::string("x"))]);
		let result = run(&mut host, &parse, 0);
		assert!(
			matches!(&result, Err(Error::Thrown(error)) if error.to_text().starts_with("ParseError"))
		);
		let caught = Expr::Catch {
			body: parse.boxed(),
			slot: Some(0),
			handler: int(0).boxed(),
		};
		assert_eq!(run(&mut host, &caught, 1), Ok(Value::int(0)));
		let impure = Expr::Impure(Arc::from("console.log"));
		assert_eq!(
			run(&mut host, &impure, 0),
			Err(Error::Impure("console.log".into()))
		);
		let compare = Expr::Compare(CompareOp::Less, int(1).boxed(), int(2).boxed());
		assert_eq!(run(&mut host, &compare, 0), Ok(Value::Boolean(true)));
	}
}
