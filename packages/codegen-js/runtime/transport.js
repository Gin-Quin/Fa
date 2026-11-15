// Shared transport storage. Codec describes values; this arena gives references
// stable identities and ownership across realms. Every offset stays valid when
// the shared buffer grows; a newly published instance needs no side message.

import { faStop } from "./stops.js";

// @fa-helper FaTransportMemory deps: faStop
/** The shared allocator and instance directory of a group of Fa workers. */
export class FaTransportMemory {
	static start = 4096;
	static buckets = 256;
	static recordSize = 64;

	/** @param {SharedArrayBuffer | undefined} buffer @param {number | undefined} maximum */
	constructor(buffer = undefined, maximum = undefined) {
		if (buffer === undefined) {
			if (typeof SharedArrayBuffer === "undefined") {
				faStop(
					"workers that share values need SharedArrayBuffer (cross-origin isolation on the web)",
				);
			}
			let reserve = maximum ?? 0xffff0000;
			for (;;) {
				try {
					buffer = new SharedArrayBuffer(Math.min(65536, reserve), { maxByteLength: reserve });
					break;
				} catch {
					if (maximum !== undefined || reserve <= 65536) faStop("out of shared transport memory");
					// Reserving a GSAB also reserves virtual address space. Start
					// with the largest arena the current host limit can provide.
					reserve = Math.max(65536, Math.floor(reserve / 2 / 65536) * 65536);
				}
			}
			if (!buffer.growable) {
				faStop("workers that share values need growable SharedArrayBuffer");
			}
			this.buffer = buffer;
			this.words = new Int32Array(buffer);
			this.longs = new BigInt64Array(buffer);
			this.bytes = new Uint8Array(buffer);
			this.words[1] = 0x46415348;
			this.words[2] = FaTransportMemory.start;
			this.longs[2] = 1n;
		} else {
			this.buffer = buffer;
			this.words = new Int32Array(buffer);
			this.longs = new BigInt64Array(buffer);
			this.bytes = new Uint8Array(buffer);
			if (Atomics.load(this.words, 1) !== 0x46415348) {
				throw new TypeError("invalid Fa shared transport memory");
			}
		}
		this.locked = false;
	}

	/** @template T @param {() => T} body @returns {T} */
	synchronized(body) {
		if (this.locked) throw new Error("reentrant Fa transport allocator");
		while (Atomics.compareExchange(this.words, 0, 0, 1) !== 0) {}
		this.locked = true;
		try {
			return body();
		} finally {
			this.locked = false;
			Atomics.store(this.words, 0, 0);
			Atomics.notify(this.words, 0);
		}
	}

	/** @param {number} offset @returns {number} */
	word(offset) {
		return this.words[offset / 4] >>> 0;
	}
	/** @param {number} offset @param {number} value */
	setWord(offset, value) {
		this.words[offset / 4] = value;
	}

	/** Allocator lock held. @param {number} length @returns {number} */
	allocateLocked(length) {
		const size = Math.ceil((length + 16) / 16) * 16;
		if (!Number.isSafeInteger(length) || length < 0 || size >= 0xffff0000) {
			faStop("out of shared transport memory");
		}
		let previous = 0;
		let block = this.word(12);
		while (block !== 0) {
			const available = this.word(block);
			const next = this.word(block + 4);
			if (available >= size) {
				let replacement = next;
				if (available - size >= 32) {
					replacement = block + size;
					this.setWord(replacement, available - size);
					this.setWord(replacement + 4, next);
					this.setWord(block, size);
				}
				this.setWord(previous === 0 ? 12 : previous + 4, replacement);
				this.bytes.fill(0, block + 16, block + this.word(block));
				return block + 16;
			}
			previous = block;
			block = next;
		}
		block = this.word(8);
		const end = block + size;
		if (end > this.buffer.maxByteLength) faStop("out of shared transport memory");
		if (end > this.buffer.byteLength) {
			const grown = Math.min(this.buffer.maxByteLength, Math.ceil(end / 65536) * 65536);
			try {
				this.buffer.grow(grown);
			} catch {
				faStop("out of shared transport memory");
			}
		}
		this.setWord(8, end);
		this.setWord(block, size);
		return block + 16;
	}

	/** Allocator lock held. @param {number} payload */
	freeLocked(payload) {
		if (payload === 0) return;
		const block = payload - 16;
		this.setWord(block + 4, this.word(12));
		this.setWord(12, block);
	}

	/** @param {number} length @returns {number} */
	allocate(length) {
		return this.synchronized(() => this.allocateLocked(length));
	}
	/** @param {number} offset */
	free(offset) {
		this.synchronized(() => this.freeLocked(offset));
	}

	/** Allocator lock held. @param {number} id @returns {number} */
	findLocked(id) {
		if (!Number.isSafeInteger(id) || id <= 0) throw new TypeError("invalid Fa shared instance id");
		let record = this.word(64 + (id % FaTransportMemory.buckets) * 4);
		while (record !== 0) {
			if (Number(this.longs[(record + 8) / 8]) === id) return record;
			record = this.word(record);
		}
		throw new TypeError("a shared Fa instance was already destroyed");
	}

	/** Creates an unpublished instance with the caller's existing owners.
	 * @param {string} name @param {number} kind @param {number} owners
	 * @returns {number} its stable id
	 */
	create(name, kind, owners = 1) {
		if (!Number.isSafeInteger(owners) || owners <= 0)
			throw new TypeError("invalid shared Fa owner count");
		const text = new TextEncoder().encode(name);
		return this.synchronized(() => {
			const id = Number(this.longs[2]);
			if (!Number.isSafeInteger(id + 1)) faStop("out of shared transport identities");
			const record = this.allocateLocked(FaTransportMemory.recordSize + text.length);
			const bucket = 64 + (id % FaTransportMemory.buckets) * 4;
			this.longs[2] += 1n;
			this.setWord(record, this.word(bucket));
			this.setWord(record + 28, kind);
			this.longs[(record + 8) / 8] = BigInt(id);
			this.longs[(record + 16) / 8] = BigInt(owners);
			this.setWord(record + 36, text.length);
			this.bytes.set(text, record + FaTransportMemory.recordSize);
			this.setWord(bucket, record);
			return id;
		});
	}

	/** @param {number} id @returns {number} */
	retain(id) {
		return this.synchronized(() => {
			const record = this.findLocked(id);
			if (this.word(record + 4) >= 2)
				throw new TypeError("a resource can't be retained during its destruction");
			Atomics.add(this.longs, (record + 16) / 8, 1n);
			return id;
		});
	}

	/** @returns {number} */
	createRealm() {
		const id = this.create("@fa-realm", 0);
		this.synchronized(() => {
			this.setWord(this.findLocked(id) + 4, 1);
		});
		return id;
	}

	/** @param {number} realm */
	checkRealmLocked(realm) {
		const record = this.findLocked(realm);
		if (this.word(record + 28) !== 0 || this.word(record + 4) !== 1) {
			throw new TypeError("a terminated Fa realm can't own values");
		}
	}

	/** @param {number} record @param {number} realm @returns {number} */
	localCountLocked(record, realm) {
		let entry = this.word(record + 24);
		while (entry !== 0) {
			if (Number(this.longs[(entry + 8) / 8]) === realm)
				return Number(this.longs[(entry + 16) / 8]);
			entry = this.word(entry);
		}
		return 0;
	}

	/** Changes only a realm ledger: the global count includes storage edges
	 * and transit owners too. Allocator lock held. @param {number} record
	 * @param {number} realm @param {number} delta
	 */
	localDeltaLocked(record, realm, delta) {
		let link = record + 24;
		let entry = this.word(link);
		while (entry !== 0 && Number(this.longs[(entry + 8) / 8]) !== realm) {
			link = entry;
			entry = this.word(link);
		}
		const previous = entry === 0 ? 0 : Number(this.longs[(entry + 16) / 8]);
		const count = previous + delta;
		if (!Number.isSafeInteger(count) || count < 0)
			throw new Error("an unowned Fa realm value was released");
		if (count === 0) {
			if (entry !== 0) {
				this.setWord(link, this.word(entry));
				this.freeLocked(entry);
			}
			return;
		}
		if (entry === 0) {
			entry = this.allocateLocked(32);
			this.longs[(entry + 8) / 8] = BigInt(realm);
			this.setWord(link, entry);
		}
		this.longs[(entry + 16) / 8] = BigInt(count);
	}

	/** Records owners which already exist globally (a committed promotion).
	 * @param {number} id @param {number} realm @param {number} count
	 */
	adoptLocal(id, realm, count) {
		this.synchronized(() => {
			this.checkRealmLocked(realm);
			this.localDeltaLocked(this.findLocked(id), realm, count);
		});
	}

	/** @param {number} id @param {number} realm */
	retainLocal(id, realm) {
		this.synchronized(() => {
			this.checkRealmLocked(realm);
			const record = this.findLocked(id);
			if (this.word(record + 4) >= 2)
				throw new TypeError("a resource can't be retained during its destruction");
			this.localDeltaLocked(record, realm, 1);
			Atomics.add(this.longs, (record + 16) / 8, 1n);
		});
	}

	/** @param {number} id @param {number} realm @returns {boolean} */
	releaseLocal(id, realm) {
		return this.synchronized(() => {
			const record = this.findLocked(id);
			this.localDeltaLocked(record, realm, -1);
			return this.releaseLocked(record, 1);
		});
	}

	/** A packet's owners move together before it is posted to another realm.
	 * The target can be terminated concurrently, so this is one transaction.
	 * @param {number[]} ids @param {number} from @param {number} to
	 */
	moveOwners(ids, from, to) {
		this.synchronized(() => {
			this.checkRealmLocked(to);
			const counts = new Map();
			for (const id of ids) counts.set(id, (counts.get(id) ?? 0) + 1);
			for (const [id, count] of counts) {
				const record = this.findLocked(id);
				if (this.localCountLocked(record, from) < count)
					throw new Error("a Fa packet has no transit owner");
			}
			if (from === to) return;
			const added = [];
			try {
				for (const [id, count] of counts) {
					this.localDeltaLocked(this.findLocked(id), to, count);
					added.push([id, count]);
				}
			} catch (error) {
				for (const [id, count] of added) this.localDeltaLocked(this.findLocked(id), to, -count);
				throw error;
			}
			for (const [id, count] of counts) this.localDeltaLocked(this.findLocked(id), from, -count);
		});
	}

	/** A successful publication turns transit owners into record edges.
	 * @param {number[]} ids @param {number} realm
	 */
	storeOwners(ids, realm) {
		this.synchronized(() => {
			for (const id of ids) this.localDeltaLocked(this.findLocked(id), realm, -1);
		});
	}

	/** After the host confirms a realm is no longer running, its unfinished
	 * scopes and queued packets can have no future reads. @param {number} realm
	 * @returns {number[]} the cells whose last owner was in this realm
	 */
	closeRealm(realm) {
		return this.synchronized(() => {
			this.checkRealmLocked(realm);
			const final = [];
			for (let bucket = 0; bucket < FaTransportMemory.buckets; bucket += 1) {
				let record = this.word(64 + bucket * 4);
				while (record !== 0) {
					const count = this.localCountLocked(record, realm);
					if (count !== 0) {
						this.localDeltaLocked(record, realm, -count);
						if (this.releaseLocked(record, count)) final.push(Number(this.longs[(record + 8) / 8]));
					}
					record = this.word(record);
				}
			}
			this.discardLocked(this.findLocked(realm));
			return final;
		});
	}

	/** @param {number} record @param {number} count @returns {boolean} */
	releaseLocked(record, count) {
		const previous = Atomics.load(this.longs, (record + 16) / 8);
		if (count <= 0 || previous < BigInt(count))
			throw new Error("a shared Fa owner was released twice");
		Atomics.sub(this.longs, (record + 16) / 8, BigInt(count));
		if (previous !== BigInt(count)) return false;
		this.setWord(record + 4, 2);
		return true;
	}

	/** The last owner finalizes outside all allocator locks.
	 * @param {number} id @returns {boolean}
	 */
	release(id) {
		return this.synchronized(() => {
			return this.releaseLocked(this.findLocked(id), 1);
		});
	}

	/** Copies a record while locked, then Codec can decode it reentrantly.
	 * @param {number} id
	 * @returns {{name:string, kind:number, owners:number, bytes:Uint8Array, edges:number[]}}
	 */
	read(id) {
		return this.synchronized(() => {
			const record = this.findLocked(id);
			const name = new TextDecoder().decode(
				this.bytes.subarray(
					record + FaTransportMemory.recordSize,
					record + FaTransportMemory.recordSize + this.word(record + 36),
				),
			);
			const offset = this.word(record + 40);
			const length = this.word(record + 44);
			const edgeOffset = this.word(record + 52);
			const count = this.word(record + 56);
			const edges = [];
			for (let index = 0; index < count; index += 1) {
				edges.push(Number(this.longs[edgeOffset / 8 + index]));
			}
			return {
				name,
				kind: this.word(record + 28),
				owners: Number(this.longs[(record + 16) / 8]),
				bytes: this.bytes.slice(offset, offset + length),
				edges,
			};
		});
	}

	/** Publishes already-retained edges and returns the previous owners.
	 * @param {number} id @param {Uint8Array} bytes @param {number[]} edges
	 * @returns {number[]}
	 */
	write(id, bytes, edges = []) {
		return this.synchronized(() => {
			const record = this.findLocked(id);
			const payload = this.allocateLocked(bytes.length);
			let edgeOffset = 0;
			try {
				edgeOffset = edges.length === 0 ? 0 : this.allocateLocked(edges.length * 8);
				this.bytes.set(bytes, payload);
				for (let index = 0; index < edges.length; index += 1) {
					this.longs[edgeOffset / 8 + index] = BigInt(edges[index]);
				}
			} catch (error) {
				this.freeLocked(payload);
				this.freeLocked(edgeOffset);
				throw error;
			}
			const previous = this.word(record + 52);
			const count = this.word(record + 56);
			const oldEdges = [];
			for (let index = 0; index < count; index += 1)
				oldEdges.push(Number(this.longs[previous / 8 + index]));
			this.freeLocked(this.word(record + 40));
			this.freeLocked(previous);
			this.setWord(record + 40, payload);
			this.setWord(record + 44, bytes.length);
			this.setWord(record + 52, edgeOffset);
			this.setWord(record + 56, edges.length);
			if (this.word(record + 4) === 0) this.setWord(record + 4, 1);
			return oldEdges;
		});
	}

	/** Aborts an unpublished promotion transaction. No external realm can
	 * own these ids; their original local owners have not changed yet.
	 * @param {number} id
	 */
	discard(id) {
		this.synchronized(() => {
			this.discardLocked(this.findLocked(id));
		});
	}

	/** @param {number} record */
	discardLocked(record) {
		const id = Number(this.longs[(record + 8) / 8]);
		const bucket = 64 + (id % FaTransportMemory.buckets) * 4;
		let link = bucket;
		while (this.word(link) !== record) link = this.word(link);
		this.setWord(link, this.word(record));
		this.freeLocked(this.word(record + 40));
		this.freeLocked(this.word(record + 52));
		let entry = this.word(record + 24);
		while (entry !== 0) {
			const next = this.word(entry);
			this.freeLocked(entry);
			entry = next;
		}
		this.freeLocked(record);
	}

	/** Finalizer finished; remove the directory entry and return its edges.
	 * @param {number} id @returns {number[]}
	 */
	finish(id) {
		return this.synchronized(() => {
			const record = this.findLocked(id);
			if (this.word(record + 4) !== 2 || this.longs[(record + 16) / 8] !== 0n) {
				throw new Error("a shared instance is still owned");
			}
			const edgeOffset = this.word(record + 52);
			const count = this.word(record + 56);
			const edges = [];
			for (let index = 0; index < count; index += 1)
				edges.push(Number(this.longs[edgeOffset / 8 + index]));
			this.discardLocked(record);
			return edges;
		});
	}
}
