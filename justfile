# Display all available commands
help:
    just --list

# alias for install
i: install

# Install dependencies for all packages
install:
    @for dir in packages/*; do \
    	if [ -f "$dir/package.json" ]; then \
    		echo "\n📦 Installing dependencies for $dir"; \
    		cd "$dir" && bun i && cd ../../; \
    	elif [ -f "$dir/Cargo.toml" ]; then \
    		echo "\n📦 Installing dependencies for $dir"; \
    		cd "$dir" && cargo build && cd ../../; \
    	fi \
    done

# Run dev for a specific package
dev package:
    @if [ "{{ package }}" = "doc" ] || [ "{{ package }}" = "documentation" ]; then \
    	cd packages/documentation && bun run dev; \
    else \
    	echo "❌ Invalid package: {{ package }}"; \
    fi

parse file="packages/parser/src/tests/fixtures/long-file.fa":
    @echo "Parsing file: {{ file }}"
    @bun -e 'const { spawnSync } = require("node:child_process"); const start = process.hrtime.bigint(); const result = spawnSync("target/release/fa-parser", ["{{ file }}"], { stdio: "inherit" }); if (result.status !== 0) { process.exit(result.status ?? 1); } const end = process.hrtime.bigint(); const elapsedUs = (end - start) / 1_000n; console.log(`Elapsed (us): ${elapsedUs}`);'

parse_ts file="packages/parser/src/tests/fixtures/long-file.ts":
    @echo "Type-checking file: {{ file }}"
    @bun -e 'const { spawnSync } = require("node:child_process"); const start = process.hrtime.bigint(); const result = spawnSync("bun", ["x", "tsc", "--noEmit", "{{ file }}"], { stdio: "inherit" }); if (result.status !== 0) { process.exit(result.status ?? 1); } const end = process.hrtime.bigint(); const elapsedUs = (end - start) / 1_000n; console.log(`Elapsed (us): ${elapsedUs}`);'
