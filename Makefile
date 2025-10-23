.PHONY: help dev build check refactor refactor-turbomcp-local refactor-preview refactor-interactive

# ============================================================================
# Configuration: Turbomcp Versions
# ============================================================================
# Detect versions from local turbomcp repo (../turbomcp/), or override:
# Usage: make refactor-turbomcp-local TURBOMCP_VERSION=1.2.3
TURBOMCP_VERSION ?= $(shell grep '^version = ' ../turbomcp/crates/turbomcp/Cargo.toml 2>/dev/null | head -1 | sed 's/version = "//;s/"//;' || echo "2.0.4")
TURBOMCP_CLIENT_VERSION ?= $(shell grep '^version = ' ../turbomcp/crates/turbomcp-client/Cargo.toml 2>/dev/null | head -1 | sed 's/version = "//;s/"//;' || echo "2.0.4")
TURBOMCP_PROTOCOL_VERSION ?= $(shell grep '^version = ' ../turbomcp/crates/turbomcp-protocol/Cargo.toml 2>/dev/null | head -1 | sed 's/version = "//;s/"//;' || echo "2.0.4")
TURBOMCP_TRANSPORT_VERSION ?= $(shell grep '^version = ' ../turbomcp/crates/turbomcp-transport/Cargo.toml 2>/dev/null | head -1 | sed 's/version = "//;s/"//;' || echo "2.0.4")

# Default target
help:
	@echo "TurboMCP Studio - Development Tasks"
	@echo ""
	@echo "Development:"
	@echo "  make dev              - Start development server"
	@echo "  make build            - Build project"
	@echo "  make check            - Type check with svelte-check"
	@echo ""
	@echo "Refactoring:"
	@echo "  make refactor-list    - Show available refactoring patterns"
	@echo "  make refactor-turbomcp-local - Convert local turbomcp deps to crates.io"
	@echo "  make refactor-preview - Preview all active refactoring patterns"
	@echo "  make refactor-interactive - Interactive refactoring (review each change)"
	@echo ""
	@echo "Advanced:"
	@echo "  make refactor PATTERN=<pattern-name> - Run specific pattern"

# ============================================================================
# Development Targets
# ============================================================================

dev:
	pnpm run dev

build:
	pnpm run build

check:
	pnpm run check

check-watch:
	pnpm run check:watch

# ============================================================================
# Refactoring Targets
# ============================================================================

# List all available patterns
refactor-list:
	@echo "Available refactoring patterns:"
	@echo ""
	@ls -1 .ast-grep-rules/patterns/*.yml 2>/dev/null | while read f; do \
		echo "  - $$(basename $$f)"; \
		grep -E "^id:" $$f | head -1 | sed 's/id: //g' | sed 's/^/    /'; \
	done
	@echo ""
	@echo "Usage: make refactor PATTERN=<pattern-name>"

# Preview all patterns (dry run, no changes)
refactor-preview:
	@echo "Previewing all refactoring patterns..."
	@echo ""
	@for pattern in .ast-grep-rules/patterns/*.yml; do \
		if [ -f "$$pattern" ]; then \
			echo "=== $$(basename $$pattern) ==="; \
			ast-grep scan -r "$$pattern" 2>&1 | head -10; \
			echo ""; \
		fi; \
	done

# Interactive refactoring (shows each match, allows approval)
refactor-interactive:
	@echo "Running refactoring patterns interactively..."
	@echo "You will be prompted to approve each change."
	@echo ""
	@for pattern in .ast-grep-rules/patterns/*.yml; do \
		if [ -f "$$pattern" ]; then \
			echo "=== $$(basename $$pattern) ==="; \
			ast-grep scan -r "$$pattern" -i || true; \
			echo ""; \
		fi; \
	done

# Run a specific pattern
refactor:
	@if [ -z "$(PATTERN)" ]; then \
		echo "Error: PATTERN not specified"; \
		echo "Usage: make refactor PATTERN=<pattern-name>"; \
		echo ""; \
		make refactor-list; \
		exit 1; \
	fi
	@if [ ! -f ".ast-grep-rules/patterns/$(PATTERN).yml" ]; then \
		echo "Error: Pattern not found: .ast-grep-rules/patterns/$(PATTERN).yml"; \
		exit 1; \
	fi
	@echo "Running pattern: $(PATTERN)"
	@echo ""
	ast-grep scan -r ".ast-grep-rules/patterns/$(PATTERN).yml" -i

# ============================================================================
# Turbomcp Dependency Refactoring
# ============================================================================

# Convert turbomcp dependencies from local path to crates.io version
# Usage: make refactor-turbomcp-local
refactor-turbomcp-local:
	@echo "Converting turbomcp dependencies from local path to crates.io version"
	@echo ""
	@echo "Detected versions from local repo:"
	@echo "  • turbomcp = \"$(TURBOMCP_VERSION)\""
	@echo "  • turbomcp-client = \"$(TURBOMCP_CLIENT_VERSION)\""
	@echo "  • turbomcp-protocol = \"$(TURBOMCP_PROTOCOL_VERSION)\""
	@echo "  • turbomcp-transport = \"$(TURBOMCP_TRANSPORT_VERSION)\""
	@echo ""
	@echo "Files to be modified:"
	@find . -name "Cargo.toml" -type f | grep -v ".git" | grep -v "target"
	@echo ""
	@echo "Running sed-based refactoring..."
	@echo ""
	@for file in $$(find . -name "Cargo.toml" -type f | grep -v ".git" | grep -v "target"); do \
		if grep -q 'path = "' "$$file" && grep -q "turbomcp" "$$file"; then \
			echo "  Updating: $$file"; \
			sed -i.bak 's/turbomcp = { version = "[^"]*", path = "[^"]*" }/turbomcp = "$(TURBOMCP_VERSION)"/g' "$$file"; \
			sed -i.bak 's/turbomcp-client = { version = "[^"]*", path = "[^"]*" }/turbomcp-client = "$(TURBOMCP_CLIENT_VERSION)"/g' "$$file"; \
			sed -i.bak 's/turbomcp-protocol = { version = "[^"]*", path = "[^"]*" }/turbomcp-protocol = "$(TURBOMCP_PROTOCOL_VERSION)"/g' "$$file"; \
			sed -i.bak 's/turbomcp-transport = { version = "[^"]*", path = "[^"]*" }/turbomcp-transport = "$(TURBOMCP_TRANSPORT_VERSION)"/g' "$$file"; \
			rm -f "$$file.bak"; \
		fi; \
	done
	@echo ""
	@echo "✅ Refactoring complete!"
	@echo ""
	@echo "Review changes with: git diff"
	@echo "Commit with: git add -A && git commit -m 'refactor: update turbomcp deps to crates.io v$(TURBOMCP_VERSION)'"
	@echo "Undo with: git checkout ."

# Preview changes before applying
refactor-turbomcp-preview:
	@echo "Preview: Local turbomcp → crates.io conversion"
	@echo ""
	@echo "Using versions from local repo:"
	@echo "  • turbomcp v$(TURBOMCP_VERSION)"
	@echo "  • turbomcp-client v$(TURBOMCP_CLIENT_VERSION)"
	@echo "  • turbomcp-protocol v$(TURBOMCP_PROTOCOL_VERSION)"
	@echo "  • turbomcp-transport v$(TURBOMCP_TRANSPORT_VERSION)"
	@echo ""
	@echo "Files to be modified:"
	@find . -name "Cargo.toml" -type f | grep -v ".git" | grep -v "target" | while read f; do \
		if grep -q 'path = "' "$$f" && grep -q "turbomcp" "$$f"; then \
			echo "  - $$f"; \
			echo "    Current lines with local path:"; \
			grep "turbomcp.*path = " "$$f" | sed 's/^/      /'; \
			echo "    Will be converted to crates.io versions"; \
		fi; \
	done
	@echo ""
	@echo "To apply changes: make refactor-turbomcp-local"

# ============================================================================
# Utility Targets
# ============================================================================

# Show git diff of changes
diff:
	git diff

# Undo all changes
clean:
	git checkout .
	@echo "All changes reverted"

# Help for refactoring
refactor-help:
	@echo "Refactoring Guide"
	@echo ""
	@echo "Available patterns:"
	@make refactor-list
	@echo ""
	@echo "Workflow:"
	@echo "  1. make refactor-preview      (preview all patterns)"
	@echo "  2. make refactor-interactive  (review & approve each change)"
	@echo "  3. git diff                   (verify changes)"
	@echo "  4. git add . && git commit -m \"refactor: ...\"  (commit)"
	@echo ""
	@echo "Or for turbomcp specifically:"
	@echo "  1. make refactor-turbomcp-preview  (preview changes)"
	@echo "  2. make refactor-turbomcp-local    (apply changes)"
	@echo "  3. git diff src-tauri/Cargo.toml   (verify)"
	@echo ""
	@echo "Docs: See .ast-grep-rules/README.md for detailed pattern documentation"
