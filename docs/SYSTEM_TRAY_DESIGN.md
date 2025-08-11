# Mimir System Tray Design Document

## Overview

This document outlines the design and implementation plan for the Mimir system tray feature, which provides users with a desktop interface for managing their AI memory vault, monitoring the memory server, and controlling access to stored memories.

## Table of Contents

1. [Current State Analysis](#current-state-analysis)
2. [Architecture Overview](#architecture-overview)
3. [Core Components](#core-components)
4. [Implementation Phases](#implementation-phases)
5. [Technical Implementation](#technical-implementation)
6. [User Experience Design](#user-experience-design)
7. [Security Considerations](#security-considerations)
8. [Development Workflow](#development-workflow)
9. [Deployment & Distribution](#deployment--distribution)
10. [Integration Points](#integration-points)

## Current State Analysis

### Existing Architecture

The Mimir codebase has a well-structured modular architecture with the following key components:

#### Core Components
- **`mimir-core`**: Shared types, errors, and configuration management
- **`mimir-db`**: Encrypted SQLite database with SQLCipher
- **`mimir-vector`**: HNSW-based vector store for similarity search
- **`mimir-llm`**: LLM integration for memory processing and summarization

#### Main Services
- **`mimir`**: Main daemon with MCP server and HTTP API
- **`mimir-cli`**: Command-line interface for vault management
- **`mimir-tray`**: System tray application (currently disabled due to Tauri compatibility)

#### Storage Layer
- **Integrated Storage Manager**: Coordinates database and vector operations
- **MCP Server**: Full Model Context Protocol implementation with memory operations
- **HTTP API**: RESTful endpoints for memory management

### Current Tray Status

The `mimir-tray` crate exists but is currently disabled in the workspace due to Tauri compatibility issues. The existing implementation includes:

- Basic Tauri 2.0 configuration
- Placeholder Rust backend structure
- HTML-based log viewer interface
- AGPL-3.0 licensing for UI components

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Mimir System Tray                        │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Tray UI   │  │  Memory     │  │  Service    │         │
│  │  (Tauri)    │  │  Viewer     │  │  Manager    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                    IPC Layer                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Tauri     │  │   HTTP      │  │   MCP       │         │
│  │   Commands  │  │   Client    │  │   Client    │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                    Mimir Daemon                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   HTTP      │  │   MCP       │  │   Storage   │         │
│  │   Server    │  │   Server    │  │   Manager   │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

### Key Design Principles

1. **Local-First**: All operations performed locally with optional cloud sync
2. **Zero-Knowledge**: Server cannot decrypt user data
3. **Privacy-Preserving**: Fine-grained consent and access control
4. **Cross-Platform**: Native experience on macOS, Windows, and Linux
5. **Modular Design**: Clear separation of concerns and reusable components

## Core Components

### 1. Tray Application (`mimir-tray`)

#### Technology Stack
- **Backend**: Rust with Tauri 2.0
- **Frontend**: Web technologies (HTML, CSS, JavaScript/TypeScript)
- **License**: AGPL-3.0 (as specified for UI components)

#### Core Features
- System tray icon with status indicator
- Context menu for quick actions
- Main window for detailed management
- Real-time status monitoring

#### Component Structure
```rust
pub struct TrayApp {
    tauri_app: tauri::App,
    service_manager: ServiceManager,
    memory_client: MemoryClient,
    config_manager: ConfigManager,
}
```

### 2. Service Manager

#### Responsibilities
- **Process Management**: Start/stop Mimir daemon
- **Status Monitoring**: Health checks and connection status
- **Configuration**: Vault settings and encryption management
- **Logging**: Real-time log viewing and filtering

#### Implementation
```rust
pub struct ServiceManager {
    daemon_process: Option<Child>,
    status: ServiceStatus,
    config: Config,
}

pub enum ServiceStatus {
    Running { pid: u32, uptime: Duration },
    Stopped,
    Starting,
    Stopping,
    Error { message: String },
}
```

### 3. Memory Viewer

#### Features
- **Memory Browser**: View stored memories by class
- **Search Interface**: Vector similarity search
- **Memory Management**: Edit, delete, and classify memories
- **Statistics**: Memory counts and storage usage

#### Data Flow
1. User initiates search or browse operation
2. Request sent via HTTP API to Mimir daemon
3. Daemon queries integrated storage manager
4. Results returned and displayed in UI

### 4. Connection Manager

#### Capabilities
- **MCP Connections**: Manage AI agent connections
- **Permission Control**: App-level access control
- **Connection Logs**: Monitor and audit connections

#### Permission Model
```rust
pub struct AppPermission {
    app_id: String,
    memory_classes: Vec<MemoryClass>,
    permissions: Vec<Permission>,
    created_at: DateTime<Utc>,
    last_used: Option<DateTime<Utc>>,
}

pub enum Permission {
    Read,
    Write,
    Delete,
    Search,
}
```

## Implementation Phases

### Phase 1: Foundation (2-3 weeks)

#### 1.1 Enable Tauri Integration
- Resolve Tauri compatibility issues
- Set up Tauri configuration and build pipeline
- Create basic tray icon and context menu

#### 1.2 Service Management
- Implement daemon start/stop functionality
- Add process monitoring and health checks
- Create service status indicators

#### 1.3 Basic UI Framework
- Design responsive web interface
- Implement navigation and layout
- Add status dashboard

#### Deliverables
- Working tray application with basic functionality
- Service start/stop capabilities
- Basic status monitoring

### Phase 2: Core Features (3-4 weeks)

#### 2.1 Memory Management
- Integrate with MCP server for memory operations
- Implement memory viewer with search
- Add memory editing and deletion capabilities

#### 2.2 Configuration Interface
- Vault settings management
- Encryption key management
- App permission controls

#### 2.3 Real-time Monitoring
- Live log viewing with filtering
- Connection status monitoring
- Performance metrics display

#### Deliverables
- Full memory management interface
- Configuration management
- Real-time monitoring capabilities

### Phase 3: Advanced Features (2-3 weeks)

#### 3.1 Connection Management
- MCP connection monitoring
- App permission management
- Connection audit logs

#### 3.2 Advanced Memory Features
- Memory classification interface
- Bulk operations
- Memory compression controls

#### 3.3 System Integration
- Auto-start configuration
- System notifications
- Keyboard shortcuts

#### Deliverables
- Complete connection management
- Advanced memory features
- Full system integration

## Technical Implementation

### Backend Architecture (Rust)

#### Core Application Structure
```rust
// Main tray application
pub struct TrayApp {
    tauri_app: tauri::App,
    service_manager: ServiceManager,
    memory_client: MemoryClient,
    config_manager: ConfigManager,
}

// Service management
pub struct ServiceManager {
    daemon_process: Option<Child>,
    status: ServiceStatus,
    config: Config,
}

// Memory client for MCP operations
pub struct MemoryClient {
    mcp_client: MCPClient,
    http_client: reqwest::Client,
    connection_status: ConnectionStatus,
}

// Configuration management
pub struct ConfigManager {
    config: Config,
    vault_path: PathBuf,
    keyset_path: PathBuf,
}
```

#### Tauri Commands
```rust
#[tauri::command]
async fn start_daemon() -> Result<ServiceStatus, String> {
    // Implementation for starting Mimir daemon
}

#[tauri::command]
async fn stop_daemon() -> Result<ServiceStatus, String> {
    // Implementation for stopping Mimir daemon
}

#[tauri::command]
async fn get_service_status() -> Result<ServiceStatus, String> {
    // Implementation for getting service status
}

#[tauri::command]
async fn search_memories(query: String) -> Result<Vec<Memory>, String> {
    // Implementation for memory search
}
```

### Frontend Architecture (Web)

#### Application State
```typescript
interface AppState {
  serviceStatus: ServiceStatus;
  memories: Memory[];
  connections: Connection[];
  logs: LogEntry[];
  config: Config;
}

interface ServiceStatus {
  status: 'running' | 'stopped' | 'starting' | 'stopping' | 'error';
  pid?: number;
  uptime?: number;
  error?: string;
}

interface Memory {
  id: string;
  content: string;
  class: MemoryClass;
  tags: string[];
  createdAt: string;
  updatedAt: string;
}
```

#### Service Classes
```typescript
class ServiceManager {
  async startDaemon(): Promise<void>;
  async stopDaemon(): Promise<void>;
  async getStatus(): Promise<ServiceStatus>;
  async getLogs(): Promise<LogEntry[]>;
}

class MemoryClient {
  async searchMemories(query: string): Promise<Memory[]>;
  async addMemory(memory: Memory): Promise<void>;
  async deleteMemory(id: string): Promise<void>;
  async updateMemory(memory: Memory): Promise<void>;
  async getMemoriesByClass(class: MemoryClass): Promise<Memory[]>;
}

class ConfigManager {
  async getConfig(): Promise<Config>;
  async updateConfig(config: Partial<Config>): Promise<void>;
  async rotateKeys(): Promise<void>;
}
```

### Communication Layer

#### Tauri Commands
- **System Operations**: Start/stop daemon, process management
- **Configuration**: Vault settings, encryption management
- **File Operations**: Log access, configuration files

#### HTTP API
- **Memory Operations**: CRUD operations for memories
- **Search**: Vector similarity search
- **Statistics**: Vault statistics and metrics

#### MCP Protocol
- **AI Agent Integration**: Direct MCP communication
- **Memory Processing**: LLM-based operations
- **Real-time Updates**: Live memory updates

#### WebSocket
- **Real-time Logs**: Live log streaming
- **Status Updates**: Service status changes
- **Connection Events**: MCP connection events

## User Experience Design

### System Tray Interface

#### Icon States
- **Connected**: Green icon - service running and healthy
- **Disconnected**: Gray icon - service stopped
- **Error**: Red icon - service error or unhealthy
- **Loading**: Animated icon - service starting/stopping

#### Context Menu
```
┌─────────────────────┐
│ 📋 Open Mimir       │
│ ├───────────────────┤
│ ▶️  Start Service   │
│ ⏹️  Stop Service    │
│ ├───────────────────┤
│ 🧠 View Memories    │
│ ⚙️  Settings        │
│ ├───────────────────┤
│ ❌ Quit             │
└─────────────────────┘
```

### Main Application Window

#### Tab Structure
1. **Dashboard Tab**
   - Service status overview
   - Quick statistics
   - Recent activity
   - System health indicators

2. **Memories Tab**
   - Memory browser with filters
   - Search interface
   - Memory editing capabilities
   - Bulk operations

3. **Connections Tab**
   - MCP connection monitoring
   - App permission management
   - Connection audit logs
   - Permission controls

4. **Settings Tab**
   - Vault configuration
   - Encryption settings
   - Auto-start options
   - Notification preferences

5. **Logs Tab**
   - Real-time log viewing
   - Log filtering by level
   - Log export capabilities
   - Performance metrics

#### Responsive Design
- **Desktop**: Full-featured interface with sidebar navigation
- **Compact Mode**: Minimal interface for small screens
- **Dark/Light Theme**: System theme integration
- **Accessibility**: Screen reader support and keyboard navigation

## Security Considerations

### Authentication & Authorization

#### Local Authentication
- **OS Keychain Integration**: Secure credential storage
- **Password-based Encryption**: Alternative authentication method
- **Biometric Authentication**: Platform-specific biometric support

#### Permission Model
- **App-level Access Control**: Granular permissions per application
- **Memory Class Isolation**: Separate permissions per memory class
- **Time-based Permissions**: Expiring access tokens

#### Audit Logging
- **Operation Logging**: All memory operations logged
- **Access Logging**: All access attempts recorded
- **Security Events**: Authentication and authorization events

### Data Protection

#### Communication Security
- **Encrypted IPC**: All inter-process communication encrypted
- **TLS for HTTP**: Secure HTTP API communication
- **MCP Security**: Secure MCP protocol implementation

#### Memory Protection
- **Memory Isolation**: Secure memory handling
- **Encryption at Rest**: All data encrypted in storage
- **Key Management**: Secure key storage and rotation

#### Privacy Features
- **PII Detection**: Automatic personal information detection
- **Data Minimization**: Minimal data collection
- **User Control**: Full user control over data

## Development Workflow

### Build System

#### Development Commands
```bash
# Development mode
cargo tauri dev

# Production build
cargo tauri build

# Cross-platform builds
cargo tauri build --target x86_64-apple-darwin
cargo tauri build --target x86_64-unknown-linux-gnu
cargo tauri build --target x86_64-pc-windows-msvc
```

#### Testing Strategy
- **Unit Tests**: Core functionality testing
- **Integration Tests**: End-to-end workflow testing
- **UI Tests**: Automated UI testing with Playwright
- **Security Tests**: Penetration testing and audit

#### Code Quality
- **Rust**: Clippy linting and rustfmt formatting
- **TypeScript**: ESLint and Prettier
- **Documentation**: Comprehensive API documentation
- **Performance**: Benchmarking and profiling

### CI/CD Pipeline

#### GitHub Actions
```yaml
name: Tray Application CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install dependencies
        run: |
          cargo install tauri-cli
          npm install
      
      - name: Run tests
        run: |
          cargo test
          npm test
      
      - name: Build application
        run: cargo tauri build
```

## Deployment & Distribution

### Package Management

#### macOS
- **Homebrew**: `brew install mimir/tap/mimir-tray`
- **DMG Installer**: Native macOS installer
- **App Store**: Optional App Store distribution

#### Linux
- **AppImage**: Portable Linux application
- **Snap**: Snapcraft package
- **Flatpak**: Flatpak package
- **Native Packages**: .deb and .rpm packages

#### Windows
- **MSI Installer**: Native Windows installer
- **Chocolatey**: `choco install mimir-tray`
- **Winget**: Microsoft package manager

### Auto-updates

#### Tauri Updater
- **Delta Updates**: Efficient update delivery
- **Rollback Support**: Safe update rollback
- **Background Updates**: Automatic background updates
- **User Control**: User-controlled update scheduling

#### Update Channels
- **Stable**: Production releases
- **Beta**: Pre-release testing
- **Nightly**: Development builds

## Integration Points

### Existing Components

#### MCP Server Integration
- **Direct Communication**: Native MCP protocol support
- **Memory Operations**: Full memory CRUD operations
- **Search Capabilities**: Vector similarity search
- **Real-time Updates**: Live memory updates

#### CLI Integration
- **Shared Configuration**: Common configuration management
- **Vault Management**: Shared vault operations
- **Key Management**: Unified encryption key management
- **Status Reporting**: Consistent status reporting

#### Database Integration
- **Direct Access**: Efficient database operations
- **Transaction Support**: ACID transaction support
- **Migration Support**: Database schema migrations
- **Backup Integration**: Automated backup support

#### Vector Store Integration
- **Search Integration**: Direct vector search capabilities
- **Embedding Management**: Efficient embedding storage
- **Similarity Metrics**: Advanced similarity calculations
- **Performance Optimization**: Optimized search performance

### External Systems

#### AI Agents
- **MCP Protocol**: Standard MCP protocol support
- **Connection Management**: AI agent connection monitoring
- **Permission Control**: Granular permission management
- **Audit Logging**: Comprehensive connection logging

#### System Services
- **Auto-start**: System boot integration
- **Background Operation**: Background service management
- **Resource Management**: System resource optimization
- **Notification Integration**: OS-level notifications

#### Development Tools
- **IDE Integration**: Development environment integration
- **Debugging Support**: Comprehensive debugging capabilities
- **Profiling Tools**: Performance profiling support
- **Documentation**: Comprehensive documentation

## Conclusion

This design document provides a comprehensive roadmap for implementing the Mimir system tray feature. The modular architecture ensures maintainability and extensibility while the phased implementation approach allows for iterative development and testing.

The system tray will provide users with intuitive control over their AI memory vault while maintaining the privacy-first, local-first principles that are core to the Mimir project. The integration with existing components ensures consistency and leverages the robust foundation already established in the codebase.

## Next Steps

1. **Phase 1 Implementation**: Begin with foundation work and basic tray functionality
2. **Tauri Compatibility**: Resolve Tauri compatibility issues
3. **Service Management**: Implement core service management features
4. **UI Framework**: Develop the basic user interface
5. **Testing**: Establish comprehensive testing framework

The implementation should follow the phased approach outlined in this document, with regular reviews and adjustments based on user feedback and technical requirements. 