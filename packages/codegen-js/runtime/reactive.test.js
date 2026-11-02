// Reactivity (advanced brief 11): sources, derived values pulled lazily,
// watch blocks flushed in declaration order, cleanup, disposal.

import { beforeEach, describe, expect, test } from "bun:test";

import {
	faChanged,
	faDerived,
	faDispose,
	faFieldSource,
	faFlush,
	faNotify,
	faReactive,
	faRead,
	faSource,
	faTrack,
	faWatch,
} from "./reactive.js";
import { FaStop } from "./stops.js";

beforeEach(() => {
	faReactive.observer = null;
	faReactive.pending = [];
	faReactive.flushing = false;
});

/**
 * A reactive cell, like the code the compiler generates for `reactive x = …`.
 * @template T
 * @param {T} initial
 */
function cell(initial) {
	let value = initial;
	const source = faSource();
	return {
		source,
		get() {
			faTrack(source);
			return value;
		},
		/** @param {T} next */
		set(next) {
			if (value !== next) {
				value = next;
				faNotify(source);
			}
		},
	};
}

describe("derived values", () => {
	test("are computed when read, then cached", () => {
		const number = cell(3);
		let runs = 0;
		const doubled = faDerived(() => {
			runs += 1;
			return number.get() * 2;
		});
		expect(runs).toBe(0);
		expect(faRead(doubled)).toBe(6);
		expect(faRead(doubled)).toBe(6);
		expect(runs).toBe(1);
	});

	test("are always up to date, computed at most once per change", () => {
		const number = cell(3);
		let runs = 0;
		const doubled = faDerived(() => {
			runs += 1;
			return number.get() * 2;
		});
		faRead(doubled);
		number.set(5);
		number.set(6);
		expect(runs).toBe(1);
		expect(faRead(doubled)).toBe(12);
		expect(faRead(doubled)).toBe(12);
		expect(runs).toBe(2);
	});

	test("depend on what their last computation read", () => {
		const first = cell("Ada");
		const last = cell("Lovelace");
		const full = cell(false);
		let runs = 0;
		const label = faDerived(() => {
			runs += 1;
			return full.get() ? `${first.get()} ${last.get()}` : first.get();
		});
		expect(faRead(label)).toBe("Ada");
		last.set("Byron");
		expect(faRead(label)).toBe("Ada");
		expect(runs).toBe(1);
		full.set(true);
		expect(faRead(label)).toBe("Ada Byron");
		expect(runs).toBe(2);
	});

	test("chain: a derived value of a derived value", () => {
		const width = cell(10);
		const area = faDerived(() => width.get() * 2);
		let runs = 0;
		const label = faDerived(() => {
			runs += 1;
			return `area ${faRead(area)}`;
		});
		expect(faRead(label)).toBe("area 20");
		width.set(11);
		expect(faRead(label)).toBe("area 22");
		expect(runs).toBe(2);
	});

	test("a recomputation giving the same flat value changes nothing downstream", () => {
		const number = cell(3);
		const parity = faDerived(() => number.get() % 2);
		let runs = 0;
		const label = faDerived(() => {
			runs += 1;
			return faRead(parity) === 0 ? "even" : "odd";
		});
		expect(faRead(label)).toBe("odd");
		number.set(5);
		expect(faRead(label)).toBe("odd");
		expect(runs).toBe(1);
	});

	test("reading itself stops the program", () => {
		/** @type {any} */
		let self;
		self = faDerived(() => faRead(self) + 1);
		expect(() => faRead(self)).toThrow(FaStop);
	});
});

describe("watch blocks", () => {
	test("run at once, then after the flush that follows a change", () => {
		const count = cell(1);
		/** @type {unknown[]} */
		const log = [];
		faWatch(() => {
			log.push(count.get());
		});
		expect(log).toEqual([1]);
		count.set(2);
		expect(log).toEqual([1]);
		faFlush();
		expect(log).toEqual([1, 2]);
	});

	test("run once per flush, however many of their dependencies changed", () => {
		const width = cell(10);
		const height = cell(20);
		const area = faDerived(() => width.get() * height.get());
		/** @type {unknown[]} */
		const log = [];
		faWatch(() => {
			log.push(faRead(area));
		});
		width.set(30);
		height.set(40);
		expect(faRead(area)).toBe(1200);
		faFlush();
		expect(log).toEqual([200, 1200]);
	});

	test("an equal write changes nothing", () => {
		const count = cell(1);
		let runs = 0;
		faWatch(() => {
			count.get();
			runs += 1;
		});
		count.set(1);
		faFlush();
		expect(runs).toBe(1);
	});

	test("don't run when a derived value they read is computed again to the same value", () => {
		const number = cell(3);
		const parity = faDerived(() => number.get() % 2);
		let runs = 0;
		faWatch(() => {
			faRead(parity);
			runs += 1;
		});
		number.set(5);
		faFlush();
		expect(runs).toBe(1);
		number.set(6);
		faFlush();
		expect(runs).toBe(2);
	});

	test("run in declaration order, and the ones they make pending run in the same flush", () => {
		const a = cell(0);
		const b = cell(0);
		/** @type {string[]} */
		const log = [];
		faWatch(() => {
			b.set(a.get() + 1);
			log.push(`first ${a.get()}`);
		});
		faWatch(() => {
			log.push(`second ${b.get()}`);
		});
		a.set(5);
		faFlush();
		expect(log).toEqual(["first 0", "second 1", "first 5", "second 6"]);
	});

	test("a cleanup runs before the next run, untracked", () => {
		const channel = cell("general");
		/** @type {string[]} */
		const log = [];
		faWatch(() => {
			const name = channel.get();
			log.push(`join ${name}`);
			return () => {
				log.push(`leave ${name}`);
				channel.get();
			};
		});
		channel.set("random");
		faFlush();
		expect(log).toEqual(["join general", "leave general", "join random"]);
		faFlush();
		expect(log.length).toBe(3);
	});

	test("a disposed watch block runs its cleanup and never runs again", () => {
		const count = cell(0);
		/** @type {string[]} */
		const log = [];
		const node = faWatch(() => {
			log.push(`run ${count.get()}`);
			return () => log.push("cleanup");
		});
		faDispose(node);
		faDispose(node);
		count.set(1);
		faFlush();
		expect(log).toEqual(["run 0", "cleanup"]);
	});

	test("watch blocks that keep scheduling each other stop the program after 100 rounds", () => {
		const ping = cell(0);
		const pong = cell(0);
		let runs = 0;
		faWatch(() => {
			runs += 1;
			pong.set(ping.get() + 1);
		});
		faWatch(() => {
			runs += 1;
			ping.set(pong.get() + 1);
		});
		expect(() => faFlush()).toThrow(/100 rounds/);
		expect(runs).toBe(102);
		expect(faReactive.flushing).toBe(false);
	});

	test("a flush while flushing does nothing: the running one takes the new blocks", () => {
		const count = cell(0);
		const other = cell(0);
		/** @type {string[]} */
		const log = [];
		faWatch(() => {
			other.set(count.get());
			faFlush();
			log.push("first");
		});
		faWatch(() => {
			log.push(`second ${other.get()}`);
		});
		count.set(1);
		faFlush();
		expect(log).toEqual(["first", "second 0", "first", "second 1"]);
	});
});

describe("fine-grained sources", () => {
	test("an element write doesn't change the length", () => {
		const numbers = [1, 2, 3];
		const source = faSource();
		/** @type {string[]} */
		const log = [];
		faWatch(() => {
			faTrack(source.length());
			log.push(`length ${numbers.length}`);
		});
		numbers[0] = 10;
		faNotify(source.index(0));
		faFlush();
		expect(log).toEqual(["length 3"]);
		numbers.push(4);
		faNotify(source.length());
		faFlush();
		expect(log).toEqual(["length 3", "length 4"]);
	});

	test("a whole read depends on every part", () => {
		const settings = { database: { connections: 4 }, name: "app" };
		const source = faSource();
		let runs = 0;
		faWatch(() => {
			faTrack(source);
			runs += 1;
		});
		settings.database.connections = 8;
		faNotify(source.field("database").field("connections"));
		faFlush();
		expect(runs).toBe(2);
	});

	test("a field read depends on its field, and on the writes of the values above it", () => {
		const source = faSource();
		/** @type {string[]} */
		const log = [];
		faWatch(() => {
			faTrack(source.field("database").field("connections"));
			log.push("connections");
		});
		faNotify(source.field("name"));
		faFlush();
		expect(log).toEqual(["connections"]);
		faNotify(source.field("database"));
		faFlush();
		expect(log).toEqual(["connections", "connections"]);
		faNotify(source);
		faFlush();
		expect(log.length).toBe(3);
	});

	test("an element read depends on that element", () => {
		const source = faSource();
		let runs = 0;
		faWatch(() => {
			faTrack(source.index(2));
			runs += 1;
		});
		faNotify(source.index(1));
		faFlush();
		expect(runs).toBe(1);
		faNotify(source.index(2));
		faFlush();
		expect(runs).toBe(2);
	});

	test("the sources of reactive fields belong to their object", () => {
		const counter = { label: "clicks", count: 1 };
		const copy = { ...counter };
		expect(faFieldSource(counter, "count")).toBe(faFieldSource(counter, "count"));
		expect(faFieldSource(copy, "count")).not.toBe(faFieldSource(counter, "count"));
		const next = faDerived(() => {
			faTrack(faFieldSource(counter, "count"));
			return counter.count + 1;
		});
		expect(faRead(next)).toBe(2);
		counter.count = 5;
		faNotify(faFieldSource(counter, "count"));
		expect(faRead(next)).toBe(6);
	});

	test("faChanged notifies after a call and gives its value", () => {
		const items = [1];
		const source = faSource();
		let runs = 0;
		faWatch(() => {
			faTrack(source.length());
			runs += 1;
		});
		expect(faChanged(items.push(2), source.length())).toBe(2);
		faFlush();
		expect(runs).toBe(2);
	});

	test("reads outside a derived value or a watch block track nothing", () => {
		const source = faSource();
		faTrack(source);
		expect(source.observers).toBe(null);
	});

	test("a missing source (a plain argument of a reactive instance) is ignored", () => {
		let runs = 0;
		/** @type {import("./reactive.js").FaSource | null} */
		const missing = null;
		faWatch(() => {
			faTrack(missing);
			faTrack(missing?.length());
			runs += 1;
		});
		faNotify(missing);
		expect(faChanged(3, missing, undefined)).toBe(3);
		faFlush();
		expect(runs).toBe(1);
	});
});
