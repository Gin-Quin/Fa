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
	/// The unit defines `fa.exit_status`: the exit code of an asynchronous
	/// program's entry function.
	pub exit_status: bool,
	/// The symbols it defines for the other objects.
	pub exports: Vec<String>,
	/// The symbols of other units it uses: `(module, symbol)`.
	pub needs: Vec<(String, String)>,
	/// The functions of a C library it exports (`lower/c_library.rs`).
	pub c_exports: Vec<String>,
	/// What it holds of the generic function values: tables and indices the
	/// driver defines (types of the run: an object holding some is never
	/// cached, so they are not in the text form).
	pub generic_values: GenericValueNeeds,
}

/// What a unit holds of the generic function values
/// (`lower/generic_values.rs`, design/decisions.md "Generic function tables
/// per unit"): the instances its code calls, the tables of the values it
/// makes, and the entries of tables it defines (an instance wave).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GenericValueNeeds {
	/// The instances of classes it calls or converts to (it reads their
	/// indices).
	pub instances: Vec<GenericInstance>,
	/// The tables of the values it makes.
	pub producers: Vec<GenericProducer>,
	/// The entries it defines, by symbol.
	pub entries: Vec<String>,
}

impl GenericValueNeeds {
	/// Whether the unit holds no generic function value.
	pub fn is_empty(&self) -> bool {
		self.instances.is_empty() && self.producers.is_empty() && self.entries.is_empty()
	}
}

/// An instance of a class of generic function types (the types equal up
/// to the names and constraints of their parameters).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericInstance {
	/// The class's key: the same in every unit.
	pub class: String,
	/// The instance's key: the stable hashes of its type arguments.
	pub key: String,
	/// Its type arguments (global types of the run).
	pub arguments: Vec<fa_analyzer::ids::TypeId>,
	/// Semantic methods selected for its generic parameters.
	pub methods: Vec<fa_analyzer::analysis::GenericMethod>,
}

impl GenericInstance {
	/// The symbol of its index in the tables of its class.
	pub fn index_symbol(&self) -> String {
		format!("fa.gv.index.{}.{}", self.class, self.key)
	}
}

/// A maker of generic function values of a class: a generic lambda (under
/// the bindings of the instance around it) or a generic function
/// declaration used as a value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericProducer {
	/// Its table's symbol.
	pub table: String,
	/// The class of its values.
	pub class: String,
	/// Where it is: its module and node.
	pub module: String,
	pub node: u32,
	/// A lambda (else a declaration).
	pub lambda: bool,
	/// The bindings of the instance around a lambda: each parameter by its
	/// owner's node and its position in the owner's generic parameters,
	/// and the type it stands for (a global type of the run).
	pub bindings: Vec<(u32, u32, fa_analyzer::ids::TypeId)>,
	/// Method selections captured from enclosing instances, by parameter owner/index.
	pub methods: Vec<(u32, u32, fa_analyzer::analysis::GenericMethod)>,
}

impl GenericProducer {
	/// The symbol of its entry for an instance of its class.
	pub fn entry_symbol(&self, instance: &GenericInstance) -> String {
		format!(
			"{}.{}",
			self.table.replacen("fa.gv.table.", "fa.gv.entry.", 1),
			instance.key
		)
	}
}

/// An entry of a producer's table for an instance of its class, to be made
/// by the unit of the producer's module (an instance wave).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GenericEntryRequest {
	pub producer: GenericProducer,
	pub instance: GenericInstance,
}

/// The tables and indices of the generic function values of a program
/// compiled per unit, defined by its entry object.
#[derive(Clone, Debug, Default)]
pub struct GenericTables {
	/// Each table: its symbol and its entries' symbols, in index order.
	pub tables: Vec<(String, Vec<String>)>,
	/// Each index: its symbol and its value.
	pub indices: Vec<(String, u64)>,
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
		for symbol in &self.exports {
			text.push_str(&format!("export {symbol}\n"));
		}
		for (module, symbol) in &self.needs {
			text.push_str(&format!("need {module} {symbol}\n"));
		}
		for symbol in &self.c_exports {
			text.push_str(&format!("cexport {symbol}\n"));
		}
		text.push_str(&format!(
			"flags {} {} {}\n",
			u8::from(self.effectful_drops),
			u8::from(self.uses_arguments),
			u8::from(self.exit_status)
		));
		text
	}

	/// Reads [`UnitSymbols::to_text`].
	pub fn from_text(text: &str) -> Option<UnitSymbols> {
		let mut symbols = UnitSymbols::default();
		let mut complete = false;
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
				"export" => symbols.exports.push(rest.to_string()),
				"cexport" => symbols.c_exports.push(rest.to_string()),
				"need" => {
					let (module, symbol) = rest.split_once(' ')?;
					symbols.needs.push((module.to_string(), symbol.to_string()));
				}
				"flags" => {
					let mut flags = rest.split(' ');
					symbols.effectful_drops = flags.next()? == "1";
					symbols.uses_arguments = flags.next()? == "1";
					symbols.exit_status = flags.next()? == "1";
					complete = true;
				}
				_ => return None,
			}
		}
		complete.then_some(symbols)
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
	/// The instances of other units' generic functions it calls.
	pub requests: Vec<InstanceRequest>,
}

/// An instance of a generic function of another unit: the function's key
/// and its type arguments (global types of the run's store).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct InstanceRequest {
	/// The generic function.
	pub key: fa_analyzer::ids::DeclKey,
	/// Its type arguments.
	pub arguments: Vec<fa_analyzer::ids::TypeId>,
	/// Throwing callback parameters: their positions and widened signatures.
	pub callbacks: Vec<(usize, fa_analyzer::ids::TypeId)>,
	/// Methods selected from the written generic arguments.
	pub methods: Vec<fa_analyzer::analysis::GenericMethod>,
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
	/// Prints the choice of the LTO (`--verbose`).
	pub verbose: bool,
	/// The whole-program facts the driver merged from every unit
	/// (`facts.rs`): set by `compile_package` for the lowering of the units,
	/// the instance waves and the entry, and hashed into the cache keys.
	pub facts: Option<std::sync::Arc<crate::facts::ProgramFacts>>,
	/// The program is a C library (`build.cLibrary`): set by
	/// `compile_package` from its `Libraries`.
	pub c_library: Option<crate::options::CLibrary>,
	/// The program is loaded in this process by the JIT instead of linked
	/// (`jit.rs`, `fa run --target=native`): its objects are compiled with
	/// emulated thread-local variables, and kept apart in the cache.
	pub in_memory: bool,
}

/// What [`crate::compile_package`] gives besides the analysis' report.
#[derive(Debug)]
pub enum PackageOutcome {
	/// The program, linked.
	Built(crate::Artifact),
	/// The program, loaded in this process ([`SplitOptions::in_memory`]).
	Loaded(Box<dyn crate::LoadedProgram>),
	/// The analysis found errors (in the report): nothing is linked.
	Errors,
	/// The program can't be compiled per unit (an asynchronous program, a
	/// construct the units don't compile yet): compile it as one module.
	Unsupported(String),
	/// The backend failed.
	Failed(crate::Error),
}

/// The module values to initialize in order and the entry of a program
/// compiled per unit: what its entry object is made of.
#[derive(Clone, Debug, Default)]
pub struct EntryInputs {
	/// The initialization order: `(module path, declaration node)`.
	pub init_order: Vec<(String, u32)>,
	/// The units, in a deterministic order.
	pub units: Vec<UnitOutput>,
	/// The tables of the generic function values.
	pub generic_tables: GenericTables,
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
			exit_status: true,
			exports: vec!["fa.library/a.f<1,2>".to_string()],
			needs: vec![("library/b".to_string(), "fa.library/b.g".to_string())],
			c_exports: vec!["farm_count".to_string()],
			generic_values: GenericValueNeeds::default(),
		};
		assert_eq!(UnitSymbols::from_text(&symbols.to_text()), Some(symbols));
	}
}
