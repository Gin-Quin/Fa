//! The I/O of the standard library natively (`design/async.md` §5, step
//! S1): `std.time.monotonic`, the files of `std.fs` and the sockets of
//! `std.net`. The runtime part is the "sockets and files" section of
//! `runtime/fa_runtime.c`.
//!
//! - **Blocking** calls (a synchronous instance, or asynchronous mode off)
//!   call the blocking functions of the runtime: a socket operation waits
//!   with `poll`, a file operation runs in place.
//! - **Awaited** calls in an async instance call `fa_*_async`, which tries
//!   the operation and answers `AWAIT_READY` (done), `AWAIT_YIELD` (done,
//!   but the coop budget is spent: the task was queued) or `AWAIT_SUSPEND`
//!   (the task waits for epoll, in a wait of its frame): the task suspends,
//!   and calls the operation again when it is resumed. A file operation is
//!   a job of the blocking pool (`fa_file_submit`), whose end wakes the
//!   task. The destroy edge of each suspension cancels the wait, or the job.
//! - **Sockets** are `Repr::Sync` blocks (`FaSocket`: a `FaShared` header
//!   whose drop closes the descriptor), like the handles of `parallel.rs`.

use fa_analyzer::analysis::{Argument, CallInfo, FieldValue, ObjectInfo};
use fa_analyzer::ids::NodeId;
use inkwell::IntPredicate;
use inkwell::values::{BasicMetadataValueEnum, IntValue, PointerValue};

use crate::error::Error;
use crate::lower::Lowering;
use crate::lower::tasks::{AWAIT_READY, WAIT_WORDS};
use crate::lower::values::{IntRepr, Repr, Val};

/// `AWAIT_YIELD` of the runtime.
const AWAIT_YIELD: u64 = 2;
/// The kinds of `fa_file_job`.
const FILE_READ: u64 = 0;
const FILE_READ_BYTES: u64 = 1;
const FILE_WRITE: u64 = 2;

/// What the destroy edge of an awaited operation does.
#[derive(Copy, Clone)]
enum Cancel<'ctx> {
	/// `fa_tcp_cancel(socket, wait, out)`: forgets the wait, closes the
	/// socket the operation made (`out`, or null).
	Socket(PointerValue<'ctx>, PointerValue<'ctx>),
	/// `fa_tcp_cancel(null, wait, out)`: a connection in progress.
	Connecting(PointerValue<'ctx>),
	/// `fa_file_cancel(job)`.
	File(PointerValue<'ctx>),
}

impl<'ctx> Lowering<'_, 'ctx> {
	/// The given arguments of a call (not the receiver nor the defaults).
	fn io_given(info: &CallInfo) -> Vec<NodeId> {
		info.arguments
			.iter()
			.filter_map(|argument| match argument {
				Argument::Given(value) => Some(*value),
				_ => None,
			})
			.collect()
	}

	/// The values of the given arguments, `None` when one never ends.
	fn io_arguments(&mut self, info: &CallInfo) -> Result<Option<Vec<Val<'ctx>>>, Error> {
		let mut values = Vec::new();
		for argument in Self::io_given(info) {
			let value = self.expr(argument)?;
			if value.repr == Repr::Never {
				return Ok(None);
			}
			values.push(value);
		}
		Ok(Some(values))
	}

	/// An integer argument as an `i64`.
	fn io_integer(&self, value: Val<'ctx>, node: NodeId) -> Result<IntValue<'ctx>, Error> {
		Ok(self
			.convert(value, Repr::Int(IntRepr::I64), node.index())?
			.int())
	}

	/// The receiver of a method of a socket: its block.
	fn socket_receiver(&mut self, call: NodeId) -> Result<Option<PointerValue<'ctx>>, Error> {
		let receiver = self.receiver(call)?;
		if receiver.repr == Repr::Never {
			return Ok(None);
		}
		let Repr::Sync(_) = receiver.repr else {
			return Err(self.unsupported(call.index(), "this socket"));
		};
		Ok(Some(receiver.value.expect("a socket").into_pointer_value()))
	}

	/// The array a `read` fills: the pointer to its header, read from its
	/// place (the call mutates it).
	fn byte_buffer(&mut self, argument: NodeId) -> Result<PointerValue<'ctx>, Error> {
		let value = self.expr(argument)?;
		if value.repr == Repr::Never {
			return Err(self.unsupported(argument.index(), "this buffer"));
		}
		let Repr::Array(_) = value.repr else {
			return Err(self.unsupported(argument.index(), "this buffer"));
		};
		Ok(value.value.expect("an array").into_pointer_value())
	}

	/// The built-ins of `std.time`, `std.fs` and `std.net` called blocking,
	/// and the members of sockets that don't wait (`close`).
	pub(crate) fn io_builtin(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Val<'ctx>, Error> {
		let i64 = self.context.i64_type();
		match path {
			"std.time.now" => {
				let now = self.call_value(self.runtime.now()?, &[])?.into_int_value();
				Ok(Val::new(Repr::Int(IntRepr::I64), now))
			}
			"std.time.monotonic" => {
				let now = self
					.call_value(self.runtime.function("fa_clock")?, &[])?
					.into_int_value();
				Ok(Val::new(Repr::Int(IntRepr::I64), now))
			}
			"File.read" | "File.readBytes" | "File.write" => {
				let Some(job) = self.file_job(node, path, info)? else {
					return Ok(Val::never());
				};
				self.call_runtime("fa_file_run", &[job.into()])?;
				self.file_finish(node, path, job)
			}
			"TcpListener#close" | "TcpStream#close" => {
				let Some(socket) = self.socket_receiver(node)? else {
					return Ok(Val::never());
				};
				self.call_runtime("fa_tcp_close", &[socket.into()])?;
				Ok(Val::void())
			}
			"TcpListener#accept" => {
				let Some(listener) = self.socket_receiver(node)? else {
					return Ok(Val::never());
				};
				let out = self.alloca_type(self.pointer_type().into(), "accepted")?;
				let error = self
					.call_value(
						self.runtime.function("fa_tcp_accept")?,
						&[listener.into(), out.into()],
					)?
					.into_pointer_value();
				self.socket_result(node, error, out)
			}
			"TcpStream.connect" => {
				let arguments = Self::io_given(info);
				let Some(values) = self.io_arguments(info)? else {
					return Ok(Val::never());
				};
				let [host, port] = values[..] else {
					return Err(self.unsupported(node.index(), path));
				};
				let (bytes, length) = self.string_parts(host)?;
				let port = self.io_integer(port, arguments[1])?;
				let out = self.alloca_type(self.pointer_type().into(), "connected")?;
				let error = self
					.call_value(
						self.runtime.function("fa_tcp_connect")?,
						&[bytes.into(), length.into(), port.into(), out.into()],
					)?
					.into_pointer_value();
				self.socket_result(node, error, out)
			}
			"TcpStream#read" => {
				let Some(stream) = self.socket_receiver(node)? else {
					return Ok(Val::never());
				};
				let arguments = Self::io_given(info);
				let Some(&argument) = arguments.first() else {
					return Err(self.unsupported(node.index(), path));
				};
				let buffer = self.byte_buffer(argument)?;
				let out = self.alloca_type(i64.into(), "count")?;
				let error = self
					.call_value(
						self.runtime.function("fa_tcp_read")?,
						&[stream.into(), buffer.into(), out.into()],
					)?
					.into_pointer_value();
				self.raise_if_error(error)?;
				let count = self.builder.build_load(i64, out, "")?;
				Ok(Val::new(Repr::Int(IntRepr::I64), count))
			}
			"TcpStream#write" => {
				let Some(stream) = self.socket_receiver(node)? else {
					return Ok(Val::never());
				};
				let Some((bytes, count)) = self.write_arguments(node, info)? else {
					return Ok(Val::never());
				};
				let error = self
					.call_value(
						self.runtime.function("fa_tcp_write")?,
						&[stream.into(), bytes.into(), count.into()],
					)?
					.into_pointer_value();
				self.raise_if_error(error)?;
				Ok(Val::void())
			}
			_ => Err(self.unsupported(node.index(), path)),
		}
	}

	/// The bytes and the count of `stream.write(bytes, count)` (-1: every byte).
	fn write_arguments(
		&mut self,
		node: NodeId,
		info: &CallInfo,
	) -> Result<Option<(PointerValue<'ctx>, IntValue<'ctx>)>, Error> {
		let arguments = Self::io_given(info);
		let Some(&first) = arguments.first() else {
			return Err(self.unsupported(node.index(), "this `write`"));
		};
		let bytes = self.expr(first)?;
		if bytes.repr == Repr::Never {
			return Ok(None);
		}
		let Repr::Array(_) = bytes.repr else {
			return Err(self.unsupported(first.index(), "these bytes"));
		};
		let count = match arguments.get(1) {
			Some(&argument) => {
				let count = self.expr(argument)?;
				if count.repr == Repr::Never {
					return Ok(None);
				}
				self.io_integer(count, argument)?
			}
			None => self.context.i64_type().const_all_ones(),
		};
		// a temporary is dropped at the end of the statement
		Ok(Some((
			bytes.value.expect("an array").into_pointer_value(),
			count,
		)))
	}

	/// The stream an operation gave (`out`), or its error raised.
	fn socket_result(
		&mut self,
		node: NodeId,
		error: PointerValue<'ctx>,
		out: PointerValue<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		self.raise_if_error(error)?;
		let stream = self.builder.build_load(self.pointer_type(), out, "")?;
		let ty = self.type_of(node);
		Ok(self.fresh(Val::new(Repr::Sync(ty), stream)))
	}

	/// The job of a file operation (its inputs copied).
	fn file_job(
		&mut self,
		node: NodeId,
		path: &str,
		info: &CallInfo,
	) -> Result<Option<PointerValue<'ctx>>, Error> {
		let Some(values) = self.io_arguments(info)? else {
			return Ok(None);
		};
		let Some(&file) = values.first() else {
			return Err(self.unsupported(node.index(), path));
		};
		let (path_bytes, path_length) = self.string_parts(file)?;
		let (kind, bytes, length) = match path {
			"File.write" => {
				let Some(&text) = values.get(1) else {
					return Err(self.unsupported(node.index(), path));
				};
				let (bytes, length) = self.string_parts(text)?;
				(FILE_WRITE, bytes, length)
			}
			_ => (
				if path == "File.read" {
					FILE_READ
				} else {
					FILE_READ_BYTES
				},
				self.pointer_type().const_null(),
				self.usize.const_zero(),
			),
		};
		let kind = self.context.i32_type().const_int(kind, false);
		let job = self
			.call_value(
				self.runtime.function("fa_file_job")?,
				&[
					kind.into(),
					path_bytes.into(),
					path_length.into(),
					bytes.into(),
					length.into(),
				],
			)?
			.into_pointer_value();
		Ok(Some(job))
	}

	/// The end of a file operation: its result, or its error raised.
	fn file_finish(
		&mut self,
		node: NodeId,
		path: &str,
		job: PointerValue<'ctx>,
	) -> Result<Val<'ctx>, Error> {
		let repr = match path {
			"File.read" => Repr::Str,
			"File.readBytes" => self.node_repr(node)?,
			_ => Repr::Void,
		};
		let slot = if repr == Repr::Void {
			None
		} else {
			Some(self.alloca(repr, "file")?)
		};
		let out = slot.map_or(self.pointer_type().const_null(), |slot| slot.pointer);
		let error = self
			.call_value(
				self.runtime.function("fa_file_finish")?,
				&[job.into(), out.into()],
			)?
			.into_pointer_value();
		self.raise_if_error(error)?;
		match slot {
			Some(slot) => {
				let value = self.load(slot)?;
				Ok(self.fresh(value))
			}
			None => Ok(Val::void()),
		}
	}

	/// `TcpListener { host, port }`: a socket listening at once.
	pub(crate) fn listener_literal(
		&mut self,
		node: NodeId,
		info: &ObjectInfo,
	) -> Result<Val<'ctx>, Error> {
		let mut host = None;
		let mut port = None;
		for &(name, value) in &info.fields {
			let FieldValue::Given(value) = value else {
				continue;
			};
			match self.types().syms.resolve(name) {
				"host" => host = Some(value),
				"port" => port = Some(value),
				_ => {}
			}
		}
		let (host_bytes, host_length) = match host {
			Some(value) => {
				let lowered = self.expr(value)?;
				if lowered.repr == Repr::Never {
					return Ok(lowered);
				}
				self.string_parts(lowered)?
			}
			None => {
				let bytes = self.constant_bytes(b"127.0.0.1");
				(bytes, self.usize.const_int(9, false))
			}
		};
		let Some(port) = port else {
			return Err(self.unsupported(node.index(), "a listener without its port"));
		};
		let lowered = self.expr(port)?;
		if lowered.repr == Repr::Never {
			return Ok(lowered);
		}
		let port = self.io_integer(lowered, port)?;
		let listener = self
			.call_value(
				self.runtime.function("fa_tcp_listen")?,
				&[host_bytes.into(), host_length.into(), port.into()],
			)?
			.into_pointer_value();
		Ok(self.fresh(Val::new(Repr::Sync(info.ty), listener)))
	}

	/// `listener.port`.
	pub(crate) fn listener_port(&self, listener: Val<'ctx>) -> Result<Val<'ctx>, Error> {
		let port = self
			.call_value(
				self.runtime.function("fa_tcp_port")?,
				&[listener.value.expect("a listener").into()],
			)?
			.into_int_value();
		Ok(Val::new(Repr::Int(IntRepr::I64), port))
	}

	// ── awaited in an async instance ──────────────────────────────────────

	/// `await` of a primitive of `std.fs` or `std.net` in an async instance.
	pub(crate) fn await_io(
		&mut self,
		node: NodeId,
		name: &str,
		call: NodeId,
	) -> Result<Val<'ctx>, Error> {
		let Some(info) = self.analysis.call(call).cloned() else {
			return Err(self.unsupported(node.index(), name));
		};
		let handle = self.coroutine().expect("a coroutine").handle;
		let i64 = self.context.i64_type();
		let pointer = self.pointer_type();
		let null = pointer.const_null();
		match name {
			"File.read" | "File.readBytes" | "File.write" => {
				let Some(job) = self.file_job(call, name, &info)? else {
					return Ok(Val::never());
				};
				let wait = self.words(WAIT_WORDS, "wait")?;
				let status = self
					.runtime_value("fa_file_submit", &[job.into(), wait.into(), handle.into()])?
					.into_int_value();
				let ready = self.builder.build_int_compare(
					IntPredicate::EQ,
					status,
					self.context.i32_type().const_int(AWAIT_READY, false),
					"",
				)?;
				let waiting = self.append_block("file_waiting");
				let done = self.append_block("file_done");
				self.builder
					.build_conditional_branch(ready, done, waiting)?;
				self.builder.position_at_end(waiting);
				self.suspend(Some(node), |this| this.cancel_io(Cancel::File(job), wait))?;
				self.builder.build_unconditional_branch(done)?;
				self.builder.position_at_end(done);
				self.file_finish(call, name, job)
			}
			"TcpListener.accept" => {
				let Some(listener) = self.socket_receiver(call)? else {
					return Ok(Val::never());
				};
				let out = self.alloca_type(pointer.into(), "accepted")?;
				let error = self.alloca_type(pointer.into(), "error")?;
				self.builder.build_store(out, null)?;
				self.builder.build_store(error, null)?;
				self.await_operation(node, Cancel::Socket(listener, out), |this, wait| {
					this.runtime_value(
						"fa_tcp_accept_async",
						&[
							listener.into(),
							wait.into(),
							handle.into(),
							out.into(),
							error.into(),
						],
					)
				})?;
				let error = self.builder.build_load(pointer, error, "")?;
				self.socket_result(call, error.into_pointer_value(), out)
			}
			"TcpStream.connect" => {
				let arguments = Self::io_given(&info);
				let Some(values) = self.io_arguments(&info)? else {
					return Ok(Val::never());
				};
				let [host, port] = values[..] else {
					return Err(self.unsupported(node.index(), name));
				};
				let (bytes, length) = self.string_parts(host)?;
				let port = self.io_integer(port, arguments[1])?;
				let out = self.alloca_type(pointer.into(), "connected")?;
				let error = self.alloca_type(pointer.into(), "error")?;
				self.builder.build_store(out, null)?;
				self.builder.build_store(error, null)?;
				self.await_operation(node, Cancel::Connecting(out), |this, wait| {
					let arguments: [BasicMetadataValueEnum<'ctx>; 7] = [
						bytes.into(),
						length.into(),
						port.into(),
						wait.into(),
						handle.into(),
						out.into(),
						error.into(),
					];
					this.runtime_value("fa_tcp_connect_async", &arguments)
				})?;
				let error = self.builder.build_load(pointer, error, "")?;
				self.socket_result(call, error.into_pointer_value(), out)
			}
			"TcpStream.read" => {
				let Some(stream) = self.socket_receiver(call)? else {
					return Ok(Val::never());
				};
				let Some(&argument) = Self::io_given(&info).first() else {
					return Err(self.unsupported(node.index(), name));
				};
				let buffer = self.byte_buffer(argument)?;
				let out = self.alloca_type(i64.into(), "count")?;
				let error = self.alloca_type(pointer.into(), "error")?;
				self.builder.build_store(error, null)?;
				self.await_operation(node, Cancel::Socket(stream, null), |this, wait| {
					this.runtime_value(
						"fa_tcp_read_async",
						&[
							stream.into(),
							buffer.into(),
							wait.into(),
							handle.into(),
							out.into(),
							error.into(),
						],
					)
				})?;
				let error = self.builder.build_load(pointer, error, "")?;
				self.raise_if_error(error.into_pointer_value())?;
				let count = self.builder.build_load(i64, out, "")?;
				Ok(Val::new(Repr::Int(IntRepr::I64), count))
			}
			"TcpStream.write" => {
				let Some(stream) = self.socket_receiver(call)? else {
					return Ok(Val::never());
				};
				let Some((bytes, count)) = self.write_arguments(call, &info)? else {
					return Ok(Val::never());
				};
				let written = self.alloca_type(i64.into(), "written")?;
				let error = self.alloca_type(pointer.into(), "error")?;
				self.builder.build_store(written, i64.const_zero())?;
				self.builder.build_store(error, null)?;
				self.await_operation(node, Cancel::Socket(stream, null), |this, wait| {
					this.runtime_value(
						"fa_tcp_write_async",
						&[
							stream.into(),
							bytes.into(),
							count.into(),
							wait.into(),
							handle.into(),
							written.into(),
							error.into(),
						],
					)
				})?;
				let error = self.builder.build_load(pointer, error, "")?;
				self.raise_if_error(error.into_pointer_value())?;
				Ok(Val::void())
			}
			_ => Err(self.unsupported(node.index(), name)),
		}
	}

	/// Calls an awaited operation (`operation`, giving `AWAIT_*`) until it
	/// ends: done at `AWAIT_READY`; after one suspension at `AWAIT_YIELD`;
	/// called again after the suspension at `AWAIT_SUSPEND`.
	fn await_operation(
		&mut self,
		node: NodeId,
		cancel: Cancel<'ctx>,
		operation: impl Fn(
			&mut Self,
			PointerValue<'ctx>,
		) -> Result<inkwell::values::BasicValueEnum<'ctx>, Error>,
	) -> Result<(), Error> {
		let wait = self.words(WAIT_WORDS, "wait")?;
		let again = self.append_block("io_again");
		let waiting = self.append_block("io_waiting");
		let yielding = self.append_block("io_yielding");
		let done = self.append_block("io_done");
		self.builder.build_unconditional_branch(again)?;
		self.builder.position_at_end(again);
		let status = operation(self, wait)?.into_int_value();
		let i32 = self.context.i32_type();
		self.builder.build_switch(
			status,
			waiting,
			&[
				(i32.const_int(AWAIT_READY, false), done),
				(i32.const_int(AWAIT_YIELD, false), yielding),
			],
		)?;
		self.builder.position_at_end(waiting);
		self.suspend(Some(node), |this| this.cancel_io(cancel, wait))?;
		self.builder.build_unconditional_branch(again)?;
		self.builder.position_at_end(yielding);
		self.suspend(Some(node), |this| this.cancel_io(cancel, wait))?;
		self.builder.build_unconditional_branch(done)?;
		self.builder.position_at_end(done);
		Ok(())
	}

	/// The destroy edge of an awaited operation.
	fn cancel_io(&mut self, cancel: Cancel<'ctx>, wait: PointerValue<'ctx>) -> Result<(), Error> {
		let null = self.pointer_type().const_null();
		match cancel {
			Cancel::Socket(socket, out) => {
				self.call_runtime("fa_tcp_cancel", &[socket.into(), wait.into(), out.into()])
			}
			Cancel::Connecting(out) => {
				self.call_runtime("fa_tcp_cancel", &[null.into(), wait.into(), out.into()])
			}
			Cancel::File(job) => self.call_runtime("fa_file_cancel", &[job.into()]),
		}
	}
}
