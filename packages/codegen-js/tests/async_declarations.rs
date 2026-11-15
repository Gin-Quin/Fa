//! Declarations describe precisely the async instances actually exported.

use fa_analyzer::{CodebaseResolver, Session};
use fa_codegen_js::{EntryOptions, Options, Target, generate_program};

#[test]
fn async_module_declarations_match_its_javascript_exports() {
	let files = [
		("package.fa", "export = { name = \"declarations\" }\n"),
		("library/clock.fa", "use { sleep } = std.time\nexport namespace = {\n\tasync function pause = (milliseconds: Integer): Integer => {\n\t\tawait sleep(milliseconds)\n\t\tmilliseconds\n\t}\n}\n"),
		("application/main.fa", "export async function = () => {\n\tconsole.log(await library.clock.pause(3)) -- prints: 3\n}\n"),
	].map(|(path, text)| (path.to_string(), text.to_string()));
	for target in [Target::Bun, Target::Node] {
		for asynchronous in [false, true] {
			let mut session = Session::new();
			session.set_target(target.name());
			session.set_resolver(Box::new(CodebaseResolver::from_texts(&files, None)));
			let program = session.analyze_package(Some("application/main"));
			let options = Options {
				declarations: true,
				asynchronous: Some(asynchronous),
				..Options::new(target, "declarations")
			};
			let output =
				generate_program(&mut session, &program, &options, &EntryOptions::default())
					.unwrap();
			let module = |path: &str| {
				&output
					.modules
					.iter()
					.find(|(name, _)| name == path)
					.unwrap()
					.1
			};
			let js = module("library/clock.js");
			let dts = module("library/clock.d.ts");
			assert!(js.contains(" as clock$pause$sync"), "{js}");
			assert!(
				dts.contains("clock$pause$sync(milliseconds: number): number;"),
				"{dts}"
			);
			assert_eq!(js.contains(" as clock$pause$async"), asynchronous, "{js}");
			assert_eq!(
				dts.contains(
					"clock$pause$async(task: unknown, milliseconds: number): Promise<number>;"
				),
				asynchronous,
				"{dts}"
			);
			assert_eq!(
				dts.contains("clock$pause(milliseconds: number): Promise<number>;"),
				asynchronous,
				"{dts}"
			);
		}
	}
}
