//! `fa test --target=native` (advanced brief §18.2): the program of a
//! package's tests. One run of the executable runs one test: `main` reads
//! the test's index from its first argument (`fa.test_index`, 0 without
//! one), initializes the module-level values (fresh for every test, D33),
//! then the module-level values of the test's file, and runs the test's
//! body as the top level would (in the coroutine of the top level, run by
//! the event loop, in an asynchronous program: a test that awaits is a
//! task). A test passes when its run exits with 0; a failed `assert` or
//! another stop exits with 101, an error that escapes with 1, as a
//! program does. The CLI (`packages/cli/src/testing.rs`) runs the tests one
//! after the other and reports them as the Bun runner does.

use fa_analyzer::ids::NodeId;
use fa_analyzer::syntax::View;
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::values::{FunctionValue, GlobalValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::values::{Repr, Val};
use crate::target::Target;

impl<'a, 'ctx> Lowering<'a, 'ctx> {
	/// `main` of the program of the tests.
	pub(super) fn lower_test_main(&mut self) -> Result<(), Error> {
		if self.target == Target::Wasm {
			return Err(Error::Unsupported {
				message: "the tests on wasm (`fa test` runs them on `bun` and `native`)"
					.to_string(),
				location: String::new(),
			});
		}
		let i32 = self.context.i32_type();
		let index = self.module.add_global(i32, None, "fa.test_index");
		index.set_initializer(&i32.const_zero());
		index.set_linkage(Linkage::Internal);
		if !self.tasks.asynchronous {
			let main = self.declare_main();
			self.begin_function(main, Repr::Void, None);
			self.read_test_index(main, index)?;
			self.lower_tests(main, index)?;
			return self.end_function();
		}
		// an asynchronous program: the tests run in the coroutine of the top
		// level, run by the event loop
		self.lower_async_main_of(
			|this, top| this.lower_tests(top, index),
			|this, main| this.read_test_index(main, index),
		)
	}

	/// `fa.test_index = atoi(argv[1])` when `main` has an argument.
	fn read_test_index(
		&mut self,
		main: FunctionValue<'ctx>,
		index: GlobalValue<'ctx>,
	) -> Result<(), Error> {
		let i32 = self.context.i32_type();
		let pointer = self.context.ptr_type(AddressSpace::default());
		let argc = main.get_nth_param(0).expect("argc").into_int_value();
		let argv = main.get_nth_param(1).expect("argv").into_pointer_value();
		let atoi = match self.module.get_function("atoi") {
			Some(atoi) => atoi,
			None => self.module.add_function(
				"atoi",
				i32.fn_type(&[pointer.into()], false),
				Some(Linkage::External),
			),
		};
		let given = self.builder.build_int_compare(
			inkwell::IntPredicate::SGT,
			argc,
			i32.const_int(1, false),
			"",
		)?;
		let read = self.append_block("test_index");
		let after = self.append_block("test_index_read");
		self.builder.build_conditional_branch(given, read, after)?;
		self.builder.position_at_end(read);
		let slot = unsafe {
			self.builder
				.build_in_bounds_gep(pointer, argv, &[i32.const_int(1, false)], "")?
		};
		let text = self.builder.build_load(pointer, slot, "")?;
		let value = self
			.builder
			.build_call(atoi, &[text.into()], "")?
			.try_as_basic_value()
			.basic()
			.expect("an index");
		self.builder.build_store(index.as_pointer_value(), value)?;
		self.builder.build_unconditional_branch(after)?;
		self.builder.position_at_end(after);
		Ok(())
	}

	/// The tests, one branch each, chosen by `fa.test_index`, in the
	/// current function (`main`, or the coroutine of the top level).
	fn lower_tests(
		&mut self,
		function: FunctionValue<'ctx>,
		index: GlobalValue<'ctx>,
	) -> Result<(), Error> {
		if !self.in_coroutine() {
			self.start_program(function)?;
		}
		let i32 = self.context.i32_type();
		let chosen = self
			.builder
			.build_load(i32, index.as_pointer_value(), "test")?
			.into_int_value();
		let tests = self.tests.clone();
		let blocks: Vec<_> = tests.iter().map(|_| self.append_block("test")).collect();
		let none = self.append_block("no_test");
		let cases: Vec<_> = blocks
			.iter()
			.enumerate()
			.map(|(position, block)| (i32.const_int(position as u64, false), *block))
			.collect();
		self.builder.build_switch(chosen, none, &cases)?;
		let entry = self.file;
		let root = NodeId::new(self.tree.root);
		// an index out of range runs nothing
		self.builder.position_at_end(none);
		self.build_return(Val::void(), root)?;
		for ((file, test), block) in tests.into_iter().zip(blocks) {
			self.builder.position_at_end(block);
			self.cleanups.borrow_mut().clear();
			// the module-level values, fresh for this test (D33)
			self.initialize_modules()?;
			self.switch_file(file);
			let root = NodeId::new(self.tree.root);
			let statements: Vec<NodeId> = match self.view(root) {
				View::Module { statements } => statements.iter().collect(),
				_ => Vec::new(),
			};
			// the test file's own values
			for statement in statements {
				if matches!(
					self.view(statement),
					View::Function { .. }
						| View::ExportFunction { .. }
						| View::TypeDecl { .. }
						| View::ExportType { .. }
						| View::Test { .. }
				) || self.is_name_alias(statement)
				{
					continue;
				}
				let mark = self.mark();
				self.statement(statement)?;
				self.end_statement(mark, None)?;
			}
			let View::Test { body, .. } = self.view(test) else {
				self.switch_file(entry);
				continue;
			};
			let mark = self.mark();
			self.statement(body)?;
			self.end_statement(mark, None)?;
			self.release_reactivity()?;
			if !self.terminated() {
				self.build_return(Val::void(), test)?;
			}
			self.switch_file(entry);
		}
		Ok(())
	}
}
