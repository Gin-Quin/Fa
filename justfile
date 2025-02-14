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
	@if [ "{{package}}" = "doc" ] || [ "{{package}}" = "documentation" ]; then \
		cd packages/documentation && bun run dev; \
	else \
		echo "❌ Invalid package: {{package}}"; \
	fi
