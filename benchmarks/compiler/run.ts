// Measures the Fa compiler on generated packages (generate.ts): wall time, CPU utilization and peak
// memory of `fa check`, `fa build --target=bun` and `fa build --target=native`, with the phases that
// `--timings` prints. See ../README.md, "Compiler".
//
// bun benchmarks/compiler/run.ts [--files <n,n,…>] [--lines <n>] [--seed <n>] [--commands <check,bun,native>]
//                                [--memory-cap <MB>] [--compare <file.json>]

import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { cpus, loadavg, platform, release } from "node:os";
import { dirname, join, resolve } from "node:path";

import { generate, type GenerateSummary } from "./generate.ts";

const compilerDirectory = import.meta.dir;
const benchmarksDirectory = dirname(compilerDirectory);
const root = dirname(benchmarksDirectory);
const buildDirectory = join(benchmarksDirectory, "build", "compiler");
/** The Fa compiler; `FA_COMPILER` overrides it, to try another build. */
const faCompiler = process.env.FA_COMPILER ?? join(root, "target", "release", "fa");

const commands = {
	check: (project: string) => ["check", project],
	bun: (project: string, output: string) => [
		"build",
		project,
		"--target=bun",
		"--minify=false",
		`--output=${output}`,
	],
	native: (project: string, output: string) => [
		"build",
		project,
		"--target=native",
		`--output=${output}`,
	],
} as const;
type CommandName = keyof typeof commands;

type Status = "ok" | "errors" | "out of memory" | "failed" | "unsupported";

type Measure = {
	files: number;
	lines: number;
	command: CommandName;
	status: Status;
	/** The last lines of stderr, for a status other than "ok". */
	detail?: string;
	/** Wall-clock time, in seconds. */
	seconds?: number;
	/** (user + system) / wall: the cores used on average. */
	cpu?: number;
	/** Peak resident set size, in bytes. */
	peakBytes?: number;
	/** The phases of `--timings`, in milliseconds. */
	phases?: Record<string, number>;
	/** The other lines of `--timings` (units, types, trees). */
	sizes?: Record<string, string>;
};

type Results = {
	date: string;
	machine: { platform: string; release: string; cpu: string; cores: number };
	/** The load average (1, 5, 15 minutes) when the run started: other work skews the times. */
	load: number[];
	compiler: string;
	seed: number;
	memoryCapMegabytes: number;
	packages: GenerateSummary[];
	measures: Measure[];
};

type Options = {
	files: number[];
	lines: number;
	seed: number;
	commands: CommandName[];
	memoryCap: number;
	compare?: string;
};

function parseOptions(argumentList: string[]): Options {
	const options: Options = {
		files: [100, 1000, 2000],
		lines: 500,
		seed: 1,
		commands: ["check", "bun", "native"],
		memoryCap: 4000,
	};
	for (let index = 0; index < argumentList.length; index++) {
		const argument = argumentList[index]!;
		const value = () => {
			const next = argumentList[++index];
			if (next === undefined) fail(`${argument} needs a value`);
			return next;
		};
		if (argument === "--files") {
			options.files = value()
				.split(",")
				.map((text) => positiveInteger(text, argument));
		} else if (argument === "--lines") options.lines = positiveInteger(value(), argument);
		else if (argument === "--seed") options.seed = positiveInteger(value(), argument);
		else if (argument === "--memory-cap") options.memoryCap = positiveInteger(value(), argument);
		else if (argument === "--commands") {
			options.commands = value()
				.split(",")
				.map((name) => {
					if (!(name in commands)) fail(`unknown command ${name}: check, bun or native`);
					return name as CommandName;
				});
		} else if (argument === "--compare") options.compare = resolve(value());
		else if (argument === "--help" || argument === "-h") {
			console.log(
				"Usage: bun benchmarks/compiler/run.ts [--files <n,n,…>] [--lines <n>] [--seed <n>] [--commands <check,bun,native>] [--memory-cap <MB>] [--compare <file.json>]",
			);
			process.exit(0);
		} else fail(`unknown argument ${argument}`);
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

function lastLines(text: string, count = 3): string {
	return text.trim().split("\n").slice(-count).join("\n");
}

/** Reads the report of `/usr/bin/time -v`. */
function parseTime(report: string) {
	const field = (name: string) =>
		report
			.split("\n")
			.find((line) => line.trim().startsWith(name))
			?.split(/:\s+/)
			.slice(1)
			.join(":")
			.trim();
	const wall = field("Elapsed (wall clock) time (h:mm:ss or m:ss)") ?? "0";
	const seconds = wall
		.split(":")
		.map(Number)
		.reduce((total, part) => total * 60 + part, 0);
	const user = Number(field("User time (seconds)") ?? 0);
	const system = Number(field("System time (seconds)") ?? 0);
	const peakKilobytes = Number(field("Maximum resident set size (kbytes)") ?? 0);
	const signal = report.match(/Command terminated by signal (\d+)/)?.[1];
	return { seconds, cpu: seconds > 0 ? (user + system) / seconds : 0, peakKilobytes, signal };
}

/** Reads the lines of `--timings`: `phase <name> <ms> ms`, and the others by their first word. */
function parseTimings(stderr: string) {
	const phases: Record<string, number> = {};
	const sizes: Record<string, string> = {};
	let inside = false;
	for (const line of stderr.split("\n")) {
		if (line.startsWith("timings: ")) {
			inside = true;
			continue;
		}
		if (!inside || !line.startsWith("  ")) continue;
		const phase = line.match(/^\s+phase (.+?)\s+([\d.]+) ms/);
		if (phase) phases[phase[1]!] = Number(phase[2]);
		else {
			const [key, ...rest] = line.trim().split(" ");
			if (key === "largest") sizes.largest = rest.slice(1).join(" ");
			else if (key === "peak") sizes.peak = rest.slice(1).join(" ");
			else sizes[key!] = rest.join(" ");
		}
	}
	return { phases, sizes };
}

/** Runs one command under `/usr/bin/time -v` and a virtual memory cap. */
function measure(
	command: CommandName,
	project: string,
	output: string,
	options: Options,
	files: number,
): Measure {
	const report = join(output, "time.txt");
	mkdirSync(output, { recursive: true });
	const argumentList = [...commands[command](project, join(output, "out")), "--timings"];
	const script = `ulimit -v ${options.memoryCap * 1024} && exec /usr/bin/time -v -o "$0" "$@"`;
	const result = spawnSync("bash", ["-c", script, report, faCompiler, ...argumentList], {
		encoding: "utf8",
		maxBuffer: 1 << 28,
	});
	const stderr = result.stderr ?? "";
	const time = existsSync(report) ? parseTime(readFileSync(report, "utf8")) : undefined;
	const base = { files, lines: options.lines, command };
	const { phases, sizes } = parseTimings(stderr);
	const measured = {
		seconds: time?.seconds,
		cpu: time?.cpu,
		peakBytes: time ? time.peakKilobytes * 1024 : undefined,
		phases,
		sizes,
	};
	if (result.status === 0) return { ...base, status: "ok", ...measured };
	const plain = stderr.replace(/timings: [\s\S]*$/, "");
	const detail = lastLines(plain);
	if (/memory allocation of \d+ bytes failed|out of memory|Cannot allocate memory/i.test(stderr)) {
		return { ...base, status: "out of memory", detail, ...measured };
	}
	if (/compiles one standalone file for now|built without LLVM/.test(stderr)) {
		return { ...base, status: "unsupported", detail: lastLines(plain, 1) };
	}
	if (result.status === 1 && /error\[/.test(stderr) && time?.signal === undefined) {
		return { ...base, status: "errors", detail, ...measured };
	}
	return {
		...base,
		status: "failed",
		detail: time?.signal ? `signal ${time.signal}: ${detail}` : detail,
		...measured,
	};
}

function formatSeconds(seconds: number | undefined): string {
	return seconds === undefined ? "" : `${seconds.toFixed(2)} s`;
}

function formatBytes(bytes: number | undefined): string {
	return bytes === undefined ? "" : `${Math.round(bytes / 1e6)} MB`;
}

function delta(current: number | undefined, previous: number | undefined): string {
	if (current === undefined || previous === undefined || previous === 0) return "";
	const change = ((current - previous) / previous) * 100;
	return ` (${change >= 0 ? "+" : ""}${change.toFixed(0)}%)`;
}

function phaseSummary(measure: Measure): string {
	const phases = measure.phases ?? {};
	const shown = ["parse", "analysis", "generation", "write", "llvm"]
		.filter((name) => phases[name] !== undefined)
		.map((name) => `${name} ${(phases[name]! / 1000).toFixed(2)}`);
	return shown.join(", ");
}

function printTable(results: Results, previous?: Results) {
	const rows = [
		["Files", "Command", "Status", "Wall", "CPU", "Peak RSS", "Phases (s)"],
		...results.measures.map((measure) => {
			const before = previous?.measures.find(
				(other) =>
					other.files === measure.files &&
					other.lines === measure.lines &&
					other.command === measure.command,
			);
			return [
				`${measure.files.toLocaleString("en-US")} × ${measure.lines}`,
				measure.command === "check" ? "fa check" : `fa build --target=${measure.command}`,
				measure.status,
				formatSeconds(measure.seconds) + delta(measure.seconds, before?.seconds),
				measure.cpu === undefined ? "" : `${measure.cpu.toFixed(2)} cores`,
				formatBytes(measure.peakBytes) + delta(measure.peakBytes, before?.peakBytes),
				phaseSummary(measure),
			];
		}),
	];
	const widths = rows[0]!.map((_, column) => Math.max(...rows.map((row) => row[column]!.length)));
	for (const [index, row] of rows.entries()) {
		console.log(row.map((cell, column) => cell.padEnd(widths[column]!)).join("  "));
		if (index === 0) console.log(widths.map((width) => "-".repeat(width)).join("  "));
	}
	const notes = results.measures.filter((measure) => measure.detail);
	for (const measure of notes) {
		console.log(
			`\n${measure.files} files, ${measure.command}: ${measure.status}\n${measure.detail}`,
		);
	}
	const sized = results.measures.filter((measure) => measure.sizes?.units);
	if (sized.length > 0) console.log("");
	for (const measure of sized) {
		const sizes = measure.sizes!;
		console.log(
			`${measure.files} files, ${measure.command}: units ${sizes.units}; types ${sizes.types}; trees ${sizes.trees}`,
		);
	}
}

function main() {
	const options = parseOptions(process.argv.slice(2));
	if (!existsSync(faCompiler)) {
		fail(`${faCompiler} is missing: build it with \`cargo build --release --bin fa\``);
	}
	const previous =
		options.compare === undefined
			? undefined
			: (JSON.parse(readFileSync(options.compare, "utf8")) as Results);
	const date = new Date().toISOString().slice(0, 10);
	const results: Results = {
		date,
		machine: {
			platform: platform(),
			release: release(),
			cpu: cpus()[0]?.model ?? "unknown",
			cores: cpus().length,
		},
		load: loadavg().map((load) => Math.round(load * 100) / 100),
		compiler: (() => {
			try {
				return execFileSync("git", ["rev-parse", "--short", "HEAD"], {
					cwd: root,
					encoding: "utf8",
				}).trim();
			} catch {
				return "unknown";
			}
		})(),
		seed: options.seed,
		memoryCapMegabytes: options.memoryCap,
		packages: [],
		measures: [],
	};
	for (const files of options.files) {
		const project = join(buildDirectory, `${files}x${options.lines}-s${options.seed}`);
		const summary = generate({
			files,
			lines: options.lines,
			seed: options.seed,
			targetShare: 0.05,
			errors: 0,
			cycle: true,
			output: project,
		});
		results.packages.push(summary);
		console.error(
			`generated ${files} files × ${options.lines} lines (${(summary.bytes / 1e6).toFixed(1)} MB)`,
		);
		for (const command of options.commands) {
			const output = join(buildDirectory, `${files}x${options.lines}-s${options.seed}.${command}`);
			rmSync(output, { recursive: true, force: true });
			console.error(`  ${command}…`);
			results.measures.push(measure(command, project, output, options, files));
		}
	}
	const resultsDirectory = join(benchmarksDirectory, "results");
	mkdirSync(resultsDirectory, { recursive: true });
	const file = join(resultsDirectory, `compiler-${date}.json`);
	writeFileSync(file, `${JSON.stringify(results, null, "\t")}\n`);
	printTable(results, previous);
	console.log(
		`\nload average at the start: ${results.load.join(", ")} (${results.machine.cores} cores)`,
	);
	console.log(`\nwrote ${file}`);
}

main();
