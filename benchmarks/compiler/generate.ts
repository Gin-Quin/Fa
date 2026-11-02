// Generates a Fa package to measure the compiler on: N library files of about L lines each, plus
// `application/main.fa`. See ../README.md, "Compiler".
//
// bun benchmarks/compiler/generate.ts [--files <n>] [--lines <n>] [--seed <n>] [--target-share <0..1>]
//                                     [--errors <n>] [--no-cycle] [--output <directory>]
//
// Every file is a namespace (`library/m0001.fa`, …) with object, union, enumeration and error types,
// a generic function, loops whose indexing is proven by facts, `when`, `throw`/`catch`, and functions
// that call the files it uses. The files form a DAG of about 20 levels: a file uses 3 to 8 files of
// earlier levels, one of them from the level just before. Three namespace files (`ringA`, `ringB`,
// `ringC`) call each other in a cycle, and three files of the middle level call them. Most files
// declare object types of the same structure (`Item`), each with its own functions. A share of the
// files reads `target` (`@if target is …`), which makes
// `fa check` analyze the package again for the other target of the configuration. A few files declare
// `static` values. Without `--errors`, `fa check` reports no diagnostic on the package.

import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

export type GenerateOptions = {
	files: number;
	lines: number;
	seed: number;
	/** The share of files that read `target`, from 0 to 1. */
	targetShare: number;
	/** The number of errors injected (type errors, unknown names, missing `when` cases). */
	errors: number;
	/** Whether three files (`ringA`, `ringB`, `ringC`) call each other in a cycle. */
	cycle: boolean;
	output: string;
};

export type GenerateSummary = {
	files: number;
	lines: number;
	bytes: number;
	levels: number;
	targetFiles: number;
	staticFiles: number;
	cycle: string[];
	errors: string[];
};

const levelCount = 20;
const ringNames = ["ringA", "ringB", "ringC"] as const;
const defaultOutput = (options: Pick<GenerateOptions, "files" | "lines" | "seed">) =>
	join(
		dirname(import.meta.dir),
		"build",
		"compiler",
		`${options.files}x${options.lines}-s${options.seed}`,
	);

/** Mulberry32: a small seeded generator, so that a seed always gives the same package. */
function random(seed: number) {
	let state = seed >>> 0;
	const next = () => {
		state = (state + 0x6d2b79f5) >>> 0;
		let t = state;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
	return {
		next,
		/** An integer from `low` to `high`, both included. */
		integer: (low: number, high: number) => low + Math.floor(next() * (high - low + 1)),
		pick: <T>(items: readonly T[]): T => items[Math.floor(next() * items.length)]!,
	};
}

type Random = ReturnType<typeof random>;

type FileModel = {
	index: number;
	name: string;
	level: number;
	/** The files it uses (indices), earlier levels only. */
	uses: number[];
	readsTarget: boolean;
	hasStatics: boolean;
	/** The file of the cycle it calls (`ringA`…), if any. */
	ring?: string;
	/** The number of blocks of functions (`compute0`, `compute1`, …), counted while generated. */
	blocks: number;
	/** Where to inject an error (block index), if any. */
	error?: { block: number; kind: number };
};

function moduleName(index: number, files: number): string {
	return `m${String(index + 1).padStart(Math.max(4, String(files).length), "0")}`;
}

/** Lays out the files: levels, the files each uses, targets, statics, the cycle, the errors. */
function plan(options: GenerateOptions, rng: Random): FileModel[] {
	const levels = Math.min(levelCount, options.files);
	const perLevel = Math.ceil(options.files / levels);
	const files: FileModel[] = [];
	const byLevel: number[][] = Array.from({ length: levels }, () => []);
	for (let index = 0; index < options.files; index++) {
		const level = Math.min(levels - 1, Math.floor(index / perLevel));
		const uses = new Set<number>();
		if (level > 0) {
			const earlier = byLevel.slice(0, level).flat();
			const wanted = Math.min(earlier.length, rng.integer(3, 8));
			// one from the level just before, so that the depth is the level count
			uses.add(rng.pick(byLevel[level - 1]!));
			while (uses.size < wanted) uses.add(rng.pick(earlier));
		}
		files.push({
			index,
			name: moduleName(index, options.files),
			level,
			uses: [...uses].sort((a, b) => a - b),
			readsTarget: rng.next() < options.targetShare,
			hasStatics: index % 25 === 3,
			blocks: 0,
		});
		byLevel[level]!.push(index);
	}
	if (options.cycle && options.files >= 3) {
		const middle = byLevel[Math.floor(levels / 2)]!;
		const callers = middle.length >= 3 ? middle.slice(0, 3) : [0, 1, 2];
		callers.forEach((caller, position) => {
			files[caller]!.ring = ringNames[position];
		});
	}
	const errorFiles = new Set<number>();
	while (errorFiles.size < Math.min(options.errors, options.files)) {
		errorFiles.add(rng.integer(0, options.files - 1));
	}
	let kind = 0;
	for (const index of [...errorFiles].sort((a, b) => a - b)) {
		files[index]!.error = { block: 0, kind: kind++ % 3 };
	}
	return files;
}

/** The lines of the header of a file: its types, statics and constructor. */
function header(file: FileModel, rng: Random): string[] {
	const fields = rng.integer(0, 3);
	const lines = [
		"export namespace = {",
		"\ttype Item = {",
		"\t\tid: Integer",
		"\t\tweight: Integer",
		"\t\tlabel: String",
		...Array.from({ length: fields }, (_, field) => `\t\textra${field}: Integer`),
		`\t\tfunction score = (self): Integer => self.id * ${rng.integer(2, 9)} + self.weight`,
		"\t}",
		"\ttype Shape = Union {",
		"\t\tCircle = { radius: Integer }",
		"\t\tSquare = { side: Integer }",
		"\t\tEmpty",
		"\t}",
		"\ttype Level = Enum { low, middle, high }",
		"\ttype Failure = Errors { Missing = { key: String }, Invalid }",
	];
	if (file.hasStatics) {
		lines.push(
			`\tstatic base = ${rng.integer(2, 9)}`,
			"\tfunction square = (x: Integer): Integer => x * x",
			"\tstatic table = @square(base)",
		);
	}
	const extras = Array.from({ length: fields }, (_, field) => `, extra${field} = ${field}`).join(
		"",
	);
	lines.push(
		`\tfunction make = (seed: Integer): Item => Item { id = seed, weight = seed %% ${rng.integer(7, 31)}, label = "${file.name}-{seed}"${extras} }`,
	);
	return lines;
}

/** The `k`th block of functions of a file: about 40 lines. */
function block(file: FileModel, k: number, files: FileModel[], rng: Random): string[] {
	const a = rng.integer(2, 9);
	const b = rng.integer(3, 17);
	const lines = [
		`\tfunction area${k} = (shape: Shape): Integer => when shape is {`,
		`\t\tShape.Circle as { radius } => radius * radius * ${a}`,
		"\t\tShape.Square as { side } => side * side",
		"\t\tShape.Empty => 0",
		"\t}",
		`\tfunction total${k} = (items: Array(Item)): Integer => {`,
		"\t\tmutable sum = 0",
		"\t\tfor index in 0..items.length {",
		`\t\t\tsum += items[index].weight %% ${b * 7}`,
		"\t\t}",
		"\t\tsum",
		"\t}",
		`\tfunction lookup${k} = (key: String, items: Array(Item)): Item throws Failure => {`,
		"\t\tfor item in items {",
		"\t\t\tif item.label == key {",
		"\t\t\t\treturn item",
		"\t\t\t}",
		"\t\t}",
		"\t\tthrow Failure.Missing { key }",
		"\t}",
		`\tfunction safe${k} = (key: String, items: Array(Item)): Integer => lookup${k}(key, items).weight catch error => -1`,
		`\tfunction firstOr${k}(Value) = (values: Array(Value), fallback: Value): Value => values.get(0) ?? fallback`,
		`\tfunction level${k} = (value: Integer): Level => if value < ${b} { .low } else if value < ${b * 10} { .middle } else { .high }`,
		`\tfunction compute${k} = (x: Integer): Integer => {`,
		`\t\tlet items = [make(x), make(x + ${a}), make(x %% ${b})]`,
		`\t\tlet shape = if x %% 2 == 0 { Shape.Circle { radius = x %% ${b} } } else { Shape.Square { side = x %% ${a} } }`,
		`\t\tmutable result = total${k}(items) + area${k}(shape) + safe${k}("${file.name}-{x}", items) + firstOr${k}([x], 0)`,
	];
	// calls to the files it uses: a function of the same rank, and their types through `make`
	const callees = file.uses.length === 0 ? [] : [rng.pick(file.uses), rng.pick(file.uses)];
	for (const callee of new Set(callees)) {
		const other = files[callee]!;
		const rank = rng.integer(0, Math.max(0, other.blocks - 1));
		lines.push(
			`\t\tresult += ${other.name}.compute${rank}(x %% ${rng.integer(50, 500)}) %% 1000`,
			`\t\tresult += ${other.name}.make(x).score() %% 100`,
		);
	}
	if (file.hasStatics) lines.push("\t\tresult += table");
	lines.push(
		`\t\twhen level${k}(result) is {`,
		"\t\t\tLevel.low => result += 1",
		"\t\t\tLevel.middle => result += 2",
		"\t\t\tLevel.high => result -= 3",
		"\t\t}",
	);
	if (file.readsTarget && k === 0) {
		lines.push(
			'\t\t@if target is "native" {',
			"\t\t\tresult += 1",
			"\t\t} @else {",
			"\t\t\tresult += 2",
			"\t\t}",
		);
	}
	if (file.ring !== undefined && k === 0) lines.push(`\t\tresult += ${file.ring}.call(x %% 5)`);
	if (file.error?.block === k) lines.push(...injectedError(file.error.kind, k));
	lines.push(`\t\tresult %% ${rng.integer(10_000, 100_000)}`, "\t}");
	return lines;
}

/** An error the analyzer reports: a type error, an unknown name, a `when` without every case. */
function injectedError(kind: number, k: number): string[] {
	switch (kind) {
		case 0:
			return [`\t\tlet injected: Integer = "not a number {x}"`, "\t\tresult += injected"];
		case 1:
			return [`\t\tresult += missingValue${k}`];
		default:
			return [
				`\t\tresult += when level${k}(x) is {`,
				"\t\t\tLevel.low => 1",
				"\t\t\tLevel.middle => 2",
				"\t\t}",
			];
	}
}

/** Generates the package and returns what it wrote. */
export function generate(options: GenerateOptions): GenerateSummary {
	const rng = random(options.seed);
	const files = plan(options, rng);
	rmSync(options.output, { recursive: true, force: true });
	mkdirSync(join(options.output, "library"), { recursive: true });
	mkdirSync(join(options.output, "application"), { recursive: true });
	const write = (path: string, text: string) => {
		writeFileSync(join(options.output, path), text);
		summary.bytes += text.length;
		summary.lines += text.split("\n").length - 1;
	};
	const summary: GenerateSummary = {
		files: options.files,
		lines: 0,
		bytes: 0,
		levels: Math.min(levelCount, options.files),
		targetFiles: files.filter((file) => file.readsTarget).length,
		staticFiles: files.filter((file) => file.hasStatics).length,
		cycle: options.cycle ? [...ringNames] : [],
		errors: files.filter((file) => file.error).map((file) => file.name),
	};
	write(
		"package.fa",
		'export = {\n\tname = "generated"\n\tversion = "0.1.0"\n\ttargets = ["bun", "native"]\n}\n',
	);
	// blocks are added up to the line count; a file calls only earlier files, whose count is known
	for (const file of files) {
		const lines = header(file, rng);
		do {
			lines.push(...block(file, file.blocks, files, rng));
			file.blocks++;
		} while (lines.length + 20 < options.lines);
		lines.push("}");
		write(`library/${file.name}.fa`, `${lines.join("\n")}\n`);
	}
	// each file of the cycle (a namespace) calls the next one
	if (options.cycle) {
		ringNames.forEach((name, position) => {
			const next = ringNames[(position + 1) % 3];
			write(
				`library/${name}.fa`,
				`export namespace = {\n\tfunction call = (x: Integer): Integer => {\n\t\tif x <= 0 {\n\t\t\treturn 0\n\t\t}\n\t\t${next}.call(x - 1) + ${position + 1}\n\t}\n}\n`,
			);
		});
	}
	// the application calls the files of the last level
	const last = files.filter((file) => file.level === summary.levels - 1);
	write(
		"application/main.fa",
		[
			"export function = (arguments: Array(String)): Integer => {",
			"\tmutable total = 0",
			...last.map((file) => `\ttotal += library.${file.name}.compute0(${file.index}) %% 1000`),
			"\tconsole.log(total)",
			"\t0",
			"}",
			"",
		].join("\n"),
	);
	return summary;
}

function parseOptions(argumentList: string[]): GenerateOptions {
	const options = {
		files: 100,
		lines: 500,
		seed: 1,
		targetShare: 0.05,
		errors: 0,
		cycle: true,
		output: undefined as string | undefined,
	};
	for (let index = 0; index < argumentList.length; index++) {
		const argument = argumentList[index]!;
		const value = () => {
			const next = argumentList[++index];
			if (next === undefined) fail(`${argument} needs a value`);
			return next;
		};
		if (argument === "--files") options.files = integer(value(), argument, 1);
		else if (argument === "--lines") options.lines = integer(value(), argument, 50);
		else if (argument === "--seed") options.seed = integer(value(), argument, 0);
		else if (argument === "--errors") options.errors = integer(value(), argument, 0);
		else if (argument === "--target-share") {
			options.targetShare = Number(value());
			if (!(options.targetShare >= 0 && options.targetShare <= 1)) {
				fail("--target-share expects a number from 0 to 1");
			}
		} else if (argument === "--no-cycle") options.cycle = false;
		else if (argument === "--output") options.output = resolve(value());
		else if (argument === "--help" || argument === "-h") {
			console.log(
				"Usage: bun benchmarks/compiler/generate.ts [--files <n>] [--lines <n>] [--seed <n>] [--target-share <0..1>] [--errors <n>] [--no-cycle] [--output <directory>]",
			);
			process.exit(0);
		} else fail(`unknown argument ${argument}`);
	}
	return { ...options, output: options.output ?? defaultOutput(options) };
}

function integer(text: string, option: string, minimum: number): number {
	const value = Number(text);
	if (!Number.isInteger(value) || value < minimum) {
		fail(`${option} expects an integer of at least ${minimum}`);
	}
	return value;
}

function fail(message: string): never {
	console.error(`error: ${message}`);
	process.exit(1);
}

if (import.meta.main) {
	const options = parseOptions(process.argv.slice(2));
	const summary = generate(options);
	console.log(
		`wrote ${options.output}: ${summary.files} files, ${summary.lines} lines, ${(summary.bytes / 1e6).toFixed(1)} MB, ` +
			`${summary.levels} levels, ${summary.targetFiles} reading target, ${summary.staticFiles} with statics, ` +
			`cycle ${summary.cycle.join(" → ") || "none"}, errors in ${summary.errors.join(", ") || "none"}`,
	);
}
