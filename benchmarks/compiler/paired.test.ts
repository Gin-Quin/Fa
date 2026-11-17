import { expect, test } from "bun:test";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import { paired } from "./paired.ts";

// A cross-language execution oracle catches dropped returns, altered constants,
// broken union arms and graph edges; a clean type check alone cannot catch these.
test("paired modules check and produce the same result, including statics and cycles", () => {
	const directory = mkdtempSync(join(tmpdir(), "fa-check-pair-"));
	const fa = resolve("target/release/fa");
	try {
		const fixture = paired(directory, 4, 100);
		for (const seed of [1, 4]) {
			paired(directory, 4, 100, seed);
			execFileSync(fa, ["check", fixture.faInput]);
			execFileSync(process.execPath, [
				"check",
				"--no-script",
				"-p",
				fixture.tsInput,
				"--threads=1",
			]);
			const expected = execFileSync(fa, ["run", fixture.faInput], {
				env: { ...process.env, PATH: `${process.env.HOME}/.bun/bin:${process.env.PATH}` },
			});
			const runner = join(fixture.tsInput, "execute.ts");
			writeFileSync(runner, 'import { main } from "./main.ts"; main([]);\n');
			const actual = execFileSync(process.execPath, [runner]);
			expect(actual.toString()).toBe(expected.toString());
		}
		const module = join(fixture.tsInput, "library/m0001.ts");
		writeFileSync(module, readFileSync(module, "utf8") + '\nconst wrong: number = "error";\n');
		expect(
			spawnSync(process.execPath, ["check", "--no-script", "-p", fixture.tsInput]).status,
		).toBe(1);
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
}, 60_000);

test("single-file fixture has no cross-file dependency", () => {
	const directory = mkdtempSync(join(tmpdir(), "fa-check-single-"));
	try {
		const fixture = paired(directory, 1, 500);
		expect(fixture.cycle).toEqual([]);
		expect(fixture.targetFiles).toBe(0);
		const source = readFileSync(join(fixture.tsInput, "library/m0001.ts"), "utf8");
		expect(source).not.toContain("import ");
		expect(source).toContain("return sum;");
		expect(source).not.toContain("return sum +=");
	} finally {
		rmSync(directory, { recursive: true, force: true });
	}
});
