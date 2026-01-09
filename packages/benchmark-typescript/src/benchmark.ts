import path from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";

const packageRoot = path.resolve(
	path.dirname(fileURLToPath(import.meta.url)),
	"..",
);
const filePath = path.join(packageRoot, "fixtures", "long-file.ts");

const compilerOptions: ts.CompilerOptions = {
	noEmit: true,
	skipLibCheck: true,
	target: ts.ScriptTarget.ES2022,
	module: ts.ModuleKind.ESNext,
	strict: true,
};

const runTypecheck = (label: string) => {
	const start = process.hrtime.bigint();
	const host = ts.createCompilerHost(compilerOptions, true);
	const program = ts.createProgram([filePath], compilerOptions, host);
	ts.getPreEmitDiagnostics(program);
	const end = process.hrtime.bigint();
	const elapsedUs = Number((end - start) / 1_000n);

	console.log(`\n[RUN] ${label}`);
	console.log(`[TIMER] ${elapsedUs}us`);
};

runTypecheck("cold");
runTypecheck("warm");
runTypecheck("hot");
