// Hot reloading (`fa develop`, decision D43): the modules of a development
// build read the declarations of the other modules through a registry, one
// entry per module (`faHotModule`); a swap loads the new versions of the
// changed modules, points their entries to them and initializes again the
// module-level values `fa` names (`faHotSwap`), when `fa develop` asks for
// it on the channel it opened for the program (`faHotListen`). Reactive
// values keep their value and their source across a swap, by declaration
// path (`faHotValue`, `faHotKeep`).
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

// @fa-helper faHotKeep deps:
/**
 * An object kept across the swaps of its module, by its key (the source of
 * a module-level `reactive` value: `library/state.js#count$s`): created by
 * the first version of the module, reused by the next ones.
 * @template T
 * @param {string} key
 * @param {() => T} create
 * @returns {T}
 */
export function faHotKeep(key, create) {
	/** @type {any} */
	const host = globalThis;
	/** @type {Map<string, any>} */
	const kept = (host.$faHotKept ??= new Map());
	if (!kept.has(key)) {
		kept.set(key, create());
	}
	return kept.get(key);
}

// @fa-helper faHotValue deps:
/**
 * At the initialization of a module-level `reactive` value (`library/state.js#count`):
 * when a previous version of its module holds it, its value is given to
 * the new version (`set`) and `true` is returned (the initializer doesn't
 * run); `get` reads the value of this version from now on.
 * @param {string} key
 * @param {() => unknown} get
 * @param {(value: any) => void} set
 * @returns {boolean}
 */
export function faHotValue(key, get, set) {
	/** @type {any} */
	const host = globalThis;
	/** @type {Map<string, () => unknown>} */
	const values = (host.$faHotValues ??= new Map());
	const previous = values.get(key);
	values.set(key, get);
	if (previous === undefined) {
		return false;
	}
	set(previous());
	return true;
}

// @fa-helper faHotSwap deps: faHotModule
/**
 * Swaps modules of a running development build: reads `$hot.json` in the
 * output folder `root` (`{ version, modules: [{ path, file }], inits: [[path,
 * name]] }`), imports each new version (`file`, written next to the module),
 * points the registry entries to them, then calls the initialization
 * functions in order. The answer says whether it worked; `fa develop`
 * restarts the program when it didn't.
 * @param {string} root the URL of the output folder
 * @returns {Promise<{ version: number, ok: boolean, error?: string }>}
 */
export async function faHotSwap(root) {
	/** @type {any} */
	const host = globalThis;
	let version = 0;
	try {
		const at = new URL("./$hot.json", root);
		// on a server, the file; in a browser, from the development server
		const fs = host.process?.getBuiltinModule?.("node:fs");
		const manifest = fs
			? JSON.parse(
					fs.readFileSync(host.process.getBuiltinModule("node:url").fileURLToPath(at), "utf8"),
				)
			: await (await fetch(at, { cache: "no-store" })).json();
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
		return { version, ok: true };
	} catch (exception) {
		return { version, ok: false, error: String(exception) };
	}
}

// @fa-helper faHotListen deps: faHotSwap
/**
 * Connects to the channel of `fa develop` (a local socket, its port and
 * the token to send first in `FA_HOT_CHANNEL`): each line `swap` swaps the modules (`faHotSwap`), and
 * its answer is sent back as a line of JSON. The channel doesn't keep the
 * program running; without the variable, nothing happens. In a browser
 * (`fa develop --target=web`), the channel is the WebSocket `/$fa-hot` of
 * the development server, which also asks for a full reload (`reload`).
 * @param {string} root the URL of the output folder
 */
export function faHotListen(root) {
	/** @type {any} */
	const host = globalThis;
	if (host.process === undefined && host.location !== undefined && host.WebSocket) {
		const scheme = host.location.protocol === "https:" ? "wss" : "ws";
		const socket = new host.WebSocket(`${scheme}://${host.location.host}/$fa-hot`);
		socket.onmessage = async (/** @type {{ data: string }} */ event) => {
			if (event.data === "reload") {
				host.location.reload();
			} else if (event.data === "swap") {
				socket.send(JSON.stringify(await faHotSwap(root)));
			}
		};
		return;
	}
	// `<port>:<token>`: the token is sent first, `fa develop` closes a
	// connection without it
	const [port, token] = String(host.process?.env?.FA_HOT_CHANNEL ?? "").split(":");
	if (!Number(port)) {
		return;
	}
	const net = host.process.getBuiltinModule("node:net");
	const socket = net.connect(Number(port), "127.0.0.1");
	socket.unref();
	socket.setEncoding("utf8");
	socket.on("error", () => {});
	socket.write(`${token}\n`);
	let buffer = "";
	socket.on("data", async (/** @type {string} */ data) => {
		buffer += data;
		let end;
		while ((end = buffer.indexOf("\n")) >= 0) {
			const line = buffer.slice(0, end);
			buffer = buffer.slice(end + 1);
			if (line === "swap") {
				socket.write(`${JSON.stringify(await faHotSwap(root))}\n`);
			}
		}
	});
}
