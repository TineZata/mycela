# Installing Protocol Buffers Compiler (protoc)

The proto-based widget system requires the Protocol Buffers compiler (`protoc`) to be installed.

## Windows

### Option 1: Scoop (Recommended)
```powershell
scoop install protobuf
```

### Option 2: Chocolatey
```powershell
choco install protobuf
```

### Option 3: Manual Installation
1. Download the latest release from: https://github.com/protocolbuffers/protobuf/releases
2. Look for `protoc-{version}-win64.zip`
3. Extract to a location (e.g., `C:\protobuf`)
4. Add to PATH or set environment variable:
   ```powershell
   $env:PROTOC = "C:\protobuf\bin\protoc.exe"
   ```

## Linux (Ubuntu/Debian)
```bash
sudo apt update
sudo apt install protobuf-compiler
```

## macOS
```bash
brew install protobuf
```

## Verify Installation

```bash
protoc --version
```

Should output something like: `libprotoc 3.21.12`

## Build Commands

### With Proto Features (requires protoc)
```bash
# Build library with proto support
cargo build --features proto

# Build WASM with proto support
cargo build --target wasm32-unknown-unknown --release --features proto
```

### Without Proto Features (no protoc needed)
```bash
# Build without proto - uses existing widgets only
cargo build

# Build WASM without proto
cargo build --target wasm32-unknown-unknown --release
```

## Troubleshooting

### Error: "Could not find `protoc`"

**Solution**: Install protoc using one of the methods above, then:

```powershell
# Windows - set environment variable
$env:PROTOC = "C:\path\to\protoc.exe"

# Or add to PATH permanently via System Properties > Environment Variables
```

```bash
# Linux/macOS - usually automatically in PATH after install
which protoc
```

### Error: "protoc-gen-prost not found"

This usually means prost-build is not finding protoc. Make sure:
1. `protoc` is in your PATH
2. Or `PROTOC` environment variable is set
3. Try running `protoc --version` to verify

### Alternative: Build Without Proto

If you don't need the proto-based dynamic widget system, you can build without it:

```bash
cargo build --no-default-features
```

This will use only the hard-coded widgets in `src/inputs/`.
