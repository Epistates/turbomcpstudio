# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository Overview

This is **MCP Studio** - a desktop application to be the "Postman for Model Context Protocol (MCP)". The repository contains both strategic planning documentation and a Tauri + SvelteKit application scaffold.

**Current State**: Phase 1 Complete - Enterprise UI foundation with Tauri + SvelteKit integration. Features complete design system, responsive layouts, theme management, and Svelte 5 runes mode compatibility. The comprehensive vision is documented in `MCP_STUDIO_ANALYSIS.md`.

## Technology Stack

- **Desktop Framework**: Tauri 2.0 (Rust backend + SvelteKit frontend)
- **Frontend**: SvelteKit 5 with TypeScript + Tailwind CSS (Svelte 5 runes mode)
- **Backend**: Rust with TurboMCP client integration
- **Build Tool**: Vite
- **Package Manager**: pnpm (fast, efficient, disk space optimization)
- **Database**: SQLite (local-first with optional cloud sync)

## Development Commands

### Frontend Development
```bash
# Start development server
pnpm run dev

# Build for production
pnpm run build

# Preview production build
pnpm run preview

# Type checking
pnpm run check
pnpm run check:watch  # Watch mode

# Install dependencies
pnpm install
```

### Linting and Type Checking
```bash
# Type checking (critical for Svelte 5 runes mode)
pnpm run check          # One-time check
pnpm run check:watch    # Watch mode

# Format and lint (when configured)
pnpm run format         # Format with Prettier (if configured)
pnpm run lint           # ESLint checking (if configured)
```

### Tauri Development
```bash
# Start Tauri development (launches desktop app)
pnpm run tauri dev

# Build Tauri application
pnpm run tauri build

# Tauri-specific commands
pnpm run tauri -- --help
```

### Testing
```bash
# Rust backend tests
cd src-tauri && cargo test

# Frontend testing (when configured)
pnpm test              # Run tests with Vitest (if configured)
pnpm test:watch        # Watch mode
pnpm test:coverage     # Coverage report
```

Currently no test framework is configured. When implementing, consider:
- **Rust**: `cargo test` in `src-tauri/` - already supported
- **Frontend**: Vitest is recommended for SvelteKit 5

## Project Structure

```
├── src/                           # SvelteKit frontend source
│   ├── routes/                   # SvelteKit routing (file-based)
│   │   └── +page.svelte         # Main application page
│   ├── lib/                     # Shared components & utilities
│   │   ├── components/          # Svelte components
│   │   │   ├── layout/         # Layout system (MasterLayout, Header, Sidebar)
│   │   │   ├── ui/             # Reusable UI components (Button, etc.)
│   │   │   ├── Dashboard.svelte
│   │   │   ├── ToolExplorer.svelte
│   │   │   ├── ServerOverview.svelte
│   │   │   └── AddServerModal.svelte
│   │   ├── stores/              # Svelte stores for state management
│   │   │   ├── serverStore.ts   # MCP server management
│   │   │   ├── uiStore.ts       # UI state (sidebar, panels)
│   │   │   └── themeStore.ts    # Theme management (light/dark)
│   │   ├── styles/              # CSS architecture
│   │   │   ├── design-tokens.css # Design system tokens
│   │   │   └── components.css   # Component styles
│   │   └── types/               # TypeScript type definitions
│   └── app.html                 # Main HTML template
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── main.rs             # Entry point
│   │   ├── lib.rs              # App initialization & IPC handlers
│   │   ├── commands/           # Tauri command handlers
│   │   ├── database/           # SQLite database layer
│   │   ├── mcp_client/         # MCP client manager
│   │   ├── types/              # Rust type definitions
│   │   └── error/              # Error handling
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Tauri configuration
├── static/                     # Static assets
├── MCP_STUDIO_ANALYSIS.md      # Strategic vision document
├── MCP_STUDIO_TECHNICAL_DESIGN.md  # Technical architecture
└── CLAUDE.md                   # This file
```

## Architecture Vision

MCP Studio leverages a **native-first, MCP-protocol-aware** architecture that fundamentally differs from web-based tools like MCP Inspector. Built on comprehensive analysis of the MCP protocol specifications and TurboMCP's enterprise capabilities.

### Core Design Philosophy
- **Native Desktop First** - No web proxy, direct process spawning and IPC
- **MCP Protocol Native** - Deep understanding of bidirectional communication and capability negotiation  
- **Enterprise Production Ready** - Circuit breakers, connection pooling, security, monitoring
- **Performance Optimized** - SIMD-accelerated JSON processing, zero-copy message handling

### Three-Layer Architecture
```
┌─────────────────────────────────────────────────────────────────┐
│  Frontend (SvelteKit + TypeScript + Tailwind)                  │
│  • Operation-specific UIs for each MCP capability              │
│  • Real-time protocol visualization                            │  
│  • Collections and scenario management                         │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  Tauri Bridge Layer                                            │
│  • Type-safe IPC with serde serialization                      │
│  • Native process spawning and management                      │
│  • File system access and window management                    │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  Native Engine (Rust + TurboMCP)                               │
│  • Production-grade MCP client with enterprise features        │
│  • Multi-transport support (STDIO/HTTP/WebSocket/TCP/Unix)     │
│  • Process lifecycle management and health monitoring          │
│  • Protocol analysis and message replay capabilities           │
│  • SQLite storage with collections and history                 │
└─────────────────────────────────────────────────────────────────┘
```

### MCP Operation-Specific UIs
1. **Tool Explorer** - Auto-generated forms from JSON schemas, response visualization
2. **Resource Browser** - Tree navigation, URI templates, subscription management  
3. **Prompt Designer** - Visual composition, variable management, A/B testing
4. **Sampling Debugger** - Step-through debugging, token tracking, conversation history
5. **Elicitation Flow Builder** - Visual workflow designer, conditional logic, form validation

### Enterprise Features
- **Collections & Scenarios** - Multi-server operation chains with variable interpolation
- **Testing Framework** - Contract testing, performance benchmarking, load testing
- **Protocol Analysis** - Real-time message inspection, flow visualization, replay
- **Team Collaboration** - Shared workspaces, version control, CI/CD integration
- **Security** - Sandboxed execution, encrypted storage, audit logging

## 🚨 CRITICAL: TurboMCP Dogfooding Policy

**MCP Studio is a PRIMARY DOGFOODING APPLICATION for TurboMCP**

### Quality Assurance Mandate
When encountering ANY issues with TurboMCP dependencies:
1. **STOP immediately** - Do not implement workarounds
2. **Create detailed bug report** for TurboMCP developers
3. **Focus on maximum Developer Experience (DX)** and MCP compliance
4. **Expect TurboMCP to be robust** - issues should be fixed upstream

### Bug Report Template
When finding TurboMCP issues, provide:
```
## TurboMCP Issue Report

**Component**: [turbomcp-client|turbomcp-protocol|turbomcp-transport]
**Version**: [current version]
**Impact**: [Developer Experience|MCP Compliance|Performance]

### Problem Description
[Clear description of the issue]

### Expected Behavior (Maximum DX)
[What developers should expect from the API]

### Current Behavior
[What actually happens]

### MCP Compliance Impact
[How this affects MCP protocol compliance]

### Code Example
[Minimal reproduction case]

### Proposed Solution
[Ideal API design for maximum DX]
```

### Developer Experience Standards
TurboMCP must provide:
- **Intuitive APIs** - Natural, obvious usage patterns
- **Full Schema Access** - Complete tool/resource/prompt schemas
- **Type Safety** - Comprehensive TypeScript/Rust types
- **Error Clarity** - Clear, actionable error messages
- **MCP Compliance** - 100% protocol specification adherence

## Integration Dependencies

### TurboMCP Framework
- **Location**: `/Users/nickpaterno/work/turbomcp/`
- **Purpose**: Production-ready MCP client with enterprise features
- **Features**: Multi-transport, circuit breakers, connection pooling, health monitoring
- **🎯 Dogfooding Role**: Primary quality assurance through real-world usage

### Reference Materials
- **MCP Inspector**: `/Users/nickpaterno/work/reference/inspector/` - Official implementation
- **MCP Protocol**: `/Users/nickpaterno/work/reference/modelcontextprotocol/` - Specification
- **Tauri Docs**: `/Users/nickpaterno/work/reference/tauri-docs/` - Framework documentation

## Tauri-Specific Development

### Adding Rust Dependencies
Edit `src-tauri/Cargo.toml` to add crates, particularly:
```toml
# For MCP integration (planned)
turbomcp = { path = "../turbomcp" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
uuid = { version = "1", features = ["v4"] }
```

### Adding Tauri Commands
1. Define command in `src-tauri/src/lib.rs`:
```rust
#[tauri::command]
async fn spawn_mcp_server(config: ServerConfig) -> Result<String, String> {
    // Implementation
}
```

2. Register in builder:
```rust
.invoke_handler(tauri::generate_handler![greet, spawn_mcp_server])
```

3. Call from frontend:
```typescript
import { invoke } from "@tauri-apps/api/core";
const result = await invoke("spawn_mcp_server", { config });
```

### Process Management
Tauri enables spawning child processes for STDIO MCP servers:
```rust
use std::process::Command;
use tauri::async_runtime;

// Spawn MCP server as child process
// Capture stdout/stderr for protocol communication
// Monitor process health and resource usage
```

## Implementation Phases

Based on `MCP_STUDIO_ANALYSIS.md`, development follows 5 phases:

1. **Foundation** (✅ Complete) - Enterprise UI foundation with Tauri + SvelteKit integration
2. **MCP Operations** - All 5 MCP operation types with specialized UIs
3. **Collections & Testing** - Scenario management and test framework
4. **Advanced Features** - Protocol analysis and performance testing  
5. **Collaboration** - Team features and cloud sync

## MCP Protocol Understanding

Unlike REST APIs, MCP involves:
- **Bidirectional Communication** - Server can initiate requests (elicitation)
- **Capability Negotiation** - Dynamic feature discovery
- **Multiple Transports** - Not just HTTP
- **AI-Native Operations** - Prompts, sampling, tool calls

## Development Best Practices

### Tauri Security
- Use Tauri's security features (CSP, allowlists)
- Validate all inputs in Rust commands
- Handle process spawning securely

### SvelteKit Patterns
- Use SvelteKit's routing for navigation
- Leverage stores for state management
- Component composition for MCP operation UIs

### Rust Integration
- Keep heavy logic in Rust backend
- Use async/await for MCP operations
- Implement proper error handling across IPC boundary

## TurboMCP 1.0.9 Preparation

MCP Studio is fully prepared for TurboMCP 1.0.9 "big client updates":

### ✅ Completed Preparation
- **Local Path Dependencies**: Using local TurboMCP repo paths for automatic updates
- **Plugin System Support**: Added `#[cfg(feature = "plugins")]` imports for retry, caching, metrics plugins
- **Completion API**: Added `complete()` method to `McpTransportClient` for autocompletion
- **Feature Flags**: Added `plugins` and `full` features in Cargo.toml
- **Client Architecture**: Already includes enterprise features (elicitation, sampling, all transports)

### 🔄 When 1.0.9 Releases
1. Uncomment the actual `client.complete()` calls in `mcp_client.rs:561-606`
2. Enable plugin system imports to use actual TurboMCP plugin types
3. Test new client capabilities with existing MCP Studio UI

### 🎯 Expected 1.0.9 Features
Based on changelog analysis, the major client improvements likely include:
- **Enhanced Plugin System**: Production-ready retry, caching, metrics plugins
- **Improved Error Handling**: Better error conversion utilities and ergonomic methods
- **Completion Protocol**: Full support for autocompletion in tool parameters
- **OAuth 2.1 Client**: Enhanced authentication with security features
- **Performance Improvements**: SIMD acceleration and connection optimizations

## Next Implementation Steps (Phase 2)

1. **Complete MCP Protocol Integration**:
   - Finish TurboMCP client integration in Rust backend
   - Implement all 5 MCP operation types (tools, resources, prompts, sampling, elicitation)
   - Add real-time protocol message inspection

2. **Enhanced Server Management**:
   - Complete server configuration UI
   - Add process health monitoring and metrics
   - Implement connection pooling and retry logic

3. **Advanced Tool Operations**:
   - Auto-generate forms from JSON schemas
   - Add tool response visualization
   - Implement tool call history and replay

## Related Documentation

- `MCP_STUDIO_ANALYSIS.md` - Complete strategic vision and business plan
- `MCP_STUDIO_TECHNICAL_DESIGN.md` - **Comprehensive technical architecture based on deep analysis**
- `README.md` - Basic setup instructions
- Reference directories contain official MCP implementations and documentation

## Key Insights from Analysis

### MCP Inspector Limitations (What We Solve)
- **Web-based constraints** → Native desktop with direct process access
- **Proxy security issues** → Sandboxed native execution
- **Basic UI capabilities** → Rich, operation-specific interfaces
- **No enterprise features** → Collections, testing, collaboration
- **Single-user focused** → Team workspaces and sharing

### TurboMCP Advantages (What We Leverage)
- **SIMD-accelerated JSON** → 2-3x performance improvement
- **Enterprise security** → OAuth, CORS, TLS, rate limiting
- **Production resilience** → Circuit breakers, health monitoring, connection pooling
- **Multi-transport native** → All MCP transports without configuration
- **Full MCP compliance** → Complete 2025-06-18 specification support

## Important Development Tools

### AST-Grep Integration
**ast-grep is available and should be leveraged for code analysis and transformation tasks**. It's particularly powerful for:
- Finding patterns across TypeScript/Rust code
- Refactoring components to Svelte 5 runes mode
- Enforcing coding standards and conventions
- Mass code transformations during architecture changes

Refer to the comprehensive ast-grep documentation in `/Users/nickpaterno/work/CLAUDE.md` for detailed usage patterns.

## Current Implementation State

### ✅ Completed Features (Phase 1)
- **Enterprise Design System**: Complete CSS custom properties architecture with design tokens
- **Responsive Master Layout**: Adaptive sidebar with resizable panels, mobile-responsive
- **Theme Management**: Light/dark/system theme support with automatic switching
- **Svelte 5 Runes Mode**: Full compatibility with modern reactive patterns
- **Tauri Integration**: Type-safe IPC with MCP command handlers
- **State Management**: Comprehensive stores for servers, UI, and theme state
- **Component Library**: Production-ready Button component with enterprise patterns

### 🚧 In Progress (Phase 2)
- **MCP Client Integration**: TurboMCP dependency configured, basic commands implemented
- **Server Management**: Connection, disconnection, configuration UI partially complete
- **Tool Explorer**: Basic structure in place, needs MCP protocol integration

### 📋 Next Priorities
1. Complete MCP protocol integration with TurboMCP
2. Implement all 5 MCP operation types (tools, resources, prompts, sampling, elicitation)
3. Add comprehensive testing framework
4. Implement collections and scenario management

## Svelte 5 Development Patterns

### Runes Mode Requirements
**CRITICAL**: This project uses Svelte 5 in runes mode. Follow these patterns:

```typescript
// ✅ Correct - Use $props() instead of export let
const { theme, title, onClick } = $props();

// ✅ Correct - Use $derived for reactive computations
const isDarkMode = $derived(theme === 'dark');

// ✅ Correct - Use $state for local component state
let isOpen = $state(false);

// ✅ Correct - Use $effect for side effects
$effect(() => {
  document.title = title;
});

// ❌ Incorrect - Don't use export let (not supported in runes mode)
export let theme: string;

// ❌ Incorrect - Don't use $: reactive statements (use $derived instead)
$: isDarkMode = theme === 'dark';
```

### Component Patterns
- **Props**: Always use `$props()` destructuring
- **Events**: Use standard DOM event handlers like `onclick={(e) => ...}`
- **Stores**: Access with `$derived($storeName)` for reactive updates
- **Conditional Components**: Components are dynamic by default, no need for `<svelte:component>`

### Common Fixes for Runes Mode
- Replace `export let prop` → `const { prop } = $props()`
- Replace `$: computed = ...` → `const computed = $derived(...)`
- Replace `$: { ... }` → `$effect(() => { ... })`
- Replace `<svelte:component>` → Direct component usage
- Wrap `{@const}` declarations in `{#if}` blocks

## Development Best Practices

### Design System Usage
- **CSS Custom Properties**: Use design tokens from `src/lib/styles/design-tokens.css`
- **Component Styling**: Follow patterns in `src/lib/styles/components.css`
- **Theme-aware**: All components support automatic light/dark switching
- **Mobile-first**: Responsive design with desktop enhancements

### State Management Patterns
- **Server State**: Use `serverStore` for MCP server management
- **UI State**: Use `uiStore` for sidebar, panel, modal states
- **Theme State**: Use `themeStore` for light/dark/system themes
- **Reactive Access**: `const data = $derived($storeName)` in components

### IPC Communication
```typescript
// ✅ Correct - Type-safe Tauri commands
import { invoke } from '@tauri-apps/api/core';

const result = await invoke('connect_server', { config });
const tools = await invoke('list_tools', { serverId });
```

### Error Handling
- **Rust Backend**: Use `thiserror` and `anyhow` for comprehensive error types
- **Frontend**: Store errors in state, display with user-friendly messages
- **IPC Errors**: Handle command failures gracefully with loading states

### Tauri-Specific Patterns