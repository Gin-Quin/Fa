// Checks that every Fa snippet of the landing page parses, with the `fa-parser` binary of the repository.
// Build it first with `cargo build -p fa-parser`.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import {
	agentLoop,
	features,
	heroFiles,
	memory,
	signature,
	signatureParts,
} from "../src/lib/landing/snippets.ts";

const parser = resolve(import.meta.dir, "../../../target/debug/fa-parser");

const snippets: Record<string, string> = {
	...Object.fromEntries(heroFiles.map((file) => [file.name, file.code])),
	signature,
	...Object.fromEntries(signatureParts.map((part) => [`signature.${part.kind}`, part.call])),
	...Object.fromEntries(Object.entries(features).map(([name, code]) => [`features.${name}`, code])),
	"memory.invalidation": memory.invalidation,
	"memory.precise": memory.precise,
	"agentLoop.before": agentLoop.before,
	"agentLoop.after": agentLoop.after,
};

const folder = mkdtempSync(join(tmpdir(), "fa-snippets-"));
let failures = 0;

for (const [name, code] of Object.entries(snippets)) {
	const file = join(folder, `${name}.fa`);
	writeFileSync(file, code);
	const result = Bun.spawnSync([parser, file], { stderr: "pipe", stdout: "pipe" });
	if (result.exitCode !== 0) {
		failures += 1;
		console.error(`✗ ${name}\n${result.stderr.toString().trim()}\n`);
	} else {
		console.log(`✓ ${name}`);
	}
}

rmSync(folder, { recursive: true });

if (failures > 0) {
	console.error(`${failures} snippet(s) do not parse`);
	process.exit(1);
}
