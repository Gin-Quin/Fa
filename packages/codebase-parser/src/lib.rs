//! `fa-codebase-parser`: the files of a Fa codebase and the name tables of
//! its folders (advanced brief §15.2), see [`codebase`].

pub mod codebase;
pub mod file_system_modules;
pub mod workspace;

pub use codebase::{
	Codebase, Configuration, Entry, FileKind, Folder, FolderId, Layout, Root, ScanDiagnostic,
	SourceFile,
};
pub use workspace::{WorkspacePackages, find_workspace, scan_workspace};

#[cfg(test)]
mod tests {
	mod file_system_modules;
}
