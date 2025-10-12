# Contributing to TurboMCP Studio

Thank you for your interest in contributing to TurboMCP Studio! This document provides guidelines and instructions for contributing to the project.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors. We expect:

- Respectful and constructive communication
- Welcoming attitude towards newcomers
- Focus on what's best for the community
- Empathy towards other community members

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue on GitHub with:

1. **Clear title** describing the issue
2. **Steps to reproduce** the problem
3. **Expected behavior** vs actual behavior
4. **Environment details**:
   - OS and version
   - TurboMCP Studio version
   - Rust version (`rustc --version`)
   - Node.js version (`node --version`)
5. **Screenshots or logs** if applicable

### Suggesting Features

We welcome feature suggestions! Please create an issue with:

1. **Clear description** of the feature
2. **Use case** - why is this needed?
3. **Proposed solution** (if you have one)
4. **Alternatives considered** (optional)

### Pull Requests

We actively welcome pull requests!

#### Before You Start

1. **Check existing issues** to avoid duplicate work
2. **Create an issue first** for major changes to discuss the approach
3. **Fork the repository** and create a branch from `main`

#### Development Setup

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/turbomcpstudio.git
cd turbomcpstudio

# Clone TurboMCP dependency
git clone https://github.com/Epistates/turbomcp.git ../turbomcp

# Install dependencies
pnpm install

# Start development server
pnpm run tauri dev
```

#### Making Changes

1. **Create a branch** with a descriptive name:
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

2. **Follow the code style**:
   - **Rust**: Run `cargo fmt` and `cargo clippy`
   - **TypeScript**: Run `pnpm run check`
   - **Svelte 5**: Use runes mode (`$state`, `$derived`, `$props()`)

3. **Write clear commit messages**:
   ```
   feat: Add tool execution history
   fix: Resolve server connection timeout
   docs: Update installation instructions
   refactor: Simplify server store logic
   ```

   Follow [Conventional Commits](https://www.conventionalcommits.org/):
   - `feat:` New features
   - `fix:` Bug fixes
   - `docs:` Documentation changes
   - `style:` Code style changes (formatting)
   - `refactor:` Code refactoring
   - `test:` Test additions or changes
   - `chore:` Maintenance tasks

4. **Test your changes**:
   ```bash
   # Run Rust tests
   cd src-tauri && cargo test

   # Run type checking
   pnpm run check

   # Test the app manually
   pnpm run tauri dev
   ```

5. **Update documentation** if needed:
   - Update README.md for user-facing changes
   - Add changelog entry for notable changes

#### Submitting a Pull Request

1. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create a Pull Request** on GitHub with:
   - Clear title and description
   - Reference to related issue (e.g., "Fixes #123")
   - Screenshots/GIFs for UI changes
   - Test results if applicable

3. **Respond to feedback**:
   - Address review comments promptly
   - Push additional commits to the same branch
   - Request re-review when ready

#### Pull Request Checklist

Before submitting, ensure:

- [ ] Code follows the project's style guidelines
- [ ] `cargo fmt` and `cargo clippy` pass (no warnings)
- [ ] `pnpm run check` passes (no TypeScript errors)
- [ ] All tests pass (`cargo test`)
- [ ] Documentation is updated if needed
- [ ] Commit messages follow Conventional Commits
- [ ] Branch is up to date with `main`

## Development Guidelines

### Project Structure

```
turbomcpstudio/
├── src/                    # SvelteKit frontend
│   ├── lib/
│   │   ├── components/    # Svelte components (use runes mode)
│   │   ├── stores/        # State management
│   │   └── utils/         # Utility functions
│   └── routes/            # SvelteKit routing
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── commands/      # Tauri command handlers
│   │   ├── mcp_client/    # MCP client integration
│   │   └── lib.rs         # Main entry point
│   └── Cargo.toml
└── static/                # Static assets
```

### Code Style

#### Svelte 5 (Runes Mode)

```typescript
// ✅ Correct - Use runes
const { prop } = $props();
let count = $state(0);
const doubled = $derived(count * 2);

// ❌ Incorrect - Legacy syntax
export let prop;
let count = 0;
$: doubled = count * 2;
```

#### TypeScript

- Use strict type checking
- Prefer `interface` over `type` for objects
- Use meaningful variable names
- Document complex functions with JSDoc

#### Rust

- Follow Rust idioms and best practices
- Use `Result` and `Option` for error handling
- Document public APIs with doc comments
- Keep functions focused and testable

### Testing

- Write tests for new features
- Ensure existing tests pass
- Aim for good test coverage
- Test edge cases and error conditions

### Performance

- Profile before optimizing
- Use Svelte's reactivity efficiently
- Minimize expensive operations in hot paths
- Keep bundle sizes reasonable

## TurboMCP Dogfooding

TurboMCP Studio is a primary dogfooding application for TurboMCP. If you encounter issues with TurboMCP:

1. **Don't implement workarounds** - report upstream
2. **Create detailed bug reports** for TurboMCP maintainers
3. **Focus on developer experience** - make it obvious when TurboMCP has issues

## Release Process

Releases are managed by maintainers. See [RELEASE.md](RELEASE.md) for details.

## Questions?

- **General questions**: [GitHub Discussions](https://github.com/Epistates/turbomcpstudio/discussions)
- **Bug reports**: [GitHub Issues](https://github.com/Epistates/turbomcpstudio/issues)
- **Security issues**: Email maintainers privately (see SECURITY.md)

## License

By contributing to TurboMCP Studio, you agree that your contributions will be licensed under the [MIT License](LICENSE).

---

Thank you for contributing to TurboMCP Studio! 🚀
