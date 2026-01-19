.PHONY: help dev build check refactor refactor-preview refactor-interactive show-turbomcp-versions

# ============================================================================
# Configuration: TurboMCP Versions
# ============================================================================
# TurboMCP is fetched from crates.io automatically
# These versions match src-tauri/Cargo.toml
TURBOMCP_VERSION ?= 2.0.4
TURBOMCP_CLIENT_VERSION ?= 2.0.4
TURBOMCP_PROTOCOL_VERSION ?= 2.0.4
TURBOMCP_TRANSPORT_VERSION ?= 2.0.4
TURBOMCP_AUTH_VERSION ?= 2.2
TURBOMCP_DPOP_VERSION ?= 2.2

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
# TurboMCP Dependency Information
# ============================================================================

# Show current TurboMCP versions (informational only - already using crates.io)
show-turbomcp-versions:
	@echo "Current TurboMCP Versions (from crates.io):"
	@echo ""
	@echo "  • turbomcp = \"$(TURBOMCP_VERSION)\""
	@echo "  • turbomcp-client = \"$(TURBOMCP_CLIENT_VERSION)\""
	@echo "  • turbomcp-protocol = \"$(TURBOMCP_PROTOCOL_VERSION)\""
	@echo "  • turbomcp-transport = \"$(TURBOMCP_TRANSPORT_VERSION)\""
	@echo "  • turbomcp-auth = \"$(TURBOMCP_AUTH_VERSION)\""
	@echo "  • turbomcp-dpop = \"$(TURBOMCP_DPOP_VERSION)\""
	@echo ""
	@echo "All dependencies are fetched from crates.io automatically."
	@echo "No local turbomcp repository needed."

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
	@echo "TurboMCP versions:"
	@echo "  make show-turbomcp-versions  (show current crates.io versions)"
	@echo ""
	@echo "Docs: See .ast-grep-rules/README.md for detailed pattern documentation"
