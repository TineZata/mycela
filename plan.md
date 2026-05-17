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

### Phase 3.5: Visual Widget Builder (Drag & Drop Designer)

**Objective**: Enable users to visually design control system screens by dragging and dropping widgets, inspired by Windows Forms visual designer

#### 3.5.1 Widget Palette & Toolbox
- Create a visual widget palette sidebar in the webview
- Available widgets:
  - **Display Widgets**: Text labels, gauges, charts, indicators
  - **Control Widgets**: Buttons, sliders, text inputs, dropdown menus
  - **Container Widgets**: Panels, groups, tabs, grids
  - **Monitoring Widgets**: PV readbacks, trend plots, alarm displays
- Each widget shows icon, name, and quick description
- Drag widgets from palette onto canvas

#### 3.5.2 Canvas & Layout Engine
- WYSIWYG (What You See Is What You Get) design surface
- Grid-based or free-form positioning
- Snap-to-grid for alignment
- Automatic layout options (flex, grid layout)
- Real-time preview with actual PV data
- Undo/redo support for all design operations

#### 3.5.3 Property Inspector
- When widget selected on canvas, show properties panel
- Editable properties:
  - **Common**: Widget type, name/ID, position, size, visible, enabled
  - **PV Binding**: PV name(s), update frequency, read/write access
  - **Display**: Format, units, precision, colors, fonts
  - **Behavior**: On-click actions, hover effects, animations
  - **Validation**: Min/max ranges, input filters, error messages
- Live property changes reflected on canvas
- Property inheritance for widget groups

#### 3.5.4 Data Binding Interface
- Drag PV names from a PV browser onto widgets
- Auto-detect compatible widgets for PV type
- Visual indicator of bound PVs (e.g., color-coded borders)
- Binding editor for complex mappings:
  - Formula fields (e.g., `PV1 + PV2 * 2`)
  - Unit conversions
  - Conditional display logic
- PV connection status display

#### 3.5.5 Screen/Layout Management
- Create multiple named screens/views
- Switch between screens via tabs or menu
- Screen properties: name, description, refresh rate
- Export/import screens as JSON
- Version control friendly format (JSON/YAML)

#### 3.5.6 Design Persistence
- Auto-save design to local storage as user edits
- Periodic save to configuration file
- Integration with `config.rs`:
  - Save screen layouts to `demo_config.json`
  - Load existing configurations into designer
  - Option to edit config via visual designer
- Backup/snapshot functionality

#### 3.5.7 Implementation Architecture

**Widget Definition Format** (JSON/TOML):
```json
{
  "id": "control_panel_main",
  "type": "screen",
  "title": "Motor Control",
  "widgets": [
    {
      "id": "motor_speed_slider",
      "type": "slider",
      "position": { "x": 10, "y": 20 },
      "size": { "width": 200, "height": 40 },
      "pv": "IOC:m1.VAL",
      "min": 0,
      "max": 100,
      "unit": "RPM"
    },
    {
      "id": "status_display",
      "type": "gauge",
      "position": { "x": 220, "y": 20 },
      "size": { "width": 150, "height": 150 },
      "pv": "IOC:m1.RBV",
      "formula": "value"
    }
  ]
}
```

**Designer Components** (HTMX + JavaScript):
- `designer.js` - Canvas manipulation, drag-drop events
- `widget-palette.html` - Searchable widget list
- `property-inspector.html` - Dynamic property editor
- `canvas.html` - Rendering engine for widgets

**Storage & Sync**:
- Designer state stored in IndexedDB (client-side cache)
- Server endpoint: `POST /api/screens/{id}` - Save screen design
- Server endpoint: `GET /api/screens/{id}` - Load screen design
- Real-time sync with file system via Axum

**Deliverables**:
- Visual designer webview component
- Widget registry system (extensible)
- Screen serialization format
- Integration with existing `demo_config.json`
- Documentation: "Creating Screens with the Designer"

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

### Sprint 3: Widget Designer & Enhancement (Weeks 5-6)
- [ ] Implement visual widget designer/builder
- [ ] Widget palette and canvas rendering
- [ ] Property inspector with data binding UI
- [ ] Screen persistence and layout management
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

1. **Functionality**: All existing features work in Zed extension + visual widget designer
2. **Designer UX**: Users can create a complete control screen in <10 minutes without coding
3. **Performance**: Server starts in <2 seconds; UI responsive; canvas renders 50+ widgets smoothly
4. **User Experience**: Installation and setup within 5 minutes; designer intuitive for Windows Forms users
5. **Adoption**: 100+ installs in first month (stretch: 500+)
6. **Stability**: <1% crash rate in production
7. **Documentation**: Complete with examples, tutorials, and troubleshooting; designer user guide included

---

## Future Enhancements (Post-MVP)

- [ ] Advanced theme customization engine (custom CSS/styling)
- [ ] Historical data visualization/archiving
- [ ] Alarm notifications in IDE
- [ ] Multi-extension coordination
- [ ] Plugin marketplace for custom widgets
- [ ] Custom widget development toolkit (SDKs for 3rd-party widgets)
- [ ] VCS integration (track PV changes and screen designs)
- [ ] Collaborative sessions (multiple users editing screens)
- [ ] Automated testing framework for control screens
- [ ] Performance profiling dashboard
- [ ] Mobile companion app (remote screen viewing)
- [ ] AI-assisted screen layout suggestions
- [ ] Integration with EPICS alarm handler
- [ ] Advanced formula editor with validation
- [ ] Screen templates library for common patterns

---

## Conclusion

Integrating the EPICS Control System UI into Zed as a standalone extension with a visual widget designer provides a powerful workflow where engineers can monitor and control systems directly within their editor. The drag-and-drop widget builder, inspired by Windows Forms visual design, enables both developers and control system engineers to create custom screens without coding. By keeping the existing Axum + HTMX stack and adding a thin webview integration layer with an intuitive designer, we achieve:

✅ **Minimal refactoring** - Reuse 95% of existing code  
✅ **Maximum compatibility** - Works with all current features  
✅ **Native IDE experience** - Seamless Zed integration  
✅ **Easy screen design** - Visual drag-and-drop builder for non-programmers  
✅ **Easy distribution** - Simple installation via registry  
✅ **Maintainability** - Clear separation of concerns  

This approach validates Rust + HTMX as a viable full-stack framework for both traditional web apps and modern IDE extensions, combined with visual design paradigms that empower domain experts to build sophisticated control interfaces.
