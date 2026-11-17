// A TypeScript renderer for generate.ts's deliberately small fixture grammar.
// It consumes the exact seeded Fa text, preserving constants, bodies and call edges.
// This is not a general Fa translator: unsupported fixture lines fail generation.
import { mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { generate } from "./generate.ts";

export function typescriptModule(source: string): string {
	const output: string[] = [];
	let section = "";
	let expressionBody = false;
	for (const original of source.trimEnd().split("\n").slice(1, -1)) {
		const line = original.trim();
		if (line === "type Item = {") {
			section = "item";
			output.push("export class Item {");
			continue;
		}
		if (section === "item") {
			if (line === "}") {
				output.push("}");
				section = "";
			} else if (line.startsWith("function score")) {
				output.push(
					`score(): number { return ${line.split("=> ")[1]!.replaceAll("self.", "this.")}; }`,
				);
			} else {
				output.push(
					line.replace(":", "!:").replace("Integer", "number").replace("String", "string") + ";",
				);
			}
			continue;
		}
		if (line === "type Shape = Union {") {
			section = "shape";
			output.push(
				'export type Shape = { kind: "Circle"; radius: number } | { kind: "Square"; side: number } | { kind: "Empty" };',
			);
			continue;
		}
		if (section === "shape") {
			if (line === "}") section = "";
			continue;
		}
		if (line.startsWith("type Level")) {
			output.push("export enum Level { low, middle, high }");
			continue;
		}
		if (line.startsWith("type Failure")) {
			output.push(
				"export class Missing extends Error { constructor(readonly key: string) { super(key); } }",
			);
			continue;
		}
		if (/^function area\d/.test(line)) {
			output.push(
				line.replace(
					/function (\w+) = \(shape: Shape\): Integer => when shape is \{/,
					"export function $1(shape: Shape): number { switch (shape.kind) {",
				),
			);
			section = "area";
			continue;
		}
		if (section === "area") {
			if (line === "}") {
				output.push("} }");
				section = "";
			} else {
				const match = /^Shape\.(\w+)(?: as \{ (\w+) \})? => (.*)$/.exec(line);
				if (!match) throw new Error(`Unrecognized union arm: ${line}`);
				const body = match[2]
					? match[3]!.replaceAll(new RegExp(`\\b${match[2]}\\b`, "g"), `shape.${match[2]}`)
					: match[3];
				output.push(`case "${match[1]}": return ${body};`);
			}
			continue;
		}
		if (/^when level\d/.test(line)) {
			output.push(line.replace(/^when (.*) is \{$/, "switch ($1) {"));
			continue;
		}
		if (/^Level\.\w+ =>/.test(line)) {
			output.push(line.replace(/^(Level\.\w+) => (.*)$/, "case $1: $2; break;"));
			continue;
		}
		let text = line
			.replaceAll("Integer", "number")
			.replaceAll("String", "string")
			.replaceAll(" %% ", " % ")
			.replace(/"([^"{}]*)\{(\w+)\}([^"{}]*)"/g, "`$1${$2}$3`")
			.replace(
				/Item \{ (.*) \}/,
				(_, fields: string) => `Object.assign(new Item(), { ${fields.replaceAll(" = ", ": ")} })`,
			)
			.replace(/Shape\.(Circle|Square) \{ (\w+) = ([^}]+) \}/g, '{ kind: "$1", $2: $3 }')
			.replace(/throw Failure.Missing \{ key \}/, "throw new Missing(key)")
			.replace(
				/for index in 0\.\.items.length \{/,
				"for (let index = 0; index < items.length; index++) {",
			)
			.replace(/for item in items \{/, "for (const item of items) {")
			.replace(/if item.label == key \{/, "if (item.label === key) {")
			.replace(/if x <= 0 \{/, "if (x <= 0) {")
			.replace(/values.get\(0\)/, "values[0]")
			.replace(/throws Failure.Missing /, "")
			.replace(/^mutable /, "let ")
			.replace(/^let (items|shape) = /, "const $1 = ")
			.replace(/^const shape = /, "const shape: Shape = ")
			.replace(/^static /, "export const ")
			.replaceAll("@square", "square")
			.replace(
				/if (.*) \{ \.low \} else if (.*) \{ \.middle \} else \{ \.high \}/,
				"($1) ? Level.low : ($2) ? Level.middle : Level.high",
			)
			.replace(/if (x % 2 == 0) \{ (\{.*?\}) \} else \{ (\{.*?\}) \}/, "($1) ? $2 : $3");
		if (/^function safe\d/.test(text)) {
			text = text.replace(
				/=> (.*) catch _error => -1/,
				"=> { try { return $1; } catch { return -1; } }",
			);
		}
		if (text.startsWith("function ")) {
			expressionBody = text.endsWith("=> {");
			text = text.replace(/^function (\w+) = (<Value>)?\(/, "export const $1 = $2(");
			if (!expressionBody) text += ";";
		} else if (expressionBody && /^(sum$|items$|result %|\w+\.call\()/.test(text)) {
			text = `return ${text};`;
		} else if (!text.endsWith("{") && text !== "}") {
			text += ";";
		}
		if (/\b(function|when|throws|static|mutable|Integer|String)\b|=> -1|@|%%/.test(text)) {
			throw new Error(`Unrecognized fixture line: ${line} -> ${text}`);
		}
		output.push(text);
	}
	const modules = [...source.matchAll(/\b(m\d+|ring[A-C])\./g)].map((match) => match[1]!);
	const imports = [...new Set(modules)]
		.sort()
		.map((name) => `import * as ${name} from "./${name}.ts";`);
	return [...imports, ...output, ""].join("\n");
}

export function paired(output: string, files: number, lines = 500, seed = 1) {
	const fa = join(output, "fa");
	const ts = join(output, "ts");
	const single = files === 1;
	const summary = generate({
		files,
		lines,
		seed,
		targetShare: 0,
		errors: 0,
		cycle: !single,
		tests: 0,
		output: fa,
	});
	rmSync(ts, { recursive: true, force: true });
	mkdirSync(join(ts, "library"), { recursive: true });
	for (const name of readdirSync(join(fa, "library"))) {
		writeFileSync(
			join(ts, "library", name.replace(".fa", ".ts")),
			typescriptModule(readFileSync(join(fa, "library", name), "utf8")),
		);
	}
	const main = readFileSync(join(fa, "application/main.fa"), "utf8");
	const names = [...new Set([...main.matchAll(/library\.(m\d+)\./g)].map((match) => match[1]!))];
	writeFileSync(
		join(ts, "main.ts"),
		[
			...names.map((name) => `import * as ${name} from "./library/${name}.ts";`),
			"export const main = (_arguments: string[]): number => {",
			"let total = 0;",
			...main
				.split("\n")
				.filter((line) => line.includes("total +="))
				.map((line) => line.replaceAll("library.", "").replaceAll("%%", "%") + ";"),
			"console.log(total);",
			"return 0;",
			"};",
			"",
		].join("\n"),
	);
	// Resolve standard declarations and the compiler from the repository-pinned dependency.
	writeFileSync(
		join(ts, "tsconfig.json"),
		JSON.stringify(
			{
				compilerOptions: {
					strict: true,
					noEmit: true,
					skipLibCheck: true,
					types: [],
					lib: ["ES2022", "DOM"],
					module: "Preserve",
					moduleResolution: "bundler",
					target: "ES2022",
					allowImportingTsExtensions: true,
				},
				include: single ? ["library/m0001.ts"] : ["**/*.ts"],
			},
			null,
			"\t",
		) + "\n",
	);
	writeFileSync(join(ts, "package.json"), '{"private":true,"type":"module"}\n');
	if (single) writeFileSync(join(fa, "single.fa"), readFileSync(join(fa, "library/m0001.fa")));
	return { ...summary, single, faInput: single ? join(fa, "single.fa") : fa, tsInput: ts };
}

if (import.meta.main) {
	console.log(
		JSON.stringify(
			paired(
				resolve(process.argv[2]!),
				Number(process.argv[3] ?? 100),
				Number(process.argv[4] ?? 500),
			),
		),
	);
}
