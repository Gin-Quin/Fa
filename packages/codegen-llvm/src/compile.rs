use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::time::{Duration, Instant};

use inkwell::context::Context;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::FileType;

use crate::error::Error;
use crate::link;
use crate::program::Program;
use crate::target::{Profile, Target};

/// The time of each phase of [`compile`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Timings {
	/// Building the module with the IR builder.
	pub build: Duration,
	/// `LLVMVerifyModule`.
	pub verify: Duration,
	/// The new pass manager pipeline of the profile.
	pub optimize: Duration,
	/// Instruction selection and object emission.
	pub emit: Duration,
	/// `cc` or `wasm-ld`.
	pub link: Duration,
}

impl Timings {
	pub fn total(&self) -> Duration {
		self.build + self.verify + self.optimize + self.emit + self.link
	}
}

/// A compiled and linked program.
#[derive(Clone, Debug)]
pub struct Artifact {
	pub program: Program,
	pub target: Target,
	pub profile: Profile,
	/// The optimized IR, written next to the object for inspection.
	pub ir: PathBuf,
	pub object: PathBuf,
	/// The executable, or the `.wasm` module.
	pub output: PathBuf,
	pub object_size: u64,
	pub output_size: u64,
	pub timings: Timings,
}

/// Builds `program` for `target`, optimizes it with the pipeline of
/// `profile`, emits an object and links it, all inside `directory`.
pub fn compile(
	program: Program,
	target: Target,
	profile: Profile,
	directory: &Path,
) -> Result<Artifact, Error> {
	fs::create_dir_all(directory)?;
	let mut timings = Timings::default();
	let machine = target.machine(profile)?;
	let context = Context::create();
	let usize = context.ptr_sized_int_type(&machine.get_target_data(), None);

	let start = Instant::now();
	let module = program.build(&context, target, usize)?;
	module.set_triple(&target.triple());
	module.set_data_layout(&machine.get_target_data().get_data_layout());
	timings.build = start.elapsed();

	let start = Instant::now();
	module.verify()?;
	timings.verify = start.elapsed();

	let start = Instant::now();
	if let Some(pipeline) = profile.pipeline() {
		module.run_passes(pipeline, &machine, PassBuilderOptions::create())?;
	}
	timings.optimize = start.elapsed();

	let stem = format!(
		"{}-{}-{}",
		program.name(),
		target_name(target),
		profile_name(profile)
	);
	let ir = directory.join(format!("{stem}.ll"));
	module.print_to_file(&ir)?;

	let object = directory.join(format!("{stem}.o"));
	let start = Instant::now();
	machine.write_to_file(&module, FileType::Object, &object)?;
	timings.emit = start.elapsed();

	let mut output = directory.join(&stem);
	if !target.output_extension().is_empty() {
		output.set_extension(target.output_extension());
	}
	let start = Instant::now();
	link::link(target, profile, &object, &output)?;
	timings.link = start.elapsed();

	Ok(Artifact {
		program,
		target,
		profile,
		ir,
		object_size: fs::metadata(&object)?.len(),
		output_size: fs::metadata(&output)?.len(),
		object,
		output,
		timings,
	})
}

/// Runs a compiled program and returns its output.
pub fn run(artifact: &Artifact) -> Result<Output, Error> {
	link::run(artifact.target, &artifact.output)
}

fn target_name(target: Target) -> &'static str {
	match target {
		Target::Native => "native",
		Target::Freestanding => "freestanding",
		Target::Wasm => "wasm",
	}
}

fn profile_name(profile: Profile) -> &'static str {
	match profile {
		Profile::Dev => "dev",
		Profile::Release => "release",
		Profile::Max => "max",
		Profile::Size => "size",
	}
}
