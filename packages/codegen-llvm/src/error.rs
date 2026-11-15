use std::fmt;

#[cfg(fa_llvm)]
use inkwell::builder::BuilderError;
#[cfg(fa_llvm)]
use inkwell::support::LLVMString;

/// An error of the backend: a program that doesn't check, a construct the
/// backend doesn't compile yet, a bug in the generated IR, an LLVM failure,
/// or a failing external tool (linker, JavaScript runtime).
#[derive(Debug)]
pub enum Error {
	/// The program has errors: the diagnostics rendered for the command line.
	Diagnostics(String),
	/// A construct this backend doesn't compile yet.
	Unsupported {
		/// What isn't supported.
		message: String,
		/// `file:line:column`.
		location: String,
	},
	/// The builder refused an instruction (a bug of the code generator).
	#[cfg(fa_llvm)]
	Builder(BuilderError),
	/// This build of the crate doesn't link LLVM.
	Unavailable,
	/// LLVM reported an error (verification, passes, object emission).
	Llvm(String),
	/// The JIT could not load the program in memory (`jit.rs`): it is
	/// linked into an executable instead.
	Jit(String),
	/// An external tool could not run or failed.
	Tool { command: String, message: String },
	/// Reading or writing a file failed.
	Io(std::io::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Diagnostics(rendered) => write!(formatter, "{rendered}"),
			Error::Unsupported { message, location } => write!(
				formatter,
				"{location}: {message} is not supported by the native backend yet"
			),
			#[cfg(fa_llvm)]
			Error::Builder(error) => write!(formatter, "IR builder error: {error}"),
			Error::Unavailable => write!(formatter, "{}", crate::LLVM_MISSING),
			Error::Llvm(message) => write!(formatter, "LLVM error: {message}"),
			Error::Jit(message) => write!(formatter, "the JIT can't load the program: {message}"),
			Error::Tool { command, message } => write!(formatter, "`{command}` failed: {message}"),
			Error::Io(error) => write!(formatter, "I/O error: {error}"),
		}
	}
}

impl std::error::Error for Error {}

#[cfg(fa_llvm)]
impl From<BuilderError> for Error {
	fn from(error: BuilderError) -> Self {
		Error::Builder(error)
	}
}

#[cfg(fa_llvm)]
impl From<LLVMString> for Error {
	fn from(message: LLVMString) -> Self {
		Error::Llvm(message.to_string())
	}
}

impl From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Error::Io(error)
	}
}
