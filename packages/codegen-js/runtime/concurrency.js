// The thread-safe types of `std.concurrency` on JavaScript (advanced brief
// 14.3, D31): a program runs on one thread, so they are plain objects
// updated with plain operations. The methods that wait (`lock`, `send`,
// `receive`) are awaited by tasks (`tasks.js`: `faLock`, `faSend`,
// `faReceive`), which queue as waiters here; a blocking call of them is a
// compile error on JavaScript (`CONC010`). The waiters are served in order,
// and the value or the lock is handed over directly (design/async.md A16).
// A copy of a handle is the same object.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

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
	}

	/** @returns {any} */
	load() {
		return this.value;
	}

	/** @param {any} value */
	store(value) {
		this.value = value;
	}

	/** @returns {number} the new value */
	increment() {
		return (this.value += 1);
	}

	/** @returns {number} the new value */
	decrement() {
		return (this.value -= 1);
	}

	/**
	 * @param {number} amount
	 * @returns {number} the new value
	 */
	add(amount) {
		return (this.value += amount);
	}

	/**
	 * @param {any} value
	 * @returns {any} the previous value
	 */
	swap(value) {
		const previous = this.value;
		this.value = value;
		return previous;
	}

	/**
	 * @param {any} expected
	 * @param {any} value
	 * @returns {boolean} whether the value was `expected`, and is now `value`
	 */
	compareAndSwap(expected, value) {
		if (this.value !== expected) {
			return false;
		}
		this.value = value;
		return true;
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
	}

	/** @returns {FaGuard | undefined} a guard, or `none` when it is locked */
	tryLock() {
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
		const waiter = mutex.waiters?.shift();
		if (waiter === undefined) {
			mutex.locked = false;
		} else {
			// handed over: the mutex stays locked
			waiter.resolve(new FaGuard(mutex));
		}
	}
}

// @fa-helper FaChannel
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
