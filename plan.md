# Integration Plan: Zed Standalone App with Webview

## Overview

This document outlines a strategy to integrate the EPICS Control System Web UI (currently Axum + HTMX) into a fully supported standalone Zed application using webview, maintaining the same technology stack while leveraging Zed's native UI capabilities.

## Current State

- **Stack**: Axum (web framework) + HTMX (lightweight JavaScript) + Maud (HTML templating)
- **Architecture**: Browser-based HTTP client connecting to Axum server
- **Features**: Real-time PV monitoring, widget-based control interface, SSE polling
- **Dependencies**: 20+ crates, minimal browser footprint (14KB HTMX)
- **Deployment**: Single binary with bundled static assets

## Goals

1. Create a native Zed extension that embeds the Axum server in-process
2. Use webview to render the existing HTML/HTMX UI without changes
3. Maintain full compatibility with the current feature set
4. Provide seamless integration with Zed's IDE environment
5. Enable EPICS control system integration directly within the Zed editor

## Implementation Strategy

### Phase 1: Foundation - Axum Server as Library

**Objective**: Refactor the current binary into a reusable library component

#### 1.1 Extract Core Server Logic
- Move Axum server setup from `main.rs` into `lib.rs` as public functions
- Create public initialization function: `pub async fn create_app_router(config: ScreenConfig) -> Router`
- Expose `AppState` and server creation utilities
- Maintain all existing route handlers and middleware

#### 1.2 Configuration Management
- Enhance `config.rs` to support multiple configuration sources:
  - File paths (current)
  - In-memory JSON (for programmatic setup)
  - Zed project metadata (future)
- Add environment variable overrides for EPICS connectivity

#### 1.3 Static Assets Embedding
- Embed HTMX, CSS, and other static assets as byte arrays using `include_bytes!` macro
- Create virtual file serving handler that serves from memory
- Alternatively, reference static assets from Zed extension resources

**Deliverables**:
- `src/lib.rs` exports a `pub async fn start_server(config, port) -> Server`
- Cargo workspace with:
  - `ctrl-sys-widgets` crate (core library)
  - `ctrl-demo-server` binary (CLI tool)
  - `ctrl-sys-zed` crate (Zed extension)

---

### Phase 2: Zed Extension Infrastructure

**Objective**: Create Zed extension that manages server lifecycle and webview

#### 2.1 Zed Extension Project Structure
```
ctrl-sys-zed/
├── extension.toml
├── Cargo.toml
├── src/
│   ├── lib.rs                 # Extension entry point
│   ├── server_manager.rs      # Axum server lifecycle
│   ├── webview_handler.rs     # Webview communication
│   └── config_provider.rs     # Integration with Zed settings
├── assets/
│   ├── icon.png
│   └── styles/
└── README.md
```

#### 2.2 Extension Manifest (`extension.toml`)
- Define extension metadata (name, version, author)
- List language servers (if needed)
- Configure commands available in Zed
- Define workspace/project integration points

#### 2.3 Server Manager Module
- Spawn Axum server in a Tokio runtime (on extension load)
- Detect available port (or use configured port)
- Track server health and connection status
- Handle graceful shutdown on extension unload
- Implement error recovery and restart logic

**Key Implementation**:
```rust
pub struct ServerManager {
    server_task: Option<JoinHandle<()>>,
    port: u16,
    status: Arc<Mutex<ServerStatus>>,
}

impl ServerManager {
    pub async fn start(config: ScreenConfig) -> Result<Self>;
    pub fn port(&self) -> u16;
    pub async fn shutdown(&mut self) -> Result<()>;
}
```

---

### Phase 3: Webview Integration

**Objective**: Embed and control webview instances within Zed panels

#### 3.1 Webview Panel Provider
- Create panel that opens webview pointing to `http://localhost:{port}`
- Initialize webview when extension loads
- Use Zed's `Panel` API for UI integration
- Position in sidebar or bottom panel (configurable)

#### 3.2 URL Management
- Dynamically construct localhost URL with detected port
- Handle webview reload/refresh
- Support dev mode (with hot reload) and release mode
- Cache port detection result

#### 3.3 Context Bridge (JavaScript ↔ Rust)
- Expose Zed workspace information to webview via context bridge
- Allow webview to:
  - Query current file path
  - Access workspace settings
  - Trigger Zed commands
- Implement bidirectional message passing

**Protocol**:
```rust
// Message from webview → Zed
#[derive(Serialize)]
struct WebviewMessage {
    command: String,
    data: serde_json::Value,
}

// Response from Zed → webview
#[derive(Serialize)]
struct ZedResponse {
    status: String,
    data: Option<serde_json::Value>,
}
```

---

### Phase 4: Data Synchronization

**Objective**: Sync EPICS PV data with Zed editor workspace

#### 4.1 PV-to-Editor Binding
- Map PV names to editor variables/metadata
- Update editor state when PVs change
- Display current PV values in status bar or inline
- Support PV status indicators in UI

#### 4.2 Editor-to-PV Commands
- Allow editor commands to trigger PV writes
- Support keybindings for common operations
- Implement undo/redo for PV commands

#### 4.3 State Synchronization
- Use shared state between server and extension
- Store session data (displayed PVs, layout, etc.)
- Persist user preferences (layout, update frequency)

---

### Phase 5: User Interface & UX

**Objective**: Provide seamless IDE integration

#### 5.1 Command Palette Integration
- `EPICS: Open Control Panel` - Launch webview
- `EPICS: Connect to IOC` - Configure connection
- `EPICS: Reload Configuration` - Reload from file
- `EPICS: Show PV Status` - Display connection status

#### 5.2 Status Bar Items
- Show EPICS connection status (connected/disconnected)
- Display number of active PVs
- Show server port and uptime
- Quick access to open control panel

#### 5.3 Settings & Configuration
- `epics.serverPort` - TCP port for Axum server
- `epics.configPath` - Path to demo_config.json
- `epics.autoStart` - Auto-start server on extension load
- `epics.pvDisplayFormat` - How to display PV values in editor
- `epics.logLevel` - Tracing/debug output level

#### 5.4 Theme & Styling
- Adapt CSS colors to match Zed theme
- Support light/dark mode switching
- Ensure accessibility (WCAG compliance)
- Use Zed's design tokens where possible

---

### Phase 6: Testing & Validation

**Objective**: Ensure reliability and compatibility

#### 6.1 Unit Tests
- Server initialization and teardown
- Configuration loading
- Route handlers
- PV monitor callbacks

#### 6.2 Integration Tests
- Extension load/unload
- Server lifecycle (start/stop)
- Webview communication
- EPICS connectivity

#### 6.3 E2E Tests
- Open extension in Zed
- Verify webview renders
- Test PV updates
- Validate editor integration

#### 6.4 Performance Benchmarks
- Server startup time
- Webview load time
- PV update latency
- Memory footprint

---

### Phase 7: Documentation & Examples

#### 7.1 User Documentation
- Installation instructions (Zed Extension Registry)
- Configuration guide
- Troubleshooting
- Screenshots and demos

#### 7.2 Developer Documentation
- Architecture overview
- Extension API reference
- Contributing guidelines
- Local development setup

#### 7.3 Example Configurations
- Basic single IOC setup
- Multi-IOC configuration
- Custom widget types
- Integration examples

---

## Technical Considerations

### Port Assignment Strategy
- Use `0` to let OS assign ephemeral port
- Query actual port via `TcpListener::local_addr()`
- Store port in module-level `Once<u16>` for reuse
- Communicate port to webview via shared state

### Async Runtime Management
- Single Tokio runtime per extension instance
- Use `spawn_blocking` for PVXS (C++ library) calls
- Properly handle cancellation on extension unload
- Implement graceful shutdown with timeout

### Security Considerations
- Server listens only on localhost (127.0.0.1)
- Validate webview origin to prevent XSS
- Sanitize configuration paths
- Implement optional authentication layer (future)

### Performance Optimization
- Lazy-load webview (only when panel opened)
- Cache static assets in memory
- Minimize JSON serialization overhead
- Use DashMap for concurrent PV cache (existing)

### Compatibility
- Support Zed v0.x (current stable)
- Test with various EPICS versions
- Ensure HTMX works correctly in embedded webview
- Maintain backward compatibility with CLI binary

---

## Risk Assessment & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|-----------|
| Webview rendering issues | Medium | High | Test early with minimal HTML; use standard HTML5 |
| Port conflicts/unavailability | Low | Medium | Dynamic port selection; configurable fallback |
| Server crash impacts IDE | High | High | Error recovery; separate crash handling; logging |
| EPICS connectivity issues | Medium | Medium | Graceful disconnection handling; status indicators |
| Performance degradation | Medium | Medium | Profiling; lazy loading; caching strategies |
| Version incompatibilities | Low | High | CI testing against Zed releases; semantic versioning |

---

## Implementation Roadmap

### Sprint 1: Foundation (Weeks 1-2)
- [ ] Refactor server code into library
- [ ] Extract static assets
- [ ] Create minimal Zed extension scaffold
- [ ] Basic server lifecycle management

### Sprint 2: Webview Integration (Weeks 3-4)
- [ ] Implement webview panel provider
- [ ] Port detection and URL handling
- [ ] Test webview rendering of existing UI
- [ ] Basic refresh/reload capability

### Sprint 3: Enhancement (Weeks 5-6)
- [ ] Context bridge implementation
- [ ] Status bar integration
- [ ] Command palette additions
- [ ] Theme/styling adaptation

### Sprint 4: Polish & Testing (Weeks 7-8)
- [ ] Comprehensive testing
- [ ] Documentation
- [ ] Performance optimization
- [ ] Release preparation

---

## Deployment Strategy

### Distribution
1. **Zed Extension Registry** (primary) - One-click install in Zed
2. **GitHub Releases** - Manual installation via `.vsix` analogue
3. **Source Distribution** - For developers/contributors

### Installation Flow
```
User opens Zed
→ Extension Manager
→ Search "EPICS Control"
→ Install ctrl-sys-zed
→ Configure IOC connection in settings.json
→ Reload Zed
→ ctrl-sys-zed activates and starts server
→ User opens command palette
→ Runs "EPICS: Open Control Panel"
→ Webview opens with familiar control UI
```

### Updates
- Semantic versioning (MAJOR.MINOR.PATCH)
- Auto-update via Zed's extension manager
- Changelog in GitHub releases
- Breaking changes communicated clearly

---

## Success Metrics

1. **Functionality**: All existing features work in Zed extension
2. **Performance**: Server starts in <2 seconds; UI responsive
3. **User Experience**: Installation and setup within 5 minutes
4. **Adoption**: 100+ installs in first month (stretch: 500+)
5. **Stability**: <1% crash rate in production
6. **Documentation**: Complete with examples and troubleshooting

---

## Future Enhancements (Post-MVP)

- [ ] Workspace integration: PV references in code
- [ ] Custom widget designer (drag & drop)
- [ ] Historical data visualization/archiving
- [ ] Alarm notifications in IDE
- [ ] Multi-extension coordination
- [ ] Plugin marketplace for custom widgets
- [ ] VCS integration (track PV changes)
- [ ] Collaborative sessions (multiple users)

---

## Conclusion

Integrating the EPICS Control System UI into Zed as a standalone extension provides a powerful workflow where engineers can monitor and control systems directly within their editor. By keeping the existing Axum + HTMX stack and adding a thin webview integration layer, we achieve:

✅ **Minimal refactoring** - Reuse 95% of existing code  
✅ **Maximum compatibility** - Works with all current features  
✅ **Native IDE experience** - Seamless Zed integration  
✅ **Easy distribution** - Simple installation via registry  
✅ **Maintainability** - Clear separation of concerns  

This approach validates Rust + HTMX as a viable full-stack framework for both traditional web apps and modern IDE extensions.
