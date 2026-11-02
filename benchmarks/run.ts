// Builds and runs the Fa benchmarks against their Rust and Goose versions. See README.md.
//
// bun benchmarks/run.ts [<name>...] [--runs <n>] [--size <n> | --quick] [--compare <file.json>]

import { execFileSync, spawnSync } from "node:child_process";
import {
	chmodSync,
	copyFileSync,
	existsSync,
	mkdirSync,
	readdirSync,
	readFileSync,
	rmSync,
	statSync,
	writeFileSync,
} from "node:fs";
import { cpus, homedir, platform, release } from "node:os";
import { basename, dirname, join, resolve } from "node:path";

const benchmarksDirectory = import.meta.dir;
const root = dirname(benchmarksDirectory);
const buildDirectory = join(benchmarksDirectory, "build");
const resultsFile = join(benchmarksDirectory, "results", "latest.json");
/** The Fa compiler; `FA_COMPILER` overrides it, to try another build. */
const faCompiler = process.env.FA_COMPILER ?? join(root, "target", "release", "fa");
/**
 * The Goose compiler (https://github.com/aardappel/goose): `GOOSE_COMPILER`, else
 * `~/.local/goose/goose` (with its `stdlib` beside it), else `goose` on the `PATH`.
 */
const gooseCompiler = findGooseCompiler();
/** Goose modules shared by the Goose programs (`import rng;`), copied next to each of them. */
const gooseModulesDirectory = join(benchmarksDirectory, "goose");
/** The line of a Goose program that holds its size, as in the Goose benchmark suite. */
const gooseSizeLine = /^(\s*let\s+\w+\s*=\s*)(\d+)(\s*;.*\/\/\s*BENCH_N.*)$/m;

/** Small sizes for `--quick`: each runs in a few milliseconds, to check that everything works. */
const quickSizes: Record<string, number> = {
	"binary-trees": 10,
	blur: 64,
	calc: 2_000,
	"fannkuch-redux": 7,
	fib: 20,
	graph: 1_000,
	interp: 10,
	lru: 20_000,
	mandelbrot: 200,
	nbody: 1000,
	particles: 1_000,
	push: 100_000,
	records: 10_000,
	respond: 2_000,
	scene: 5,
	sexp: 200,
	sieve: 100_000,
	"sort-objects": 10_000,
	"spectral-norm": 100,
	"string-building": 10_000,
	strlist: 10_000,
	sum: 100_000,
	tree: 10,
	"word-count": 10_000,
	words: 10_000,
};

const faTargets = ["native", "wasm", "bun"] as const;
type FaTarget = (typeof faTargets)[number];

/** A build or a run that did not give a time, with the reason. */
type Status =
	"ok" | "no compiler" | "no source" | "build failed" | "no output" | "run failed" | "wrong output";

type Measure = {
	status: Status;
	/** Median wall-clock time of the runs, in seconds. */
	seconds?: number;
	/** Size of the stripped executable, or of the `.wasm` or `.js` file, in bytes. */
	bytes?: number;
	/** What went wrong, for a status other than "ok". */
	detail?: string;
};

type BenchmarkResult = {
	name: string;
	/** `rustc -C opt-level=3` */
	rust: Measure;
	/** `rustc -C opt-level=3 -C overflow-checks=on`: the same overflow semantics as Fa. */
	rustChecked: Measure;
	/** Stripped size of `rustc -C opt-level=3 -C panic=abort -C strip=symbols`. */
	rustSmallBytes?: number;
	/** Size of the default `rustc -C opt-level=3` build, before stripping. */
	rustDefaultUnstrippedBytes?: number;
	fa: Record<FaTarget, Measure>;
	/** Fa native with `--overflow-checks=true`, to compare with Rust with overflow checks. */
	faNativeChecked?: Measure;
	/** `goose build -O2 --standalone`: the Goose program compiled to C, then by the C compiler. */
	goose?: Measure;
};

type Results = {
	date: string;
	machine: { platform: string; release: string; cpu: string; cores: number };
	versions: {
		rustc: string;
		bun: string;
		fa: string | null;
		goose?: string | null;
		/** The C compiler of the Goose builds: `GOOSE_CC`, else the one `goose toolchain` finds. */
		gooseCc?: string;
	};
	runs: number;
	size: number | "default" | "quick";
	benchmarks: BenchmarkResult[];
};

type Options = { names: string[]; runs: number; size?: number; quick: boolean; compare?: string };

function parseOptions(argumentList: string[]): Options {
	const options: Options = { names: [], runs: 3, quick: false };
	for (let index = 0; index < argumentList.length; index++) {
		const argument = argumentList[index]!;
		const value = () => {
			const next = argumentList[++index];
			if (next === undefined) fail(`${argument} needs a value`);
			return next;
		};
		if (argument === "--runs") options.runs = positiveInteger(value(), "--runs");
		else if (argument === "--size") options.size = positiveInteger(value(), "--size");
		else if (argument === "--quick") options.quick = true;
		else if (argument === "--compare") options.compare = resolve(value());
		else if (argument === "--help" || argument === "-h") {
			console.log(
				"Usage: bun benchmarks/run.ts [<name>...] [--runs <n>] [--size <n> | --quick] [--compare <file.json>]",
			);
			process.exit(0);
		} else if (argument.startsWith("--")) fail(`unknown option ${argument}`);
		else options.names.push(argument);
	}
	return options;
}

function positiveInteger(text: string, option: string): number {
	const value = Number(text);
	if (!Number.isInteger(value) || value < 1) fail(`${option} expects a positive integer`);
	return value;
}

function fail(message: string): never {
	console.error(`error: ${message}`);
	process.exit(1);
}

function listBenchmarks(): string[] {
	return readdirSync(benchmarksDirectory)
		.filter((name) => existsSync(join(benchmarksDirectory, name, "main.rs")))
		.sort();
}

function version(command: string, argumentList: string[]): string {
	try {
		return execFileSync(command, argumentList, { encoding: "utf8" }).trim();
	} catch {
		return "unknown";
	}
}

/** Runs a command, returning its exit code and output; never throws. */
function execute(command: string, argumentList: string[], cwd = root) {
	const result = spawnSync(command, argumentList, { cwd, encoding: "utf8", maxBuffer: 1 << 26 });
	return {
		ok: result.status === 0,
		stdout: result.stdout ?? "",
		stderr: (result.stderr ?? "") + (result.error ? String(result.error) : ""),
	};
}

function findGooseCompiler(): string | undefined {
	if (process.env.GOOSE_COMPILER !== undefined) return process.env.GOOSE_COMPILER;
	const local = join(homedir(), ".local", "goose", "goose");
	if (existsSync(local)) return local;
	const found = spawnSync("sh", ["-c", "command -v goose"], { encoding: "utf8" });
	return found.status === 0 ? found.stdout.trim() || undefined : undefined;
}

function rustc(source: string, output: string, flags: string[]): string | undefined {
	const result = execute("rustc", ["--edition", "2024", ...flags, "-o", output, source]);
	return result.ok ? undefined : result.stderr.trim();
}

/** The size of a stripped copy of an executable. */
function strippedSize(executable: string): number {
	const copy = `${executable}.stripped`;
	copyFileSync(executable, copy);
	execute("strip", [copy]);
	const bytes = statSync(copy).size;
	rmSync(copy);
	return bytes;
}

function median(values: number[]): number {
	const sorted = [...values].sort((a, b) => a - b);
	const middle = Math.floor(sorted.length / 2);
	return sorted.length % 2 === 1 ? sorted[middle]! : (sorted[middle - 1]! + sorted[middle]!) / 2;
}

/**
 * Runs a program once to check its output against the expected one, then `runs` times to time it.
 * The first run doubles as a warm-up (file cache, CPU frequency).
 */
function timeProgram(command: string[], expected: string | undefined, runs: number): Measure {
	const [program, ...argumentList] = command as [string, ...string[]];
	const check = execute(program, argumentList);
	if (!check.ok) return { status: "run failed", detail: lastLines(check.stderr) };
	if (expected !== undefined && check.stdout !== expected) {
		return { status: "wrong output", detail: firstDifference(expected, check.stdout) };
	}
	const times: number[] = [];
	for (let run = 0; run < runs; run++) {
		const start = performance.now();
		const result = spawnSync(program, argumentList, { stdio: "ignore" });
		const elapsed = (performance.now() - start) / 1000;
		if (result.status !== 0) return { status: "run failed", detail: `exit code ${result.status}` };
		times.push(elapsed);
	}
	return { status: "ok", seconds: median(times) };
}

function lastLines(text: string): string {
	return text.trim().split("\n").slice(-3).join("\n");
}

function firstDifference(expected: string, actual: string): string {
	const expectedLines = expected.split("\n");
	const actualLines = actual.split("\n");
	for (let line = 0; line < Math.max(expectedLines.length, actualLines.length); line++) {
		if (expectedLines[line] !== actualLines[line]) {
			return `line ${line + 1}: expected ${JSON.stringify(expectedLines[line] ?? "")}, got ${JSON.stringify(actualLines[line] ?? "")}`;
		}
	}
	return "different output";
}

/** Builds the Fa program for a target into its own directory, and returns how to run it. */
function buildFa(
	source: string,
	target: FaTarget,
	directory: string,
	extraArguments: string[] = [],
): { command: string[]; bytes: number } | Measure {
	if (!existsSync(faCompiler)) return { status: "no compiler", detail: `${faCompiler} is missing` };
	rmSync(directory, { recursive: true, force: true });
	mkdirSync(directory, { recursive: true });
	// The compiler is expected to write its output into `--output`; see README.md.
	const build = execute(faCompiler, [
		"build",
		source,
		`--target=${target}`,
		`--output=${directory}`,
		...extraArguments,
	]);
	if (!build.ok) return { status: "build failed", detail: lastLines(build.stderr + build.stdout) };
	const files = readdirSync(directory).map((name) => join(directory, name));
	const script = files.find((file) => file.endsWith(".js") || file.endsWith(".mjs"));
	if (target === "native") {
		const executable = files.find(
			(file) => statSync(file).isFile() && !/\.(js|mjs|wasm|d\.ts|map)$/.test(file),
		);
		if (executable === undefined) return { status: "no output", detail: "no executable" };
		chmodSync(executable, 0o755);
		return { command: [executable], bytes: strippedSize(executable) };
	}
	if (target === "wasm") {
		const module = files.find((file) => file.endsWith(".wasm"));
		if (module === undefined || script === undefined) {
			return { status: "no output", detail: "expected a .wasm file and its JavaScript loader" };
		}
		return { command: [process.execPath, script], bytes: statSync(module).size };
	}
	if (script === undefined) return { status: "no output", detail: "no .js file" };
	return { command: [process.execPath, script], bytes: statSync(script).size };
}

/**
 * Builds the Goose program of a benchmark: a copy of `main.goose`, with the shared modules, compiled
 * with `goose build -O2 --standalone` (the C compiler is Goose's choice: `GOOSE_CC`, else `cc`).
 * A program with a `// BENCH_N` line, from the Goose suite, takes its size there rather than from
 * its arguments: the runner rewrites the line when a size is given.
 */
function buildGoose(
	source: string,
	directory: string,
	size: number | undefined,
	sizeArguments: string[],
): { command: string[]; bytes: number } | Measure {
	if (!existsSync(source)) return { status: "no source" };
	if (gooseCompiler === undefined || !existsSync(gooseCompiler)) {
		return { status: "no compiler", detail: "no Goose compiler (GOOSE_COMPILER)" };
	}
	rmSync(directory, { recursive: true, force: true });
	mkdirSync(directory, { recursive: true });
	if (existsSync(gooseModulesDirectory)) {
		for (const module of readdirSync(gooseModulesDirectory)) {
			copyFileSync(join(gooseModulesDirectory, module), join(directory, module));
		}
	}
	let text = readFileSync(source, "utf8");
	const sized = gooseSizeLine.test(text);
	if (sized && size !== undefined) text = text.replace(gooseSizeLine, `$1${size}$3`);
	const program = join(directory, "main.goose");
	writeFileSync(program, text);
	const executable = join(directory, "main");
	const build = execute(gooseCompiler, ["build", "-O2", "--standalone", "-o", executable, program]);
	if (!build.ok) return { status: "build failed", detail: lastLines(build.stderr + build.stdout) };
	if (!existsSync(executable)) return { status: "no output", detail: "no executable" };
	return {
		command: sized ? [executable] : [executable, ...sizeArguments],
		bytes: strippedSize(executable),
	};
}

function runBenchmark(name: string, options: Options): BenchmarkResult {
	const directory = join(buildDirectory, name);
	mkdirSync(directory, { recursive: true });
	const rustSource = join(benchmarksDirectory, name, "main.rs");
	const faSource = join(benchmarksDirectory, name, "main.fa");
	const size = options.quick ? quickSizes[name] : options.size;
	const sizeArguments = size === undefined ? [] : [String(size)];

	const rustBinary = join(directory, "rust");
	const rustCheckedBinary = join(directory, "rust-checked");
	const rustSmallBinary = join(directory, "rust-small");
	const failed = (detail: string): Measure => ({ status: "build failed", detail });
	const result: BenchmarkResult = {
		name,
		rust: { status: "ok" },
		rustChecked: { status: "ok" },
		fa: {
			native: { status: "no compiler" },
			wasm: { status: "no compiler" },
			bun: { status: "no compiler" },
		},
	};

	const rustError = rustc(rustSource, rustBinary, ["-C", "opt-level=3"]);
	if (rustError !== undefined) {
		result.rust = failed(rustError);
		result.rustChecked = failed(rustError);
		return result;
	}
	const checkedError = rustc(rustSource, rustCheckedBinary, [
		"-C",
		"opt-level=3",
		"-C",
		"overflow-checks=on",
	]);
	const smallError = rustc(rustSource, rustSmallBinary, [
		"-C",
		"opt-level=3",
		"-C",
		"panic=abort",
		"-C",
		"strip=symbols",
	]);
	if (smallError === undefined) result.rustSmallBytes = strippedSize(rustSmallBinary);
	result.rustDefaultUnstrippedBytes = statSync(rustBinary).size;

	const reference = execute(rustBinary, sizeArguments);
	if (!reference.ok) {
		result.rust = { status: "run failed", detail: lastLines(reference.stderr) };
		return result;
	}
	const expected = reference.stdout;
	writeFileSync(join(directory, "expected.txt"), expected);

	result.rust = {
		...timeProgram([rustBinary, ...sizeArguments], expected, options.runs),
		bytes: strippedSize(rustBinary),
	};
	result.rustChecked =
		checkedError === undefined
			? timeProgram([rustCheckedBinary, ...sizeArguments], expected, options.runs)
			: failed(checkedError);

	for (const target of faTargets) {
		const built = buildFa(faSource, target, join(directory, `fa-${target}`));
		if ("status" in built) {
			result.fa[target] = built;
			continue;
		}
		result.fa[target] = {
			...timeProgram([...built.command, ...sizeArguments], expected, options.runs),
			bytes: built.bytes,
		};
	}
	const checked = buildFa(faSource, "native", join(directory, "fa-native-checked"), [
		"--overflow-checks=true",
	]);
	result.faNativeChecked =
		"status" in checked
			? checked
			: timeProgram([...checked.command, ...sizeArguments], expected, options.runs);
	const goose = buildGoose(
		join(benchmarksDirectory, name, "main.goose"),
		join(directory, "goose"),
		size,
		sizeArguments,
	);
	result.goose =
		"status" in goose
			? goose
			: { ...timeProgram(goose.command, expected, options.runs), bytes: goose.bytes };
	return result;
}

function formatSeconds(measure: Measure): string {
	if (measure.status !== "ok" || measure.seconds === undefined) return shortStatus(measure.status);
	return measure.seconds < 1
		? `${(measure.seconds * 1000).toFixed(0)} ms`
		: `${measure.seconds.toFixed(2)} s`;
}

function formatBytes(bytes: number | undefined): string {
	if (bytes === undefined) return "-";
	if (bytes < 1024) return `${bytes} B`;
	if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KiB`;
	return `${(bytes / 1024 / 1024).toFixed(2)} MiB`;
}

function shortStatus(status: Status): string {
	const labels: Record<Status, string> = {
		ok: "-",
		"no compiler": "n/a",
		"no source": "n/a",
		"build failed": "build fail",
		"no output": "no output",
		"run failed": "run fail",
		"wrong output": "mismatch",
	};
	return labels[status];
}

function ratio(measure: Measure, reference: Measure): string {
	if (measure.seconds === undefined || reference.seconds === undefined) return "-";
	return `${(measure.seconds / reference.seconds).toFixed(2)}x`;
}

/** The relative change of a value from the previous run, like "+4.2%". */
function delta(current: number | undefined, previous: number | undefined): string {
	if (current === undefined || previous === undefined || previous === 0) return "";
	const change = ((current - previous) / previous) * 100;
	if (Math.abs(change) < 0.05) return " (=)";
	const sign = change > 0 ? "+" : "";
	return ` (${sign}${change.toFixed(1)}%)`;
}

function printTable(results: Results, previous?: Results) {
	const headers = [
		"Benchmark",
		"Rust",
		"Rust ovf",
		"Fa native",
		"vs Rust",
		"Fa ovf",
		"Fa wasm",
		"Fa Bun",
		"Goose",
		"Fa vs Goose",
		"Rust size",
		"Rust default",
		"Fa native",
		"Fa wasm",
		"Fa JS",
		"Goose size",
	];
	const rows = results.benchmarks.map((benchmark) => {
		const old = previous?.benchmarks.find((candidate) => candidate.name === benchmark.name);
		const goose: Measure = benchmark.goose ?? { status: "no compiler" };
		const time = (measure: Measure, oldMeasure: Measure | undefined) =>
			formatSeconds(measure) + delta(measure.seconds, oldMeasure?.seconds);
		const size = (bytes: number | undefined, oldBytes: number | undefined) =>
			formatBytes(bytes) + delta(bytes, oldBytes);
		return [
			benchmark.name,
			time(benchmark.rust, old?.rust),
			time(benchmark.rustChecked, old?.rustChecked),
			time(benchmark.fa.native, old?.fa.native),
			ratio(benchmark.fa.native, benchmark.rust),
			time(benchmark.faNativeChecked ?? { status: "no compiler" }, old?.faNativeChecked),
			time(benchmark.fa.wasm, old?.fa.wasm),
			time(benchmark.fa.bun, old?.fa.bun),
			time(goose, old?.goose),
			ratio(benchmark.fa.native, goose),
			size(benchmark.rustSmallBytes, old?.rustSmallBytes),
			size(benchmark.rust.bytes, old?.rust.bytes),
			size(benchmark.fa.native.bytes, old?.fa.native.bytes),
			size(benchmark.fa.wasm.bytes, old?.fa.wasm.bytes),
			size(benchmark.fa.bun.bytes, old?.fa.bun.bytes),
			size(goose.bytes, old?.goose?.bytes),
		];
	});
	const widths = headers.map((header, column) =>
		Math.max(header.length, ...rows.map((row) => row[column]!.length)),
	);
	const line = (cells: string[]) =>
		cells
			.map((cell, column) =>
				column === 0 ? cell.padEnd(widths[column]!) : cell.padStart(widths[column]!),
			)
			.join("  ");
	console.log();
	if (previous !== undefined) {
		console.log(`Compared with the run of ${previous.date}.`);
		if (previous.size !== results.size || previous.runs !== results.runs) {
			console.log(
				`Warning: that run used the size ${previous.size} and ${previous.runs} runs, this one ${results.size} and ${results.runs}.`,
			);
		}
	}
	console.log(line(headers));
	console.log(widths.map((width) => "-".repeat(width)).join("  "));
	for (const row of rows) console.log(line(row));
	console.log();
	console.log(
		"Times: median of the runs. Rust: -C opt-level=3; Rust ovf: with -C overflow-checks=on, compare with Fa ovf (Fa native with --overflow-checks=true); fa build wraps by default, like rustc -O.",
	);
	console.log(
		"Sizes: stripped. Rust size: -C panic=abort -C strip=symbols; Rust default: -C opt-level=3, stripped.",
	);
	console.log(
		`Goose: goose build -O2 --standalone (Goose to C, then ${results.versions.gooseCc ?? "the C compiler"} at -O2), stripped; n/a: no main.goose or no compiler.`,
	);

	const problems = results.benchmarks.flatMap((benchmark) =>
		(
			[
				["Rust", benchmark.rust],
				["Rust ovf", benchmark.rustChecked],
				...faTargets.map((target) => [`Fa ${target}`, benchmark.fa[target]]),
				["Fa ovf", benchmark.faNativeChecked ?? { status: "ok" }],
				["Goose", benchmark.goose ?? { status: "ok" }],
			] as [string, Measure][]
		)
			.filter(
				([, measure]) =>
					measure.status !== "ok" &&
					measure.status !== "no compiler" &&
					measure.status !== "no source",
			)
			.map(
				([label, measure]) =>
					`${benchmark.name}, ${label}: ${measure.status}\n    ${measure.detail ?? ""}`,
			),
	);
	if (!existsSync(faCompiler))
		console.log(`Fa: compiler not available (${faCompiler} is missing).`);
	if (gooseCompiler === undefined || !existsSync(gooseCompiler))
		console.log("Goose: compiler not available (set GOOSE_COMPILER).");
	for (const problem of problems) console.log(problem);
}

function main() {
	const options = parseOptions(process.argv.slice(2));
	if (options.quick && options.size !== undefined) fail("--quick and --size can't be combined");
	const available = listBenchmarks();
	for (const name of options.names) {
		if (!available.includes(name))
			fail(`unknown benchmark ${name} (available: ${available.join(", ")})`);
	}
	const previous =
		options.compare === undefined
			? undefined
			: (JSON.parse(readFileSync(options.compare, "utf8")) as Results);
	const names = options.names.length > 0 ? options.names : available;

	const results: Results = {
		date: new Date().toISOString(),
		machine: {
			platform: platform(),
			release: release(),
			cpu: cpus()[0]?.model ?? "unknown",
			cores: cpus().length,
		},
		versions: {
			rustc: version("rustc", ["--version"]),
			bun: Bun.version,
			fa: existsSync(faCompiler) ? version(faCompiler, ["version"]) : null,
			goose:
				gooseCompiler !== undefined && existsSync(gooseCompiler)
					? version(gooseCompiler, ["version"])
					: null,
			gooseCc: process.env.GOOSE_CC ?? "goose's default",
		},
		runs: options.runs,
		size: options.quick ? "quick" : (options.size ?? "default"),
		benchmarks: [],
	};
	for (const name of names) {
		process.stderr.write(`${name}...\n`);
		results.benchmarks.push(runBenchmark(name, options));
	}

	mkdirSync(dirname(resultsFile), { recursive: true });
	writeFileSync(resultsFile, `${JSON.stringify(results, null, "\t")}\n`);
	printTable(results, previous);
	console.log(`\nResults written to ${basename(dirname(resultsFile))}/${basename(resultsFile)}.`);
}

main();
