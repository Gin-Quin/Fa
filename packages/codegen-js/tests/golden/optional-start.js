// helpers: faIsBytes, faQuote, faFormatRange, faSides, faFormatNested, faFormat, FaTask, faSched, faEnd, faCancel, faDrop, faDropTasks, faIsError, faCancelSignal, faExit, FaStop, faStopMessage, faIsStackOverflow, faWriteError, faReport, faFatal, faSettle, faRunTask, faStartCall, faFinished, faDetach, faLater, faMacrotask, faReady, faAlive, faWhenDone, faFormatError, FaError, Cancelled, faResult, faJoin, faRunAsync

const copy$Box = (value) => ({ name: value.name });

function defaultSuffix() {
	console.log("default");
	return "!";
}

function Box$describe(self, suffix = defaultSuffix()) {
	return self.name + suffix;
}

function Box$maybe(self) {
	return undefined;
}

function Box$length(self) {
	return self.name.length;
}

function Box$tagged(self, value) {
	return value;
}

async function check($task, box) {
	const task = box === undefined ? undefined : faStartCall($task, Box$describe, box);
	try {
		if (task === undefined) {
			console.log("none");
		} else {
			console.log(await faJoin($task, task));
		}
		const size = box === undefined ? undefined : faStartCall($task, Box$length, box);
		try {
			if (!(size === undefined)) {
				console.log(String(await faJoin($task, size)));
			}
			const nullable = box === undefined ? undefined : faStartCall($task, Box$maybe, box);
			try {
				if (!(nullable === undefined)) {
					console.log(faFormat(await faJoin($task, nullable)));
				}
				const text = !(box === undefined) ? box.name : undefined;
				const trimmed = text === undefined ? undefined : faFinished(() => text.trim());
				try {
					if (!(trimmed === undefined)) {
						console.log(await faJoin($task, trimmed));
					}
					let $t1;
					if (box !== undefined) {
						const $t0 = copy$Box(box);
						$t1 = faDetach(faStartCall($task, Box$describe, $t0, "detached"));
					}
					let $t3;
					if (box !== undefined) {
						let $t2;
						if (!(box === undefined)) {
							$t2 = 7;
						} else {
							console.log("bad argument");
							$t2 = 0;
						}
						$t3 = faStartCall($task, Box$tagged, box, $t2);
					}
					const skipped = $t3;
					try {
						if (!(skipped === undefined)) {
							console.log(String(await faJoin($task, skipped)));
						}
					} finally { faDropTasks(skipped); }
				} finally { faDropTasks(trimmed); }
			} finally { faDropTasks(nullable); }
		} finally { faDropTasks(size); }
	} finally { faDropTasks(task); }
}

async function main($task) {
	await check($task, undefined);
	await check($task, { name: "Ada" });
}

faRunAsync(main);
