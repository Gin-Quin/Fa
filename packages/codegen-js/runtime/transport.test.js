import { describe, expect, test } from "bun:test";

import { FaTransportMemory } from "./transport.js";

/** @param {any} message @returns {Promise<{worker:Worker, value:any}>} */
function worker(message) {
	const thread = new Worker(new URL("./test-fixtures/transport-worker.js", import.meta.url));
	return new Promise((resolve, reject) => {
		thread.onmessage = (event) => resolve({ worker: thread, value: event.data });
		thread.onerror = (event) => {
			thread.terminate();
			reject(event.error ?? event.message);
		};
		thread.postMessage(message);
	});
}

describe("shared transport memory", () => {
	test("realm ledgers release local owners and leave stored edges alone", () => {
		const arena = new FaTransportMemory();
		const first = arena.createRealm();
		const second = arena.createRealm();
		const id = arena.create("Resource", 1, 4);
		arena.adoptLocal(id, first, 3);
		arena.moveOwners([id, id], first, second);
		expect(arena.read(id).owners).toBe(4);
		expect(arena.closeRealm(first)).toEqual([]);
		expect(arena.read(id).owners).toBe(3);
		expect(arena.closeRealm(second)).toEqual([]);
		expect(arena.read(id).owners).toBe(1);
		// The fourth owner is a record edge, independent of both realms.
		expect(arena.release(id)).toBe(true);
		arena.finish(id);
	});

	test("a packet move rolls back every target ledger if allocation fails", () => {
		const arena = new FaTransportMemory();
		const from = arena.createRealm();
		const to = arena.createRealm();
		const first = arena.create("first", 1);
		const second = arena.create("second", 1);
		arena.adoptLocal(first, from, 1);
		arena.adoptLocal(second, from, 1);
		const allocate = arena.allocateLocked.bind(arena);
		let attempts = 0;
		arena.allocateLocked = (length) => {
			if (++attempts === 2) throw new RangeError("test allocation failure");
			return allocate(length);
		};
		expect(() => arena.moveOwners([first, second], from, to)).toThrow("test allocation failure");
		arena.allocateLocked = allocate;
		expect(arena.closeRealm(to)).toEqual([]);
		expect(arena.read(first).owners).toBe(1);
		expect(arena.read(second).owners).toBe(1);
		expect(arena.closeRealm(from).sort()).toEqual([first, second].sort());
		arena.finish(first);
		arena.finish(second);
	});

	test("two realms retain the same identity and only its last owner finalizes", () => {
		const origin = new FaTransportMemory();
		const id = origin.create("Resource", 1);
		origin.write(id, new Uint8Array([1, 2]));
		const other = new FaTransportMemory(structuredClone(origin.buffer));
		other.retain(id);
		expect(origin.read(id).owners).toBe(2);
		expect(origin.release(id)).toBe(false);
		expect([...other.read(id).bytes]).toEqual([1, 2]);
		expect(other.release(id)).toBe(true);
		expect(other.finish(id)).toEqual([]);
		expect(() => origin.read(id)).toThrow("destroyed");
	});

	test("growing the buffer leaves existing views and identities valid", () => {
		const origin = new FaTransportMemory();
		const other = new FaTransportMemory(origin.buffer);
		const id = origin.create("Large", 1);
		const bytes = new Uint8Array(200000);
		bytes[199999] = 73;
		origin.write(id, bytes);
		expect(other.buffer.byteLength).toBeGreaterThan(200000);
		expect(other.bytes.length).toBe(other.buffer.byteLength);
		expect(other.read(id).bytes[199999]).toBe(73);
		expect(other.release(id)).toBe(true);
		other.finish(id);
	});

	test("replacement publishes new edges before returning the old owners", () => {
		const arena = new FaTransportMemory();
		const first = arena.create("first", 1);
		const second = arena.create("second", 1);
		const parent = arena.create("parent", 1);
		arena.write(parent, new Uint8Array([1]), [arena.retain(first)]);
		expect(arena.write(parent, new Uint8Array([2]), [arena.retain(second)])).toEqual([first]);
		expect(arena.release(first)).toBe(false);
		expect(arena.read(first).owners).toBe(1);
		expect(arena.read(parent).edges).toEqual([second]);
		expect(arena.release(parent)).toBe(true);
		expect(arena.finish(parent)).toEqual([second]);
		expect(arena.release(second)).toBe(false);
		expect(arena.release(first)).toBe(true);
		arena.finish(first);
		expect(arena.release(second)).toBe(true);
		arena.finish(second);
	});

	test("reused blocks never give an old id to another instance", () => {
		const arena = new FaTransportMemory(undefined, 65536);
		const old = arena.create("old", 1);
		arena.write(old, new Uint8Array(20000));
		expect(arena.release(old)).toBe(true);
		arena.finish(old);
		const fresh = arena.create("fresh", 1);
		arena.write(fresh, new Uint8Array([9]));
		expect(fresh).not.toBe(old);
		expect(() => arena.retain(old)).toThrow("destroyed");
		expect(arena.read(fresh).name).toBe("fresh");
		for (let index = 0; index < 500; index += 1) {
			arena.write(fresh, new Uint8Array(index % 2 === 0 ? 10000 : 12000));
		}
		expect(arena.release(fresh)).toBe(true);
		arena.finish(fresh);
	});

	test("parallel retains and allocation do not lose owners", async () => {
		const arena = new FaTransportMemory();
		const id = arena.create("kept", 1);
		arena.write(id, new Uint8Array([42]));
		const results = await Promise.all([
			worker({ buffer: arena.buffer, id, rounds: 3000 }),
			worker({ buffer: arena.buffer, id, rounds: 3000 }),
		]);
		for (const result of results) {
			expect(result.value.done).toBe(true);
			result.worker.terminate();
		}
		expect(arena.read(id).owners).toBe(1);
		expect([...arena.read(id).bytes]).toEqual([42]);
		expect(arena.release(id)).toBe(true);
		arena.finish(id);
	});

	test("an instance created by a worker survives the end of that realm", async () => {
		const arena = new FaTransportMemory();
		const made = await worker({ buffer: arena.buffer, create: true });
		made.worker.terminate();
		expect(arena.read(made.value.id).name).toBe("late");
		expect([...arena.read(made.value.id).bytes]).toEqual([7, 8, 9]);
		expect(arena.release(made.value.id)).toBe(true);
		arena.finish(made.value.id);
	});
});
