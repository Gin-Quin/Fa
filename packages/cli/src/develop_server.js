// The development server of `fa develop --target=web` (`develop.rs`): it
// serves the web build of the program, and relays the requests of `fa
// develop` (its channel, `FA_HOT_CHANNEL`) to the pages open in browsers
// (a WebSocket on `/$fa-hot`, `faHotListen`): `swap` swaps modules in every
// page and answers with the first page's answer (with none open, the swap
// is done: the pages load the new modules), `reload` reloads every page.
// `fa develop` fills in the root folder and the port.
import { connect } from "node:net";

const root = __ROOT__;
const port = __PORT__;
const page =
	'<!doctype html><meta charset="utf-8"><title>fa develop</title><script type="module" src="/main.js"></script>';
/** @type {Set<any>} */
const pages = new Set();
/** @type {Array<(answer: string) => void>} */
let waiting = [];

const server = Bun.serve({
	port,
	async fetch(request, server) {
		const path = decodeURIComponent(new URL(request.url).pathname);
		if (path === "/$fa-hot") {
			return server.upgrade(request)
				? undefined
				: new Response("expected a WebSocket", { status: 400 });
		}
		const headers = { "cache-control": "no-store" };
		if (path === "/" || path === "/index.html") {
			const index = Bun.file(`${root}/index.html`);
			return (await index.exists())
				? new Response(index, { headers })
				: new Response(page, { headers: { ...headers, "content-type": "text/html" } });
		}
		if (path.includes("..")) {
			return new Response("not found", { status: 404 });
		}
		const file = Bun.file(root + path);
		return (await file.exists())
			? new Response(file, { headers })
			: new Response("not found", { status: 404 });
	},
	websocket: {
		open(socket) {
			pages.add(socket);
		},
		close(socket) {
			pages.delete(socket);
		},
		message(_socket, message) {
			const answers = waiting;
			waiting = [];
			for (const answer of answers) {
				answer(String(message));
			}
		},
	},
});
console.error(`fa develop: serving ${root} on http://localhost:${server.port}`);

/** The version of the swap `fa develop` prepared (`$hot.json`). */
async function version() {
	try {
		return (await Bun.file(`${root}/$hot.json`).json()).version;
	} catch {
		return 0;
	}
}

/** Swaps the modules in the pages: the first answer, or done without a page. */
async function swap() {
	if (pages.size === 0) {
		return JSON.stringify({ version: await version(), ok: true, pages: 0 });
	}
	const answer = new Promise((resolve) => waiting.push(resolve));
	for (const socket of pages) {
		socket.send("swap");
	}
	const late = new Promise((resolve) =>
		setTimeout(
			async () =>
				resolve(JSON.stringify({ version: await version(), ok: false, error: "no page answered" })),
			10_000,
		),
	);
	return await Promise.race([answer, late]);
}

const channel = Number(process.env.FA_HOT_CHANNEL);
if (channel) {
	const socket = connect(channel, "127.0.0.1");
	socket.setEncoding("utf8");
	socket.on("error", () => process.exit(0));
	socket.on("close", () => process.exit(0));
	let buffer = "";
	socket.on("data", async (/** @type {string} */ data) => {
		buffer += data;
		let end;
		while ((end = buffer.indexOf("\n")) >= 0) {
			const line = buffer.slice(0, end);
			buffer = buffer.slice(end + 1);
			if (line === "swap") {
				socket.write(`${await swap()}\n`);
			} else if (line === "reload") {
				for (const page of pages) {
					page.send("reload");
				}
				socket.write(`${JSON.stringify({ ok: true, pages: pages.size })}\n`);
			}
		}
	});
}
