# AGENTS.md - AI Agent Guide for Mimir Repository

> **Purpose**: This document provides comprehensive guidance for AI agents (LLMs) working on the Mimir codebase to ensure consistent, safe, and effective contributions.

## 🏗️ Project Architecture Overview

### **Core Concept**
Mimir is a **local-first, zero-knowledge AI memory vault** that provides privacy-preserving memory management for AI applications. The architecture follows a modular design with clear separation of concerns.

### **Workspace Structure**
```
mimir/
├── crates/                    # Core Rust components (Apache-2.0)
│   ├── mimir-core/           # Shared types, errors, config
│   ├── mimir/                # Main daemon + MCP server  
│   ├── mimir-vector/         # Vector store (HNSW)
│   ├── mimir-db/             # Encrypted SQLite database
│   ├── mimir-guardrails/     # PII detection & classification
│   ├── mimir-compression/    # Memory summarization
│   ├── mimir-sdk/            # Client library
│   ├── mimir-cli/            # Command-line interface
│   └── mimir-tray/           # Desktop UI (AGPL-3.0)
├── bindings/                 # Language bindings
│   ├── python/               # PyO3 Python bindings
│   ├── nodejs/               # napi-rs Node.js bindings
│   └── wasm/                 # WebAssembly bindings
├── docs/                     # Documentation
├── examples/                 # Usage examples
└── tests/                    # Integration tests
```

## 🔧 Development Guidelines for AI Agents

### **1. Before Making Changes**

**ALWAYS perform these checks:**
```bash
# Check current compilation status
cargo check --workspace

# Run tests to understand current state
cargo test --workspace

# Check for linting issues
cargo clippy --workspace

# Verify formatting
cargo fmt --all --check
```

### **2. Understanding Dependencies**

**Workspace Dependencies**: All crates share dependencies defined in root `Cargo.toml`
```toml
[workspace.dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
# ... etc
```

**Crate Dependencies**: Reference workspace deps like:
```toml
[dependencies]
tokio.workspace = true
serde.workspace = true
mimir-core = { path = "../mimir-core" }
```

### **3. Core Types and Patterns**

**Primary Types** (in `mimir-core/src/types.rs`):
- `Memory` - Core memory object with content, class, metadata
- `MemoryClass` - Classification (Personal, Work, Health, Financial, Other)
- `MemoryIngestion` - Input structure for storing memories
- `MemoryQuery` - Query structure for retrieving memories
- `MemoryResult` - Search result with score

**Error Handling** (in `mimir-core/src/error.rs`):
```rust
use mimir_core::{Result, MimirError};

fn example_function() -> Result<String> {
    // Use Result<T> = std::result::Result<T, MimirError>
    Ok("success".to_string())
}
```

**Configuration** (in `mimir-core/src/config.rs`):
```rust
let config = MimirConfig::default(); // Provides sensible defaults
```

## 🔐 Security Considerations for AI Agents

### **Critical Security Principles**

1. **Never log sensitive data** - Memory content, keys, personal information
2. **Use proper error handling** - Don't leak sensitive info in error messages  
3. **Follow encryption patterns** - Use established crypto utilities
4. **Validate inputs** - Always sanitize external inputs
5. **Respect access control** - Honor app permissions and memory classes

### **Safe Patterns**
```rust
// ✅ GOOD: Safe error handling
match decrypt_memory(&encrypted_content) {
    Ok(content) => process_content(content),
    Err(_) => return Err(MimirError::Encryption("Failed to decrypt".to_string())),
}

// ❌ BAD: Leaking sensitive information
match decrypt_memory(&encrypted_content) {
    Ok(content) => process_content(content),
    Err(e) => return Err(MimirError::Encryption(format!("Failed: {}", e))), // May leak keys!
}
```

## 🧪 Testing Patterns

### **Test Structure**
- **Unit tests**: In `src/` files using `#[cfg(test)]`
- **Integration tests**: In `tests/` directory
- **Documentation tests**: In `///` doc comments
- **Benchmarks**: In `benches/` directory

### **Common Test Patterns**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mimir_core::{MemoryClass, Memory};

    #[test]
    fn test_memory_creation() {
        let memory = Memory {
            id: uuid::Uuid::new_v4(),
            content: "test content".to_string(),
            class: MemoryClass::Personal,
            // ... other fields
        };
        assert_eq!(memory.class, MemoryClass::Personal);
    }

    #[tokio::test]
    async fn test_async_operation() {
        // Use tokio::test for async tests
        let result = some_async_function().await;
        assert!(result.is_ok());
    }
}
```

## 📋 Common Tasks and Patterns

### **Adding a New Crate**

1. **Create directory structure**:
   ```bash
   mkdir -p crates/new-crate/src
   ```

2. **Create Cargo.toml**:
   ```toml
   [package]
   name = "mimir-new-crate"
   version.workspace = true
   edition.workspace = true
   license.workspace = true
   # ... other workspace fields

   [dependencies]
   mimir-core = { path = "../mimir-core" }
   # ... other deps
   ```

3. **Add to workspace**:
   ```toml
   # In root Cargo.toml
   [workspace]
   members = [
       # ... existing crates
       "crates/new-crate",
   ]
   ```

4. **Create lib.rs with documentation**:
   ```rust
   //! Brief description of the crate
   //!
   //! Longer description with usage examples.

   use mimir_core::{Result, MimirError};

   /// Main struct for this crate
   pub struct NewComponent {
       // fields
   }

   impl NewComponent {
       /// Create a new instance
       pub fn new() -> Result<Self> {
           Ok(Self {})
       }
   }
   ```

### **Adding New API Endpoints** (for mimir daemon)

1. **Define in server.rs**:
   ```rust
   use axum::{Json, extract::Query};
   use mimir_core::{MemoryQuery, MemoryResult};

   async fn handle_memory_query(
       Query(query): Query<MemoryQuery>
   ) -> Result<Json<Vec<MemoryResult>>, MimirError> {
       // Implementation
       Ok(Json(vec![]))
   }
   ```

2. **Add route in router setup**:
   ```rust
   let app = Router::new()
       .route("/memories/search", get(handle_memory_query))
       // ... other routes
   ```

### **Adding Language Bindings**

**Python (PyO3)**:
```rust
use pyo3::prelude::*;

#[pyclass]
struct PyMemory {
    inner: mimir_core::Memory,
}

#[pymethods]
impl PyMemory {
    #[new]
    fn new(content: String) -> Self {
        // Implementation
    }
}
```

**Node.js (napi-rs)**:
```rust
use napi_derive::napi;

#[napi]
pub struct JsMemory {
    inner: mimir_core::Memory,
}

#[napi]
impl JsMemory {
    #[napi(constructor)]
    pub fn new(content: String) -> Self {
        // Implementation
    }
}
```

## 🔍 Code Quality Standards

### **Formatting and Linting**
```bash
# Format code (required before commits)
cargo fmt --all

# Check linting (must pass)
cargo clippy --workspace -- -D warnings

# Run tests (must pass)
cargo test --workspace
```

### **Documentation Standards**
- **Public APIs**: Always document with `///`
- **Examples**: Include usage examples in docs
- **Errors**: Document when functions can panic or return errors
- **Safety**: Document any unsafe code blocks

```rust
/// Retrieves memories matching the given query.
///
/// # Arguments
/// * `query` - The search query parameters
///
/// # Returns
/// * `Ok(Vec<MemoryResult>)` - Matching memories with relevance scores
/// * `Err(MimirError)` - If search fails or access is denied
///
/// # Examples
/// ```
/// use mimir_core::{MemoryQuery, MemoryClass};
/// 
/// let query = MemoryQuery {
///     query: "coffee preferences".to_string(),
///     class_filter: Some(vec![MemoryClass::Personal]),
///     top_k: 5,
///     // ... other fields
/// };
/// let results = memory_store.search(query).await?;
/// ```
pub async fn search(&self, query: MemoryQuery) -> Result<Vec<MemoryResult>> {
    // Implementation
}
```

## 🚨 Error Handling Patterns

### **Result Types**
```rust
use mimir_core::{Result, MimirError};

// Function that can fail
fn process_memory(content: &str) -> Result<ProcessedMemory> {
    if content.is_empty() {
        return Err(MimirError::Guardrails("Empty content".to_string()));
    }
    // ... processing
    Ok(processed)
}

// Async function that can fail
async fn store_memory(memory: Memory) -> Result<()> {
    // Use ? operator for error propagation
    let processed = process_memory(&memory.content)?;
    database.store(processed).await?;
    Ok(())
}
```

### **Error Creation**
```rust
// Use appropriate error variants
return Err(MimirError::Database(anyhow::anyhow!("Connection failed")));
return Err(MimirError::VectorStore("Index corrupted".to_string()));
return Err(MimirError::AccessDenied("App not permitted".to_string()));
```

## 📁 File Organization Patterns

### **Crate Structure**
```
crate-name/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API and module declarations
│   ├── error.rs            # Crate-specific errors (if needed)
│   ├── types.rs            # Crate-specific types
│   ├── config.rs           # Configuration structures
│   └── module_name.rs      # Feature modules
├── tests/                  # Integration tests
│   └── integration_test.rs
└── benches/                # Benchmarks (if needed)
    └── benchmark.rs
```

### **Module Declaration Pattern**
```rust
// lib.rs
pub mod types;
pub mod config;
pub mod engine;

pub use types::*;
pub use engine::Engine;
// Re-export important types
```

## 🔄 Development Workflow

### **Before Implementing Features**
1. **Read existing code** to understand patterns
2. **Check PROJECT_STATUS.md** for current implementation status
3. **Review ROADMAP.md** for feature priorities
4. **Understand dependencies** between components

### **Implementation Process**
1. **Start with stub implementation** that compiles
2. **Add comprehensive tests** before full implementation
3. **Implement incrementally** with frequent testing
4. **Document as you go** - don't leave it for later
5. **Run full test suite** before considering complete

### **Validation Checklist**
- [ ] Code compiles without warnings
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Clippy passes (`cargo clippy --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] Documentation is complete
- [ ] No security anti-patterns introduced
- [ ] Integration with existing components works

## 🎯 Component-Specific Guidance

### **mimir-core**
- **Role**: Shared types, errors, configuration
- **Key Files**: `types.rs`, `error.rs`, `config.rs`
- **Principle**: Stable API, minimal dependencies
- **Changes**: Require careful consideration of impact on all other crates

### **mimir**
- **Role**: Main daemon process with MCP server
- **Key Files**: `main.rs`, `server.rs`, `mcp.rs`
- **Principle**: Robust, secure, performant
- **Changes**: Focus on reliability and security

### **mimir-vector**
- **Role**: Vector similarity search with HNSW
- **Key Pattern**: Async operations, performance-critical
- **Focus**: Memory efficiency, search speed

### **mimir-db**
- **Role**: Encrypted database operations
- **Key Pattern**: Secure by default, transaction safety
- **Focus**: Data integrity, access control

### **mimir-guardrails**
- **Role**: Content analysis and PII detection
- **Key Pattern**: ML model integration, privacy-preserving
- **Focus**: Accuracy, performance, privacy

### **Language Bindings**
- **Python**: Use PyO3 patterns, async-compatible
- **Node.js**: Use napi-rs patterns, Promise-based
- **WASM**: Use wasm-bindgen, browser-compatible

## 🔧 Troubleshooting Common Issues

### **Compilation Errors**
- **Missing dependencies**: Check workspace vs. crate-specific deps
- **Version conflicts**: Ensure all crates use workspace versions
- **Feature flags**: Some dependencies may need specific features

### **Test Failures**
- **Async tests**: Ensure using `#[tokio::test]`
- **Integration tests**: May need test data setup
- **Timing issues**: Use appropriate timeouts for async operations

### **Clippy Warnings**
- **Dead code**: May be stub implementations (acceptable during development)
- **Complexity**: Consider breaking down large functions
- **Performance**: Address suggestions for hot paths

## 📚 Additional Resources

- **CONTRIBUTING.md** - Detailed contribution guidelines
- **SECURITY.md** - Security policies and reporting
- **PROJECT_STATUS.md** - Current implementation status
- **docs/ROADMAP.md** - Development timeline and priorities
- **Rust docs**: `cargo doc --workspace --open`

## 🤖 AI Agent Best Practices

1. **Read before writing** - Understand existing patterns
2. **Start small** - Make incremental, testable changes
3. **Follow conventions** - Use established patterns and naming
4. **Test thoroughly** - Write tests for new functionality
5. **Document everything** - Future agents (and humans) will thank you
6. **Security first** - Always consider privacy and security implications
7. **Ask for clarification** - When in doubt, request more context

---

*This document is maintained to ensure AI agents can work effectively within the Mimir codebase. Update when new patterns or conventions are established.* 