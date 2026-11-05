//! Dependencies (advanced brief §17, architecture/dependencies.md): the
//! sources of `dependencies.fa`, their versions, the lockfile and the cache.
//!
//! - A source is a sibling package (`"../shared"`, built from source, never
//!   locked), a Git repository (`"github:owner/repo@4"`, `"gitlab:…"`,
//!   `"git+<url>@4.2"`), a remote file (`"https://…/types.fa"`) or a remote
//!   folder (`"https://…/ui@2.1.0/"`, listed by its `manifest.fa`).
//! - The versions of a repository are its semantic version tags; for each
//!   repository and major version, the latest version satisfying every range
//!   is chosen ([`resolve`]); different majors coexist.
//! - `dependencies.lock` records the version, the commit and the SHA-256 of
//!   every remote source (decision D45 for the content of several files); a
//!   locked version is kept until `fa update` names it, and a content that
//!   changed is `DEP001`.
//! - The cache (`$FA_CACHE`, else `~/.cache/fa`) holds a mirror of each Git
//!   repository and the sources by hash: a build whose dependencies are all
//!   locked and cached needs no network.
//!
//! Git is used through the `git` command. `FA_GITHUB_BASE` and
//! `FA_GITLAB_BASE` replace `https://github.com` and `https://gitlab.com`
//! (the tests use local bare repositories).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

use fa_analyzer::comptime::host::{DependencySource, dependency_sources, run_generate};
use fa_analyzer::configuration::{self, ConfigValue, ConfigurationKind};
use fa_analyzer::diagnostics::{SourceView, render_all};
use fa_analyzer::modules::{CodebaseResolver, ModuleEntry};
use fa_analyzer::mounts::{Mount, MountedResolver};
use fa_codebase_parser::{Codebase, Layout};

use crate::sha256;

/// The files of a Git repository that a dependency uses.
const GIT_PATHS: [&str; 3] = ["package.fa", "dependencies.fa", "library"];

// --------------------------------------------------------------- versions

/// A semantic version (`4.2.1`, `5.0.0-beta.1`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Version {
	pub major: u64,
	pub minor: u64,
	pub patch: u64,
	/// The pre-release (`beta.1`).
	pub pre: Option<String>,
}

impl Version {
	/// Parses `4.2.1` or `v4.2.1`.
	pub(crate) fn parse(text: &str) -> Option<Version> {
		let text = text.strip_prefix('v').unwrap_or(text);
		let (numbers, pre) = match text.split_once('-') {
			Some((numbers, pre)) if !pre.is_empty() => (numbers, Some(pre.to_string())),
			Some(_) => return None,
			None => (text, None),
		};
		let mut parts = numbers.split('.');
		let major = number(parts.next()?)?;
		let minor = number(parts.next()?)?;
		let patch = number(parts.next()?)?;
		if parts.next().is_some() {
			return None;
		}
		Some(Version {
			major,
			minor,
			patch,
			pre,
		})
	}
}

/// A number of a version: digits, no leading zero.
fn number(text: &str) -> Option<u64> {
	if text.is_empty()
		|| !text.bytes().all(|byte| byte.is_ascii_digit())
		|| (text.len() > 1 && text.starts_with('0'))
	{
		return None;
	}
	text.parse().ok()
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		(self.major, self.minor, self.patch)
			.cmp(&(other.major, other.minor, other.patch))
			.then_with(|| match (&self.pre, &other.pre) {
				(None, None) => std::cmp::Ordering::Equal,
				(None, Some(_)) => std::cmp::Ordering::Greater,
				(Some(_), None) => std::cmp::Ordering::Less,
				(Some(a), Some(b)) => compare_pre(a, b),
			})
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

/// Compares two pre-releases, identifier by identifier (semver §11).
fn compare_pre(a: &str, b: &str) -> std::cmp::Ordering {
	let mut a = a.split('.');
	let mut b = b.split('.');
	loop {
		match (a.next(), b.next()) {
			(None, None) => return std::cmp::Ordering::Equal,
			(None, Some(_)) => return std::cmp::Ordering::Less,
			(Some(_), None) => return std::cmp::Ordering::Greater,
			(Some(x), Some(y)) => {
				let order = match (x.parse::<u64>(), y.parse::<u64>()) {
					(Ok(x), Ok(y)) => x.cmp(&y),
					(Ok(_), Err(_)) => std::cmp::Ordering::Less,
					(Err(_), Ok(_)) => std::cmp::Ordering::Greater,
					(Err(_), Err(_)) => x.cmp(y),
				};
				if order.is_ne() {
					return order;
				}
			}
		}
	}
}

impl fmt::Display for Version {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
		if let Some(pre) = &self.pre {
			write!(f, "-{pre}")?;
		}
		Ok(())
	}
}

/// A version range: `@4`, `@4.2`, `@4.2.1`, `@5.0.0-beta.1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Range {
	pub major: u64,
	pub minor: Option<u64>,
	pub patch: Option<u64>,
	pub pre: Option<String>,
}

impl Range {
	/// Parses a range.
	pub(crate) fn parse(text: &str) -> Option<Range> {
		if let Some(version) = Version::parse(text) {
			return Some(Range {
				major: version.major,
				minor: Some(version.minor),
				patch: Some(version.patch),
				pre: version.pre,
			});
		}
		let text = text.strip_prefix('v').unwrap_or(text);
		let mut parts = text.split('.');
		let major = number(parts.next()?)?;
		let minor = match parts.next() {
			Some(minor) => Some(number(minor)?),
			None => None,
		};
		if parts.next().is_some() {
			return None;
		}
		Some(Range {
			major,
			minor,
			patch: None,
			pre: None,
		})
	}

	/// Whether a version satisfies the range: a pre-release only when the
	/// range names it exactly.
	pub(crate) fn matches(&self, version: &Version) -> bool {
		version.major == self.major
			&& self.minor.is_none_or(|minor| version.minor == minor)
			&& self.patch.is_none_or(|patch| version.patch == patch)
			&& version.pre == self.pre
	}
}

impl fmt::Display for Range {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.major)?;
		if let Some(minor) = self.minor {
			write!(f, ".{minor}")?;
		}
		if let Some(patch) = self.patch {
			write!(f, ".{patch}")?;
		}
		if let Some(pre) = &self.pre {
			write!(f, "-{pre}")?;
		}
		Ok(())
	}
}

// ---------------------------------------------------------------- sources

/// The source of a dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Source {
	/// A sibling package of the workspace, by its path from the package.
	Path(String),
	/// A Git repository.
	Git {
		/// The repository as locked (`github:farmers/farm`).
		repository: String,
		/// The URL `git` clones.
		url: String,
		/// The range, `None` when unversioned.
		range: Option<Range>,
	},
	/// A remote file or folder (a folder ends with `/`).
	Remote { url: String, folder: bool },
	/// A `DependencySource` with `generate`: a remote file, fetched and
	/// locked, whose content the `generate` function of the key in the
	/// `dependencies.fa` file turns into the Fa source of the dependency.
	Generated {
		url: String,
		/// The `dependencies.fa` file declaring it.
		file: PathBuf,
		key: String,
	},
}

impl Source {
	/// Parses the source of a dependency.
	pub(crate) fn parse(text: &str) -> Result<Source, String> {
		if text == "." || text == ".." || text.starts_with("./") || text.starts_with("../") {
			return Ok(Source::Path(text.to_string()));
		}
		for (scheme, variable, host) in [
			("github:", "FA_GITHUB_BASE", "https://github.com"),
			("gitlab:", "FA_GITLAB_BASE", "https://gitlab.com"),
		] {
			if let Some(rest) = text.strip_prefix(scheme) {
				let (path, range) = split_range(rest)?;
				let parts: Vec<&str> = path.split('/').collect();
				if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
					return Err(format!(
						"`{text}` is not a repository: write `{scheme}owner/repository@4`"
					));
				}
				let base = std::env::var(variable)
					.ok()
					.filter(|base| !base.is_empty())
					.unwrap_or_else(|| host.to_string());
				return Ok(Source::Git {
					repository: format!("{scheme}{path}"),
					url: format!("{}/{path}.git", base.trim_end_matches('/')),
					range,
				});
			}
		}
		if let Some(rest) = text.strip_prefix("git+") {
			let (url, range) = split_range(rest)?;
			if url.is_empty() || url.starts_with('-') {
				return Err(format!(
					"`{text}` is not a repository: write its URL after `git+`, ex: `git+https://example.com/farm.git@4`"
				));
			}
			return Ok(Source::Git {
				repository: format!("git+{url}"),
				url: url.to_string(),
				range,
			});
		}
		if ["https://", "http://", "file://"]
			.iter()
			.any(|scheme| text.starts_with(scheme))
		{
			return Ok(Source::Remote {
				url: text.to_string(),
				folder: text.ends_with('/'),
			});
		}
		Err(format!(
			"`{text}` is not a source: write a Git repository (`github:owner/repository@4`), a sibling package (`../shared`), or a URL (`https://…/types.fa`)"
		))
	}

	/// The source as written in the lockfile (a path: only outside a
	/// workspace).
	fn locked(&self) -> Option<&str> {
		match self {
			Source::Path(path) => Some(path),
			Source::Git { repository, .. } => Some(repository),
			Source::Remote { url, .. } | Source::Generated { url, .. } => Some(url),
		}
	}
}

/// Splits `owner/repository@4.2` into the repository and its range; the
/// `@` of a URL's user (`git@host`) is not a range.
fn split_range(text: &str) -> Result<(&str, Option<Range>), String> {
	match text.rsplit_once('@') {
		Some((path, range)) if !range.contains('/') && !range.contains(':') => {
			match Range::parse(range) {
				Some(parsed) => Ok((path, Some(parsed))),
				None => Err(format!(
					"`@{range}` is not a version range: write `@4`, `@4.2` or `@4.2.1`"
				)),
			}
		}
		_ => Ok((text, None)),
	}
}

/// Whether the path of a URL names a version (`ui@2.1.0`, `/v3/`,
/// `types-1.2.fa`), not its host (`dev1.example.com`, an IP address): an
/// unversioned URL is `DEP004`.
fn is_versioned(url: &str) -> bool {
	let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
	let rest = rest.split(['?', '#']).next().unwrap_or(rest);
	let Some((_, path)) = rest.split_once('/') else {
		return false;
	};
	path.split('/').any(|segment| {
		let numbered = segment.as_bytes().windows(3).any(|window| {
			window[0].is_ascii_digit() && window[1] == b'.' && window[2].is_ascii_digit()
		});
		let tagged = segment
			.split('@')
			.skip(1)
			.any(|version| version.starts_with(|c: char| c.is_ascii_digit()));
		let major = segment
			.strip_prefix('v')
			.is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
		numbered || tagged || major
	})
}

/// Whether a Git URL names a remote repository (`https://`, `ssh://`,
/// `git://`, `user@host:path`), not one on the disk (a path, `file://`) nor
/// a transport helper (`ext::`).
fn is_remote_git(url: &str) -> bool {
	if let Some((scheme, _)) = url.split_once("://") {
		return matches!(scheme, "https" | "http" | "ssh" | "git");
	}
	// scp-like: `[user@]host:path`
	match url.split_once(':') {
		Some((host, path)) => {
			host.len() > 1 && !host.contains(['/', '\\']) && !path.starts_with(':')
		}
		None => false,
	}
}

/// The source as written when it reads the local disk: a path, a `file://`
/// URL, or a Git repository that is not remote.
fn local_source(source: &Source) -> Option<&str> {
	match source {
		Source::Path(path) => Some(path),
		Source::Remote { url, .. } | Source::Generated { url, .. } => {
			(!url.starts_with("https://") && !url.starts_with("http://")).then_some(url.as_str())
		}
		Source::Git { repository, .. } => repository
			.strip_prefix("git+")
			.filter(|url| !is_remote_git(url))
			.map(|_| repository.as_str()),
	}
}

/// `DEP008` when a dependency fetched from Git or a remote source (`name`)
/// declares a source on the local disk (decision "Dependency security").
fn check_fetched_source(name: &str, key: &str, source: &Source) -> Result<(), String> {
	// the tests serve their "remote" files and repositories with `file://`
	let served = |local: &str| {
		std::env::var_os("FA_TEST_FILE_SOURCES").is_some_and(|value| value == "1")
			&& local.trim_start_matches("git+").starts_with("file://")
	};
	match local_source(source) {
		Some(local) if !served(local) => Err(format!(
			"error[DEP008]: `{name}` depends on `{key}` by a local source (`{local}`): a dependency fetched from Git or a remote source can only depend on remote sources"
		)),
		_ => Ok(()),
	}
}

/// Whether a path from a manifest or a Git tree is a relative path inside
/// its folder on every system: `/` separators, no empty, `.` or `..`
/// segment, no backslash, colon (drive letter) or control character.
pub(crate) fn is_portable_path(path: &str) -> bool {
	!path.is_empty()
		&& !path
			.chars()
			.any(|c| c == '\\' || c == ':' || c.is_control())
		&& path
			.split('/')
			.all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Whether a text is a hash of the lockfile: `sha256:` and 64 lowercase
/// hexadecimal digits.
fn is_lock_hash(text: &str) -> bool {
	text.strip_prefix("sha256:").is_some_and(|hex| {
		hex.len() == 64
			&& hex
				.bytes()
				.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
	})
}

/// Whether a text is a Git commit: 40 (SHA-1) or 64 (SHA-256) lowercase
/// hexadecimal digits.
fn is_commit(text: &str) -> bool {
	matches!(text.len(), 40 | 64)
		&& text
			.bytes()
			.all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

// --------------------------------------------------------------- lockfile

/// An entry of `dependencies.lock`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LockEntry {
	pub source: String,
	pub version: Option<String>,
	pub commit: Option<String>,
	pub hash: String,
}

/// The name of the lockfile.
pub(crate) const LOCKFILE: &str = "dependencies.lock";

/// Reads a lockfile; a missing file is an empty one.
pub(crate) fn read_lockfile(path: &Path) -> Result<Vec<LockEntry>, String> {
	let Ok(text) = fs::read_to_string(path) else {
		return Ok(Vec::new());
	};
	let invalid = || {
		format!(
			"error: `{}` is not a valid lockfile\n  help: delete it, then run `fa update` to write it again",
			path.display()
		)
	};
	let Some(ConfigValue::Array(items)) = configuration::evaluate_literal(&text) else {
		return Err(invalid());
	};
	let mut entries = Vec::new();
	for item in items {
		let field = |name: &str| {
			item.field(name)
				.and_then(ConfigValue::as_str)
				.map(str::to_string)
		};
		let (Some(source), Some(hash)) = (field("source"), field("hash")) else {
			return Err(invalid());
		};
		// the hash names a folder of the cache, the commit is given to `git`
		if !is_lock_hash(&hash) {
			return Err(format!(
				"{}\n  note: the hash of `{source}` is not `sha256:` followed by 64 hexadecimal digits",
				invalid()
			));
		}
		let commit = field("commit");
		if let Some(commit) = commit.as_deref().filter(|commit| !is_commit(commit)) {
			return Err(format!(
				"{}\n  note: the commit of `{source}` is not a commit (`{commit}`)",
				invalid()
			));
		}
		entries.push(LockEntry {
			source,
			version: field("version"),
			commit,
			hash,
		});
	}
	Ok(entries)
}

/// The text of a lockfile: sorted by source then version.
pub(crate) fn render_lockfile(entries: &[LockEntry]) -> String {
	let mut entries = entries.to_vec();
	entries.sort_by(|a, b| {
		a.source.cmp(&b.source).then_with(|| {
			let version = |entry: &LockEntry| entry.version.as_deref().and_then(Version::parse);
			version(a).cmp(&version(b))
		})
	});
	entries.dedup();
	let mut out = String::from("-- dependencies.lock: generated by `fa`, do not edit\n");
	if entries.is_empty() {
		out.push_str("export = []\n");
		return out;
	}
	out.push_str("export = [\n");
	for entry in &entries {
		out.push_str("\t{\n");
		out.push_str(&format!("\t\tsource = {}\n", quote(&entry.source)));
		if let Some(version) = &entry.version {
			out.push_str(&format!("\t\tversion = {}\n", quote(version)));
		}
		if let Some(commit) = &entry.commit {
			out.push_str(&format!("\t\tcommit = {}\n", quote(commit)));
		}
		out.push_str(&format!("\t\thash = {}\n", quote(&entry.hash)));
		out.push_str("\t}\n");
	}
	out.push_str("]\n");
	out
}

/// A Fa string literal.
fn quote(text: &str) -> String {
	let mut out = String::from("\"");
	for c in text.chars() {
		match c {
			'"' => out.push_str("\\\""),
			'\\' => out.push_str("\\\\"),
			'{' => out.push_str("{{"),
			'}' => out.push_str("}}"),
			_ => out.push(c),
		}
	}
	out.push('"');
	out
}

// ------------------------------------------------------------------ cache

/// The cache of `fa`: `$FA_CACHE`, else `$XDG_CACHE_HOME/fa` or
/// `~/.cache/fa` (Linux), `~/Library/Caches/fa` (macOS), `%LOCALAPPDATA%\fa`.
pub(crate) fn cache_root() -> PathBuf {
	let variable = |name: &str| std::env::var_os(name).filter(|value| !value.is_empty());
	if let Some(cache) = variable("FA_CACHE") {
		return PathBuf::from(cache);
	}
	if cfg!(windows)
		&& let Some(local) = variable("LOCALAPPDATA")
	{
		return PathBuf::from(local).join("fa");
	}
	if let Some(cache) = variable("XDG_CACHE_HOME") {
		return PathBuf::from(cache).join("fa");
	}
	match variable("HOME") {
		Some(home) if cfg!(target_os = "macos") => PathBuf::from(home).join("Library/Caches/fa"),
		Some(home) => PathBuf::from(home).join(".cache/fa"),
		None => std::env::temp_dir().join("fa-cache"),
	}
}

/// The hash of several files (decision D45): sorted by path, each as its
/// path, a zero byte, its length, a zero byte, then its bytes.
pub(crate) fn hash_files(files: &[(String, Vec<u8>)]) -> String {
	let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
	sorted.sort_by(|a, b| a.0.cmp(&b.0));
	let mut hash = sha256::Sha256::new();
	for (path, bytes) in sorted {
		hash.update(path.as_bytes());
		hash.update(&[0]);
		hash.update(bytes.len().to_string().as_bytes());
		hash.update(&[0]);
		hash.update(bytes);
	}
	format!("sha256:{}", sha256::to_hex(&hash.finish()))
}

/// The files under a folder, by relative path (`/` separators). A symbolic
/// link is read only when it names a file inside the folder; a linked
/// folder is skipped (no escape, no loop).
fn read_tree(directory: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
	let root = fs::canonicalize(directory)?;
	let mut files = Vec::new();
	let mut stack = vec![(directory.to_path_buf(), String::new())];
	while let Some((folder, prefix)) = stack.pop() {
		for entry in fs::read_dir(&folder)? {
			let entry = entry?;
			let name = entry.file_name().to_string_lossy().to_string();
			let relative = if prefix.is_empty() {
				name
			} else {
				format!("{prefix}/{name}")
			};
			let kind = entry.file_type()?;
			if kind.is_dir() {
				stack.push((entry.path(), relative));
			} else if kind.is_symlink() {
				let inside = fs::canonicalize(entry.path())
					.is_ok_and(|target| target.starts_with(&root) && target.is_file());
				if inside {
					files.push((relative, fs::read(entry.path())?));
				}
			} else {
				files.push((relative, fs::read(entry.path())?));
			}
		}
	}
	files.sort();
	Ok(files)
}

/// The folder of the cache holding the content of a hash (checked by
/// `read_lockfile`, or computed).
fn source_folder(hash: &str) -> PathBuf {
	assert!(is_lock_hash(hash), "not a hash: {hash}");
	cache_root()
		.join("sources")
		.join(hash.trim_start_matches("sha256:"))
}

/// Removes a damaged entry of the cache: only a folder directly inside
/// the `sources` folder of the cache, after resolving both paths.
fn remove_cached(folder: &Path) {
	let root = fs::canonicalize(cache_root().join("sources"));
	let resolved = fs::canonicalize(folder);
	if let (Ok(root), Ok(resolved)) = (root, resolved)
		&& resolved.parent() == Some(root.as_path())
		&& fs::symlink_metadata(folder).is_ok_and(|metadata| metadata.is_dir())
	{
		let _ = fs::remove_dir_all(resolved);
	}
}

/// Stores files into the cache under their hash, atomically (written into a
/// new folder, then renamed); returns the folder.
fn store(hash: &str, files: &[(String, Vec<u8>)]) -> Result<PathBuf, String> {
	let folder = source_folder(hash);
	if folder.is_dir() {
		return Ok(folder);
	}
	let parent = folder.parent().unwrap_or(Path::new("."));
	let partial = parent.join(format!(
		".partial-{}-{}",
		std::process::id(),
		hash.trim_start_matches("sha256:")
	));
	if let Some((path, _)) = files.iter().find(|(path, _)| !is_portable_path(path)) {
		return Err(format!(
			"error: can't write `{path}` into the cache: it is not a relative path inside the folder"
		));
	}
	let write = || -> std::io::Result<()> {
		let _ = fs::remove_dir_all(&partial);
		for (path, bytes) in files {
			let path = partial.join(path);
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent)?;
			}
			fs::write(path, bytes)?;
		}
		fs::create_dir_all(&partial)?;
		Ok(())
	};
	write().map_err(|error| {
		format!(
			"error: can't write into the cache `{}`: {error}",
			parent.display()
		)
	})?;
	if fs::rename(&partial, &folder).is_err() {
		// another `fa` stored it first
		let _ = fs::remove_dir_all(&partial);
		if !folder.is_dir() {
			return Err(format!(
				"error: can't write into the cache `{}`",
				folder.display()
			));
		}
	}
	Ok(folder)
}

// -------------------------------------------------------------------- git

/// Runs `git` and returns its standard output.
fn git(arguments: &[&str], directory: Option<&Path>) -> Result<Vec<u8>, String> {
	let mut command = Command::new("git");
	if let Some(directory) = directory {
		command.arg("--git-dir").arg(directory);
	}
	let output = command
		.args(arguments)
		.env("GIT_TERMINAL_PROMPT", "0")
		.output()
		.map_err(|error| format!("can't run `git`: {error}"))?;
	if output.status.success() {
		Ok(output.stdout)
	} else {
		Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
	}
}

/// The mirror of a repository in the cache: cloned the first time, fetched
/// again when `refresh`.
fn mirror(url: &str, refresh: bool) -> Result<PathBuf, String> {
	let key = &sha256::hex(url.as_bytes())[..24];
	let folder = cache_root().join("git").join(format!("{key}.git"));
	let unreachable = |error: String| {
		format!(
			"error[DEP006]: can't reach the repository `{url}`: {error}\n  help: check the network and the source in `dependencies.fa`"
		)
	};
	if !folder.is_dir() {
		let parent = folder.parent().unwrap_or(Path::new("."));
		fs::create_dir_all(parent).map_err(|error| error.to_string())?;
		let partial = parent.join(format!(".partial-{}-{key}.git", std::process::id()));
		let _ = fs::remove_dir_all(&partial);
		let partial_text = partial.to_string_lossy().to_string();
		git(
			&["clone", "--quiet", "--mirror", "--", url, &partial_text],
			None,
		)
		.map_err(unreachable)?;
		if fs::rename(&partial, &folder).is_err() {
			let _ = fs::remove_dir_all(&partial);
		}
	} else if refresh {
		git(
			&["fetch", "--quiet", "--prune", "--tags", "origin"],
			Some(&folder),
		)
		.or_else(|_| git(&["remote", "update", "--prune"], Some(&folder)))
		.map_err(unreachable)?;
	}
	Ok(folder)
}

/// The semantic version tags of a mirror, with their commits.
fn tags(mirror: &Path) -> Result<Vec<(Version, String)>, String> {
	let output = git(
		&[
			"for-each-ref",
			"--format=%(refname:strip=2) %(objectname) %(*objectname)",
			"refs/tags",
		],
		Some(mirror),
	)?;
	let mut found = Vec::new();
	for line in String::from_utf8_lossy(&output).lines() {
		let mut parts = line.split_whitespace();
		let (Some(tag), Some(object)) = (parts.next(), parts.next()) else {
			continue;
		};
		// an annotated tag points at its commit through `*objectname`
		let commit = parts.next().unwrap_or(object);
		if let Some(version) = Version::parse(tag) {
			found.push((version, commit.to_string()));
		}
	}
	found.sort_by(|a, b| b.0.cmp(&a.0));
	found.dedup_by(|a, b| a.0 == b.0);
	Ok(found)
}

/// The files a dependency uses at a commit: `package.fa`, `dependencies.fa`
/// and `library/`.
fn files_at(mirror: &Path, commit: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
	let mut arguments = vec!["ls-tree", "-r", "-z", commit, "--"];
	arguments.extend(GIT_PATHS);
	let listing = git(&arguments, Some(mirror))?;
	let mut files = Vec::new();
	for record in listing.split(|&byte| byte == 0) {
		let record = String::from_utf8_lossy(record);
		let Some((meta, path)) = record.split_once('\t') else {
			continue;
		};
		let mut meta = meta.split_whitespace();
		let (_, kind, object) = (meta.next(), meta.next(), meta.next());
		if kind != Some("blob") {
			continue;
		}
		let Some(object) = object else {
			continue;
		};
		if !is_portable_path(path) {
			return Err(format!(
				"the file `{}` has a path that is not portable (a backslash, a colon, `.` or `..`)",
				path.escape_debug()
			));
		}
		let bytes = git(&["cat-file", "blob", object], Some(mirror))?;
		files.push((path.to_string(), bytes));
	}
	files.sort();
	Ok(files)
}

// ----------------------------------------------------------------- remote

/// Fetches a URL: `file://` from the disk, `http(s)://` with `curl`.
fn fetch(url: &str) -> Result<Vec<u8>, String> {
	let failed = |error: String| {
		format!(
			"error[DEP006]: can't fetch `{url}`: {error}\n  help: check the network and the source"
		)
	};
	if let Some(path) = url.strip_prefix("file://") {
		return fs::read(path).map_err(|error| failed(error.to_string()));
	}
	let output = Command::new("curl")
		.args([
			"--fail",
			"--silent",
			"--show-error",
			"--location",
			"--max-time",
			"60",
		])
		.arg(url)
		.output()
		.map_err(|error| failed(format!("can't run `curl`: {error}")))?;
	if output.status.success() {
		Ok(output.stdout)
	} else {
		Err(failed(
			String::from_utf8_lossy(&output.stderr).trim().to_string(),
		))
	}
}

/// The file name of a remote file's URL (`types.fa`).
fn file_name(url: &str) -> String {
	let path = url.split(['?', '#']).next().unwrap_or(url);
	let name = path.rsplit('/').next().unwrap_or("module.fa");
	if name.ends_with(".fa") {
		name.to_string()
	} else {
		format!("{name}.fa")
	}
}

// ------------------------------------------------------------- resolution

/// Who asks for dependencies: a package (with the workspace's shared
/// dependencies merged in), or a library.
#[derive(Clone, Debug)]
pub(crate) struct Requester {
	/// How it is named in messages (`api`, `github:farmers/farm@4.2.1`).
	pub name: String,
	/// Its folder: relative paths start there.
	pub directory: PathBuf,
	/// Its dependencies: key → source.
	pub dependencies: BTreeMap<String, Source>,
}

/// What `fa` resolves for: a workspace, or a package outside a workspace.
#[derive(Clone, Debug)]
pub(crate) struct Scope {
	/// The folder of the lockfile: the workspace, or the package.
	pub root: PathBuf,
	/// The packages: (name, folder).
	pub packages: Vec<(String, PathBuf)>,
	/// Whether it is a workspace.
	pub workspace: bool,
}

/// Which locked versions `fa update` resolves again.
#[derive(Clone, Debug, Default)]
pub(crate) enum Update {
	/// None: the lockfile is kept.
	#[default]
	None,
	/// Every dependency.
	All,
	/// The dependencies of these keys.
	Keys(BTreeSet<String>),
}

/// What a library is.
#[derive(Clone, Debug)]
pub(crate) enum LibraryKind {
	/// A package of the workspace.
	Package,
	/// A version of a Git repository.
	Git,
	/// A remote file.
	File { name: String },
	/// A remote folder and the files of its manifest.
	Folder { files: Vec<String> },
}

/// A dependency resolved: its files and its own dependencies.
#[derive(Clone, Debug)]
pub(crate) struct Library {
	/// What it is.
	pub kind: LibraryKind,
	/// The folder of its files (a package, or the cache).
	pub directory: PathBuf,
	/// Its module prefix (`$dependencies/farm@4.2.1`).
	pub prefix: String,
	/// How it is shown in messages.
	pub shown: String,
	/// Its dependencies: key → library.
	pub dependencies: BTreeMap<String, usize>,
	/// The targets it supports, if it lists them.
	pub targets: Option<Vec<String>>,
}

/// The dependencies of a scope, resolved.
#[derive(Clone, Debug, Default)]
pub(crate) struct Resolution {
	/// Every library.
	pub libraries: Vec<Library>,
	/// The direct dependencies of each package (by folder): key → library.
	pub packages: BTreeMap<PathBuf, BTreeMap<String, usize>>,
	/// The warnings (`DEP004`).
	pub warnings: Vec<String>,
	/// The changes to the lockfile (`fa update`): `(source, before, after)`.
	pub changes: Vec<(String, Option<String>, Option<String>)>,
}

/// What evaluating `dependencies.fa` files needs (decision "`@import`"): the
/// sources of their `@import`s, resolved first with the same lockfile (their
/// entries join it) and mounted for the interpreter.
#[derive(Clone, Debug)]
struct Bootstrap {
	/// The folder of the lockfile.
	root: PathBuf,
	workspace: bool,
	locked: Vec<LockEntry>,
	update: Update,
	/// The nesting: a library whose `dependencies.fa` imports, …
	depth: u32,
	/// The entries of the lockfile of the sources resolved.
	entries: Vec<LockEntry>,
	/// Their warnings (`DEP004`).
	warnings: Vec<String>,
}

impl Bootstrap {
	/// The same lockfile and mode, one level deeper, nothing resolved yet.
	fn nested(&self) -> Bootstrap {
		Bootstrap {
			depth: self.depth + 1,
			entries: Vec::new(),
			warnings: Vec::new(),
			..self.clone()
		}
	}

	/// The dependencies a `dependencies.fa` declares (none without the file).
	fn read_declared(&mut self, file: &Path) -> Result<BTreeMap<String, Source>, String> {
		let Ok(text) = fs::read_to_string(file) else {
			return Ok(BTreeMap::new());
		};
		let (value, diagnostics) =
			configuration::evaluate_source(&text, ConfigurationKind::Dependencies);
		let shown = file.display().to_string();
		let has_object = matches!(&value, Some(ConfigValue::Object(fields))
		if fields.iter().any(|(_, value)| matches!(value, ConfigValue::Object(_))));
		if configuration::is_computed(&diagnostics) || has_object {
			return self.read_computed(file, &text);
		}
		let errors: Vec<_> = diagnostics
			.iter()
			.filter(|diagnostic| diagnostic.severity == fa_analyzer::Severity::Error)
			.cloned()
			.collect();
		if !errors.is_empty() {
			return Err(render_all(&errors, &SourceView::new(&shown, &text)));
		}
		let Some(ConfigValue::Object(fields)) = value else {
			return Ok(BTreeMap::new());
		};
		let mut declared = BTreeMap::new();
		for (key, value) in fields {
			if !fa_codebase_parser::codebase::is_identifier(&key) {
				return Err(format!(
					"{shown}: error[DEP005]: `{key}` can't name a dependency: a key is an identifier"
				));
			}
			let source = match &value {
				ConfigValue::String(text) => text.clone(),
				_ => {
					return Err(format!(
						"{shown}: error: the source of `{key}` is a string, ex: `\"github:owner/repository@4\"`"
					));
				}
			};
			let source = Source::parse(&source)
				.map_err(|message| format!("{shown}: error: `{key}`: {message}"))?;
			declared.insert(key, source);
		}
		Ok(declared)
	}

	/// The session of a `dependencies.fa` file alone, analyzed (its export
	/// computed, or holding `generate` functions): the errors, rendered.
	/// The sources it imports are resolved and mounted first.
	fn session(
		&mut self,
		file: &Path,
		text: &str,
	) -> Result<(fa_analyzer::Session, fa_analyzer::ids::FileId), String> {
		let files = vec![
			(
				"package.fa".to_string(),
				"export = { name = \"dependencies\" }\n".to_string(),
			),
			("dependencies.fa".to_string(), text.to_string()),
		];
		let main = CodebaseResolver::from_texts(&files, None);
		let shown = file.display().to_string();
		let imports = import_dependencies(&shown, &imports_of_text(text))?;
		let mut session = fa_analyzer::Session::new();
		if imports.is_empty() {
			session.set_resolver(Box::new(main));
		} else {
			session.set_resolver(Box::new(self.mount_imports(file, main, imports)?));
		}
		let Some(id) = session.file_of_module("dependencies") else {
			return Err(format!("fa: can't read `{shown}`"));
		};
		session.analyze_file(id);
		let errors: Vec<_> = session
			.diagnostics(id)
			.iter()
			.filter(|diagnostic| diagnostic.severity == fa_analyzer::Severity::Error)
			.cloned()
			.collect();
		if !errors.is_empty() {
			return Err(render_all(&errors, &SourceView::new(&shown, text)));
		}
		Ok((session, id))
	}

	/// The dependencies of a `dependencies.fa` file that the compile-time
	/// interpreter evaluates: a computed export, or `DependencySource` objects.
	fn read_computed(
		&mut self,
		file: &Path,
		text: &str,
	) -> Result<BTreeMap<String, Source>, String> {
		let shown = file.display().to_string();
		let (mut session, id) = self.session(file, text)?;
		let sources = dependency_sources(&mut session, id)
			.map_err(|message| format!("{shown}: error: {message}"))?;
		let mut declared = BTreeMap::new();
		for (key, source) in sources {
			if !fa_codebase_parser::codebase::is_identifier(&key) {
				return Err(format!(
					"{shown}: error[DEP005]: `{key}` can't name a dependency: a key is an identifier"
				));
			}
			let source = match source {
				DependencySource::Source(text) => Source::parse(&text)
					.map_err(|message| format!("{shown}: error: `{key}`: {message}"))?,
				DependencySource::Generated { source, .. } => match Source::parse(&source) {
					Ok(Source::Remote { url, folder: false }) => Source::Generated {
						url,
						file: file.to_path_buf(),
						key: key.clone(),
					},
					_ => {
						return Err(format!(
							"{shown}: error: the `source` of `{key}` is the URL of a file its `generate` reads, ex: `\"https://api.example.com/openapi.json\"`"
						));
					}
				},
			};
			declared.insert(key, source);
		}
		Ok(declared)
	}

	/// The module system of a `dependencies.fa` file whose `@import`s are
	/// resolved (`imports`: their keys, `fa_analyzer::scan::import_name`).
	fn mount_imports(
		&mut self,
		file: &Path,
		main: CodebaseResolver,
		imports: BTreeMap<String, Source>,
	) -> Result<MountedResolver, String> {
		if self.depth > 4 {
			return Err(format!(
				"error: the `@import`s of `{}` import too deep",
				file.display()
			));
		}
		let directory = file.parent().unwrap_or(Path::new(".")).to_path_buf();
		let requester = Requester {
			name: file.display().to_string(),
			directory: directory.clone(),
			dependencies: imports,
		};
		let scope = Scope {
			root: self.root.clone(),
			packages: Vec::new(),
			workspace: self.workspace,
		};
		let mut inner = Resolver::new(&scope, self.nested(), vec![requester.clone()]);
		inner.resolve_git()?;
		let map = inner.key_map(&requester)?;
		inner.resolution.packages.insert(directory.clone(), map);
		self.entries.extend(inner.lock_entries());
		self.entries.extend(inner.bootstrap.entries.clone());
		self.warnings.extend(inner.resolution.warnings.clone());
		self.warnings.extend(inner.bootstrap.warnings.clone());
		// compile-time code: no target to check
		mount(main, &directory, &inner.resolution, "")
	}
}

/// The sources of the `@import("…")` calls of a source, sorted.
fn imports_of_text(text: &str) -> Vec<String> {
	if !text.contains("@import") {
		return Vec::new();
	}
	let tree = fa_parser::parse(text);
	let mut found: Vec<String> = (0..tree.nodes.len())
		.filter_map(|index| {
			fa_analyzer::scan::import_source(&tree, fa_analyzer::ids::NodeId::new(index))
				.map(str::to_string)
		})
		.collect();
	found.sort();
	found.dedup();
	found
}

/// The `@import` sources as dependencies, under the keys the analyzer
/// looks them up with; a local path is `META010`.
fn import_dependencies(
	shown: &str,
	sources: &[String],
) -> Result<BTreeMap<String, Source>, String> {
	let mut found = BTreeMap::new();
	for source in sources {
		let parsed = match Source::parse(source) {
			Ok(Source::Path(_)) => {
				return Err(format!(
					"{shown}: error[META010]: `@import(\"{source}\")` names a local folder: local files are reached by their names, sibling packages through `dependencies.fa`"
				));
			}
			Ok(parsed) => parsed,
			Err(message) => return Err(format!("{shown}: error: `@import`: {message}")),
		};
		found.insert(fa_analyzer::scan::import_name(source), parsed);
	}
	Ok(found)
}

/// The `@import` sources of the files of a package (its configuration
/// files and the files of `library/`, `application/` and `scripts/`).
fn package_imports(directory: &Path) -> Result<BTreeMap<String, Source>, String> {
	fn walk(folder: &Path, recursive: bool, files: &mut Vec<PathBuf>) {
		let Ok(entries) = fs::read_dir(folder) else {
			return;
		};
		for entry in entries.flatten() {
			let name = entry.file_name().to_string_lossy().to_string();
			if name.starts_with('.') {
				continue;
			}
			let path = entry.path();
			// a symbolic link to a folder is not followed (no loop)
			let Ok(kind) = entry.file_type() else {
				continue;
			};
			if kind.is_dir() {
				if recursive && !fa_codebase_parser::SKIPPED_FOLDERS.contains(&name.as_str()) {
					walk(&path, true, files);
				}
			} else if name.ends_with(".fa") {
				files.push(path);
			}
		}
	}
	let mut files = Vec::new();
	walk(directory, false, &mut files);
	for root in ["library", "application", "scripts"] {
		walk(&directory.join(root), true, &mut files);
	}
	files.sort();
	let mut sources = Vec::new();
	for file in &files {
		if let Ok(text) = fs::read_to_string(file) {
			sources.extend(imports_of_text(&text));
		}
	}
	sources.sort();
	sources.dedup();
	import_dependencies(&directory.display().to_string(), &sources)
}

/// A request for a Git repository.
#[derive(Clone, Debug)]
struct GitRequest {
	repository: String,
	url: String,
	range: Option<Range>,
	requester: String,
}

/// Resolves the dependencies of a scope, reading and writing its lockfile
/// (unless `write` is false).
pub(crate) fn resolve(scope: &Scope, update: &Update, write: bool) -> Result<Resolution, String> {
	let lock_path = scope.root.join(LOCKFILE);
	let locked = read_lockfile(&lock_path)?;
	let mut bootstrap = Bootstrap {
		root: scope.root.clone(),
		workspace: scope.workspace,
		locked,
		update: update.clone(),
		depth: 0,
		entries: Vec::new(),
		warnings: Vec::new(),
	};
	let shared = if scope.workspace {
		let file = scope.root.join("dependencies.fa");
		let mut shared = bootstrap.read_declared(&file)?;
		let text = fs::read_to_string(&file).unwrap_or_default();
		shared.extend(import_dependencies(
			&file.display().to_string(),
			&imports_of_text(&text),
		)?);
		shared
	} else {
		BTreeMap::new()
	};
	let mut requesters = Vec::new();
	for (name, directory) in &scope.packages {
		let mut dependencies = shared.clone();
		// a key of the package replaces the same key of the workspace
		dependencies.extend(bootstrap.read_declared(&directory.join("dependencies.fa"))?);
		// the sources of its `@import`s
		dependencies.extend(package_imports(directory)?);
		requesters.push(Requester {
			name: name.clone(),
			directory: directory.clone(),
			dependencies,
		});
	}
	let own = requesters.len();
	if !scope.workspace {
		add_path_requesters(&mut requesters, &mut bootstrap)?;
	}
	let mut resolver = Resolver::new(scope, bootstrap, requesters.clone());
	// the keys `fa update` names must exist (an `@import`: its source)
	if let Update::Keys(keys) = update {
		for key in keys {
			if !requesters.iter().any(|requester| {
				requester.dependencies.contains_key(key)
					|| requester
						.dependencies
						.contains_key(&fa_analyzer::scan::import_name(key))
			}) {
				return Err(format!(
					"error: no dependency is named `{key}` in `dependencies.fa`"
				));
			}
		}
	}
	resolver.resolve_git()?;
	// the libraries, then the key maps
	for requester in &requesters[..own] {
		let map = resolver.key_map(requester)?;
		resolver
			.resolution
			.packages
			.insert(requester.directory.clone(), map);
	}
	let mut entries = resolver.lock_entries();
	entries.extend(resolver.bootstrap.entries.clone());
	entries.sort_by(|a, b| a.source.cmp(&b.source));
	entries.dedup();
	let warnings = std::mem::take(&mut resolver.bootstrap.warnings);
	for warning in warnings {
		if !resolver.resolution.warnings.contains(&warning) {
			resolver.resolution.warnings.push(warning);
		}
	}
	resolver.resolution.changes = lock_changes(&resolver.locked, &entries);
	if write {
		let text = render_lockfile(&entries);
		let previous = fs::read_to_string(&lock_path).ok();
		let needed = match &previous {
			Some(previous) => *previous != text,
			None => !entries.is_empty(),
		};
		if needed {
			fs::write(&lock_path, text).map_err(|error| {
				format!("error: can't write `{}`: {error}", lock_path.display())
			})?;
		}
	}
	Ok(resolver.resolution)
}

/// The resolutions shared: (scope root, lockfile written, result).
type Shared = Option<Vec<(PathBuf, bool, Result<Resolution, String>)>>;

/// The resolutions of a command, shared by its packages and targets: on
/// while a [`Sharing`] lives.
static SHARED: Mutex<Shared> = Mutex::new(None);

/// While it lives, [`resolve_shared`] resolves each scope once: a command
/// that checks or builds several packages or targets (never a watcher,
/// which must see the changes) reads every file once.
pub(crate) struct Sharing;

impl Drop for Sharing {
	fn drop(&mut self) {
		*SHARED.lock().unwrap_or_else(|error| error.into_inner()) = None;
	}
}

/// Shares the resolutions until the result is dropped.
pub(crate) fn share_resolutions() -> Sharing {
	*SHARED.lock().unwrap_or_else(|error| error.into_inner()) = Some(Vec::new());
	Sharing
}

/// [`resolve`] without update, once per scope while a [`Sharing`] lives.
pub(crate) fn resolve_shared(scope: &Scope, write: bool) -> Result<Resolution, String> {
	let known = |shared: &Shared| {
		shared.as_ref().and_then(|shared| {
			shared
				.iter()
				.find(|(root, written, _)| *root == scope.root && *written == write)
				.map(|(_, _, result)| result.clone())
		})
	};
	if let Some(result) = known(&SHARED.lock().unwrap_or_else(|error| error.into_inner())) {
		return result;
	}
	let result = resolve(scope, &Update::None, write);
	if let Some(shared) = SHARED
		.lock()
		.unwrap_or_else(|error| error.into_inner())
		.as_mut()
	{
		shared.push((scope.root.clone(), write, result.clone()));
	}
	result
}

/// The changes from a lockfile to another, `(source, before, after)`: an
/// entry is the same dependency in both when it has the same source and,
/// for a repository, the same major version.
fn lock_changes(
	before: &[LockEntry],
	after: &[LockEntry],
) -> Vec<(String, Option<String>, Option<String>)> {
	let key = |entry: &'_ LockEntry| -> (String, Option<u64>) {
		let major = entry
			.version
			.as_deref()
			.and_then(Version::parse)
			.map(|version| version.major);
		(entry.source.clone(), major)
	};
	let shown = |entry: &LockEntry| entry.version.clone().unwrap_or_else(|| entry.hash.clone());
	let after_by_key: BTreeMap<_, &LockEntry> = after
		.iter()
		.rev()
		.map(|entry| (key(entry), entry))
		.collect();
	let before_keys: BTreeSet<_> = before.iter().map(key).collect();
	let after_set: BTreeSet<&LockEntry> = after.iter().collect();
	let mut changes = Vec::new();
	for entry in before {
		if !after_set.contains(entry) {
			changes.push((
				entry.source.clone(),
				Some(shown(entry)),
				after_by_key.get(&key(entry)).map(|other| shown(other)),
			));
		}
	}
	for entry in after {
		if !before_keys.contains(&key(entry)) {
			changes.push((entry.source.clone(), None, Some(shown(entry))));
		}
	}
	changes
}

/// A chosen version of a repository (for one major version).
#[derive(Clone, Debug)]
struct Chosen {
	version: Version,
	commit: String,
	hash: String,
	directory: PathBuf,
	/// Its own dependencies.
	declared: BTreeMap<String, Source>,
	/// Its library, once created.
	library: Option<usize>,
}

/// A remote file or folder, fetched.
#[derive(Clone, Debug)]
struct Fetched {
	hash: String,
	library: Option<usize>,
}

struct Resolver<'s> {
	scope: &'s Scope,
	locked: Vec<LockEntry>,
	update: Update,
	/// The evaluation of `dependencies.fa` files and their `@import`s.
	bootstrap: Bootstrap,
	resolution: Resolution,
	/// The library of each package of the scope that is a dependency.
	packages: BTreeMap<PathBuf, usize>,
	/// The chosen version of each (repository, major).
	chosen: BTreeMap<(String, u64), Chosen>,
	/// The versions materialized, by (repository, version, commit).
	materialized: BTreeMap<(String, String, String), Chosen>,
	/// The remote files and folders, by URL.
	remote: BTreeMap<String, Fetched>,
	/// The libraries generated by a `DependencySource`, by declaring file
	/// and key.
	generated: BTreeMap<(PathBuf, String), usize>,
	/// The packages outside a workspace that a path names, by their path
	/// from the lockfile's folder (locked by their content).
	paths: BTreeMap<String, Fetched>,
	/// The mirror of each repository URL.
	mirrors: BTreeMap<String, PathBuf>,
	/// The repositories whose mirror was fetched again in this run.
	refreshed: BTreeSet<String>,
	/// The packages of the scope and their dependencies.
	requesters: Vec<Requester>,
}

impl<'s> Resolver<'s> {
	/// A resolver of a scope's requesters, with the lockfile and the mode
	/// of `bootstrap`.
	fn new(scope: &'s Scope, bootstrap: Bootstrap, requesters: Vec<Requester>) -> Self {
		Resolver {
			scope,
			locked: bootstrap.locked.clone(),
			update: bootstrap.update.clone(),
			bootstrap,
			resolution: Resolution::default(),
			packages: BTreeMap::new(),
			chosen: BTreeMap::new(),
			materialized: BTreeMap::new(),
			remote: BTreeMap::new(),
			paths: BTreeMap::new(),
			generated: BTreeMap::new(),
			mirrors: BTreeMap::new(),
			refreshed: BTreeSet::new(),
			requesters,
		}
	}

	/// Whether `fa update` resolves a locked source again.
	fn updating(&self, source: &str) -> bool {
		match &self.update {
			Update::None => false,
			Update::All => true,
			Update::Keys(keys) => self.requesters.iter().any(|requester| {
				requester.dependencies.iter().any(|(key, declared)| {
					let named = keys.contains(key)
						|| key
							.strip_prefix("@import ")
							.is_some_and(|imported| keys.contains(imported));
					named && declared.locked() == Some(source)
				})
			}),
		}
	}

	/// The key a package names a source with, in messages (else the
	/// source itself).
	fn key_of(&self, source: &str) -> String {
		self.requesters
			.iter()
			.flat_map(|requester| &requester.dependencies)
			.find(|(key, declared)| {
				!key.starts_with("@import ") && declared.locked() == Some(source)
			})
			.map_or_else(|| source.to_string(), |(key, _)| key.clone())
	}

	/// The mirror of a repository, fetched again at most once per run.
	fn mirror(&mut self, url: &str, refresh: bool) -> Result<PathBuf, String> {
		if let Some(folder) = self.mirrors.get(url)
			&& (!refresh || self.refreshed.contains(url))
		{
			return Ok(folder.clone());
		}
		let folder = mirror(url, refresh)?;
		if refresh {
			self.refreshed.insert(url.to_string());
		}
		self.mirrors.insert(url.to_string(), folder.clone());
		Ok(folder)
	}

	/// Chooses a version of every repository asked for, transitively, until
	/// no choice changes.
	fn resolve_git(&mut self) -> Result<(), String> {
		let requesters = self.requesters.clone();
		for requester in &requesters {
			check_requester(requester)?;
		}
		for _ in 0..32 {
			let mut requests: Vec<GitRequest> = Vec::new();
			for requester in &requesters {
				for source in requester.dependencies.values() {
					if let Source::Git {
						repository,
						url,
						range,
					} = source
					{
						requests.push(GitRequest {
							repository: repository.clone(),
							url: url.clone(),
							range: range.clone(),
							requester: format!("`{}`", requester.name),
						});
					}
				}
			}
			for ((repository, _), chosen) in &self.chosen {
				for (key, source) in &chosen.declared {
					if !matches!(source, Source::Path(_)) {
						check_fetched_source(
							&format!("{repository}@{}", chosen.version),
							key,
							source,
						)?;
					}
					match source {
						Source::Git {
							repository: other,
							url,
							range,
						} => requests.push(GitRequest {
							repository: other.clone(),
							url: url.clone(),
							range: range.clone(),
							requester: format!("`{repository}@{}`", chosen.version),
						}),
						Source::Path(path) => {
							return Err(format!(
								"error[DEP008]: `{repository}@{}` depends on `{key}` by a path (`{path}`): a library of a repository can only depend on remote sources",
								chosen.version
							));
						}
						Source::Remote { .. } | Source::Generated { .. } => {}
					}
				}
			}
			// the groups: (repository, major) → requests
			let mut groups: BTreeMap<(String, u64), Vec<GitRequest>> = BTreeMap::new();
			for request in requests {
				let major = match &request.range {
					Some(range) => range.major,
					None => self.latest_major(&request)?,
				};
				groups
					.entry((request.repository.clone(), major))
					.or_default()
					.push(request);
			}
			let mut next = BTreeMap::new();
			for ((repository, major), group) in &groups {
				let chosen = self.choose(repository, *major, group)?;
				next.insert((repository.clone(), *major), chosen);
			}
			let same = next.len() == self.chosen.len()
				&& next.iter().all(|(key, chosen)| {
					self.chosen
						.get(key)
						.is_some_and(|other| other.version == chosen.version)
				});
			self.chosen = next;
			if same {
				return Ok(());
			}
		}
		Err("error: the versions of the dependencies don't settle: check the ranges of `dependencies.fa`".to_string())
	}

	/// The latest major version of an unversioned repository (its locked
	/// one, unless updated).
	fn latest_major(&mut self, request: &GitRequest) -> Result<u64, String> {
		let warning = format!(
			"warning[DEP004]: `{}` has no version range: {} gets its latest version, which can change; write `{}@<major>`",
			request.repository, request.requester, request.repository
		);
		if !self.resolution.warnings.contains(&warning) {
			self.resolution.warnings.push(warning);
		}
		if !self.updating(&request.repository)
			&& let Some(version) = self
				.locked
				.iter()
				.filter(|entry| entry.source == request.repository)
				.filter_map(|entry| entry.version.as_deref().and_then(Version::parse))
				.max()
		{
			return Ok(version.major);
		}
		let mirror = self.mirror(&request.url, true)?;
		tags(&mirror)?
			.into_iter()
			.find(|(version, _)| version.pre.is_none())
			.map(|(version, _)| version.major)
			.ok_or_else(|| {
				format!(
					"error[DEP002]: `{}` has no version: tag a commit with a semantic version, ex: `1.0.0`",
					request.repository
				)
			})
	}

	/// Chooses the version of a repository for a major version: the locked
	/// one when it satisfies every range, else the latest that does.
	fn choose(
		&mut self,
		repository: &str,
		major: u64,
		group: &[GitRequest],
	) -> Result<Chosen, String> {
		let url = group[0].url.clone();
		let satisfies = |version: &Version| {
			version.major == major
				&& group.iter().all(|request| {
					request
						.range
						.as_ref()
						.is_none_or(|range| range.matches(version))
				}) && (version.pre.is_none()
				|| group.iter().any(|request| {
					request
						.range
						.as_ref()
						.is_some_and(|range| range.pre.is_some())
				}))
		};
		let updating = self.updating(repository);
		if !updating
			&& let Some(entry) = self.locked.iter().find(|entry| {
				entry.source == repository
					&& entry
						.version
						.as_deref()
						.and_then(Version::parse)
						.is_some_and(|version| satisfies(&version))
			}) && let (Some(version), Some(commit)) = (
			entry.version.as_deref().and_then(Version::parse),
			entry.commit.clone(),
		) {
			let hash = entry.hash.clone();
			return self.materialize(repository, &url, version, commit, Some(hash));
		}
		// the tags: the mirror as it is, then fetched again
		let mut mirror = self.mirror(&url, updating)?;
		let mut candidates = tags(&mirror)?;
		if !candidates.iter().any(|(version, _)| satisfies(version)) && !updating {
			mirror = self.mirror(&url, true)?;
			candidates = tags(&mirror)?;
		}
		let Some((version, commit)) = candidates
			.into_iter()
			.find(|(version, _)| satisfies(version))
		else {
			let mut asked = String::new();
			for request in group {
				let range = request
					.range
					.as_ref()
					.map_or("any version".to_string(), |range| format!("@{range}"));
				asked.push_str(&format!(
					"\n  {range} is requested by {}",
					request.requester
				));
			}
			return Err(format!(
				"error[DEP002]: no version {major}.x of `{repository}` satisfies every range:{asked}\n  help: align the ranges of `dependencies.fa`"
			));
		};
		// a version locked with another commit keeps its hash (DEP001)
		let locked_hash = (!updating)
			.then(|| {
				self.locked
					.iter()
					.find(|entry| {
						entry.source == repository
							&& entry.version.as_deref() == Some(&version.to_string())
					})
					.map(|entry| entry.hash.clone())
			})
			.flatten();
		self.materialize(repository, &url, version, commit, locked_hash)
	}

	/// The files of a version: from the cache when its hash is there, else
	/// extracted from the mirror and checked against the locked hash.
	fn materialize(
		&mut self,
		repository: &str,
		url: &str,
		version: Version,
		commit: String,
		locked_hash: Option<String>,
	) -> Result<Chosen, String> {
		// a version already materialized in this run (each iteration of
		// `resolve_git` chooses every group again)
		let id = (repository.to_string(), version.to_string(), commit.clone());
		if let Some(chosen) = self.materialized.get(&id) {
			return Ok(Chosen {
				library: None,
				..chosen.clone()
			});
		}
		let chosen = self.materialize_new(repository, url, version, commit, locked_hash)?;
		self.materialized.insert(id, chosen.clone());
		Ok(chosen)
	}

	/// [`Self::materialize`] of a version not materialized yet.
	fn materialize_new(
		&mut self,
		repository: &str,
		url: &str,
		version: Version,
		commit: String,
		locked_hash: Option<String>,
	) -> Result<Chosen, String> {
		let found = |bootstrap: &mut Bootstrap,
		             hash: String,
		             directory: PathBuf|
		 -> Result<Chosen, String> {
			let mut declared = bootstrap.read_declared(&directory.join("dependencies.fa"))?;
			declared.extend(package_imports(&directory)?);
			Ok(Chosen {
				version: version.clone(),
				commit: commit.clone(),
				hash,
				directory,
				declared,
				library: None,
			})
		};
		if let Some(hash) = &locked_hash {
			let folder = source_folder(hash);
			if folder.is_dir() {
				if read_tree(&folder).is_ok_and(|files| hash_files(&files) == *hash) {
					return found(&mut self.bootstrap, hash.clone(), folder);
				}
				// a damaged entry of the cache
				remove_cached(&folder);
			}
		}
		let mut mirror = self.mirror(url, false)?;
		let mut files = files_at(&mirror, &commit);
		if files.is_err() {
			mirror = self.mirror(url, true)?;
			files = files_at(&mirror, &commit);
		}
		let files = files.map_err(|error| {
			format!(
				"error[DEP006]: the commit {commit} of `{repository}` ({version}) is not in the repository: {error}\n  help: run `fa update` to resolve it again"
			)
		})?;
		let hash = hash_files(&files);
		if let Some(locked) = &locked_hash
			&& *locked != hash
		{
			let key = self.key_of(repository);
			return Err(format!(
				"error[DEP001]: the content of `{key}` changed since it was locked\n  source: {repository}@{version}\n  help: run `fa update {key}` to accept the new version"
			));
		}
		// the version of `package.fa` is the version of the tag (DEP003)
		let declared_version = files
			.iter()
			.find(|(path, _)| path == "package.fa")
			.and_then(|(_, bytes)| {
				let text = String::from_utf8_lossy(bytes);
				configuration::evaluate_source(&text, ConfigurationKind::Package)
					.0
					.and_then(|value| {
						value
							.field("version")
							.and_then(ConfigValue::as_str)
							.map(str::to_string)
					})
			});
		if declared_version.as_deref().and_then(Version::parse) != Some(version.clone()) {
			return Err(format!(
				"error[DEP003]: the version {version} of `{repository}` is rejected: the `version` of its `package.fa` is {}\n  help: tag the commit whose `package.fa` says `version = \"{version}\"`",
				declared_version.map_or("missing".to_string(), |text| format!("`{text}`"))
			));
		}
		let directory = store(&hash, &files)?;
		found(&mut self.bootstrap, hash, directory)
	}

	/// A remote file or folder: from the cache when locked, else fetched
	/// and checked against the locked hash.
	fn fetch_remote(&mut self, url: &str, folder: bool) -> Result<usize, String> {
		if let Some(fetched) = self.remote.get(url)
			&& let Some(library) = fetched.library
		{
			return Ok(library);
		}
		if !is_versioned(url) {
			self.resolution.warnings.push(format!(
				"warning[DEP004]: `{url}` has no version: its content can change, and the build then fails; prefer a URL with a version"
			));
		}
		let updating = self.updating(url);
		let locked = self
			.locked
			.iter()
			.find(|entry| entry.source == url)
			.map(|entry| entry.hash.clone())
			.filter(|_| !updating);
		let name = file_name(url);
		let mut cached = None;
		if let Some(hash) = &locked {
			let directory = source_folder(hash);
			if let Ok(files) = read_tree(&directory) {
				let actual = if folder {
					hash_files(&files)
				} else {
					files
						.first()
						.map(|(_, bytes)| format!("sha256:{}", sha256::hex(bytes)))
						.unwrap_or_default()
				};
				if actual == *hash {
					let listed: Vec<String> = files
						.iter()
						.map(|(path, _)| path.clone())
						.filter(|path| path != "manifest.fa")
						.collect();
					cached = Some((hash.clone(), directory, listed));
				} else {
					remove_cached(&directory);
				}
			}
		}
		let (hash, directory, listed) = match cached {
			Some(found) => found,
			None => {
				let (hash, files, listed) = if folder {
					let manifest = fetch(&format!("{url}manifest.fa")).map_err(|_| {
						format!(
							"error[DEP007]: the remote folder `{url}` has no `manifest.fa`\n  help: serve `{url}manifest.fa`, exporting the paths of its files: `export = [\"button.fa\"]`"
						)
					})?;
					let text = String::from_utf8_lossy(&manifest).to_string();
					let paths = configuration::evaluate_literal(&text)
						.and_then(|value| value.as_strings())
						.ok_or_else(|| {
							format!(
								"error[DEP007]: the `manifest.fa` of `{url}` doesn't export a list of paths, ex: `export = [\"button.fa\"]`"
							)
						})?;
					let mut files = vec![("manifest.fa".to_string(), manifest)];
					for path in &paths {
						if !is_portable_path(path) || path == "manifest.fa" {
							return Err(format!(
								"error[DEP007]: the `manifest.fa` of `{url}` lists `{}`, which is outside the folder\n  help: list relative paths inside the folder, with `/` separators",
								path.escape_debug()
							));
						}
						files.push((path.clone(), fetch(&format!("{url}{path}"))?));
					}
					(hash_files(&files), files, paths)
				} else {
					let bytes = fetch(url)?;
					let hash = format!("sha256:{}", sha256::hex(&bytes));
					(hash, vec![(name.clone(), bytes)], vec![name.clone()])
				};
				if let Some(locked) = &locked
					&& *locked != hash
				{
					let key = self.key_of(url);
					return Err(format!(
						"error[DEP001]: the content of `{key}` changed since it was locked\n  source: {url}\n  help: run `fa update {key}` to accept the new version"
					));
				}
				let directory = store(&hash, &files)?;
				(hash, directory, listed)
			}
		};
		let kind = if folder {
			LibraryKind::Folder { files: listed }
		} else {
			LibraryKind::File { name }
		};
		let index = self.add_library(Library {
			kind: kind.clone(),
			directory: directory.clone(),
			prefix: format!(
				"$dependencies/{}-{}",
				sanitize(
					url.trim_end_matches('/')
						.rsplit('/')
						.next()
						.unwrap_or("remote")
				),
				&hash.trim_start_matches("sha256:")[..12]
			),
			shown: url.trim_end_matches('/').to_string(),
			dependencies: BTreeMap::new(),
			targets: None,
		});
		self.remote.insert(
			url.to_string(),
			Fetched {
				hash,
				library: Some(index),
			},
		);
		// its `@import`s (a remote file or folder sees no other dependency)
		let mut sources = Vec::new();
		let files = match &kind {
			LibraryKind::File { name } => vec![name.clone()],
			LibraryKind::Folder { files } => files.clone(),
			_ => Vec::new(),
		};
		for file in &files {
			if let Ok(text) = fs::read_to_string(directory.join(file)) {
				sources.extend(imports_of_text(&text));
			}
		}
		sources.sort();
		sources.dedup();
		if !sources.is_empty() {
			let shown = url.trim_end_matches('/').to_string();
			let requester = Requester {
				name: shown.clone(),
				directory: directory.clone(),
				dependencies: import_dependencies(&shown, &sources)?,
			};
			let map = self.key_map(&requester)?;
			self.resolution.libraries[index].dependencies = map;
		}
		Ok(index)
	}

	/// Adds a library.
	fn add_library(&mut self, library: Library) -> usize {
		self.resolution.libraries.push(library);
		self.resolution.libraries.len() - 1
	}

	/// The library of a source declared by a requester.
	fn library_of(
		&mut self,
		requester: &Requester,
		key: &str,
		source: &Source,
	) -> Result<usize, String> {
		match source {
			Source::Path(path) => self.package_library(requester, key, path),
			Source::Remote { url, folder } => self.fetch_remote(url, *folder),
			Source::Generated { url, file, key } => self.generated_library(key, url, file),
			Source::Git {
				repository,
				range,
				url,
			} => {
				let chosen_major = self
					.chosen
					.keys()
					.filter(|(other, _)| other == repository)
					.map(|(_, major)| *major)
					.max();
				let request = GitRequest {
					repository: repository.clone(),
					url: url.clone(),
					range: range.clone(),
					requester: format!("`{}`", requester.name),
				};
				let major = match (range, chosen_major) {
					(Some(range), _) => range.major,
					(None, Some(major)) => major,
					(None, None) => self.latest_major(&request)?,
				};
				// a repository only an `@import` of a remote file asks for:
				// chosen now, for this request alone
				if !self.chosen.contains_key(&(repository.clone(), major)) {
					let chosen = self.choose(repository, major, &[request])?;
					self.chosen.insert((repository.clone(), major), chosen);
				}
				self.git_library(repository, major)
			}
		}
	}

	/// The library of a `DependencySource` with `generate`: its source is
	/// fetched and locked like a remote file, then `generate` runs on its
	/// content (the compile-time interpreter): the Fa source it returns is
	/// the module of the dependency, stored in the cache by its hash.
	/// `DEP010` when `generate` fails.
	fn generated_library(&mut self, key: &str, url: &str, file: &Path) -> Result<usize, String> {
		let id = (file.to_path_buf(), key.to_string());
		if let Some(&library) = self.generated.get(&id) {
			return Ok(library);
		}
		let raw = self.fetch_remote(url, false)?;
		let fetched = &self.resolution.libraries[raw];
		let LibraryKind::File { name } = &fetched.kind else {
			return Err(format!("error: `{url}` is not a file"));
		};
		let content = fs::read(fetched.directory.join(name))
			.map_err(|error| format!("error: can't read the cached content of `{url}`: {error}"))?;
		let shown = file.display().to_string();
		let failed = |message: String| {
			format!("{shown}: error[DEP010]: the `generate` of `{key}` failed: {message}")
		};
		let content =
			String::from_utf8(content).map_err(|_| failed(format!("`{url}` is not UTF-8 text")))?;
		let text = fs::read_to_string(file)
			.map_err(|error| format!("fa: can't read `{shown}`: {error}"))?;
		let (mut session, declaration) = self.bootstrap.session(file, &text)?;
		let generate = dependency_sources(&mut session, declaration)
			.map_err(|message| format!("{shown}: error: {message}"))?
			.into_iter()
			.find_map(|(other, source)| match source {
				DependencySource::Generated { generate, .. } if other == key => Some(generate),
				_ => None,
			})
			.ok_or_else(|| failed("it is not declared anymore".to_string()))?;
		let generated = run_generate(&mut session, &generate, &content).map_err(failed)?;
		let bytes = generated.into_bytes();
		let name = format!("{}.fa", sanitize(key));
		let files = [(name.clone(), bytes)];
		let hash = hash_files(&files);
		let directory = store(&hash, &files)?;
		let index = self.add_library(Library {
			kind: LibraryKind::File { name },
			directory,
			prefix: format!(
				"$dependencies/{}-{}",
				sanitize(key),
				&hash.trim_start_matches("sha256:")[..12]
			),
			shown: format!("{} (generated)", url.trim_end_matches('/')),
			dependencies: BTreeMap::new(),
			targets: None,
		});
		self.generated.insert(id, index);
		Ok(index)
	}

	/// The library of a chosen version of a repository.
	fn git_library(&mut self, repository: &str, major: u64) -> Result<usize, String> {
		let key = (repository.to_string(), major);
		let Some(chosen) = self.chosen.get(&key).cloned() else {
			return Err(format!("error: `{repository}` was not resolved"));
		};
		if let Some(library) = chosen.library {
			return Ok(library);
		}
		let name = repository
			.trim_end_matches(".git")
			.rsplit(['/', ':'])
			.next()
			.unwrap_or(repository);
		let targets = package_targets(&chosen.directory);
		let index = self.add_library(Library {
			kind: LibraryKind::Git,
			directory: chosen.directory.clone(),
			prefix: format!("$dependencies/{}@{}", sanitize(name), chosen.version),
			shown: format!("{repository}@{}", chosen.version),
			dependencies: BTreeMap::new(),
			targets,
		});
		if let Some(entry) = self.chosen.get_mut(&key) {
			entry.library = Some(index);
		}
		let requester = Requester {
			name: format!("{repository}@{}", chosen.version),
			directory: chosen.directory.clone(),
			dependencies: chosen.declared.clone(),
		};
		let map = self.key_map(&requester)?;
		self.resolution.libraries[index].dependencies = map;
		Ok(index)
	}

	/// The library of a sibling package (`DEP008` when the path is not a
	/// package of the workspace).
	fn package_library(
		&mut self,
		requester: &Requester,
		key: &str,
		path: &str,
	) -> Result<usize, String> {
		let target = fs::canonicalize(requester.directory.join(path)).ok();
		let package = target.as_ref().and_then(|target| {
			self.scope
				.packages
				.iter()
				.find(|(_, directory)| fs::canonicalize(directory).ok().as_ref() == Some(target))
				.cloned()
		});
		let Some((name, directory)) = package else {
			// outside a workspace: a folder holding a package, locked by its content
			if !self.scope.workspace
				&& let Some(target) = target
				&& target.join("package.fa").is_file()
			{
				return self.path_library(key, path, target);
			}
			let what = if self.scope.workspace {
				"a package of the workspace"
			} else {
				"a folder holding a package (a `package.fa`)"
			};
			return Err(format!(
				"{}: error[DEP008]: `{key}` = \"{path}\" is not {what}",
				requester.directory.join("dependencies.fa").display()
			));
		};
		if let Some(&library) = self.packages.get(&directory) {
			return Ok(library);
		}
		let index = self.add_library(Library {
			kind: LibraryKind::Package,
			directory: directory.clone(),
			prefix: format!("$dependencies/{}", sanitize(&name)),
			shown: shown_path(&directory),
			dependencies: BTreeMap::new(),
			targets: package_targets(&directory),
		});
		self.packages.insert(directory.clone(), index);
		let Some(own) = self
			.requesters
			.iter()
			.find(|other| other.directory == directory)
			.cloned()
		else {
			return Ok(index);
		};
		let map = self.key_map(&own)?;
		self.resolution.libraries[index].dependencies = map;
		Ok(index)
	}

	/// The library of a package outside a workspace that a path names
	/// (decision "Path dependencies outside a workspace"): built from its
	/// folder, locked by the hash of its `package.fa`, `dependencies.fa` and
	/// `library/` (`DEP001` when they changed since, until `fa update`).
	fn path_library(&mut self, key: &str, path: &str, directory: PathBuf) -> Result<usize, String> {
		let source = relative_path(&self.scope.root, &directory);
		if let Some(fetched) = self.paths.get(&source)
			&& let Some(library) = fetched.library
		{
			return Ok(library);
		}
		let files = package_files(&directory).map_err(|error| {
			format!(
				"error: can't read the dependency `{key}` (`{path}`): {error}\n  help: check the path in `dependencies.fa`"
			)
		})?;
		let hash = hash_files(&files);
		let updating = self.updating(&source) || self.updating(path);
		if !updating
			&& let Some(entry) = self.locked.iter().find(|entry| entry.source == source)
			&& entry.hash != hash
		{
			return Err(format!(
				"error[DEP001]: the content of `{key}` changed since it was locked\n  source: {source}\n  help: run `fa update {key}` to accept the new version"
			));
		}
		let index = self.add_library(Library {
			kind: LibraryKind::Package,
			directory: directory.clone(),
			prefix: format!("$dependencies/{}", sanitize(&package_name(&directory))),
			shown: shown_path(&directory),
			dependencies: BTreeMap::new(),
			targets: package_targets(&directory),
		});
		self.paths.insert(
			source,
			Fetched {
				hash,
				library: Some(index),
			},
		);
		self.packages.insert(directory.clone(), index);
		if let Some(own) = self
			.requesters
			.iter()
			.find(|other| other.directory == directory)
			.cloned()
		{
			let map = self.key_map(&own)?;
			self.resolution.libraries[index].dependencies = map;
		}
		Ok(index)
	}

	/// The libraries of a requester's keys.
	fn key_map(&mut self, requester: &Requester) -> Result<BTreeMap<String, usize>, String> {
		check_requester(requester)?;
		let mut map = BTreeMap::new();
		for (key, source) in &requester.dependencies {
			let library = self.library_of(requester, key, source)?;
			map.insert(key.clone(), library);
		}
		Ok(map)
	}

	/// The entries of the lockfile: the chosen versions and the remote
	/// sources.
	fn lock_entries(&self) -> Vec<LockEntry> {
		let mut entries: Vec<LockEntry> = self
			.chosen
			.iter()
			.map(|((repository, _), chosen)| LockEntry {
				source: repository.clone(),
				version: Some(chosen.version.to_string()),
				commit: Some(chosen.commit.clone()),
				hash: chosen.hash.clone(),
			})
			.collect();
		for (path, fetched) in &self.paths {
			entries.push(LockEntry {
				source: path.clone(),
				version: None,
				commit: None,
				hash: fetched.hash.clone(),
			});
		}
		for (url, fetched) in &self.remote {
			entries.push(LockEntry {
				source: url.clone(),
				version: None,
				commit: None,
				hash: fetched.hash.clone(),
			});
		}
		entries.sort_by(|a, b| a.source.cmp(&b.source));
		entries
	}
}

/// Whether a folder holds a dependency fetched from Git or a remote
/// source: it is inside the cache.
fn is_fetched(directory: &Path) -> bool {
	let root = cache_root();
	let root = fs::canonicalize(&root).unwrap_or(root);
	let directory = fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf());
	directory.starts_with(root)
}

/// `DEP008` when a requester fetched from Git or a remote source (or the
/// `dependencies.fa` of one) declares a source on the local disk.
fn check_requester(requester: &Requester) -> Result<(), String> {
	if !is_fetched(&requester.directory) {
		return Ok(());
	}
	for (key, source) in &requester.dependencies {
		check_fetched_source(&requester.name, key, source)?;
	}
	Ok(())
}

/// Adds the packages that the path dependencies of a package outside a
/// workspace name, transitively, as requesters: their Git dependencies are
/// resolved with the others.
fn add_path_requesters(
	requesters: &mut Vec<Requester>,
	bootstrap: &mut Bootstrap,
) -> Result<(), String> {
	let mut index = 0;
	while index < requesters.len() {
		let requester = requesters[index].clone();
		index += 1;
		for source in requester.dependencies.values() {
			let Source::Path(path) = source else {
				continue;
			};
			// a path that names no package is `DEP008`, reported by `package_library`
			let Ok(directory) = fs::canonicalize(requester.directory.join(path)) else {
				continue;
			};
			let known = requesters.iter().any(|other| {
				fs::canonicalize(&other.directory).is_ok_and(|other| other == directory)
			});
			if known || !directory.join("package.fa").is_file() {
				continue;
			}
			let mut dependencies = bootstrap.read_declared(&directory.join("dependencies.fa"))?;
			dependencies.extend(package_imports(&directory)?);
			requesters.push(Requester {
				name: shown_path(&directory),
				dependencies,
				directory,
			});
		}
	}
	Ok(())
}

/// The files of a package that a dependency uses, by relative path:
/// `package.fa`, `dependencies.fa` and `library/`.
fn package_files(directory: &Path) -> std::io::Result<Vec<(String, Vec<u8>)>> {
	let mut files = Vec::new();
	for name in ["package.fa", "dependencies.fa"] {
		match fs::read(directory.join(name)) {
			Ok(bytes) => files.push((name.to_string(), bytes)),
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
			Err(error) => return Err(error),
		}
	}
	let library = directory.join("library");
	if library.is_dir() {
		for (path, bytes) in read_tree(&library)? {
			files.push((format!("library/{path}"), bytes));
		}
	}
	files.sort();
	Ok(files)
}

/// The `name` of a package's `package.fa`, else the name of its folder.
fn package_name(directory: &Path) -> String {
	fs::read_to_string(directory.join("package.fa"))
		.ok()
		.and_then(|text| {
			configuration::evaluate_source(&text, ConfigurationKind::Package)
				.0?
				.field("name")?
				.as_str()
				.map(str::to_string)
		})
		.or_else(|| {
			directory
				.file_name()
				.map(|name| name.to_string_lossy().to_string())
		})
		.unwrap_or_else(|| "package".to_string())
}

/// The path from a folder to another, `/` separated (`../shared`).
fn relative_path(from: &Path, to: &Path) -> String {
	let from = fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf());
	let from: Vec<_> = from.components().collect();
	let to: Vec<_> = to.components().collect();
	let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
	let mut parts: Vec<String> = vec!["..".to_string(); from.len() - common];
	parts.extend(
		to[common..]
			.iter()
			.map(|part| part.as_os_str().to_string_lossy().to_string()),
	);
	match parts.first().map(String::as_str) {
		None => ".".to_string(),
		Some("..") => parts.join("/"),
		Some(_) => format!("./{}", parts.join("/")),
	}
}

/// The targets a package lists in its `package.fa`, if literal.
fn package_targets(directory: &Path) -> Option<Vec<String>> {
	let text = fs::read_to_string(directory.join("package.fa")).ok()?;
	configuration::evaluate_source(&text, ConfigurationKind::Package)
		.0?
		.field("targets")?
		.as_strings()
}

/// A path shown from the current folder when it is inside it.
pub(crate) fn shown_path(path: &Path) -> String {
	let current = std::env::current_dir()
		.ok()
		.and_then(|directory| fs::canonicalize(directory).ok());
	let absolute = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
	match current {
		Some(current) => match absolute.strip_prefix(&current) {
			Ok(relative) if relative.as_os_str().is_empty() => ".".to_string(),
			Ok(relative) => relative.display().to_string(),
			Err(_) => absolute.display().to_string(),
		},
		None => absolute.display().to_string(),
	}
}

/// A name usable in a module path.
fn sanitize(name: &str) -> String {
	name.chars()
		.map(|c| {
			if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@') {
				c
			} else {
				'_'
			}
		})
		.collect()
}

// ----------------------------------------------------------------- mounts

/// The module system of a package and its dependencies: its own files
/// (`main`), its keys, and a mount per library it reaches.
pub(crate) fn mount(
	main: CodebaseResolver,
	package: &Path,
	resolution: &Resolution,
	target: &str,
) -> Result<MountedResolver, String> {
	let Some(keys) = resolution.packages.get(package) else {
		return Ok(MountedResolver {
			main,
			dependencies: BTreeMap::new(),
			mounts: Vec::new(),
		});
	};
	// the libraries reached from the package
	let mut seen = BTreeSet::new();
	let mut stack: Vec<usize> = keys.values().copied().collect();
	while let Some(library) = stack.pop() {
		if seen.insert(library) {
			stack.extend(resolution.libraries[library].dependencies.values().copied());
		}
	}
	let reached: Vec<usize> = seen.into_iter().collect();
	let entry_of = |library: usize| -> ModuleEntry {
		let found = &resolution.libraries[library];
		if found.directory == package {
			// a package of the workspace that depends back on this one
			return ModuleEntry::Folder("library".to_string());
		}
		match &found.kind {
			LibraryKind::Package | LibraryKind::Git => {
				ModuleEntry::Folder(format!("{}/library", found.prefix))
			}
			LibraryKind::File { name } => ModuleEntry::File(format!(
				"{}/{}",
				found.prefix,
				crate::project::without_fa(name)
			)),
			LibraryKind::Folder { .. } => ModuleEntry::Folder(found.prefix.clone()),
		}
	};
	// DEP005: a key is not a root entry of the package
	let codebase = &main.codebase;
	for key in keys.keys() {
		let collides = key == "library"
			|| codebase
				.roots()
				.any(|(_, folder)| codebase.folders[folder.index()].entries.contains_key(key));
		if collides {
			return Err(format!(
				"{}: error[DEP005]: the dependency `{key}` has the name of a file or folder of the package: rename the key in `dependencies.fa`",
				package.join("dependencies.fa").display()
			));
		}
	}
	// DEP009: the dependencies list the target
	for &library in &reached {
		let found = &resolution.libraries[library];
		if let Some(targets) = &found.targets
			&& !target.is_empty()
			&& !targets.iter().any(|other| other == target)
			&& found.directory != package
		{
			let name = keys
				.iter()
				.find(|&(_, &other)| other == library)
				.map_or(found.shown.as_str(), |(key, _)| key.as_str());
			return Err(format!(
				"error[DEP009]: `{name}` doesn't support the target `{target}`: its `package.fa` lists {}",
				targets
					.iter()
					.map(|target| format!("`{target}`"))
					.collect::<Vec<_>>()
					.join(", ")
			));
		}
	}
	let mut mounts = Vec::new();
	for &library in &reached {
		let found = &resolution.libraries[library];
		if found.directory == package {
			continue;
		}
		let codebase = match &found.kind {
			LibraryKind::Package | LibraryKind::Git => Codebase::scan_library(&found.directory)
				.map_err(|error| {
					format!("fa: can't read `{}`: {error}", found.directory.display())
				})?,
			LibraryKind::File { name } => {
				Codebase::from_paths(Layout::Standalone, [name.as_str()], None)
			}
			LibraryKind::Folder { files } => {
				Codebase::from_paths(Layout::Standalone, files.iter().map(String::as_str), None)
			}
		};
		mounts.push(Mount {
			prefix: found.prefix.clone(),
			shown: found.shown.clone(),
			resolver: CodebaseResolver::new(codebase, Some(found.directory.clone())),
			dependencies: found
				.dependencies
				.iter()
				.map(|(key, &other)| (key.clone(), entry_of(other)))
				.collect(),
		});
	}
	Ok(MountedResolver {
		main,
		dependencies: keys
			.iter()
			.map(|(key, &library)| (key.clone(), entry_of(library)))
			.collect(),
		mounts,
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn versions_and_ranges() {
		let version = |text| Version::parse(text).unwrap();
		assert!(version("4.10.0") > version("4.9.3"));
		assert!(version("5.0.0") > version("5.0.0-beta.2"));
		assert!(version("5.0.0-beta.10") > version("5.0.0-beta.2"));
		assert!(version("5.0.0-beta") < version("5.0.0-beta.1"));
		assert_eq!(version("v4.2.1"), version("4.2.1"));
		assert_eq!(Version::parse("4.2"), None);
		assert_eq!(Version::parse("04.2.1"), None);
		let range = |text| Range::parse(text).unwrap();
		assert!(range("4").matches(&version("4.9.0")));
		assert!(!range("4").matches(&version("5.0.0")));
		assert!(!range("4").matches(&version("4.9.0-beta.1")));
		assert!(range("4.2").matches(&version("4.2.7")));
		assert!(!range("4.2").matches(&version("4.3.0")));
		assert!(range("4.2.1").matches(&version("4.2.1")));
		assert!(!range("4.2.1").matches(&version("4.2.2")));
		assert!(range("0").matches(&version("0.3.0")));
		assert!(range("5.0.0-beta.1").matches(&version("5.0.0-beta.1")));
		assert_eq!(Range::parse("4.x"), None);
		assert_eq!(range("4.2").to_string(), "4.2");
	}

	#[test]
	fn sources() {
		assert_eq!(
			Source::parse("../shared"),
			Ok(Source::Path("../shared".to_string()))
		);
		let Ok(Source::Git {
			repository, range, ..
		}) = Source::parse("github:farmers/farm@4.2")
		else {
			panic!("a Git source");
		};
		assert_eq!(repository, "github:farmers/farm");
		assert_eq!(range, Range::parse("4.2"));
		let Ok(Source::Git {
			repository,
			url,
			range,
		}) = Source::parse("git+ssh://git@example.com/farm.git")
		else {
			panic!("a Git source");
		};
		assert_eq!(repository, "git+ssh://git@example.com/farm.git");
		assert_eq!(url, "ssh://git@example.com/farm.git");
		assert_eq!(range, None);
		assert!(Source::parse("github:farmers@4").is_err());
		assert!(Source::parse("github:farmers/farm@four").is_err());
		assert_eq!(
			Source::parse("https://cdn.example.com/ui@2.1.0/"),
			Ok(Source::Remote {
				url: "https://cdn.example.com/ui@2.1.0/".to_string(),
				folder: true
			})
		);
		assert!(Source::parse("farm").is_err());
		assert!(is_versioned("https://cdn.example.com/ui@2.1.0/"));
		assert!(is_versioned("https://api.example.com/v2/types.fa"));
		assert!(!is_versioned("https://api.weather.com/fa/types.fa"));
	}

	#[test]
	fn the_lockfile_is_sorted_and_read_back() {
		let entries = vec![
			LockEntry {
				source: "https://api.weather.com/fa/types.fa".to_string(),
				version: None,
				commit: None,
				hash: format!("sha256:{}", "e".repeat(64)),
			},
			LockEntry {
				source: "github:farmers/farm".to_string(),
				version: Some("4.10.0".to_string()),
				commit: Some("9".repeat(40)),
				hash: format!("sha256:{}", "a".repeat(64)),
			},
			LockEntry {
				source: "github:farmers/farm".to_string(),
				version: Some("3.2.0".to_string()),
				commit: Some("1".repeat(40)),
				hash: format!("sha256:{}", "b".repeat(64)),
			},
		];
		let text = render_lockfile(&entries);
		assert!(text.starts_with("-- dependencies.lock: generated by `fa`, do not edit\nexport = [\n\t{\n\t\tsource = \"github:farmers/farm\"\n\t\tversion = \"3.2.0\"\n"), "{text}");
		let directory = tempfile_folder("lockfile");
		let path = directory.join(LOCKFILE);
		fs::write(&path, &text).unwrap();
		let read = read_lockfile(&path).unwrap();
		assert_eq!(read.len(), 3);
		assert_eq!(render_lockfile(&read), text);
		assert!(fa_parser::parse(&text).diagnostics.is_empty());
	}

	#[test]
	fn a_forged_lockfile_is_rejected() {
		let directory = tempfile_folder("forged");
		let path = directory.join(LOCKFILE);
		let entry = |hash: &str, commit: &str| {
			format!(
				"export = [\n\t{{\n\t\tsource = \"github:a/b\"\n\t\tversion = \"1.0.0\"\n\t\tcommit = \"{commit}\"\n\t\thash = \"{hash}\"\n\t}}\n]\n"
			)
		};
		let good_hash = format!("sha256:{}", "0".repeat(64));
		let good_commit = "a".repeat(40);
		fs::write(&path, entry(&good_hash, &good_commit)).unwrap();
		assert_eq!(read_lockfile(&path).unwrap().len(), 1);
		for (hash, commit) in [
			("sha256:/tmp/victim", good_commit.as_str()),
			("sha256:../../victim", good_commit.as_str()),
			(&format!("sha256:{}", "A".repeat(64)), good_commit.as_str()),
			("sha256:00", good_commit.as_str()),
			(good_hash.as_str(), "--output=/tmp/x"),
			(good_hash.as_str(), "HEAD"),
		] {
			fs::write(&path, entry(hash, commit)).unwrap();
			let error = read_lockfile(&path).unwrap_err();
			assert!(error.contains("is not a valid lockfile"), "{error}");
		}
		fs::write(&path, "export = [ {").unwrap();
		assert!(
			read_lockfile(&path)
				.unwrap_err()
				.contains("is not a valid lockfile")
		);
	}

	#[test]
	fn paths_of_manifests_and_trees_are_portable() {
		for path in ["button.fa", "layout/grid.fa", "library/a.b.fa", "a-b_c"] {
			assert!(is_portable_path(path), "{path}");
		}
		for path in [
			"",
			"/etc/passwd",
			"../x",
			"a/../../x",
			"a//b",
			"./a",
			"a/.",
			"..\\x",
			"C:\\x",
			"C:x",
			"\\\\host\\x",
			"a\0b",
			"a/\n",
		] {
			assert!(!is_portable_path(path), "{path:?}");
		}
	}

	#[test]
	fn local_sources_are_recognized() {
		let local = |text: &str| local_source(&Source::parse(text).unwrap()).is_some();
		for text in [
			"file:///home/u/.ssh/id_rsa",
			"git+file:///srv/repository.git@1",
			"git+/srv/repository.git@1",
			"git+../repository@1",
			"git+C:\\repository@1",
			"git+ext::sh -c touch% /tmp/x@1",
			"../shared",
		] {
			assert!(local(text), "{text}");
		}
		for text in [
			"https://example.com/types@1.0.fa",
			"http://example.com/types@1.0.fa",
			"github:farmers/farm@4",
			"git+https://example.com/farm.git@4",
			"git+ssh://git@example.com/farm.git@4",
			"git+git@example.com:farmers/farm.git@4",
		] {
			assert!(!local(text), "{text}");
		}
		assert!(Source::parse("git+--upload-pack=touch /tmp/x@1").is_err());
	}

	#[test]
	fn only_the_path_of_a_url_names_its_version() {
		for url in [
			"https://example.com/ui@2.1.0/",
			"https://example.com/v3/types.fa",
			"https://example.com/types-1.2.fa",
		] {
			assert!(is_versioned(url), "{url}");
		}
		for url in [
			"https://dev1.example.com/x.fa",
			"https://192.168.1.10/x.fa",
			"https://example.com/types.fa?v=2",
			"https://v2.example.com/types.fa",
		] {
			assert!(!is_versioned(url), "{url}");
		}
	}

	#[test]
	fn the_changes_of_the_lockfile_pair_majors() {
		let entry = |version: &str, hash: char| LockEntry {
			source: "github:a/b".to_string(),
			version: Some(version.to_string()),
			commit: None,
			hash: format!("sha256:{}", hash.to_string().repeat(64)),
		};
		let changes = lock_changes(
			&[entry("1.0.0", 'a'), entry("2.0.0", 'b')],
			&[
				entry("1.1.0", 'c'),
				entry("2.0.0", 'b'),
				entry("3.0.0", 'd'),
			],
		);
		assert_eq!(
			changes,
			[
				(
					"github:a/b".to_string(),
					Some("1.0.0".to_string()),
					Some("1.1.0".to_string())
				),
				("github:a/b".to_string(), None, Some("3.0.0".to_string())),
			]
		);
	}

	#[test]
	fn the_hash_of_several_files_follows_d45() {
		let files = vec![
			("library/b.fa".to_string(), b"b".to_vec()),
			("library/a.fa".to_string(), b"aa".to_vec()),
		];
		let mut bytes = Vec::new();
		bytes.extend_from_slice(b"library/a.fa\x002\x00aa");
		bytes.extend_from_slice(b"library/b.fa\x001\x00b");
		assert_eq!(
			hash_files(&files),
			format!("sha256:{}", sha256::hex(&bytes))
		);
	}

	fn tempfile_folder(name: &str) -> PathBuf {
		let directory =
			std::env::temp_dir().join(format!("fa-dependencies-{name}-{}", std::process::id()));
		let _ = fs::remove_dir_all(&directory);
		fs::create_dir_all(&directory).unwrap();
		directory
	}
}
