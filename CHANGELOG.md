# Changelog

All notable changes to TurboMCP Studio will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Upcoming features and improvements

### Changed
- Upcoming changes

## [0.1.0] - 2025-10-16

### Added

Initial release of TurboMCP Studio - Enterprise-grade desktop application for Model Context Protocol development and testing.

#### Core Features

- **Native Desktop Application**: Built with Tauri 2.0 for macOS, Windows, and Linux
- **Enterprise UI**: Complete design system with light/dark themes and responsive layouts
- **MCP Server Management**: Connect, configure, and manage Model Context Protocol servers
- **Tool Explorer**: Interface for discovering and testing MCP tools
- **Resource Browser**: Navigate and inspect MCP resources
- **Prompt Designer**: Create and test MCP prompts
- **Protocol Inspector**: Real-time protocol message viewing
- **Settings Management**: Configure application preferences and themes
- **Sampling & Elicitation**: Handle MCP sampling requests and user input elicitation
- **System Monitoring**: Real-time health dashboard with connection status tracking

#### Platform Support

- **macOS**: Universal binary for Intel and Apple Silicon with full Unix socket support
- **Windows**: Native x64 build with TCP, HTTP/SSE, and WebSocket transports
- **Linux**: x64 build with full Unix socket and network transport support

#### Technology Stack

- **Frontend**: SvelteKit 5 with Svelte 5 runes mode, TypeScript, and Tailwind CSS
- **Backend**: Rust with Tauri 2.0 for native performance
- **MCP Client**: Powered by [TurboMCP](https://github.com/Epistates/turbomcp) for enterprise-grade MCP operations
- **Database**: SQLite for local-first data storage
- **Build System**: Vite with HMR for fast development

#### Architecture

Three-layer architecture for optimal performance:
- **Frontend Layer**: SvelteKit + TypeScript + Tailwind CSS
- **Bridge Layer**: Tauri IPC with type-safe serialization
- **Backend Layer**: Rust + TurboMCP for native MCP operations

#### Key Capabilities

- **Multi-Transport Support**: STDIO, HTTP, WebSocket, TCP, and Unix socket connections
- **Type Safety**: Full TypeScript integration with comprehensive type definitions
- **Production Ready**: Enterprise-grade error handling and state management
- **Developer Experience**: Fast development with hot module replacement
- **Cross-Platform**: Single codebase for macOS, Windows, and Linux

### Fixed

- Cross-platform Unix socket handling with OS-specific compilation guards
- Platform-aware transport filtering (Unix sockets hidden on Windows)
- TypeScript type safety and null checking
- Release workflow permissions and Tauri 2.0 compatibility

### Known Limitations

- **Testing Framework**: Automated tests not yet implemented
- **Collections**: Advanced workflow features coming in future releases
- **UI Warnings**: 161 non-critical Svelte checker warnings (CSS and accessibility false positives)

### Requirements

- **Node.js**: 20.x or later
- **Rust**: 1.70 or later
- **pnpm**: 9.x or later

**Platform-specific**:
- **macOS**: 11.0 (Big Sur) or later
- **Windows**: Windows 10 (1809+) or Windows 11
- **Linux**: Modern distribution with GTK 3.24+

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

[Unreleased]: https://github.com/Epistates/turbomcpstudio/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Epistates/turbomcpstudio/releases/tag/v0.1.0
