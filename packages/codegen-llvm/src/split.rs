//! Programs compiled per unit (design/parallel-pipeline.md §3.3, step 6):
//! the data the units, the entry object and the link exchange. Plain data,
//! available without LLVM (the `fa` command holds them).

use std::path::PathBuf;

/// What the entry object needs to know of a unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitSymbols {
	/// The initializations it defines, `fa.init.<module>.<node>`.
	pub inits: Vec<(String, u32)>,
	/// The drops of module values it defines, `fa.drop.<module>.<node>`.
	pub drops: Vec<(String, u32)>,
	/// The module whose top level it defines (`fa.main.<module>`): the
	/// program's entry.
	pub entry: Option<String>,
	/// The ids of union members its box functions handle.
	pub boxes: Vec<u64>,
	/// A drop of the unit does more than free memory (the drops at exit run).
	pub effectful_drops: bool,
	/// The unit reads `arguments`, or calls the entry function.
	pub uses_arguments: bool,
}

impl UnitSymbols {
	/// A text form (the object cache of development builds).
	pub fn to_text(&self) -> String {
		let mut text = String::new();
		for (module, node) in &self.inits {
			text.push_str(&format!("init {node} {module}\n"));
		}
		for (module, node) in &self.drops {
			text.push_str(&format!("drop {node} {module}\n"));
		}
		if let Some(entry) = &self.entry {
			text.push_str(&format!("entry {entry}\n"));
		}
		for id in &self.boxes {
			text.push_str(&format!("box {id}\n"));
		}
		text.push_str(&format!(
			"flags {} {}\n",
			u8::from(self.effectful_drops),
			u8::from(self.uses_arguments)
		));
		text
	}

	/// Reads [`UnitSymbols::to_text`].
	pub fn from_text(text: &str) -> Option<UnitSymbols> {
		let mut symbols = UnitSymbols::default();
		let mut flags = false;
		for line in text.lines() {
			let (word, rest) = line.split_once(' ')?;
			match word {
				"init" | "drop" => {
					let (node, module) = rest.split_once(' ')?;
					let entry = (module.to_string(), node.parse().ok()?);
					if word == "init" {
						symbols.inits.push(entry);
					} else {
						symbols.drops.push(entry);
					}
				}
				"entry" => symbols.entry = Some(rest.to_string()),
				"box" => symbols.boxes.push(rest.parse().ok()?),
				"flags" => {
					let (effectful, arguments) = rest.split_once(' ')?;
					symbols.effectful_drops = effectful == "1";
					symbols.uses_arguments = arguments == "1";
					flags = true;
				}
				_ => return None,
			}
		}
		flags.then_some(symbols)
	}
}

/// A unit compiled: its object (or ThinLTO bitcode) and its symbols.
#[derive(Clone, Debug)]
pub struct UnitOutput {
	/// The unit's name: its first module path.
	pub name: String,
	/// The object file.
	pub object: PathBuf,
	/// What the entry object needs.
	pub symbols: UnitSymbols,
	/// Taken from the object cache (not compiled again).
	pub cached: bool,
}

/// How the units of a program are compiled and linked.
#[derive(Clone, Debug, Default)]
pub struct SplitOptions {
	/// Release: ThinLTO bitcode, optimized across modules at the link.
	/// Development: plain objects, linked with `--gc-sections`.
	pub lto: bool,
	/// With `lto`: whole-program LTO (`--lto=full`): plain bitcode, the
	/// units merged into one module at the link (the smallest output; the
	/// link's memory grows with the program).
	pub full_lto: bool,
	/// The folder of the object cache (development): a unit whose sources,
	/// dependencies' interfaces and options didn't change is not compiled
	/// again.
	pub cache: Option<PathBuf>,
	/// The folder of the runtime objects (one per target and options).
	pub runtime_cache: Option<PathBuf>,
	/// The threads of the link's ThinLTO backends (0: the cores).
	pub jobs: usize,
}

/// The module values to initialize in order and the entry of a program
/// compiled per unit: what its entry object is made of.
#[derive(Clone, Debug, Default)]
pub struct EntryInputs {
	/// The initialization order: `(module path, declaration node)`.
	pub init_order: Vec<(String, u32)>,
	/// The units, in a deterministic order.
	pub units: Vec<UnitOutput>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unit_symbols_round_trip() {
		let symbols = UnitSymbols {
			inits: vec![("library/a b".to_string(), 4)],
			drops: vec![("library/a".to_string(), 4)],
			entry: Some("application/main".to_string()),
			boxes: vec![1, 1 << 62],
			effectful_drops: true,
			uses_arguments: false,
		};
		assert_eq!(UnitSymbols::from_text(&symbols.to_text()), Some(symbols));
	}
}
