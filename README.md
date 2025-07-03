# Mimir – Local-First, Zero-Knowledge AI Memory Vault

[![Rust](https://github.com/your-org/mimir/actions/workflows/rust.yml/badge.svg)](https://github.com/your-org/mimir/actions/workflows/rust.yml)
[![Security Audit](https://github.com/your-org/mimir/actions/workflows/security.yml/badge.svg)](https://github.com/your-org/mimir/actions/workflows/security.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

> **Privacy-first memory layer for AI applications** – Remember across sessions without compromising your data.

Mimir provides developers and end-users with a privacy-first memory layer that lets any LLM application remember across sessions without handing raw data to third parties.

## 🎯 Design Principles

- **🏠 Local-first** – Runs entirely on user hardware; offline by default
- **🔐 Zero-knowledge cloud** – Optional encrypted sync; server cannot decrypt  
- **✋ Fine-grained consent** – Users classify memories and control app access
- **🔍 Open-source & audited** – Apache-2.0 core; public CI and security reviews

## 🚀 Quick Start

### Installation

```bash
# macOS
brew install safememory/tap/mimir

# Linux/Windows (cargo required)
cargo install mimir safe-memory

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
safe-memory status
```

### Usage Examples

#### Python

```python
from safe_memory import Memory

# Connect to local daemon
mem = Memory(app_id="my-chatbot", allow=["work", "personal"])

# Store a memory
mem.ingest("I'm allergic to penicillin")

# Retrieve relevant context
context = mem.retrieve("medical allergies", top_k=3)
reply = llm.chat(context + user_input)
```

#### Node.js

```javascript
import { Memory } from "safe-memory";

const mem = new Memory({ 
  appId: "my-assistant",
  allow: ["personal"] 
});

await mem.ingest("My favorite coffee is Ethiopian single-origin");
const context = await mem.retrieve("coffee preferences");
```

#### REST API (MCP)

```bash
curl -X POST http://localhost:8100/memories \
  -H "Content-Type: application/json" \
  -d '{"content": "Meeting with Alice at 3pm", "class": "work"}'
```

## 🏗️ Architecture

```
┌──────── Chat / Agent ──────┐    JSON-RPC/MCP     ┌──────────────┐
│      Any LLM client        ├─────────────────────►│  safe_memoryd│
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
| Cloud breach | Client-side XChaCha20; server never sees keys |
| Rogue app access | ACL filters + per-class encryption |
| Model inversion | PII redaction before LLM processing |
| Data retention | TTL jobs delete/summarize aged memories |
| Supply chain | Reproducible builds, signed releases |

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

## 📋 Roadmap

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

## 🤝 Community

- **Discord**: [Join our community](https://discord.gg/mimir)
- **Issues**: [GitHub Issues](https://github.com/your-org/mimir/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/mimir/discussions)
- **Security**: See [SECURITY.md](SECURITY.md) for reporting vulnerabilities

## 🙏 Acknowledgments

- Inspired by privacy-first principles from [Solid](https://solidproject.org/)
- Vector search powered by [HNSW](https://github.com/nmslib/hnswlib)
- Built with love using [Rust](https://rust-lang.org/) 🦀

---

*"The best way to predict the future is to invent it, but the best way to remember it is to own it."* 