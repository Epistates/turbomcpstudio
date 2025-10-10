# MCP Studio

**The Postman for Model Context Protocol (MCP)**

A native desktop application for developing, testing, and debugging Model Context Protocol servers. Built with Tauri, SvelteKit, and powered by TurboMCP for enterprise-grade MCP operations.

[![Release](https://img.shields.io/github/v/release/YOUR_ORG/turbomcpstudio)](https://github.com/YOUR_ORG/turbomcpstudio/releases)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Features

### Phase 1 (v0.1.0) - Foundation ✅
- **Enterprise UI Foundation**: Complete design system with light/dark themes
- **Native Desktop App**: Built with Tauri 2.0 for macOS, Windows, and Linux
- **Modern Stack**: SvelteKit 5 with Svelte 5 runes mode and TypeScript
- **TurboMCP Integration**: Foundation for production-grade MCP operations
- **Server Management**: Connect, configure, and manage MCP servers
- **Responsive Design**: Adaptive layouts with mobile support

### Coming Soon
- **Tool Explorer**: Auto-generated forms from JSON schemas, response visualization
- **Resource Browser**: Tree navigation, URI templates, subscription management
- **Prompt Designer**: Visual composition, variable management, A/B testing
- **Sampling Debugger**: Step-through debugging, token tracking, conversation history
- **Elicitation Flow**: Visual workflow designer, conditional logic, form validation
- **Collections & Testing**: Multi-server operation chains, contract testing, load testing
- **Protocol Analysis**: Real-time message inspection, flow visualization, replay
- **Team Collaboration**: Shared workspaces, version control integration

## Installation

### Download Pre-built Binaries

Download the latest release for your platform:

**[→ Download Latest Release](https://github.com/YOUR_ORG/turbomcpstudio/releases/latest)**

#### macOS
- **Universal DMG** (Intel + Apple Silicon): `MCP-Studio_x.x.x_universal.dmg`
- **Intel DMG**: `MCP-Studio_x.x.x_x64.dmg`
- **Apple Silicon DMG**: `MCP-Studio_x.x.x_aarch64.dmg`

Requirements: macOS 11.0 (Big Sur) or later

#### Windows
- **MSI Installer**: `MCP-Studio_x.x.x_x64_en-US.msi` (recommended)
- **EXE Installer**: `MCP-Studio_x.x.x_x64-setup.exe`

Requirements: Windows 10 (1809+) or Windows 11

#### Linux
- **AppImage** (universal): `mcp-studio_x.x.x_amd64.AppImage`
  ```bash
  chmod +x mcp-studio_x.x.x_amd64.AppImage
  ./mcp-studio_x.x.x_amd64.AppImage
  ```
- **DEB Package** (Debian/Ubuntu):
  ```bash
  sudo dpkg -i mcp-studio_x.x.x_amd64.deb
  ```
- **RPM Package** (Fedora/RHEL):
  ```bash
  sudo rpm -i mcp-studio-x.x.x-1.x86_64.rpm
  ```

Requirements: Modern Linux distribution with GTK 3.24+

## Building from Source

### Prerequisites

#### All Platforms
- **Node.js**: 20.x or later ([Download](https://nodejs.org/))
- **pnpm**: 9.x or later
  ```bash
  npm install -g pnpm
  ```
- **Rust**: 1.70 or later ([rustup.rs](https://rustup.rs/))
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

#### macOS
- **Xcode Command Line Tools**:
  ```bash
  xcode-select --install
  ```

#### Windows
- **Visual Studio Build Tools**: [Download](https://visualstudio.microsoft.com/downloads/)
  - Install "Desktop development with C++" workload
- **WebView2**: Usually pre-installed on Windows 10/11
  - If missing: [Download WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)

#### Linux (Debian/Ubuntu)
```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

#### Linux (Fedora/RHEL)
```bash
sudo dnf install -y \
  webkit2gtk4.1-devel \
  openssl-devel \
  curl \
  wget \
  file \
  libappindicator-gtk3-devel \
  librsvg2-devel
```

### Clone the Repository

```bash
git clone https://github.com/YOUR_ORG/turbomcpstudio.git
cd turbomcpstudio
```

### Install Dependencies

```bash
# Install frontend dependencies
pnpm install

# TurboMCP dependencies are handled automatically
# The project uses local path dependencies that point to:
# /Users/nickpaterno/work/turbomcp/
#
# You have two options:
#
# Option 1: Clone TurboMCP to the same parent directory
git clone https://github.com/Epistates/turbomcp.git ../turbomcp

# Option 2: Update Cargo.toml to use git dependencies (see RELEASE.md)
```

### Development Build

```bash
# Start development server with hot-reload
pnpm run tauri dev

# This will:
# 1. Start Vite dev server (frontend) on http://localhost:1420
# 2. Compile Rust backend
# 3. Launch desktop app with hot-reload enabled
```

### Production Build

```bash
# Build optimized production binary (no installer)
pnpm run tauri build -- --no-bundle

# Build with installers for your platform
pnpm run tauri build

# Output locations:
# macOS:   src-tauri/target/release/bundle/dmg/
# Windows: src-tauri/target/release/bundle/msi/ and .../nsis/
# Linux:   src-tauri/target/release/bundle/appimage/, deb/, rpm/
```

### Type Checking

```bash
# Run type checker once
pnpm run check

# Run in watch mode (during development)
pnpm run check:watch
```

### Testing

```bash
# Run Rust tests
cd src-tauri
cargo test

# Run Rust tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Lint Rust code
cargo clippy --all-targets --all-features

# Format Rust code
cargo fmt
```

## Development

### Project Structure

```
turbomcpstudio/
├── src/                          # SvelteKit frontend
│   ├── routes/                  # File-based routing
│   ├── lib/
│   │   ├── components/         # Svelte components
│   │   ├── stores/             # State management
│   │   ├── styles/             # CSS architecture
│   │   └── types/              # TypeScript types
│   └── app.html
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── commands/           # Tauri command handlers
│   │   ├── mcp_client/         # MCP client manager
│   │   ├── database/           # SQLite layer
│   │   └── lib.rs              # Entry point
│   ├── Cargo.toml
│   └── tauri.conf.json         # Tauri configuration
├── static/                      # Static assets
└── package.json
```

### Key Technologies

- **Frontend**: SvelteKit 5 + TypeScript + Tailwind CSS
- **Backend**: Rust + Tauri 2.0
- **MCP Client**: TurboMCP (enterprise-grade)
- **Database**: SQLite (local-first)
- **Build Tool**: Vite
- **Package Manager**: pnpm

### Architecture

MCP Studio uses a three-layer architecture:

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

### Development Workflow

1. **Start dev environment**:
   ```bash
   pnpm run tauri dev
   ```

2. **Make changes**:
   - Frontend: Edit files in `src/` (hot-reload automatic)
   - Backend: Edit files in `src-tauri/src/` (auto-recompile)

3. **Type check**:
   ```bash
   pnpm run check
   ```

4. **Test**:
   ```bash
   cd src-tauri && cargo test
   ```

5. **Build for production**:
   ```bash
   pnpm run tauri build
   ```

### IDE Setup (Recommended)

**VS Code** with extensions:
- [Svelte for VS Code](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode)
- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [Even Better TOML](https://marketplace.visualstudio.com/items?itemName=tamasfe.even-better-toml)

**Settings** (`.vscode/settings.json`):
```json
{
  "editor.formatOnSave": true,
  "rust-analyzer.cargo.features": "all",
  "svelte.enable-ts-plugin": true
}
```

## Documentation

- **[CHANGELOG.md](CHANGELOG.md)**: Release history and version notes
- **[RELEASE.md](RELEASE.md)**: Release process and versioning guide
- **[CLAUDE.md](CLAUDE.md)**: Development guidelines and project overview
- **[MCP_STUDIO_ANALYSIS.md](MCP_STUDIO_ANALYSIS.md)**: Strategic vision and business plan
- **[MCP_STUDIO_TECHNICAL_DESIGN.md](MCP_STUDIO_TECHNICAL_DESIGN.md)**: Technical architecture

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'feat: Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes (formatting)
- `refactor:` Code refactoring
- `test:` Test additions or changes
- `chore:` Maintenance tasks

### Code Quality

Before submitting:
- Run `pnpm run check` (TypeScript type checking)
- Run `cargo test` (Rust tests)
- Run `cargo clippy` (Rust linting)
- Run `cargo fmt` (Rust formatting)

## Troubleshooting

### Build Errors

**Error**: "could not find `turbomcp` in the crate root"
- **Solution**: Clone TurboMCP to `../turbomcp` or update Cargo.toml dependencies

**Error**: "webkit2gtk not found" (Linux)
- **Solution**: Install webkit2gtk-4.1-dev package (see Prerequisites)

**Error**: "VCRUNTIME140.dll was not found" (Windows)
- **Solution**: Install Visual Studio C++ Redistributable

### Runtime Issues

**Issue**: App won't start on macOS
- **Solution**: Right-click app → "Open" → "Open" (bypass Gatekeeper on first run)

**Issue**: "App is damaged and can't be opened" (macOS)
- **Solution**: Run `xattr -cr /Applications/MCP\ Studio.app`

**Issue**: Database errors
- **Solution**: Delete `~/.config/turbomcpstudio/` directory and restart

### Getting Help

- **Issues**: [GitHub Issues](https://github.com/YOUR_ORG/turbomcpstudio/issues)
- **Discussions**: [GitHub Discussions](https://github.com/YOUR_ORG/turbomcpstudio/discussions)
- **Documentation**: Check the docs listed above

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tauri](https://tauri.app/)
- Powered by [TurboMCP](https://github.com/Epistates/turbomcp)
- UI framework: [SvelteKit](https://kit.svelte.dev/)
- MCP specification: [Model Context Protocol](https://modelcontextprotocol.io/)

---

**Status**: Phase 1 Complete (v0.1.0) - Foundation established
**Next**: Phase 2 - Full MCP protocol integration
