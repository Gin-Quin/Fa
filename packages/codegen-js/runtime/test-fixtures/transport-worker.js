import { FaTransportMemory } from "../transport.js";

globalThis.onmessage = (event) => {
	const { buffer, id, rounds, create } = event.data;
	const arena = new FaTransportMemory(buffer);
	if (create) {
		const fresh = arena.create("late", 1);
		arena.write(fresh, new Uint8Array([7, 8, 9]));
		globalThis.postMessage({ id: fresh });
		return;
	}
	for (let index = 0; index < rounds; index += 1) {
		arena.retain(id);
		if (arena.release(id)) throw new Error("the parent still owns this instance");
		const temporary = arena.create("temporary", 1);
		arena.write(temporary, new Uint8Array([index & 255]));
		if (!arena.release(temporary)) throw new Error("the temporary has one owner");
		arena.finish(temporary);
	}
	globalThis.postMessage({ done: true });
};
