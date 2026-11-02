// Reactivity (advanced brief 11, `advanced/reactivity.md`): push-pull
// signals.
//
// - A reactive value stays a plain JavaScript value; its *source* is a side
//   object (`count$s`): a version number and observers. A reactive
//   collection or object has one source for the whole value and, created on
//   the first read that needs them, child sources for its length or size,
//   for each element or key read individually, and for each field.
// - `faTrack(source)` records the source in the current observer (a derived
//   value or a watch block being computed). `faNotify(source)` bumps the
//   versions of the source, of the sources below it, and of the whole values
//   above it (not of their other children: writing an element doesn't
//   change the length), and pushes the change to their observers: a derived
//   node is only marked dirty, a watch node becomes pending, once.
// - `faRead(derived)` pulls: a dirty derived value checks the versions of
//   what it read last time (pulling the derived ones), and computes again
//   only when one changed. It is always up to date, and computed at most
//   once per change.
// - `faFlush()` runs the pending watch blocks in declaration order, each
//   once, after the statement of the outermost running code that changed
//   what they read; the blocks they make pending run after them, for at
//   most 100 rounds, after which the program stops.
//
// Values that are not reactive never touch this file: the compiler only
// instruments the places the analyzer says are reactive.

import { faStop } from "./stops.js";

// @fa-helper faReactive
/**
 * The reactive state of the thread: the observer being computed, the
 * pending watch blocks, whether they are running, the sequence of the
 * watch blocks (their declaration order).
 * @type {{ observer: FaObserver | null, pending: FaWatchNode[], flushing: boolean, sequence: number }}
 */
export const faReactive = { observer: null, pending: [], flushing: false, sequence: 0 };

/**
 * @typedef {FaDerivedNode | FaWatchNode} FaObserver
 */

// @fa-helper FaSource
/**
 * A source: a version, its observers, and its children (created on demand)
 * below it. The root source of a value has no parent.
 */
export class FaSource {
	/**
	 * @param {FaSource | null} [parent]
	 */
	constructor(parent = null) {
		/** @type {number} */
		this.version = 0;
		/** @type {Set<any> | null} */
		this.observers = null;
		/** @type {FaSource | null} */
		this.parent = parent;
		/** @type {Map<unknown, FaSource> | null} the fields, by name */
		this.fields = null;
		/** @type {Map<unknown, FaSource> | null} the elements or entries, by index or key */
		this.items = null;
		/** @type {FaSource | null} the length or the size */
		this.count = null;
	}

	/**
	 * The source of a field.
	 * @param {string} name
	 * @returns {FaSource}
	 */
	field(name) {
		this.fields ??= new Map();
		let child = this.fields.get(name);
		if (child === undefined) {
			child = new FaSource(this);
			this.fields.set(name, child);
		}
		return child;
	}

	/**
	 * The source of an element (by index) or of a map entry (by key).
	 * @param {unknown} key
	 * @returns {FaSource}
	 */
	index(key) {
		this.items ??= new Map();
		let child = this.items.get(key);
		if (child === undefined) {
			child = new FaSource(this);
			this.items.set(key, child);
		}
		return child;
	}

	/**
	 * The source of the length (or the size).
	 * @returns {FaSource}
	 */
	length() {
		this.count ??= new FaSource(this);
		return this.count;
	}
}

// @fa-helper faSource deps: FaSource
/**
 * A new source, for a `reactive` value.
 * @returns {FaSource}
 */
export function faSource() {
	return new FaSource();
}

// @fa-helper faFieldSource deps: FaSource
/**
 * The source of a `reactive` field of an object: created on demand, and
 * kept outside the object, so that a copy of the object (which copies its
 * properties) starts without dependents.
 * @param {object} object
 * @param {string} name
 * @returns {FaSource}
 */
export function faFieldSource(object, name) {
	let sources = faFieldSources.get(object);
	if (sources === undefined) {
		sources = new Map();
		faFieldSources.set(object, sources);
	}
	let source = sources.get(name);
	if (source === undefined) {
		source = new FaSource();
		sources.set(name, source);
	}
	return source;
}

/** @type {WeakMap<object, Map<string, FaSource>>} */
const faFieldSources = new WeakMap();

// @fa-helper faTrack deps: faReactive
/**
 * Records that the observer being computed read `source` (a source or a
 * derived node): nothing outside a derived value or a watch block, nor for
 * a missing source (a parameter of a reactive instance given a plain value).
 * @param {{ version: number, observers: Set<any> | null } | null | undefined} source
 * @returns {void}
 */
export function faTrack(source) {
	const observer = faReactive.observer;
	if (observer === null || source == null) {
		return;
	}
	const sources = observer.sources;
	for (let index = 0; index < sources.length; index += 2) {
		if (sources[index] === source) {
			return;
		}
	}
	sources.push(source, source.version);
	source.observers ??= new Set();
	source.observers.add(observer);
}

// @fa-helper faMark deps: faReactive
/**
 * Pushes a change to an observer: a derived node becomes dirty (and so do
 * its observers, transitively); a watch node becomes pending, once.
 * @param {any} observer
 * @returns {void}
 */
export function faMark(observer) {
	if (observer.watch) {
		if (!observer.pending && !observer.disposed) {
			observer.pending = true;
			faReactive.pending.push(observer);
		}
		return;
	}
	if (observer.dirty) {
		return;
	}
	observer.dirty = true;
	if (observer.observers !== null) {
		for (const next of observer.observers) {
			faMark(next);
		}
	}
}

// @fa-helper faNotify deps: faMark
/**
 * A write of the place of `source`: bumps it and the sources below it, then
 * the whole values above it, and pushes the change to their observers.
 * Nothing for a missing source (a parameter of a reactive instance given a
 * plain value).
 * @param {import("./reactive.js").FaSource | null | undefined} source
 * @returns {void}
 */
export function faNotify(source) {
	if (source == null) {
		return;
	}
	faBump(source);
	for (let parent = source.parent; parent !== null; parent = parent.parent) {
		parent.version += 1;
		if (parent.observers !== null) {
			for (const observer of [...parent.observers]) {
				faMark(observer);
			}
		}
	}
}

/**
 * Bumps a source and every source below it.
 * @param {import("./reactive.js").FaSource} source
 * @returns {void}
 */
function faBump(source) {
	source.version += 1;
	if (source.observers !== null) {
		for (const observer of [...source.observers]) {
			faMark(observer);
		}
	}
	if (source.fields !== null) {
		for (const child of source.fields.values()) {
			faBump(child);
		}
	}
	if (source.items !== null) {
		for (const child of source.items.values()) {
			faBump(child);
		}
	}
	if (source.count !== null) {
		faBump(source.count);
	}
}

// @fa-helper faChanged deps: faNotify
/**
 * Notifies sources after a call that changed them, and gives the call's value.
 * @template T
 * @param {T} value
 * @param {...(import("./reactive.js").FaSource | null | undefined)} sources
 * @returns {T}
 */
export function faChanged(value, ...sources) {
	for (const source of sources) {
		faNotify(source);
	}
	return value;
}

// @fa-helper faUnsubscribe
/**
 * Removes an observer from the observers of what it read, and forgets it.
 * @param {{ sources: any[] }} observer
 * @returns {void}
 */
export function faUnsubscribe(observer) {
	const sources = observer.sources;
	for (let index = 0; index < sources.length; index += 2) {
		sources[index].observers?.delete(observer);
	}
	observer.sources = [];
}

// @fa-helper faOutdated deps: faRefresh
/**
 * Whether one of the sources an observer read changed since: the derived
 * ones are brought up to date first.
 * @param {{ sources: any[] }} observer
 * @returns {boolean}
 */
export function faOutdated(observer) {
	const sources = observer.sources;
	for (let index = 0; index < sources.length; index += 2) {
		const source = sources[index];
		if (source.compute !== undefined) {
			faRefresh(source);
		}
		if (source.version !== sources[index + 1]) {
			return true;
		}
	}
	return false;
}

// @fa-helper FaDerivedNode
/**
 * A derived value: its computation, its cached value, whether it may be
 * stale, what it read last time (`[source, version, …]`), and its own
 * version and observers (it is the source of what reads it).
 */
export class FaDerivedNode {
	/**
	 * @param {() => unknown} compute
	 */
	constructor(compute) {
		/** @type {() => unknown} */
		this.compute = compute;
		/** @type {unknown} */
		this.value = undefined;
		/** @type {boolean} never computed, or something it read changed */
		this.dirty = true;
		/** @type {boolean} */
		this.computed = false;
		/** @type {any[]} */
		this.sources = [];
		/** @type {number} */
		this.version = 0;
		/** @type {Set<any> | null} */
		this.observers = null;
		/** @type {boolean} */
		this.computing = false;
	}
}

// @fa-helper faDerived deps: FaDerivedNode
/**
 * A `derived` value: computed when it is read.
 * @param {() => unknown} compute
 * @returns {FaDerivedNode}
 */
export function faDerived(compute) {
	return new FaDerivedNode(compute);
}

// @fa-helper faRefresh deps: faReactive faUnsubscribe faOutdated faStop
/**
 * Brings a derived value up to date: computes it again when it was never
 * computed, or when what it read changed. Its version changes only when its
 * value does (a flat value equal to the previous one changes nothing).
 * @param {import("./reactive.js").FaDerivedNode} node
 * @returns {void}
 */
export function faRefresh(node) {
	if (!node.dirty) {
		return;
	}
	if (node.computed && !faOutdated(node)) {
		node.dirty = false;
		return;
	}
	if (node.computing) {
		faStop("a derived value depends on itself");
	}
	faUnsubscribe(node);
	const saved = faReactive.observer;
	faReactive.observer = node;
	node.computing = true;
	let value;
	try {
		value = node.compute();
	} finally {
		node.computing = false;
		faReactive.observer = saved;
	}
	node.dirty = false;
	const changed = !node.computed || !Object.is(value, node.value) || typeof value === "object";
	node.computed = true;
	node.value = value;
	if (changed) {
		node.version += 1;
	}
}

// @fa-helper faRead deps: faRefresh faTrack
/**
 * Reads a derived value: up to date, and tracked by the observer being
 * computed.
 * @param {import("./reactive.js").FaDerivedNode} node
 * @returns {any}
 */
export function faRead(node) {
	faRefresh(node);
	faTrack(node);
	return node.value;
}

// @fa-helper FaWatchNode
/**
 * A watch block: its body, its cleanup, what it read last time, its
 * sequence number (declaration order), whether it is pending or disposed.
 */
export class FaWatchNode {
	/**
	 * @param {() => unknown} run
	 * @param {number} sequence
	 */
	constructor(run, sequence) {
		/** @type {() => unknown} */
		this.run = run;
		/** @type {number} */
		this.sequence = sequence;
		/** @type {(() => void) | undefined} */
		this.cleanup = undefined;
		/** @type {any[]} */
		this.sources = [];
		/** @type {boolean} */
		this.pending = false;
		/** @type {boolean} */
		this.disposed = false;
		/** @type {true} */
		this.watch = true;
	}
}

// @fa-helper faRunWatch deps: faReactive faUnsubscribe
/**
 * Runs a watch block: its previous cleanup (untracked), then its body under
 * tracking; a function returned by the body is the next cleanup.
 * @param {import("./reactive.js").FaWatchNode} node
 * @returns {void}
 */
export function faRunWatch(node) {
	const cleanup = node.cleanup;
	node.cleanup = undefined;
	faUnsubscribe(node);
	const saved = faReactive.observer;
	faReactive.observer = null;
	try {
		if (cleanup !== undefined) {
			cleanup();
		}
		faReactive.observer = node;
		const result = node.run();
		if (typeof result === "function") {
			node.cleanup = /** @type {() => void} */ (result);
		}
	} finally {
		faReactive.observer = saved;
	}
}

// @fa-helper faWatch deps: faReactive FaWatchNode faRunWatch
/**
 * A `watch` block: runs at once, and again after a value it read changed.
 * @param {() => unknown} run the body; it may return its cleanup
 * @returns {FaWatchNode}
 */
export function faWatch(run) {
	faReactive.sequence += 1;
	const node = new FaWatchNode(run, faReactive.sequence);
	faRunWatch(node);
	return node;
}

// @fa-helper faDispose deps: faReactive faUnsubscribe
/**
 * Destroys a watch block (`subscription.stop()`, the end of its scope) or a
 * derived value: runs the cleanup, and leaves the observers of what it read.
 * @param {any} node
 * @returns {void}
 */
export function faDispose(node) {
	if (node.disposed) {
		return;
	}
	node.disposed = true;
	faUnsubscribe(node);
	const cleanup = node.cleanup;
	node.cleanup = undefined;
	if (cleanup !== undefined) {
		const saved = faReactive.observer;
		faReactive.observer = null;
		try {
			cleanup();
		} finally {
			faReactive.observer = saved;
		}
	}
}

// @fa-helper faFlush deps: faReactive faOutdated faRunWatch faStop
/**
 * Runs the pending watch blocks, in declaration order, each once; then the
 * ones they made pending, for at most 100 rounds. Nothing while they run
 * (the running flush takes the new ones).
 * @returns {void}
 */
export function faFlush() {
	if (faReactive.flushing || faReactive.pending.length === 0) {
		return;
	}
	faReactive.flushing = true;
	try {
		let rounds = 0;
		while (faReactive.pending.length > 0) {
			rounds += 1;
			if (rounds > 100) {
				faReactive.pending = [];
				faStop("the `watch` blocks kept changing what they read: stopped after 100 rounds");
			}
			const batch = faReactive.pending.sort((a, b) => a.sequence - b.sequence);
			faReactive.pending = [];
			for (const node of batch) {
				node.pending = false;
				if (!node.disposed && faOutdated(node)) {
					faRunWatch(node);
				}
			}
		}
	} finally {
		faReactive.flushing = false;
	}
}
