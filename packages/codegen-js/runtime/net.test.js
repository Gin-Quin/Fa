// TCP sockets on Bun (design/async.md §5): a listener, connections, reads
// and writes, errors, and cancelled waits.

import { describe, expect, test } from "bun:test";

import {
	faAccept,
	faConnect,
	faNetHost,
	faSocketClose,
	faSocketRead,
	faSocketWrite,
	FaTcpListener,
	NetError,
} from "./net.js";
import { faCancel, faCancelSignal, FaTask } from "./tasks.js";

describe("faNetHost", () => {
	test("takes IPv4 addresses and localhost only", () => {
		expect(faNetHost("localhost")).toBe("127.0.0.1");
		expect(faNetHost("10.0.0.255")).toBe("10.0.0.255");
		expect(faNetHost("10.0.0.256")).toBeNull();
		expect(faNetHost("example.com")).toBeNull();
	});
});

describe("sockets", () => {
	test("an echo through a listener on a free port", async () => {
		const task = new FaTask(null);
		const listener = new FaTcpListener("127.0.0.1", 0);
		expect(listener.port).toBeGreaterThan(0);
		const accepting = faAccept(task, listener);
		const client = await faConnect(new FaTask(null), "localhost", listener.port);
		const server = await accepting;
		await faSocketWrite(task, client, [1, 2, 3, 4], 3);
		const buffer = [0, 0, 0, 0, 0];
		let received = 0;
		while (received < 3) {
			received += await faSocketRead(task, server, buffer);
		}
		expect(buffer).toEqual([1, 2, 3, 0, 0]);
		faSocketClose(client);
		expect(await faSocketRead(task, server, buffer)).toBe(0);
		faSocketClose(server);
		faSocketClose(listener);
	});

	test("errors are NetErrors with the system's text", async () => {
		const task = new FaTask(null);
		await expect(faConnect(task, "127.0.0.1", 1)).rejects.toEqual(
			new NetError("connect: Connection refused"),
		);
		await expect(faConnect(task, "example.invalid", 80)).rejects.toBeInstanceOf(NetError);
		const listener = new FaTcpListener("127.0.0.1", 0);
		const taken = new FaTcpListener("127.0.0.1", listener.port);
		await expect(faAccept(task, taken)).rejects.toEqual(
			new NetError("can't listen: Address already in use"),
		);
		faSocketClose(listener);
	});

	test("a task waiting in accept is cancelled at its wait", async () => {
		const task = new FaTask(null);
		const listener = new FaTcpListener("127.0.0.1", 0);
		const accepting = faAccept(task, listener);
		faCancel(task);
		await expect(accepting).rejects.toBe(faCancelSignal);
		faSocketClose(listener);
	});
});
