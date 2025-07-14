<p align="center">
  <img src="assets/logo.png" alt="Mimir Logo" width="180"/>
</p>

# Mimir – Local-First, Zero-Knowledge AI Memory Vault

[![Rust](https://github.com/your-org/mimir/actions/workflows/rust.yml/badge.svg)](https://github.com/your-org/mimir/actions/workflows/rust.yml)
[![Security Audit](https://github.com/your-org/mimir/actions/workflows/security.yml/badge.svg)](https://github.com/your-org/mimir/actions/workflows/security.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

> **Privacy-first memory layer for AI applications** – Remember across sessions without compromising your data.

Mimir provides developers and end-users with a privacy-first memory layer that lets any LLM application remember across sessions without handing raw data to third parties.

---

## 📑 Table of Contents
- [Design Principles](#-design-principles)
- [Quick Start](#-quick-start)
- [Usage Examples](#usage-examples)
- [Architecture](#-architecture)
- [Security & Privacy](#-security--privacy)
- [Development](#-development)
- [Roadmap](#-roadmap)
- [License](#-license)
- [Community](#-community)
- [Acknowledgments](#-acknowledgments)
- [Screenshots / Demo](#-screenshots--demo)
- [Contact / Support](#-contact--support)

---

## 🎯 Design Principles

- **🏠 Local-first** – Runs entirely on user hardware; offline by default
- **🔐 Zero-knowledge cloud** – Optional encrypted sync; server cannot decrypt  
- **✋ Fine-grained consent** – Users classify memories and control app access
- **🔍 Open-source & audited** – Apache-2.0 core; public CI and security reviews

## 🚀 Quick Start

### Installation

```bash
# macOS
brew install mimir/tap/mimir

# Linux/Windows (cargo required)
cargo install mimir

# From source
git clone https://github.com/your-org/mimir
cd mimir
cargo build --release
```

### Initialize & Start

```bash
# Initialize vault
mimir init

# Start daemon
mimir start &

# Check status
mimir status
```

### Usage Examples


#### REST API (MCP)

```bash
curl -X POST http://localhost:8100/memories \
  -H "Content-Type: application/json" \
  -d '{"content": "Meeting with Alice at 3pm", "class": "work"}'
```

## 🏗️ Architecture

```
┌──────── Chat / Agent ──────┐    JSON-RPC/MCP     ┌──────────────┐
│      Any LLM client        ├─────────────────────►│      mimir   │
└────────────────────────────┘                     │   (Rust)     │
          ▲ retrieve context                        ├────┬─────────┤
          │                                        │    │         │
          ▼ write turns                          policy vault     │
               IPC / gRPC                              │          │
                                                       ▼          ▼
                                           SQLCipher + HNSW + KV
```

### Core Components

| Component | Description | Key Technologies |
|-----------|-------------|------------------|
| **mimir** | MCP server daemon | `axum`, `tokio`, JSON-RPC |
| **Vector Store** | HNSW similarity search | `hnswlib-rs`, 768-D embeddings |
| **Database** | Encrypted SQLite | `SQLCipher`, metadata + graph |
| **Guardrails** | PII detection & classification | `TinyBERT-ONNX`, `presidio-rs` |
| **Compression** | Memory summarization | `llama_cpp_rs`, token limits |
| **SDK** | Language bindings | `PyO3`, `napi-rs`, `wasm-bindgen` |
| **Tray UI** | Desktop management | `Tauri`, permission controls |

## 🔒 Security & Privacy

| Threat | Mitigation |
|--------|------------|
| Data breach | Client-side XChaCha20; server never sees keys |
| Rogue app access | ACL filters + per-class encryption |
| Model inversion | PII redaction before LLM processing |
| Data retention | TTL jobs delete/summarize aged memories |

### Memory Classification

Memories are automatically classified and encrypted per-class:

- **personal** – Personal conversations, preferences
- **work** – Professional context, meetings  
- **health** – Medical information, allergies
- **financial** – Payment info, financial context
- **custom** – User-defined categories

Access control is enforced at retrieval time based on app permissions.

## 🛠️ Development

### Prerequisites

- Rust 1.75+ with cargo
- SQLCipher development libraries
- ONNX Runtime (for guardrails)

### Building

```bash
# Full workspace build
cargo build --workspace

# Individual components
cargo build -p mimir
cargo build -p mimir-cli

# Release build
cargo build --release --workspace
```

### Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test integration

# Benchmarks
cargo bench
```

### Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Commit your changes: `git commit -m 'Add amazing feature'`
4. Push to the branch: `git push origin feature/amazing-feature`
5. Open a Pull Request

## �� Roadmap

- [x] **v0.1.0** - Core daemon with basic MCP server *(Current)*
- [ ] **v0.2.0** - Vector store and embedding pipeline
- [ ] **v0.3.0** - Guardrails and PII detection  
- [ ] **v0.4.0** - Tray UI and access control
- [ ] **v0.5.0** - Language bindings (Python, Node, WASM)
- [ ] **v1.0.0** - Production release with encryption

See [ROADMAP.md](docs/ROADMAP.md) for detailed timeline.

## 📄 License

- **Core components** (daemon, SDK, guardrails): [Apache License 2.0](LICENSE-APACHE)
- **UI components** (tray widget): [AGPL-3.0](LICENSE-AGPL) to keep derivative UIs open-source

graph TD
    A["User Conversations"] --> B["Ingest Text Tool"]
    B --> C["Text Preprocessing"]
    C --> D["Gemma3 1B<br/>Content Extractor"]
    D --> E["Raw Memory Candidates"]
    E --> F["Gemma3 1B<br/>Summarizer"]
    F --> G["Summarized Memories"]
    G --> H["Similarity Check<br/>(Vector Store)"]
    H --> I{"Duplicate<br/>Detection"}
    I -->|"New"| J["Direct Storage"]
    I -->|"Similar"| K["Gemma3 1B<br/>Conflict Resolver"]
    K --> L{"Resolution<br/>Decision"}
    L -->|"Merge"| M["Update Existing"]
    L -->|"Replace"| N["Replace Existing"]
    L -->|"Keep Both"| O["Store as New"]
    
    P["Add Memories Tool<br/>(Deliberate)"] --> Q["Manual Memory Input"]
    Q --> F
    
    J --> R["Final Storage<br/>(Database + Vector)"]
    M --> R
    N --> R
    O --> R
    
    R --> S["Memory Vault"]
    
    style D fill:#e1f5fe
    style F fill:#e1f5fe
    style K fill:#e1f5fe
    style S fill:#c8e6c9W

## 🤝 Community

- **Issues**: [GitHub Issues](https://github.com/ryanwillie/mimir/issues)
- **Discussions**: [GitHub Discussions](https://github.com/ryanwillie/mimir/discussions)
- **Security**: See [SECURITY.md](SECURITY.md) for reporting vulnerabilities

## 🙏 Acknowledgments

- Vector search powered by [HNSW](https://github.com/nmslib/hnswlib)
- Built with love using [Rust](https://rust-lang.org/) 🦀

---

*"The best way to predict the future is to invent it, but the best way to remember it is to own it."* 

## 🖼️ Screenshots / Demo

<!-- Add screenshots, GIFs, or demo links here -->

---

## 📬 Contact / Support

- For help, questions, or feedback, please open an [issue](https://github.com/ryanwillie/mimir/issues) or join the [discussions](https://github.com/ryanwillie/mimir/discussions).
- For security concerns, see [SECURITY.md](SECURITY.md). 