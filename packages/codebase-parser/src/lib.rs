//! `fa-codebase-parser`: the files of a Fa codebase and the name tables of
//! its folders (advanced brief §15.2), see [`codebase`].

pub mod codebase;
pub mod file_system_modules;

pub use codebase::{
	Codebase, Configuration, Entry, FileKind, Folder, FolderId, Layout, Root, ScanDiagnostic,
	SourceFile,
};

#[cfg(test)]
mod tests {
	mod file_system_modules;
}
