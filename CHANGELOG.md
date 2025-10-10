# Changelog

All notable changes to MCP Studio will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [0.1.0] - 2025-10-10

### Added - Phase 1: Foundation

#### Enterprise UI Foundation
- **Design System**: Complete CSS custom properties architecture with design tokens
- **Responsive Master Layout**: Adaptive sidebar with resizable panels and mobile support
- **Theme Management**: Light/dark/system theme support with automatic switching
- **Component Library**: Production-ready Button component with enterprise patterns

#### Svelte 5 Integration
- **Runes Mode**: Full compatibility with modern Svelte 5 reactive patterns
- **State Management**: Comprehensive stores for servers, UI, and theme state
- **Type Safety**: Complete TypeScript integration throughout frontend

#### Tauri Desktop Integration
- **Native Desktop App**: Built with Tauri 2.0 for native performance
- **IPC Layer**: Type-safe command handlers for MCP operations
- **Process Management**: Foundation for spawning and managing MCP server processes
- **Security**: Sandboxed execution with proper capability configuration

#### MCP Foundation
- **TurboMCP Integration**: Local development dependencies configured
- **Basic Commands**: Connection and configuration command structure
- **Server Management**: Initial server store and connection handling
- **Database Layer**: SQLite integration for local storage

#### Developer Experience
- **Development Environment**: Fast Vite-based development with HMR
- **Type Checking**: Strict TypeScript with svelte-check integration
- **Documentation**: Comprehensive CLAUDE.md for development guidance
- **Build System**: Complete build pipeline for all platforms

#### Components
- **Dashboard**: Main application dashboard with server overview
- **Server Overview**: MCP server status and management
- **Tool Explorer**: Basic structure for tool interaction (UI only)
- **Protocol Inspector**: Foundation for protocol message viewing
- **Settings**: Theme and configuration management
- **Add Server Modal**: Server configuration interface

#### Technical Foundation
- **Tauri 2.0**: Native desktop framework with Rust backend
- **SvelteKit 5**: Modern frontend framework with runes mode
- **TypeScript**: Full type safety across the application
- **Tailwind CSS**: Utility-first styling with custom design system
- **pnpm**: Fast, efficient package management
- **SQLite**: Local-first database with async operations

### Architecture
- **Three-Layer Architecture**:
  - Frontend: SvelteKit + TypeScript + Tailwind
  - Bridge: Tauri IPC with type-safe serialization
  - Backend: Rust + TurboMCP for enterprise MCP operations

### Documentation
- **Strategic Vision**: Complete analysis in MCP_STUDIO_ANALYSIS.md
- **Technical Design**: Comprehensive architecture in MCP_STUDIO_TECHNICAL_DESIGN.md
- **Development Guide**: CLAUDE.md with development patterns and best practices
- **Release Process**: RELEASE.md with detailed release instructions

### Known Limitations
- **MCP Protocol**: Core protocol operations are scaffolded but not fully functional
- **Tool Execution**: UI only, no actual tool invocation yet
- **Resource Browser**: Not yet implemented
- **Prompt Designer**: Not yet implemented
- **Sampling Debugger**: Not yet implemented
- **Collections**: Not yet implemented
- **Testing Framework**: No automated tests yet

### Dependencies
- **TurboMCP**: Uses local path dependencies (requires git clone for CI)
  - See RELEASE.md for details on dependency management
- **Node.js**: Requires Node.js 20+
- **Rust**: Requires Rust 1.70+
- **Platform Requirements**:
  - macOS: 11.0 (Big Sur) or later
  - Windows: Windows 10 (1809+) or Windows 11
  - Linux: Modern distribution with GTK 3.24+

### Notes
This is the initial release of MCP Studio - "Postman for MCP". It establishes the foundational architecture and UI framework. The application is functional for basic server management but does not yet implement complete MCP protocol operations.

**Dogfooding Status**: This release serves as the foundation for dogfooding TurboMCP. Issues discovered should be reported to TurboMCP developers for upstream fixes rather than implementing workarounds.

**Next Phase**: Phase 2 will focus on completing MCP protocol integration and implementing all five MCP operation types (tools, resources, prompts, sampling, elicitation).

---

## Release Notes Format

Each release follows this structure:

### Added
New features and capabilities

### Changed
Changes to existing functionality

### Deprecated
Features that will be removed in future versions

### Removed
Features that have been removed

### Fixed
Bug fixes

### Security
Security-related changes

---

[Unreleased]: https://github.com/YOUR_ORG/turbomcpstudio/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/YOUR_ORG/turbomcpstudio/releases/tag/v0.1.0
