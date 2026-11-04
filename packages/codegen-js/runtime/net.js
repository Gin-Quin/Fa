// `std.net` on Bun and Node (design/async.md §5, step S1, JavaScript side):
// TCP sockets through the `node:net` module (`process.getBuiltinModule`).
//
// - Sockets can only be awaited there (a blocking call is `CONC010`): every
//   wait is a promise ended by the socket's events, or by the cancellation
//   of the waiting task (`task.wake`), as the other waits of `tasks.js`.
// - `TcpListener { host, port }` listens when it is evaluated
//   (`FaTcpListener`); a failure (a port taken, an unknown host) is thrown
//   by its first `accept()`. On Bun, `listener.port` is known at once; on
//   Node, the port of `port = 0` is known once the listener listens (after
//   the first wait).
// - A stream keeps what it received until `read` takes it, and pauses the
//   socket beyond 64 KiB. `write` returns once the bytes are handed to the
//   system.
// - Sockets don't keep the process alive by themselves (`unref`): only
//   while a task waits on them, so that a deadlock is still detected and a
//   socket dropped without `close()` doesn't hold the program.
// - Errors are `NetError { message }`, `"<operation>: <the system's text>"`,
//   as natively.
//
// Each helper starts at a `// @fa-helper <name> deps: <names>` line and ends at
// the next one: `fa-codegen-js` extracts only the helpers a program uses.

import { FaError } from "./errors.js";
import { faSystemMessage } from "./system.js";
import { faAlive } from "./tasks.js";

/** @typedef {import("./tasks.js").FaTask} FaTask */

// @fa-helper NetError deps: FaError
/** A failure of a socket (`std.net`). */
export class NetError extends FaError {
	static faName = "NetError";

	/** @param {string} message the operation, then the system's text */
	constructor(message) {
		super();
		/** @type {string} */
		this.message = message;
	}
}

// @fa-helper faNetHost
/**
 * The address of a host of `std.net`: an IPv4 address, or `localhost`
 * (`127.0.0.1`); `null` for anything else (no name resolution, as natively).
 * @param {string} host
 * @returns {string | null}
 */
export function faNetHost(host) {
	if (host === "localhost") {
		return "127.0.0.1";
	}
	const parts = host.split(".");
	if (parts.length !== 4) {
		return null;
	}
	for (const part of parts) {
		if (!/^[0-9]{1,3}$/.test(part) || Number(part) > 255) {
			return null;
		}
	}
	return host;
}

// @fa-helper FaSocketWaits deps: faAlive
/**
 * The tasks waiting on a socket: the socket keeps the process alive while
 * one waits, and an event wakes them all (each checks again what it waits
 * for).
 */
export class FaSocketWaits {
	/** @param {any} handle the `net.Server` or `net.Socket` */
	constructor(handle) {
		/** @type {any} */
		this.handle = handle;
		/** @type {Set<() => void>} */
		this.waiters = new Set();
	}

	/** Wakes the waiting tasks. */
	notify() {
		const waiters = this.waiters;
		this.waiters = new Set();
		for (const wake of waiters) {
			wake();
		}
	}

	/**
	 * Waits for the next event of the socket, or the cancellation of `task`.
	 * @param {FaTask} task
	 * @param {string} where what it waits on (for the deadlock message)
	 * @returns {Promise<void>}
	 */
	async wait(task, where) {
		if (this.waiters.size === 0) {
			this.handle?.ref?.();
		}
		/** @type {() => void} */
		let wake = () => {};
		await new Promise((resolve) => {
			wake = () => resolve(undefined);
			this.waiters.add(wake);
			task.wake = wake;
			task.where = where;
		});
		this.waiters.delete(wake);
		task.wake = null;
		if (this.waiters.size === 0) {
			this.handle?.unref?.();
		}
		faAlive(task);
	}
}

// @fa-helper FaTcpStream deps: FaSocketWaits
/** A `TcpStream`: a connected socket and the bytes received, not read yet. */
export class FaTcpStream {
	/**
	 * @param {any} socket a `net.Socket`
	 * @param {boolean} connected whether it is connected already (accepted)
	 */
	constructor(socket, connected) {
		/** @type {any} */
		this.socket = socket;
		/** @type {boolean} */
		this.connected = connected;
		/** @type {Uint8Array[]} */
		this.chunks = [];
		/** @type {number} the bytes of the first chunk already read */
		this.offset = 0;
		/** @type {number} the bytes received and not read */
		this.size = 0;
		/** @type {boolean} the other side closed the connection */
		this.ended = false;
		/** @type {any} */
		this.error = null;
		/** @type {boolean} */
		this.closed = false;
		this.waits = new FaSocketWaits(socket);
		socket.setNoDelay?.(true);
		socket.on("connect", () => {
			this.connected = true;
			this.waits.notify();
		});
		socket.on("data", (/** @type {Uint8Array} */ chunk) => {
			this.chunks.push(chunk);
			this.size += chunk.length;
			if (this.size > 65536) {
				socket.pause();
			}
			this.waits.notify();
		});
		socket.on("end", () => {
			this.ended = true;
			this.waits.notify();
		});
		socket.on("close", () => {
			this.ended = true;
			this.waits.notify();
		});
		socket.on("error", (/** @type {any} */ error) => {
			this.error ??= error;
			this.waits.notify();
		});
		socket.unref();
	}
}

// @fa-helper FaTcpListener deps: FaSocketWaits FaTcpStream NetError faNetHost faSystemMessage
/** A `TcpListener`: a listening socket and the connections not accepted yet. */
export class FaTcpListener {
	/**
	 * `TcpListener { host, port }`: listens at once.
	 * @param {string} host
	 * @param {number} port
	 */
	constructor(host, port) {
		/** @type {FaTcpStream[]} */
		this.queue = [];
		/** @type {NetError | null} the failure thrown by `accept` */
		this.error = null;
		/** @type {number} */
		this.requested = port;
		/** @type {any} */
		this.server = null;
		this.waits = new FaSocketWaits(null);
		/** @type {any} */
		const host_ = globalThis;
		const net = host_.process?.getBuiltinModule?.("node:net");
		const address = faNetHost(host);
		if (net === undefined) {
			this.error = new NetError("can't listen: this platform has no sockets");
			return;
		}
		if (address === null) {
			this.error = new NetError("can't listen: unknown host (an IPv4 address or localhost)");
			return;
		}
		const server = net.createServer();
		this.server = server;
		this.waits.handle = server;
		server.on("connection", (/** @type {any} */ socket) => {
			this.queue.push(new FaTcpStream(socket, true));
			this.waits.notify();
		});
		server.on("error", (/** @type {any} */ error) => {
			this.error ??= new NetError(`can't listen: ${faSystemMessage(error)}`);
			this.waits.notify();
		});
		server.listen(port, address);
		server.unref();
	}

	/** The port it listens on. */
	get port() {
		return this.server?.address()?.port ?? this.requested;
	}
}

// @fa-helper faAccept
/**
 * `await listener.accept()`.
 * @param {FaTask} task
 * @param {FaTcpListener} listener
 * @returns {Promise<FaTcpStream>}
 */
export async function faAccept(task, listener) {
	for (;;) {
		const stream = listener.queue.shift();
		if (stream !== undefined) {
			return stream;
		}
		if (listener.error !== null) {
			throw listener.error;
		}
		await listener.waits.wait(task, "TcpListener.accept");
	}
}

// @fa-helper faConnect deps: FaTcpStream NetError faNetHost faSystemMessage
/**
 * `await TcpStream.connect(host, port)`.
 * @param {FaTask} task
 * @param {string} host
 * @param {number} port
 * @returns {Promise<FaTcpStream>}
 */
export async function faConnect(task, host, port) {
	/** @type {any} */
	const host_ = globalThis;
	const net = host_.process?.getBuiltinModule?.("node:net");
	const address = faNetHost(host);
	if (net === undefined) {
		throw new NetError("connect: this platform has no sockets");
	}
	if (address === null) {
		throw new NetError("connect: unknown host (an IPv4 address or localhost)");
	}
	const stream = new FaTcpStream(net.connect({ host: address, port }), false);
	let done = false;
	try {
		while (!stream.connected && stream.error === null) {
			await stream.waits.wait(task, "TcpStream.connect");
		}
		if (stream.error !== null) {
			throw new NetError(`connect: ${faSystemMessage(stream.error)}`);
		}
		done = true;
		return stream;
	} finally {
		if (!done) {
			stream.socket.destroy();
		}
	}
}

// @fa-helper faSocketRead deps: NetError faSystemMessage
/**
 * `await stream.read(buffer)`: fills the first elements of `buffer` with
 * the bytes received, and gives their count (0 once the other side closed
 * the connection).
 * @param {FaTask} task
 * @param {FaTcpStream} stream
 * @param {number[]} buffer
 * @returns {Promise<number>}
 */
export async function faSocketRead(task, stream, buffer) {
	for (;;) {
		if (stream.closed) {
			throw new NetError("read: Bad file descriptor");
		}
		if (stream.size > 0 && buffer.length > 0) {
			let count = 0;
			while (count < buffer.length && stream.chunks.length > 0) {
				const chunk = stream.chunks[0];
				const taken = Math.min(buffer.length - count, chunk.length - stream.offset);
				for (let index = 0; index < taken; index++) {
					buffer[count + index] = chunk[stream.offset + index];
				}
				count += taken;
				stream.offset += taken;
				if (stream.offset === chunk.length) {
					stream.chunks.shift();
					stream.offset = 0;
				}
			}
			stream.size -= count;
			if (stream.size <= 65536 && stream.socket.isPaused?.()) {
				stream.socket.resume();
			}
			return count;
		}
		if (stream.error !== null) {
			throw new NetError(`read: ${faSystemMessage(stream.error)}`);
		}
		if (stream.ended || buffer.length === 0) {
			return 0;
		}
		await stream.waits.wait(task, "TcpStream.read");
	}
}

// @fa-helper faSocketWrite deps: NetError faSystemMessage
/**
 * `await stream.write(bytes, count)`: the first `count` bytes (all of them
 * by default), handed to the system before it returns.
 * @param {FaTask} task
 * @param {FaTcpStream} stream
 * @param {number[]} bytes
 * @param {number} [count]
 * @returns {Promise<void>}
 */
export async function faSocketWrite(task, stream, bytes, count = bytes.length) {
	if (stream.closed) {
		throw new NetError("write: Bad file descriptor");
	}
	if (stream.error !== null) {
		throw new NetError(`write: ${faSystemMessage(stream.error)}`);
	}
	const length = Math.max(0, Math.min(count, bytes.length));
	const data = Uint8Array.from(length === bytes.length ? bytes : bytes.slice(0, length));
	/** @type {any} */
	let failure = null;
	let written = false;
	stream.socket.write(data, (/** @type {any} */ error) => {
		failure = error ?? null;
		written = true;
		stream.waits.notify();
	});
	while (!written && stream.error === null) {
		await stream.waits.wait(task, "TcpStream.write");
	}
	failure ??= stream.error;
	if (failure !== null) {
		throw new NetError(`write: ${faSystemMessage(failure)}`);
	}
}

// @fa-helper faSocketClose deps: FaTcpStream
/**
 * `close()` of a listener or a stream (it is dropped).
 * @param {FaTcpListener | FaTcpStream} socket
 * @returns {void}
 */
export function faSocketClose(socket) {
	if (socket instanceof FaTcpStream) {
		if (!socket.closed) {
			socket.closed = true;
			socket.socket.end();
			socket.socket.unref();
		}
		return;
	}
	socket.server?.close();
	for (const stream of socket.queue) {
		faSocketClose(stream);
	}
	socket.queue = [];
}
