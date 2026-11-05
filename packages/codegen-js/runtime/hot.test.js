import { describe, expect, test } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { faHotModule, faHotSwap } from "./hot.js";

describe("hot reloading (D43)", () => {
	test("a registry entry keeps its first namespace until a swap", () => {
		const first = { greet: () => "one" };
		const entry = faHotModule("test/first.js", first);
		expect(faHotModule("test/first.js", { greet: () => "other" })).toBe(entry);
		expect(entry.ns.greet()).toBe("one");
	});

	test("a swap imports the new modules, then initializes the values named", async () => {
		const folder = mkdtempSync(join(tmpdir(), "fa-hot-"));
		try {
			const entry = faHotModule("library/greet.js", { greet: () => "one" });
			writeFileSync(
				join(folder, "greet.hot1.js"),
				"let count;\nexport function $init3() { count = 41; }\nexport const greet = () => `two ${count + 1}`;\n",
			);
			writeFileSync(
				join(folder, "$hot.json"),
				JSON.stringify({
					version: 1,
					modules: [{ path: "library/greet.js", file: "greet.hot1.js" }],
					inits: [["library/greet.js", "$init3"]],
				}),
			);
			await faHotSwap(pathToFileURL(`${folder}/`).href);
			expect(entry.ns.greet()).toBe("two 42");
			expect(JSON.parse(readFileSync(join(folder, "$hot-done.json"), "utf8"))).toEqual({
				version: 1,
				ok: true,
			});
			// a module that doesn't load: the swap fails, and says why
			writeFileSync(
				join(folder, "$hot.json"),
				JSON.stringify({
					version: 2,
					modules: [{ path: "library/greet.js", file: "missing.js" }],
					inits: [],
				}),
			);
			await faHotSwap(pathToFileURL(`${folder}/`).href);
			const done = JSON.parse(readFileSync(join(folder, "$hot-done.json"), "utf8"));
			expect(done.ok).toBe(false);
			expect(entry.ns.greet()).toBe("two 42");
		} finally {
			rmSync(folder, { recursive: true, force: true });
		}
	});
});
