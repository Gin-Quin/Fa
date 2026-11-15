// Executes emitted web ES modules with browser globals only. Node APIs belong
// to this host, never the context or the module linker (including in workers).
import { readFile } from "node:fs/promises";
import { fileURLToPath, pathToFileURL } from "node:url";
import vm from "node:vm";
import {
	MessageChannel as NodeMessageChannel,
	parentPort,
	Worker as NodeWorker,
	workerData,
} from "node:worker_threads";

const entry = workerData?.entry ?? pathToFileURL(process.argv[2]).href;
const root = new URL("./", entry).href;
const workers = new Set();
const timers = new Map();
const ports = new Set();
let nextTimer = 0;
let ended = false;
// The generator directly invokes a synchronous main that cannot fail and
// returns void; those modules have no faExit and finish naturally with 0.

function finish(code) {
	if (parentPort) throw new Error("a worker unexpectedly ran a program entry");
	if (ended) throw new Error("the web program reported its status twice");
	ended = true;
	process.exitCode = Number(code);
	for (const worker of workers) worker.terminate();
	for (const timer of timers.values()) clearTimeout(timer);
	for (const port of ports) port.close();
}

// Node ports and timer handles expose server-only methods. Keep them private;
// generated code receives only the corresponding web interfaces.
class MessagePort extends EventTarget {
	#port;
	onmessage = null;
	onmessageerror = null;
	constructor(port) {
		super();
		this.#port = port;
		ports.add(this);
		port.on("message", (data) => {
			const event = new MessageEvent("message", { data: cloneIntoContext(data) });
			this.onmessage?.(event);
			this.dispatchEvent(event);
		});
	}
	postMessage(data, transfer) {
		this.#port.postMessage(data, transfer);
	}
	start() {
		this.#port.start();
	}
	close() {
		ports.delete(this);
		this.#port.close();
	}
}
class MessageChannel {
	constructor() {
		const channel = new NodeMessageChannel();
		this.port1 = new MessagePort(channel.port1);
		this.port2 = new MessagePort(channel.port2);
	}
}

class Worker extends EventTarget {
	#thread;
	onmessage = null;
	onerror = null;
	constructor(url, options) {
		super();
		if (options?.type !== "module") throw new Error("only module Web Workers are supported");
		const resolved = moduleURL(String(url), entry);
		this.#thread = new NodeWorker(new URL(import.meta.url), {
			workerData: { entry: resolved },
		});
		workers.add(this);
		this.#thread.on("message", (data) => {
			const event = new MessageEvent("message", { data: cloneIntoContext(data) });
			this.onmessage?.(event);
			this.dispatchEvent(event);
		});
		this.#thread.on("error", (error) => {
			const event = new Event("error");
			this.onerror?.(event);
			this.dispatchEvent(event);
			// Unhandled worker exceptions must fail the test, not disappear.
			console.error(error);
			process.exitCode = 1;
		});
	}
	postMessage(data, transfer) {
		this.#thread.postMessage(data, transfer);
	}
	terminate() {
		workers.delete(this);
		void this.#thread.terminate();
	}
}

function moduleURL(specifier, referrer) {
	const url = new URL(specifier, referrer);
	if (url.protocol !== "file:" || !url.href.startsWith(root)) {
		throw new Error(`web sandbox refuses import ${specifier}`);
	}
	return url.href;
}

const events = new EventTarget();
const globals = {
	console: {
		log: (...args) => console.log(...args),
		error: (...args) => console.error(...args),
		warn: (...args) => console.warn(...args),
		info: (...args) => console.info(...args),
		debug: (...args) => console.debug(...args),
	},
	TextEncoder,
	TextDecoder,
	URL,
	URLSearchParams,
	fetch,
	Headers,
	Request,
	Response,
	AbortController,
	AbortSignal,
	Blob,
	Event,
	EventTarget,
	MessageEvent,
	MessageChannel,
	MessagePort,
	Worker,
	crypto: globalThis.crypto,
	crossOriginIsolated: true,
	performance: { now: () => performance.now(), timeOrigin: performance.timeOrigin },
	structuredClone: (value, options) => cloneIntoContext(structuredClone(value, options)),
	queueMicrotask,
	atob,
	btoa,
	setTimeout(callback, delay, ...args) {
		const id = ++nextTimer;
		timers.set(
			id,
			setTimeout(() => {
				timers.delete(id);
				callback(...args);
			}, delay),
		);
		return id;
	},
	clearTimeout(id) {
		clearTimeout(timers.get(id));
		timers.delete(id);
	},
	setInterval(callback, delay, ...args) {
		const id = ++nextTimer;
		timers.set(id, setInterval(callback, delay, ...args));
		return id;
	},
	clearInterval(id) {
		clearInterval(timers.get(id));
		timers.delete(id);
	},
	addEventListener: events.addEventListener.bind(events),
	removeEventListener: events.removeEventListener.bind(events),
	dispatchEvent: events.dispatchEvent.bind(events),
	onmessage: null,
};
if (parentPort) globals.postMessage = (data, transfer) => parentPort.postMessage(data, transfer);
const context = vm.createContext(globals);
// Built-ins (including WebAssembly, SharedArrayBuffer and Atomics) come from
// the context's own realm, preserving instanceof checks in generated code.
vm.runInContext("globalThis.self = globalThis", context);
if (parentPort) {
	vm.runInContext(
		`
		globalThis.WorkerGlobalScope = class WorkerGlobalScope extends EventTarget {};
		globalThis.DedicatedWorkerGlobalScope = class DedicatedWorkerGlobalScope extends WorkerGlobalScope {};
		Object.setPrototypeOf(globalThis, DedicatedWorkerGlobalScope.prototype);
	`,
		context,
	);
}

// Structured-clone results belong to the receiving realm in a browser.
// Node delivers host-realm objects: restore their standard prototypes so that
// generated instanceof / Object.getPrototypeOf checks see browser semantics.
const cloneIntoContext = vm.runInContext(
	`(value) => {
	const seen = new WeakSet();
	function revive(value) {
		if (value === null || typeof value !== "object" || seen.has(value)) return value;
		seen.add(value);
		const kind = Object.prototype.toString.call(value).slice(8, -1);
		const type = globalThis[kind];
		if (typeof type === "function") Object.setPrototypeOf(value, type.prototype);
		if (kind === "Map") {
			for (const [key, item] of value) { revive(key); revive(item); }
		} else if (kind === "Set") {
			for (const item of value) revive(item);
		} else if (ArrayBuffer.isView(value)) {
			revive(value.buffer);
		} else {
			for (const item of Object.values(value)) revive(item);
		}
		return value;
	}
	return revive(value);
}`,
	context,
);
const modules = new Map();
const evaluations = new Map();
const status = new vm.SyntheticModule(
	["recordExit"],
	function () {
		this.setExport("recordExit", finish);
	},
	{ context },
);

async function load(url) {
	if (!modules.has(url)) {
		modules.set(
			url,
			(async () => {
				let source = await readFile(fileURLToPath(url), "utf8");
				// faExit is a no-op on web. Observe its argument without supplying a
				// process shim or changing the runtime's error/status decisions.
				if (source.includes("function faExit(code) {")) {
					source =
						'import { recordExit as $harnessExit } from "fa-harness:status";\n' +
						source.replace(
							"function faExit(code) {",
							"function faExit(code) { $harnessExit(code);",
						);
				}
				return new vm.SourceTextModule(source, {
					context,
					identifier: url,
					initializeImportMeta(meta) {
						meta.url = url;
					},
					async importModuleDynamically(specifier, referrer) {
						const dependency = await load(moduleURL(specifier, referrer.identifier));
						await evaluate(dependency);
						return dependency;
					},
				});
			})(),
		);
	}
	return modules.get(url);
}

async function link(specifier, referrer) {
	if (specifier === "fa-harness:status") return status;
	return load(moduleURL(specifier, referrer.identifier));
}

async function evaluate(module) {
	if (!evaluations.has(module.identifier)) {
		evaluations.set(
			module.identifier,
			(async () => {
				if (module.status === "unlinked") await module.link(link);
				await module.evaluate();
			})(),
		);
	}
	return evaluations.get(module.identifier);
}

// Buffer messages until the worker has installed its web onmessage handler.
const messages = [];
let ready = false;
function receive(data) {
	const event = new MessageEvent("message", { data: cloneIntoContext(data) });
	globals.onmessage?.(event);
	events.dispatchEvent(event);
}
parentPort?.on("message", (data) => (ready ? receive(data) : messages.push(data)));
try {
	await evaluate(await load(entry));
	ready = true;
	for (const message of messages) receive(message);
} catch (error) {
	ended = true;
	console.error(error);
	process.exitCode = 1;
	for (const worker of workers) worker.terminate();
}
