use std::fmt;

use inkwell::builder::BuilderError;
use inkwell::support::LLVMString;

/// An error of the backend: a bug in the generated IR, an LLVM failure, or a
/// failing external tool (linker, JavaScript runtime).
#[derive(Debug)]
pub enum Error {
	/// The builder refused an instruction (a bug of the code generator).
	Builder(BuilderError),
	/// LLVM reported an error (verification, passes, object emission).
	Llvm(String),
	/// An external tool could not run or failed.
	Tool { command: String, message: String },
	/// Reading or writing a file failed.
	Io(std::io::Error),
}

impl fmt::Display for Error {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Error::Builder(error) => write!(formatter, "IR builder error: {error}"),
			Error::Llvm(message) => write!(formatter, "LLVM error: {message}"),
			Error::Tool { command, message } => write!(formatter, "`{command}` failed: {message}"),
			Error::Io(error) => write!(formatter, "I/O error: {error}"),
		}
	}
}

impl std::error::Error for Error {}

impl From<BuilderError> for Error {
	fn from(error: BuilderError) -> Self {
		Error::Builder(error)
	}
}

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
