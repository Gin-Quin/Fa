// Hot reloading (`fa develop`, decision D43): the modules of a development
// build read the declarations of the other modules through a registry, one
// entry per module (`faHotModule`); a swap loads the new versions of the
// changed modules, points their entries to them and initializes again the
// module-level values `fa` names (`faHotListen`).
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one (`runtime.rs`).

// @fa-helper faHotModule deps:
/**
 * The registry entry of a module of a development build, by its path from
 * the output folder (`library/greet.js`): `ns` is the module's current
 * namespace. The first call creates it with `namespace`; a swap replaces
 * `ns`, so that every reader sees the new declarations.
 * @param {string} path
 * @param {object} [namespace]
 * @returns {{ ns: any }}
 */
export function faHotModule(path, namespace) {
	/** @type {any} */
	const host = globalThis;
	/** @type {Map<string, { ns: any }>} */
	const registry = (host.$faHot ??= new Map());
	let entry = registry.get(path);
	if (entry === undefined) {
		entry = { ns: namespace };
		registry.set(path, entry);
	}
	return entry;
}

// @fa-helper faHotSwap deps: faHotModule
/**
 * Swaps modules of a running development build: reads `$hot.json` in the
 * output folder `root` (`{ version, modules: [{ path, file }], inits: [[path,
 * name]] }`), imports each new version (`file`, written next to the module),
 * points the registry entries to them, then calls the initialization
 * functions in order. Writes `$hot-done.json` (`{ version, ok, error? }`)
 * for `fa develop`, which restarts the program when the swap failed.
 * @param {string} root the URL of the output folder
 * @returns {Promise<void>}
 */
export async function faHotSwap(root) {
	/** @type {any} */
	const host = globalThis;
	const fs = host.process.getBuiltinModule("node:fs");
	const url = host.process.getBuiltinModule("node:url");
	const at = (/** @type {string} */ name) => url.fileURLToPath(new URL(name, root));
	let version = 0;
	/** @type {{ version: number, ok: boolean, error?: string }} */
	let result;
	try {
		const manifest = JSON.parse(fs.readFileSync(at("./$hot.json"), "utf8"));
		version = manifest.version;
		/** @type {Array<[string, any]>} */
		const loaded = [];
		for (const { path, file } of manifest.modules) {
			loaded.push([path, await import(new URL(`./${file}`, root).href)]);
		}
		for (const [path, namespace] of loaded) {
			faHotModule(path, namespace).ns = namespace;
		}
		for (const [path, name] of manifest.inits) {
			faHotModule(path).ns[name]();
		}
		result = { version, ok: true };
	} catch (exception) {
		result = { version, ok: false, error: String(exception) };
	}
	fs.writeFileSync(at("./$hot-done.json"), JSON.stringify(result));
}

// @fa-helper faHotListen deps: faHotSwap
/**
 * Swaps modules when `fa develop` sends `SIGUSR2` (`faHotSwap`); the
 * listener doesn't keep the program running.
 * @param {string} root the URL of the output folder
 */
export function faHotListen(root) {
	/** @type {any} */
	const host = globalThis;
	host.process?.on?.("SIGUSR2", () => {
		void faHotSwap(root);
	});
}
