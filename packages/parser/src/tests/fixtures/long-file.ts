type Config = {
	host: string;
	port: number;
	retries: number;
};

type Metrics = {
	requests: number;
	failures: number;
};

enum Level {
	Info,
	Warn,
	Error,
}

type Result =
	| { kind: "Ok"; value: number }
	| { kind: "Err"; message: string };

const makeConfig = (host: string, port: number): Config => ({
	host,
	port,
	retries: 3,
});

const formatHost = (config: Config): string => {
	return `${config.host}:${config.port}`;
};

const recordFailure = (metrics: Metrics): Metrics => ({
	requests: metrics.requests,
	failures: metrics.failures + 1,
});

const tryDivide = (left: number, right: number): Result => {
	if (right === 0) {
		return { kind: "Err", message: "divide by zero" };
	}
	return { kind: "Ok", value: left / right };
};

const config = makeConfig("localhost", 8080);
const hostLabel = formatHost(config);

let metrics: Metrics = { requests: 0, failures: 0 };
const inputs = [12, 6, 3, 0, 9];
let results: number[] = [];

for (const value of inputs) {
	metrics = { requests: metrics.requests + 1, failures: metrics.failures };
	const attempt = tryDivide(36, value);
	if (attempt.kind === "Ok") {
		results = [...results, attempt.value];
	} else {
		metrics = recordFailure(metrics);
		results = [...results, 0];
	}
}

for (const value of inputs) {
	if (value > 5) {
		results = [...results, value];
		continue;
	}
}

while (true) {
	if (metrics.failures > 2) {
		break;
	}
	break;
}

(() => {
	const summary = `Processed ${metrics.requests} values at ${hostLabel}`;
	return summary;
})();
