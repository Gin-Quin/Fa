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
// `compareExchange` and its value JSON text read by whoever holds it. A
// channel can't be given to a worker yet (a stop).
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { faStop } from "./stops.js";

// @fa-helper FaAtomic
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
	}

	/** Moves the value into shared memory, before it is sent to a worker. */
	share() {
		if (this.cell !== null || typeof SharedArrayBuffer === "undefined") {
			return;
		}
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

// @fa-helper FaGuard
/**
 * The guard of a locked mutex: `value` reads and writes the protected value.
 */
export class FaGuard {
	/** @param {{ value: any, locked: boolean }} mutex */
	constructor(mutex) {
		/** @type {{ value: any, locked: boolean } | null} */
		this.mutex = mutex;
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

// @fa-helper FaMutex deps: FaGuard
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
	}

	/**
	 * Moves the value into shared memory, before it is sent to a worker: the
	 * lock is a word taken with `compareExchange`, the value is kept as JSON
	 * text there and read back by whoever locks it (a flat value, as the
	 * documentation asks; strings and arrays of them work too).
	 */
	share() {
		if (this.cell !== null || typeof SharedArrayBuffer === "undefined") {
			return;
		}
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

// @fa-helper faUnlock deps: FaGuard
/**
 * Releases the lock of a guard at the end of its scope (`none`: nothing).
 * @param {{ mutex: { locked: boolean } | null } | undefined | null} guard
 */
export function faUnlock(guard) {
	if (guard != null && guard.mutex !== null) {
		const mutex = /** @type {any} */ (guard.mutex);
		guard.mutex = null;
		// shared with a worker: the shared lock
		if (mutex.cell != null) {
			mutex.release();
			return;
		}
		const waiter = mutex.waiters?.shift();
		if (waiter === undefined) {
			mutex.locked = false;
		} else {
			// handed over: the mutex stays locked
			waiter.resolve(new FaGuard(mutex));
		}
	}
}

// @fa-helper FaChannel deps: faStop
/**
 * A `Channel(T)`: a queue, bounded by its capacity (`Infinity` without one).
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
	}

	/**
	 * @param {any} value
	 * @returns {boolean} whether it was sent (`false` when full or closed)
	 */
	trySend(value) {
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

	/** A channel can't cross to a JavaScript worker yet: a stop. */
	share() {
		faStop("a Channel can't be passed to a JavaScript worker yet (atomics and mutexes can)");
	}

	close() {
		this.closed = true;
		for (const receiver of this.receivers.splice(0)) {
			receiver.resolve(undefined);
		}
		for (const sender of this.senders.splice(0)) {
			sender.resolve(false);
		}
	}
}
