# BYML and AAMP Reader API Roadmap

## Executive Summary

This document outlines the implementation plan for adding zero-copy reader APIs to the roead library for BYML and AAMP formats, while maintaining backward compatibility with the existing owned/editable API. The new reader APIs will provide maximum performance for read-only operations by minimizing heap allocations and avoiding unnecessary data copying.

## Background

The current roead API parses entire BYML and AAMP files into owned, mutable data structures (`Byml` and `ParameterIO` respectively). While this provides a convenient API for editing, it comes with performance costs:

- Full heap allocation of all data structures
- Copying of all string data and binary content
- Recursive parsing of the entire document structure upfront

The abandoned `new-readers` branch provides valuable insights, showing a partial implementation of an AAMP reader with zero-copy semantics and lazy parsing.

## Goals

1. **Performance**: Provide zero-copy, minimal-allocation readers for high-performance read-only scenarios
2. **Compatibility**: Maintain existing API in current locations with identical behavior
3. **Consistency**: New reader APIs should feel familiar to users of the existing APIs
4. **Flexibility**: Allow gradual migration and mixed usage patterns
5. **Feature Parity**: Support all existing functionality through appropriate API design
6. **YAML Serialization**: Support serializing to YAML directly from reader API, without first parsing owned structures

## Proposed Architecture

### Module Organization

```
src/
├── byml/
│   ├── mod.rs           # Existing owned API (Byml enum, etc.)
│   ├── parser.rs        # Existing parser
│   ├── writer.rs        # Existing writer
│   ├── text.rs          # Existing YAML support
│   └── reader/          # NEW: Zero-copy reader API
│       ├── mod.rs       # Reader types and traits
│       ├── document.rs  # BymlReader (document root)
│       ├── node.rs      # BymlNodeReader (individual nodes)
│       └── iterators.rs # Iterator implementations
├── aamp/
│   ├── mod.rs           # Existing owned API (ParameterIO, etc.)
│   ├── parser.rs        # Existing parser
│   ├── writer.rs        # Existing writer
│   ├── text.rs          # Existing YAML support
│   └── reader/          # NEW: Zero-copy reader API (extend new-readers work)
│       ├── mod.rs       # Reader types and traits
│       ├── document.rs  # ParameterIOReader
│       ├── list.rs      # ParameterListReader
│       ├── object.rs    # ParameterObjectReader
│       └── iterators.rs # Iterator implementations
```

### Feature Configuration

Update `Cargo.toml` to support reader-specific features:

```toml
[features]
# Existing features
aamp = ["almost", "binrw", "indexmap", "num-traits"]
byml = ["binrw", "almost", "num-traits"]

# New reader features (optional for users who only want readers)
byml-reader = ["binrw"]  # Minimal deps for reader-only usage
aamp-reader = ["binrw"]  # Minimal deps for reader-only usage

# Convenience features
readers = ["byml-reader", "aamp-reader"]
default = ["aamp", "byml", "sarc", "yaz0"]  # Unchanged
```

## BYML Reader API Design

### Core Types

#### `BymlReader<'a>`
The main document reader, analogous to the owned `Byml` type but with zero-copy semantics.

```rust
pub struct BymlReader<'a> {
    data: &'a [u8],
    header: BymlHeader,
    string_table: &'a [u8],
    hash_key_table: Option<&'a [u8]>,
    root_offset: u32,
}

impl<'a> BymlReader<'a> {
    /// Create a new BYML reader from binary data
    pub fn new(data: &'a [u8]) -> Result<Self>;
    
    /// Get the version of the BYML format
    pub fn version(&self) -> u16;
    
    /// Get the root node of the document
    pub fn root(&self) -> BymlNodeReader<'a>;
    
    /// Get endianness of the document
    pub fn endian(&self) -> Endian;
}
```

#### `BymlNodeReader<'a>`
Individual node reader that can represent any BYML node type with zero-copy access.

```rust
pub struct BymlNodeReader<'a> {
    reader: &'a BymlReader<'a>,
    node_type: NodeType,
    value_data: &'a [u8],  // Points to the node's value data
    offset: u32,           // Offset in original data for container types
}

impl<'a> BymlNodeReader<'a> {
    /// Get the type of this node
    pub fn node_type(&self) -> NodeType;
    
    /// Check if this node is null
    pub fn is_null(&self) -> bool;
    
    // Type-specific accessors (zero-copy where possible)
    pub fn as_bool(&self) -> Result<bool>;
    pub fn as_i32(&self) -> Result<i32>;
    pub fn as_u32(&self) -> Result<u32>;
    pub fn as_i64(&self) -> Result<i64>;
    pub fn as_u64(&self) -> Result<u64>;
    pub fn as_f32(&self) -> Result<f32>;
    pub fn as_f64(&self) -> Result<f64>;
    pub fn as_str(&self) -> Result<&'a str>;        // Zero-copy string access
    pub fn as_binary(&self) -> Result<&'a [u8]>;    // Zero-copy binary access
    
    // Container accessors
    pub fn as_array(&self) -> Result<BymlArrayReader<'a>>;
    pub fn as_map(&self) -> Result<BymlMapReader<'a>>;
    pub fn as_hash_map(&self) -> Result<BymlHashMapReader<'a>>;
    
    // Indexing support (similar to existing API)
    pub fn get<I: BymlIndex>(&self, index: I) -> Option<BymlNodeReader<'a>>;
    pub fn try_get<I: BymlIndex>(&self, index: I) -> Result<Option<BymlNodeReader<'a>>>;

    pub fn to_text(&self) -> Result<String>;
}
```

#### Container Readers

```rust
pub struct BymlArrayReader<'a> {
    reader: &'a BymlReader<'a>,
    node_types: &'a [u8],
    values_offset: u32,
    len: u32,
}

impl<'a> BymlArrayReader<'a> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, index: usize) -> Option<BymlNodeReader<'a>>;
    pub fn try_get(&self, index: usize) -> Result<Option<BymlNodeReader<'a>>>;
    pub fn iter(&self) -> BymlArrayIterator<'a>;
}

pub struct BymlMapReader<'a> {
    reader: &'a BymlReader<'a>,
    keys_offset: u32,
    values_offset: u32,
    len: u32,
}

impl<'a> BymlMapReader<'a> {
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn get(&self, key: &str) -> Option<BymlNodeReader<'a>>;
    pub fn try_get(&self, key: &str) -> Result<Option<BymlNodeReader<'a>>>;
    pub fn contains_key(&self, key: &str) -> bool;
    pub fn keys(&self) -> BymlMapKeysIterator<'a>;
    pub fn values(&self) -> BymlMapValuesIterator<'a>;
    pub fn iter(&self) -> BymlMapIterator<'a>;
}

// Similar for BymlHashMapReader
```

### Implementation Strategy

#### Zero-Copy String Access
- Parse string table once during reader construction
- Store references to string table sections
- Resolve string indices to `&str` slices directly from the table

#### Lazy Container Parsing
- Container types store only offset and metadata initially
- Parse individual elements on-demand during iteration/access
- Cache frequently accessed structural information

#### Memory Safety
- All lifetimes tied to the original data slice
- Bounds checking on all offset-based access
- Validate offsets during construction and access

## AAMP Reader API Design

Building on the existing work in the `new-readers` branch:

### Core Types

#### `ParameterIOReader<'a>`
```rust
pub struct ParameterIOReader<'a> {
    data: &'a [u8],
    header: AampHeader,
    root: ParameterListReader<'a>,
}

impl<'a> ParameterIOReader<'a> {
    /// Create a new AAMP reader from binary data
    pub fn new(data: &'a [u8]) -> Result<Self>;
    
    /// Get the document type (usually "xml")
    pub fn doc_type(&self) -> Result<&'a str>;
    
    /// Get the data version
    pub fn version(&self) -> u32;
    
    /// Get the root parameter list
    pub fn root(&self) -> &ParameterListReader<'a>;
    
    // Convenience methods for root access
    pub fn list(&self, name: impl Into<Name>) -> Option<ParameterListReader<'a>>;
    pub fn object(&self, name: impl Into<Name>) -> Option<ParameterObjectReader<'a>>;

    pub fn to_text(&self) -> Result<String>;
}
```

#### Enhanced from new-readers work

Extend the existing `ParameterListReader`, `ParameterObjectReader` from the new-readers branch with:

```rust
impl<'a> ParameterListReader<'a> {
    // Additional convenience methods
    pub fn get_list(&self, name: impl Into<Name>) -> Option<ParameterListReader<'a>>;
    pub fn get_object(&self, name: impl Into<Name>) -> Option<ParameterObjectReader<'a>>;
    
    // Enhanced iteration
    pub fn lists(&self) -> impl Iterator<Item = (Name, ParameterListReader<'a>)>;
    pub fn objects(&self) -> impl Iterator<Item = (Name, ParameterObjectReader<'a>)>;
}

impl<'a> ParameterObjectReader<'a> {
    // Type-safe parameter access with zero-copy for binary data
    pub fn get_str(&self, name: impl Into<Name>) -> Option<&'a str>;
    pub fn get_binary(&self, name: impl Into<Name>) -> Option<&'a [u8]>;
    pub fn get_bool(&self, name: impl Into<Name>) -> Option<bool>;
    // ... other primitive getters
    
    // Enhanced iteration  
    pub fn iter(&self) -> impl Iterator<Item = (Name, ParameterReader<'a>)>;
}
```

### Parameter Value Reader

```rust
pub enum ParameterReader<'a> {
    Bool(bool),
    F32(f32),
    I32(i32),
    // ... other primitive types
    String(&'a str),      // Zero-copy string access
    Binary(&'a [u8]),     // Zero-copy binary access  
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Color([f32; 4]),
    Curve([CurvePoint; 30]),
    BufferInt(&'a [i32]), // Zero-copy buffer access
    BufferF32(&'a [f32]), // Zero-copy buffer access
    BufferU8(&'a [u8]),   // Zero-copy buffer access
    StringRef(&'a str),   // Zero-copy string reference
    StringRefTable(&'a [&'a str]), // Zero-copy string reference table
}
```

## API Integration and Compatibility

### Module Exports

Update the main module files to expose both APIs:

```rust
// src/byml/mod.rs
pub use self::reader::{BymlReader, BymlNodeReader, BymlArrayReader, BymlMapReader};

// src/aamp/mod.rs  
pub use self::reader::{ParameterIOReader, ParameterListReader, ParameterObjectReader, ParameterReader};

// src/lib.rs
#[cfg(feature = "byml")]
pub mod byml {
    pub use super::byml::*;
}

#[cfg(feature = "aamp")]
pub mod aamp {
    pub use super::aamp::*;
}
```

### Conversion Between APIs

Provide conversion utilities for users who need both approaches:

```rust
impl<'a> BymlNodeReader<'a> {
    /// Convert this reader node to an owned Byml (allocates)
    pub fn to_owned(&self) -> Result<Byml>;
}

impl<'a> ParameterIOReader<'a> {
    /// Convert this reader to an owned ParameterIO (allocates)
    pub fn to_owned(&self) -> Result<ParameterIO>;
}

impl Byml {
    /// Create a reader from owned data (zero-copy via serialization)
    pub fn as_reader(&self) -> Result<BymlNodeReader<'_>>;
}
```

## Implementation Details

### Memory Layout Assumptions

The reader APIs assume standard Nintendo binary format layout:
- Little-endian for Switch, big-endian for Wii U (BYML only; AAMP is always little-endian)
- 4-byte alignment for most structures  
- String tables with null-terminated UTF-8 strings
- Relative offsets calculated from specific base addresses

### Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum ReaderError {
    #[error("Invalid binary format: {0}")]
    InvalidFormat(String),
    #[error("Unexpected end of data at offset {0:#x}")]
    UnexpectedEnd(u32),
    #[error("Invalid offset: {0:#x}")]
    InvalidOffset(u32),
    #[error("Invalid node type: {0:?}")]
    InvalidNodeType(u8),
    #[error("String encoding error: {0}")]
    StringEncoding(#[from] std::str::Utf8Error),
}
```

### Performance Considerations

#### Optimization Strategies
1. **Minimal Validation**: Validate only what's immediately accessed
2. **Offset Caching**: Cache resolved offsets for repeated access
3. **SIMD Operations**: Use SIMD for bulk string/data operations where applicable
4. **Branch Prediction**: Optimize hot paths for common node types

#### Memory Usage
- Reader structs should be small (typically 1-4 pointers + metadata)
- No heap allocation during normal operation
- Temporary allocations only for error reporting

### Thread Safety

Reader APIs are `Send + Sync` when the underlying data is:
- All operations are read-only
- No internal mutability except for caching (use atomic operations)
- Multiple readers can safely access the same data concurrently

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_byml_reader_basic() {
        let data = include_bytes!("../../test/byml/A-1_Dynamic.byml");
        let reader = BymlReader::new(data).unwrap();
        let root = reader.root();
        
        // Test zero-copy string access
        assert_eq!(root.get("someKey").unwrap().as_str().unwrap(), "expectedValue");
        
        // Test container access  
        let array = root.get("arrayKey").unwrap().as_array().unwrap();
        assert_eq!(array.len(), 5);
    }
    
    #[test]
    fn test_conversion_consistency() {
        let data = include_bytes!("../../test/byml/A-1_Dynamic.byml");
        let owned = Byml::from_binary(data).unwrap();
        let reader = BymlReader::new(data).unwrap();
        
        // Ensure reader and owned APIs produce equivalent results
        assert_eq!(owned["key1"], reader.root().get("key1").unwrap().to_owned().unwrap());
    }
}
```

### Integration Tests

- Test all existing BYML/AAMP files in test suite with new reader APIs
- Test YAML serialization against existing YAML files in the test suite
- Verify performance improvements in benchmark suite
- Test error handling with malformed binary data
- Validate thread safety with concurrent access tests

### Benchmark Suite

```rust
#[cfg(test)]
mod benches {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    
    fn bench_byml_owned_vs_reader(c: &mut Criterion) {
        let data = include_bytes!("../../test/byml/large_file.byml");
        
        c.bench_function("byml_owned_parse", |b| {
            b.iter(|| {
                let owned = Byml::from_binary(black_box(data)).unwrap();
                black_box(owned);
            })
        });
        
        c.bench_function("byml_reader_parse", |b| {
            b.iter(|| {
                let reader = BymlReader::new(black_box(data)).unwrap();
                black_box(reader);
            })
        });
    }
}
```

## Documentation Requirements

### API Documentation

Each public type and method needs:
- Clear description of zero-copy semantics
- Lifetime requirements explanation
- Performance characteristics
- Usage examples
- Error conditions

### Migration Guide

Create `MIGRATION.md` documenting:
- When to use readers vs owned APIs
- Performance trade-offs
- Code examples showing equivalent operations
- Common migration patterns

### Examples

Create example programs showing:
- Basic reader usage for each format
- Performance-critical scenarios  
- Mixed usage patterns
- Conversion between APIs

## Timeline and Milestones

### Phase 1: BYML Reader Foundation (2-3 weeks)
- [ ] Implement basic `BymlReader` and `BymlNodeReader`
- [ ] Support primitive types with zero-copy semantics
- [ ] Basic array/map container support
- [ ] Unit tests for core functionality

### Phase 2: BYML Reader Polish (1-2 weeks)  
- [ ] Complete container implementations
- [ ] Iterator support
- [ ] Error handling improvements
- [ ] Performance optimization
- [ ] Add YAML serialization

### Phase 3: AAMP Reader Enhancement (2-3 weeks)
- [ ] Port and enhance new-readers branch work
- [ ] Implement missing functionality
- [ ] Zero-copy parameter access
- [ ] Add YAML serialization
- [ ] Complete test coverage

### Phase 4: Integration and Documentation (1-2 weeks)
- [ ] API integration and exports
- [ ] Conversion utilities between APIs
- [ ] Comprehensive documentation
- [ ] Migration guide
- [ ] Performance benchmarks

### Phase 5: Testing and Refinement (1 week)
- [ ] Integration testing with existing test suite
- [ ] Performance validation
- [ ] API refinement based on feedback
- [ ] Final documentation review

## Risks and Mitigation

### Risk: Complex Lifetime Management
**Mitigation**: Keep reader structs simple, use consistent lifetime patterns, comprehensive testing

### Risk: Performance Regression in Edge Cases  
**Mitigation**: Comprehensive benchmarking, profile-guided optimization, fallback strategies

### Risk: Binary Format Evolution
**Mitigation**: Version-aware parsing, graceful degradation, extensive validation

### Risk: Memory Safety Issues
**Mitigation**: Comprehensive bounds checking, careful offset validation, extensive testing with malformed data

## Success Criteria

1. **Performance**: Reader APIs should be 2-5x faster than owned APIs for read-only workloads
2. **Memory**: Reader APIs should use 10-50x less memory than owned APIs
3. **Compatibility**: Existing tests pass unchanged with owned APIs
4. **Usability**: Reader APIs feel natural to existing users
5. **Coverage**: All existing functionality available through reader APIs

## Future Considerations

### Potential Extensions
- Streaming/incremental parsing for very large files
- Memory-mapped file support
- WASM compatibility optimizations
- Custom allocator support for owned API

### API Evolution
- Consider stabilization timeline for reader APIs
- Feedback collection and iteration process
- Potential API breaking changes in major versions

---

This roadmap provides a comprehensive plan for implementing high-performance reader APIs while maintaining the existing functionality and user experience. The phased approach allows for iterative development and validation, ensuring both performance goals and compatibility requirements are met.
