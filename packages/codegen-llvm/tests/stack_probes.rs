//! Native frame probes must touch guard pages even when one frame exceeds a page.

#[cfg(all(fa_llvm, target_arch = "x86_64", target_os = "linux"))]
#[test]
fn a_large_recursive_frame_stops_at_the_stack_guard() {
	use std::path::PathBuf;
	use std::process::Command;
	use std::sync::Arc;

	use fa_analyzer::session::Session;
	use fa_codegen_llvm::{OptLevel, Options, Target, compile};

	// O0 keeps this live tuple in memory across recursion: more than two pages.
	let items = std::iter::repeat_n("n", 1024)
		.collect::<Vec<_>>()
		.join(", ");
	let source = format!(
		"function deep = (n: Integer): Integer => {{\nlet wide = ({items})\nlet result = deep(n + 1)\nresult + wide[0] + wide[1023]\n}}\nconsole.log(\"start\")\nconsole.log(deep(0))\n"
	);
	let mut session = Session::new();
	let file = session.set_source("main", Arc::from(source));
	session.analyze_file(file);
	let output = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("stack-probes/main");
	let options = Options {
		opt_level: OptLevel::O0,
		asynchronous: Some(false),
		emit_ir: true,
		leak_check: true,
		..Options::default()
	};
	let artifact = compile(
		&session,
		file,
		"stack-probes.fa",
		Target::Native,
		&options,
		&output,
	)
	.expect("compile the recursive frame");
	let ir = std::fs::read_to_string(artifact.ir.as_ref().unwrap()).unwrap();
	assert!(ir.contains("\"probe-stack\"=\"inline-asm\""));
	let assembly = Command::new("objdump")
		.arg("-d")
		.arg(&artifact.output)
		.output()
		.unwrap();
	assert!(assembly.status.success());
	let assembly = String::from_utf8_lossy(&assembly.stdout);
	let body = assembly
		.split("<fa.deep>:")
		.nth(1)
		.expect("deep's assembly");
	let body = body.split("\n\n").next().unwrap();
	assert!(body.contains("$0x1000,%rsp"), "{body}");
	let run = Command::new("bash")
		.args(["--noprofile", "--norc", "-c", "ulimit -s 256; exec \"$0\""])
		.arg(&artifact.output)
		.output()
		.unwrap();
	assert_eq!(run.status.code(), Some(101), "{run:?}");
	assert_eq!(run.stdout, b"start\n");
	assert!(
		String::from_utf8_lossy(&run.stderr).contains("stack overflow"),
		"{run:?}"
	);
}
