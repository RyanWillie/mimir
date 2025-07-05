# Contributing to Mimir

Thank you for your interest in contributing to Mimir! This document provides guidelines and information for contributors.

## 🤝 Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## 🚀 Getting Started

### Prerequisites

- **Rust 1.75+** with cargo
- **Git** for version control
- **SQLCipher** development libraries
- **ONNX Runtime** (for guardrails features)

### Setting Up Development Environment

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/your-username/mimir.git
   cd mimir
   ```

2. **Install system dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install libsqlcipher-dev
   
   # macOS
   brew install sqlcipher
   
   # Windows (using vcpkg)
   vcpkg install sqlcipher
   ```

3. **Install Rust toolchain components**
   ```bash
   rustup component add rustfmt clippy
   cargo install cargo-audit cargo-llvm-cov
   ```

4. **Build and test**
   ```bash
   cargo build --workspace
   cargo test --workspace
   ```

## 🛠️ Development Workflow

### Branch Naming

Use descriptive branch names with prefixes:
- `feature/` - New features
- `fix/` - Bug fixes  
- `docs/` - Documentation changes
- `refactor/` - Code refactoring
- `test/` - Test improvements

Example: `feature/vector-store-optimization`

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

Examples:
- `feat(vector): add HNSW index optimization`
- `fix(db): resolve SQLCipher connection leak`
- `docs: update installation instructions`

### Code Quality

Before submitting a PR, ensure:

1. **Formatting**: `cargo fmt --all`
2. **Linting**: `cargo clippy --workspace -- -D warnings`
3. **Tests**: `cargo test --workspace`
4. **Security**: `cargo audit`

### Testing

- **Unit tests**: Test individual functions and modules
- **Integration tests**: Test component interactions
- **Documentation tests**: Ensure examples in docs work
- **Benchmarks**: Performance-critical code should include benchmarks

```bash
# Run all tests
cargo test --workspace

# Run specific test
cargo test -p mimir-core test_memory_classification

# Run benchmarks
cargo bench

# Generate coverage report
cargo llvm-cov --workspace --html
```

## 📁 Project Structure

```
mimir/
├── crates/              # Core Rust crates
│   ├── mimir-core/      # Shared types and utilities
│   ├── mimir/           # Main daemon
│   ├── mimir-vector/    # Vector store (HNSW)
│   ├── mimir-db/        # Encrypted database
│   ├── mimir-guardrails/# PII detection & classification
│   ├── mimir-compression/# Memory summarization  
│   ├── mimir-sdk/       # Client library
│   ├── mimir-cli/       # Command-line interface
│   └── mimir-tray/      # Desktop UI (AGPL-3.0)
├── bindings/            # Language bindings
│   ├── python/          # Python bindings (PyO3)
│   ├── nodejs/          # Node.js bindings (napi-rs)
│   └── wasm/           # WebAssembly bindings
├── docs/               # Documentation
├── examples/           # Usage examples
└── tests/             # Integration tests
```

## 🎯 Areas for Contribution

### 🔥 High Priority
- **Vector Store**: HNSW index implementation and optimization
- **Guardrails**: PII detection using TinyBERT-ONNX
- **Compression**: LLM-based memory summarization
- **Security**: Audit cryptographic implementations

### 📚 Documentation
- API documentation improvements
- Usage examples and tutorials
- Architecture documentation
- Performance benchmarking

### 🧪 Testing
- Increase test coverage
- Add integration tests
- Performance benchmarks
- Security testing

### 🌐 Language Bindings
- Python SDK improvements
- Node.js SDK enhancements
- WebAssembly optimizations
- Additional language support

## 📋 Pull Request Process

1. **Create an issue** first to discuss significant changes
2. **Fork** the repository and create your feature branch
3. **Implement** your changes with tests
4. **Ensure** all CI checks pass
5. **Update** documentation if needed
6. **Submit** a pull request with a clear description

### PR Checklist

- [ ] Code follows project style guidelines
- [ ] Tests added/updated for new functionality
- [ ] Documentation updated (if applicable)
- [ ] No breaking changes without discussion
- [ ] Commit messages follow convention
- [ ] CI checks pass
- [ ] Security considerations addressed

## 🔒 Security Considerations

This project handles sensitive user data. Please consider:

- **Cryptographic best practices** for encryption/decryption
- **Memory safety** for sensitive data handling
- **Access control** implementation correctness
- **Dependencies** security via `cargo audit`

Report security vulnerabilities privately via [SECURITY.md](SECURITY.md).

## 📖 Documentation

### Code Documentation

- Use `///` for public API documentation
- Include examples in documentation
- Document panics, errors, and safety considerations
- Keep docs up-to-date with code changes

### Architecture Documentation

Located in `docs/architecture/`:
- System design decisions
- Component interactions
- Security model
- Performance characteristics

## 🎨 UI/UX Guidelines (Tray App)

The tray application uses **AGPL-3.0** license to keep derivative UIs open-source:

- Follow platform-specific design guidelines
- Prioritize privacy and user control
- Clear visual indicators for security states
- Accessible design practices

## 🚢 Release Process

Releases follow semantic versioning (SemVer):

- **Patch** (0.1.1): Bug fixes, security patches
- **Minor** (0.2.0): New features, non-breaking changes
- **Major** (1.0.0): Breaking changes

## 💬 Getting Help

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: Questions and community discussion
- **Discord**: Real-time community chat
- **Documentation**: Comprehensive guides and API docs

## 🙏 Recognition

Contributors are recognized in:
- `CONTRIBUTORS.md` file
- Release notes
- Annual contributor highlights

Thank you for contributing to Mimir! Your efforts help build a more private and secure AI ecosystem.