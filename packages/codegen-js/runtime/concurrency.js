// The thread-safe types of `std.concurrency` on JavaScript (advanced brief
// 14.3, D31): a program runs on one thread, so they are plain objects
// updated with plain operations. The methods that wait (`lock`, `send`,
// `receive`) are awaited by tasks (`tasks.js`: `faLock`, `faSend`,
// `faReceive`), which queue as waiters here; a blocking call of them is a
// compile error on JavaScript (`CONC010`). The waiters are served in order,
// and the value or the lock is handed over directly (design/async.md A16).
// A copy of a handle is the same object.
//
// Given to a worker (step J2, `workers.js`), an atomic or a mutex is first
// moved into shared memory (`share`: a `SharedArrayBuffer`, which the
// structured clone of the message shares): an atomic's value is then a
// cell updated with `Atomics`, a mutex's lock a word taken with
// `compareExchange` and its value JSON text read by whoever holds it; a
// channel's queue is JSON text too, behind a lock word, with a version
// word that waiting tasks (`Atomics.waitAsync`) and blocked workers
// (`Atomics.wait`) wait on. Without shared memory (a web page
// that isn't cross-origin isolated) the program stops with what to do
// (`faNeedSharedMemory`): Bun and Node always have it.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { faDropOwned, faOwnShared, faReleaseShared, faRetainShared } from "./ownership.js";
import { faStop } from "./stops.js";

// @fa-helper faNeedSharedMemory deps: faStop
/**
 * Stops the program when this platform can't share memory with a worker:
 * a web page that isn't cross-origin isolated has no `SharedArrayBuffer`.
 * @param {string} what like `a Channel`
 * @returns {void}
 */
export function faNeedSharedMemory(what) {
	if (typeof SharedArrayBuffer === "undefined") {
		faStop(
			`${what} given to a worker needs memory shared between threads (\`SharedArrayBuffer\`), which a web page only has when it is cross-origin isolated: serve it with the headers \`Cross-Origin-Opener-Policy: same-origin\` and \`Cross-Origin-Embedder-Policy: require-corp\``,
		);
	}
}

// @fa-helper FaAtomic deps: faNeedSharedMemory faOwnShared
/**
 * An `AtomicInteger` (a number) or an `AtomicBoolean`. The orderings make no
 * difference on one thread.
 */
export class FaAtomic {
	/** @param {any} value */
	constructor(value) {
		/** @type {any} */
		this.value = value;
		/**
		 * Once shared with a worker (`share`): the value in shared memory, a
		 * `BigInt64Array` (an integer) or an `Int32Array` (a boolean).
		 * @type {BigInt64Array | Int32Array | null}
		 */
		this.cell = null;
		faOwnShared(this, () => {});
	}

	/** Moves the value into shared memory, before it is sent to a worker. */
	share() {
		if (this.cell !== null) {
			return;
		}
		faNeedSharedMemory("an atomic");
		this.cell =
			typeof this.value === "boolean"
				? new Int32Array(new SharedArrayBuffer(4))
				: new BigInt64Array(new SharedArrayBuffer(8));
		Atomics.store(/** @type {any} */ (this.cell), 0, this.raw(this.value));
		/** @type {string} */
		this.$faShared = "atomic";
	}

	/**
	 * The value of a raw cell value.
	 * @param {any} raw
	 * @returns {any}
	 */
	cooked(raw) {
		return typeof raw === "bigint" ? Number(raw) : raw !== 0;
	}

	/**
	 * The raw cell value of a value.
	 * @param {any} value
	 * @returns {any}
	 */
	raw(value) {
		return typeof value === "boolean" ? (value ? 1 : 0) : BigInt(value);
	}

	/** @returns {any} */
	load() {
		const cell = /** @type {any} */ (this.cell);
		return cell === null ? this.value : this.cooked(Atomics.load(cell, 0));
	}

	/** @param {any} value */
	store(value) {
		const cell = /** @type {any} */ (this.cell);
		if (cell === null) {
			this.value = value;
		} else {
			Atomics.store(cell, 0, this.raw(value));
		}
	}

	/** @returns {number} the new value */
	increment() {
		return this.add(1);
	}

	/** @returns {number} the new value */
	decrement() {
		return this.add(-1);
	}

	/**
	 * @param {number} amount
	 * @returns {number} the new value
	 */
	add(amount) {
		const cell = /** @type {any} */ (this.cell);
		if (cell === null) {
			return (this.value += amount);
		}
		return Number(Atomics.add(cell, 0, BigInt(amount))) + amount;
	}

	/**
	 * @param {any} value
	 * @returns {any} the previous value
	 */
	swap(value) {
		const cell = /** @type {any} */ (this.cell);
		if (cell === null) {
			const previous = this.value;
			this.value = value;
			return previous;
		}
		return this.cooked(Atomics.exchange(cell, 0, this.raw(value)));
	}

	/**
	 * @param {any} expected
	 * @param {any} value
	 * @returns {boolean} whether the value was `expected`, and is now `value`
	 */
	compareAndSwap(expected, value) {
		const cell = /** @type {any} */ (this.cell);
		if (cell === null) {
			if (this.value !== expected) {
				return false;
			}
			this.value = value;
			return true;
		}
		const raw = this.raw(expected);
		return Atomics.compareExchange(cell, 0, raw, this.raw(value)) === raw;
	}
}

// @fa-helper FaGuard deps: faOwnShared faRetainShared faReleaseGuard
/**
 * The guard of a locked mutex: `value` reads and writes the protected value.
 */
export class FaGuard {
	/** @param {{ value: any, locked: boolean }} mutex */
	constructor(mutex) {
		/** @type {{ value: any, locked: boolean } | null} */
		this.mutex = mutex;
		faRetainShared(mutex);
		faOwnShared(this, faReleaseGuard);
	}

	/** @returns {any} */
	get value() {
		const mutex = /** @type {{ value: any }} */ (this.mutex);
		return mutex.value;
	}

	/** @param {any} value */
	set value(value) {
		const mutex = /** @type {{ value: any }} */ (this.mutex);
		mutex.value = value;
	}
}

// @fa-helper faOwnMutex deps: faOwnShared faDropOwned
/** @template {{cell: Int32Array | null, value: any}} T @param {T} mutex @returns {T} */
export function faOwnMutex(mutex) {
	return faOwnShared(mutex, (value) => {
		if (value.cell === null) {
			const held = value.value;
			value.value = undefined;
			faDropOwned(held);
		}
	});
}

// @fa-helper FaMutex deps: FaGuard faNeedSharedMemory faOwnMutex
/**
 * A `Mutex(T)`: the value, and whether a guard holds it.
 */
export class FaMutex {
	/** @param {any} value */
	constructor(value) {
		/** @type {any} */
		this.value = value;
		/** @type {boolean} */
		this.locked = false;
		/** @type {Array<{ resolve: (guard: any) => void }>} the tasks waiting for the lock, in order */
		this.waiters = [];
		/**
		 * Once shared with a worker (`share`): the lock word and the length of
		 * the value's text, in shared memory; the text follows (`bytes`).
		 * @type {Int32Array | null}
		 */
		this.cell = null;
		/** @type {Uint8Array | null} */
		this.bytes = null;
		faOwnMutex(this);
	}

	/**
	 * Moves the value into shared memory, before it is sent to a worker: the
	 * lock is a word taken with `compareExchange`, the value is kept as JSON
	 * text there and read back by whoever locks it (a flat value, as the
	 * documentation asks; strings and arrays of them work too).
	 */
	share() {
		if (this.cell !== null) {
			return;
		}
		faNeedSharedMemory("a Mutex");
		const size = Math.max(65536, 4 * this.text(this.value).length);
		const memory = new SharedArrayBuffer(8 + size);
		this.cell = new Int32Array(memory, 0, 2);
		this.bytes = new Uint8Array(memory, 8);
		Atomics.store(this.cell, 0, this.locked ? 1 : 0);
		this.keep();
		/** @type {string} */
		this.$faShared = "mutex";
	}

	/**
	 * The JSON text of a value (`none` and big integers tagged).
	 * @param {any} value
	 * @returns {string}
	 */
	text(value) {
		return JSON.stringify(value, (_, item) =>
			typeof item === "bigint" ? { $b: item.toString() } : item === undefined ? { $n: 0 } : item,
		);
	}

	/** Writes the value into shared memory (the lock is held). */
	keep() {
		const cell = /** @type {Int32Array} */ (this.cell);
		const text = this.text(this.value);
		const bytes = /** @type {Uint8Array} */ (this.bytes);
		const { read, written } = new TextEncoder().encodeInto(text, bytes);
		if (read < text.length) {
			throw new RangeError("the value of a mutex shared with a worker is too large");
		}
		Atomics.store(cell, 1, written);
	}

	/** Reads the value from shared memory (the lock is held). */
	fetch() {
		const cell = /** @type {Int32Array} */ (this.cell);
		const shared = /** @type {Uint8Array} */ (this.bytes);
		const bytes = shared.slice(0, Atomics.load(cell, 1));
		this.value = JSON.parse(new TextDecoder().decode(bytes), (_, item) => {
			if (item !== null && typeof item === "object") {
				if ("$b" in item) return BigInt(item.$b);
				if ("$n" in item) return undefined;
			}
			return item;
		});
	}

	/** @returns {boolean} whether this thread took the shared lock */
	acquire() {
		const cell = /** @type {Int32Array} */ (this.cell);
		if (Atomics.compareExchange(cell, 0, 0, 1) !== 0) {
			return false;
		}
		this.fetch();
		return true;
	}

	/** Gives the shared lock back, the value written first. */
	release() {
		const cell = /** @type {Int32Array} */ (this.cell);
		this.keep();
		Atomics.store(cell, 0, 0);
		Atomics.notify(cell, 0);
	}

	/** @returns {FaGuard | undefined} a guard, or `none` when it is locked */
	tryLock() {
		if (this.cell !== null) {
			return this.acquire() ? new FaGuard(this) : undefined;
		}
		if (this.locked) {
			return undefined;
		}
		this.locked = true;
		return new FaGuard(this);
	}
}

// @fa-helper faReleaseGuard deps: FaGuard faReleaseShared
/**
 * Releases the lock of a guard at the end of its scope (`none`: nothing).
 * @param {{ mutex: { locked: boolean } | null } | undefined | null} guard
 */
export function faReleaseGuard(guard) {
	if (guard != null && guard.mutex !== null) {
		const mutex = /** @type {any} */ (guard.mutex);
		guard.mutex = null;
		// shared with a worker: the shared lock
		if (mutex.cell != null) {
			mutex.release();
		} else {
			const waiter = mutex.waiters?.shift();
			if (waiter === undefined) {
				mutex.locked = false;
			} else {
				// handed over: the mutex stays locked
				waiter.resolve(new FaGuard(mutex));
			}
		}
		faReleaseShared(mutex);
	}
}

// @fa-helper faUnlock deps: faReleaseShared
/** @param {object | undefined | null} guard */
export function faUnlock(guard) {
	faReleaseShared(guard);
}

// @fa-helper faOwnChannel deps: faOwnShared faDropOwned
/** @template {{cell: Int32Array | null, items: any[], head: number}} T @param {T} channel @returns {T} */
export function faOwnChannel(channel) {
	return faOwnShared(channel, (value) => {
		// A SAB may still have live handles in other realms. Its data is
		// released by the host when those wrappers disappear, never closed here.
		if (value.cell === null) {
			const held = value.items.slice(value.head);
			value.items = [];
			value.head = 0;
			faDropOwned(held);
		}
	});
}

// @fa-helper FaChannel deps: faNeedSharedMemory faOwnChannel faDropOwned
/**
 * A `Channel(T)`: a queue, bounded by its capacity (`Infinity` without one).
 * Given to a worker (`share`), its queue moves into shared memory: the
 * words `[lock, closed, length of the text, version]`, then the queue as
 * JSON text (`none` and big integers tagged, as a shared mutex's value).
 * The waits of tasks and of a worker's thread then wait for the version to
 * change (`faSend`, `faReceive`: `Atomics.waitAsync`; `faSendSync`,
 * `faReceiveSync`: `Atomics.wait`).
 */
export class FaChannel {
	/** @param {number | undefined} capacity */
	constructor(capacity) {
		/** @type {any[]} */
		this.items = [];
		/** @type {number} the index of the next item to receive */
		this.head = 0;
		/** @type {number} */
		this.capacity = capacity === undefined ? Infinity : capacity;
		/** @type {boolean} */
		this.closed = false;
		/** @type {Array<{ resolve: (value: any) => void }>} the tasks waiting to receive, in order */
		this.receivers = [];
		/** @type {Array<{ resolve: (sent: boolean) => void, value: any }>} the tasks waiting to send, in order */
		this.senders = [];
		/**
		 * Once shared with a worker (`share`): its words in shared memory.
		 * @type {Int32Array | null}
		 */
		this.cell = null;
		/** @type {Uint8Array | null} */
		this.bytes = null;
		faOwnChannel(this);
	}

	/**
	 * @param {any} value
	 * @returns {boolean} whether it was sent (`false` when full or closed)
	 */
	trySend(value) {
		const sent = this.offer(value);
		if (!sent) faDropOwned(value);
		return sent;
	}

	/** An attempt of a waiting send retains its value when full.
	 * @param {any} value @returns {boolean}
	 */
	offer(value) {
		if (this.cell !== null) {
			return this.pollSend(value) === 1;
		}
		if (this.closed) {
			return false;
		}
		const receiver = this.receivers.shift();
		if (receiver !== undefined) {
			receiver.resolve(value);
			return true;
		}
		if (this.items.length - this.head >= this.capacity) {
			return false;
		}
		this.items.push(value);
		return true;
	}

	/** @returns {any} the next value, `none` when there is none */
	tryReceive() {
		if (this.cell !== null) {
			const value = this.pollReceive(FaChannel);
			return value === FaChannel ? undefined : value;
		}
		if (this.head === this.items.length) {
			return undefined;
		}
		const value = this.items[this.head];
		this.head += 1;
		const sender = this.senders.shift();
		if (sender !== undefined) {
			this.items.push(sender.value);
			sender.resolve(true);
		}
		if (this.head > 32 && this.head * 2 > this.items.length) {
			this.items = this.items.slice(this.head);
			this.head = 0;
		}
		return value;
	}

	/**
	 * Moves the queue into shared memory, before the channel is sent to a
	 * worker: the values waiting to be sent join it, the tasks waiting to
	 * receive try again when its version changes.
	 */
	share() {
		if (this.cell !== null) {
			return;
		}
		faNeedSharedMemory("a Channel");
		const memory = new SharedArrayBuffer(16 + (1 << 20));
		this.cell = new Int32Array(memory, 0, 4);
		this.bytes = new Uint8Array(memory, 16);
		const items = this.items.slice(this.head);
		for (const sender of this.senders.splice(0)) {
			items.push(sender.value);
			sender.resolve(true);
		}
		this.write(items);
		Atomics.store(this.cell, 1, this.closed ? 1 : 0);
		this.items = [];
		this.head = 0;
		/** @type {string} */
		this.$faShared = "channel";
		/** @type {any} */
		const atomics = Atomics;
		const cell = this.cell;
		for (const receiver of this.receivers.splice(0)) {
			// tried again when the version changes (`faWaitShared`)
			const poll = (/** @type {number} */ attempt) => {
				const version = Atomics.load(cell, 3);
				const value = this.pollReceive(FaChannel);
				if (value !== FaChannel) {
					receiver.resolve(value);
				} else if (typeof atomics.waitAsync === "function") {
					atomics.waitAsync(cell, 3, version, 1000).value.then(() => poll(attempt + 1));
				} else {
					setTimeout(() => poll(attempt + 1), Math.min(16, 2 ** Math.min(attempt, 4)));
				}
			};
			poll(0);
		}
	}

	/** Takes the lock word of the shared queue (held for a few operations). */
	lock() {
		const cell = /** @type {Int32Array} */ (this.cell);
		while (Atomics.compareExchange(cell, 0, 0, 1) !== 0) {
			// another thread holds it briefly
		}
	}

	/**
	 * Gives the lock word back; a change wakes the waiting threads.
	 * @param {boolean} changed
	 */
	unlock(changed) {
		const cell = /** @type {Int32Array} */ (this.cell);
		Atomics.store(cell, 0, 0);
		if (changed) {
			Atomics.add(cell, 3, 1);
			Atomics.notify(cell, 3);
		}
	}

	/** @returns {any[]} the shared queue (the lock is held) */
	read() {
		const cell = /** @type {Int32Array} */ (this.cell);
		const shared = /** @type {Uint8Array} */ (this.bytes);
		const bytes = shared.slice(0, Atomics.load(cell, 2));
		return JSON.parse(new TextDecoder().decode(bytes), (_, item) => {
			if (item !== null && typeof item === "object") {
				if ("$b" in item) return BigInt(item.$b);
				if ("$n" in item) return undefined;
			}
			return item;
		});
	}

	/**
	 * Writes the shared queue (the lock is held, or the channel not shared yet).
	 * @param {any[]} items
	 */
	write(items) {
		const cell = /** @type {Int32Array} */ (this.cell);
		const text = JSON.stringify(items, (_, item) =>
			typeof item === "bigint" ? { $b: item.toString() } : item === undefined ? { $n: 0 } : item,
		);
		const bytes = /** @type {Uint8Array} */ (this.bytes);
		const { read, written } = new TextEncoder().encodeInto(text, bytes);
		if (read < text.length) {
			throw new RangeError("the values in a channel shared with a worker are too large");
		}
		Atomics.store(cell, 2, written);
	}

	/**
	 * Sends a value through the shared queue.
	 * @param {any} value
	 * @returns {number} 1: sent, 0: full, -1: closed
	 */
	pollSend(value) {
		const cell = /** @type {Int32Array} */ (this.cell);
		this.lock();
		if (Atomics.load(cell, 1) === 1) {
			this.unlock(false);
			return -1;
		}
		const items = this.read();
		if (items.length >= this.capacity) {
			this.unlock(false);
			return 0;
		}
		items.push(value);
		this.write(items);
		this.unlock(true);
		return 1;
	}

	/**
	 * Receives a value from the shared queue.
	 * @param {any} waiting what to give while there is none and the channel is open
	 * @returns {any} the value, `none` once closed and empty, else `waiting`
	 */
	pollReceive(waiting) {
		const cell = /** @type {Int32Array} */ (this.cell);
		this.lock();
		const items = this.read();
		if (items.length === 0) {
			const closed = Atomics.load(cell, 1) === 1;
			this.unlock(false);
			return closed ? undefined : waiting;
		}
		const value = items.shift();
		this.write(items);
		this.unlock(true);
		return value;
	}

	close() {
		if (this.cell !== null) {
			this.lock();
			Atomics.store(this.cell, 1, 1);
			this.unlock(true);
			return;
		}
		this.closed = true;
		for (const receiver of this.receivers.splice(0)) {
			receiver.resolve(undefined);
		}
		for (const sender of this.senders.splice(0)) {
			sender.resolve(false);
		}
	}
}
