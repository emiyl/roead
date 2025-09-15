# GitHub Copilot Instructions for roead

## Project Overview

**roead** is a Rust library that ports the functionality of the oead C++ library for handling common Nintendo file formats used in modern first-party games. It provides safe, idiomatic Rust implementations for parsing and writing Nintendo EAD/EPD file formats.

### Supported File Formats
- **AAMP** (binary parameter archive) - Nintendo's parameter format
- **BYML** (binary YAML) - Nintendo's binary YAML format (versions 2-7)
- **SARC** (archive) - Nintendo's archive format
- **Yaz0** - Nintendo's compression algorithm (via C++ FFI bindings)

## Architecture & Design Patterns

### Module Structure
```
src/
├── lib.rs          # Main library entry point with error types and common exports
├── types.rs        # Common types like Endian enum
├── util.rs         # Utility functions and helper code
├── yaml.rs         # YAML serialization support
├── aamp/           # AAMP format implementation
├── byml/           # BYML format implementation  
├── sarc/           # SARC format implementation
├── yaz0.rs         # Yaz0 Rust interface
└── yaz0.cpp        # Yaz0 C++ implementation (FFI)
```

### Key Design Principles
1. **Feature-gated modules**: Each major format is behind a feature flag
2. **Zero-copy parsing**: Uses `binrw` for efficient binary deserialization
3. **Serde integration**: Optional serde support via `with-serde` feature
4. **YAML compatibility**: Optional YAML export for AAMP/BYML via `yaml` feature
5. **Memory safety**: C++ FFI wrapped in safe Rust interfaces

### Feature Flags
- `aamp` - AAMP format support (default)
- `byml` - BYML format support (default) 
- `sarc` - SARC format support (default)
- `yaz0` - Yaz0 compression via C++ FFI (default)
- `yaml` - YAML serialization support
- `with-serde` - Serde derive support
- `aamp-names` - AAMP parameter name support

## Development Environment

### Build Requirements
- **Rust**: MSRV 1.80 (stable channel)
- **CMake**: 3.12+ (for yaz0 C++ compilation)
- **C++ Compiler**: C++17 support required
- **Git submodules**: `lib/zlib-ng` must be initialized

### Initial Setup
```bash
git clone https://github.com/NiceneNerd/roead
cd roead
git submodule update --init --recursive
```

### Build Commands
```bash
# Check compilation
cargo check

# Build with all features
cargo build --all-features

# Run tests (requires test data)
cargo test --all-features

# Format code (uses rustfmt.toml config)
cargo fmt
```

### Testing Strategy
- **Unit tests**: Inline `#[test]` functions throughout modules
- **Integration tests**: Test data in `test/` directory
- **Feature testing**: Tests run with `--all-features` in CI
- **Cross-platform**: CI runs on Ubuntu, Windows, macOS

## Code Style & Conventions

### Rust Formatting
Uses custom `rustfmt.toml` (requires nightly toolchain) with:
- Unix newlines
- Field init shorthand
- Try operator shorthand  
- Import grouping: `StdExternalCrate`
- Comment wrapping enabled
- Struct field alignment threshold: 2

### Error Handling
- Central `Error` enum in `lib.rs` using `thiserror`
- Module-specific error variants
- `Result<T>` type alias for `std::result::Result<T, Error>`
- No `unwrap()` in non-test code (enforced by clippy)

### Performance Considerations
- Uses `smartstring` for efficient string handling
- `rustc-hash` for faster hashing where applicable
- `binrw` for zero-copy binary parsing
- Lazy static initialization where needed

## Working with File Formats

### AAMP (Parameter Archives)
- Located in `src/aamp/`
- Supports parameter objects, lists, and primitive types
- Text parsing/writing via `aamp/text.rs`
- Binary parsing/writing via `aamp/parser.rs` and `aamp/writer.rs`

### BYML (Binary YAML)
- Located in `src/byml/`
- Reader API in `byml/reader/` for streaming access
- Full document parsing in `byml/mod.rs`
- Text export via `byml/text.rs`

### SARC (Archives)
- Located in `src/sarc/`
- File parsing in `sarc/parse.rs`
- Archive writing in `sarc/write.rs`
- Supports nested archives and file metadata

### Yaz0 (Compression)
- Rust interface in `src/yaz0.rs`
- C++ implementation in `src/yaz0.cpp`
- Uses CXX for safe FFI bindings
- Requires zlib-ng submodule

## FFI & C++ Integration

### CXX Bridge Setup
The `yaz0` module uses CXX for C++ interop:
- Build script in `build.rs` handles C++ compilation
- CXX bridge defined in `src/yaz0.rs`
- C++ headers in `src/include/oead/`
- Links against custom-built zlib-ng

### Build Configuration
- Windows: Uses MSVC, links `zlibd`
- Unix: Uses standard C++ compiler, links `zlib`
- Cross-compilation supported for common targets

## Testing & Quality Assurance

### Test Organization
- Unit tests embedded in source files with `#[test]`
- Test data in `test/` directory (not committed)
- Integration tests validate round-trip parsing
- Feature-gated tests ensure compatibility

### Continuous Integration
- GitHub Actions in `.github/workflows/`
- Multi-platform testing (Linux, Windows, macOS)
- All features tested together
- CMake and C++ toolchain setup automated

### Code Quality Tools
- Clippy lints with custom configuration
- rustfmt with project-specific settings
- Warning-level lints for lifetime syntax
- Unused code detection enabled

## Common Patterns & Idioms

### Binary Parsing
```rust
use binrw::{binread, BinRead};

#[binread]
#[br(little)]
struct FileHeader {
    magic: [u8; 4],
    version: u32,
    // ...
}
```

### Error Propagation
```rust
fn parse_file(data: &[u8]) -> Result<FileFormat> {
    let header = FileHeader::read(&mut Cursor::new(data))?;
    // Process...
    Ok(result)
}
```

### Feature-Gated Code
```rust
#[cfg(feature = "yaml")]
pub fn to_yaml(&self) -> Result<String> {
    // YAML export implementation
}
```

## Contributing Guidelines

### Code Submission
1. Follow existing code style and patterns
2. Add appropriate feature gates for optional functionality
3. Include unit tests for new parsing logic
4. Update documentation for public APIs
5. Ensure cross-platform compatibility

### Performance Expectations
- Zero-copy parsing where possible
- Minimal allocations in hot paths
- Efficient string handling with `smartstring`
- Memory safety without performance penalties

### Documentation Standards
- Public APIs require rustdoc comments
- Module-level documentation explains format details
- Examples provided for complex APIs
- Links to Nintendo file format documentation where available

## Dependencies & Ecosystem

### Core Dependencies
- `binrw` - Binary reading/writing
- `thiserror` - Error handling
- `smartstring` - Efficient strings
- `rustc-hash` - Fast hashing

### Optional Dependencies
- `serde` - Serialization framework
- `indexmap` - Ordered maps
- `ryml` - YAML parsing (rapid-yaml bindings)
- `base64` - Binary data encoding
- `cxx` - C++ FFI bridge

### Build Dependencies
- `cxx-build` - C++ compilation
- `rustc_version` - Rust version detection

This project bridges the gap between Nintendo's binary formats and Rust's safety guarantees, prioritizing both performance and correctness in game development tooling.
