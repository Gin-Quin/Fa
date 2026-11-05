import { describe, expect, test } from "bun:test";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { faHotKeep, faHotModule, faHotSwap, faHotValue } from "./hot.js";

describe("hot reloading (D43)", () => {
	test("a registry entry keeps its first namespace until a swap", () => {
		const first = { greet: () => "one" };
		const entry = faHotModule("test/first.js", first);
		expect(faHotModule("test/first.js", { greet: () => "other" })).toBe(entry);
		expect(entry.ns.greet()).toBe("one");
	});

	test("a reactive value and its source are kept by declaration path", () => {
		const source = faHotKeep("test/state.js#count$s", () => ({ observers: [] }));
		expect(faHotKeep("test/state.js#count$s", () => ({ observers: [] }))).toBe(source);
		let first = 0;
		expect(
			faHotValue(
				"test/state.js#count",
				() => first,
				(value) => (first = value),
			),
		).toBe(false);
		first = 7;
		let second = 0;
		expect(
			faHotValue(
				"test/state.js#count",
				() => second,
				(value) => (second = value),
			),
		).toBe(true);
		expect(second).toBe(7);
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
			const root = pathToFileURL(`${folder}/`).href;
			expect(await faHotSwap(root)).toEqual({ version: 1, ok: true });
			expect(entry.ns.greet()).toBe("two 42");
			// a module that doesn't load: the swap fails, and says why
			writeFileSync(
				join(folder, "$hot.json"),
				JSON.stringify({
					version: 2,
					modules: [{ path: "library/greet.js", file: "missing.js" }],
					inits: [],
				}),
			);
			const failed = await faHotSwap(root);
			expect(failed.ok).toBe(false);
			expect(entry.ns.greet()).toBe("two 42");
		} finally {
			rmSync(folder, { recursive: true, force: true });
		}
	});
});
