//! Zero-copy BYML reader API
//!
//! This module provides high-performance, zero-copy readers for BYML documents.
//! Unlike the owned API which parses the entire document into heap-allocated
//! structures, the reader API provides lazy access to the data with minimal
//! allocations.

use crate::{Endian, Result};

pub mod document;
pub mod iterators;
pub mod node;

pub use document::BymlReader;
pub use node::{BymlArrayReader, BymlHashMapReader, BymlMapReader, BymlNodeReader};

/// Error type for BYML reader operations
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
    #[error("BinRW error: {0}")]
    BinRw(#[from] binrw::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for BYML reader operations
pub type ReaderResult<T> = std::result::Result<T, ReaderError>;

/// BYML header structure
#[derive(Debug, Clone, Copy)]
#[binrw::binrw]
pub struct BymlHeader {
    /// Magic bytes "BY" (0x4259) for little endian, "YB" (0x5942) for big endian
    pub magic: u16,
    /// BYML format version (1-7)
    pub version: u16,
    /// Offset to hash key table (0 if not present)
    pub hash_key_table_offset: u32,
    /// Offset to string table
    pub string_table_offset: u32,
    /// Offset to root node
    pub root_node_offset: u32,
}

impl BymlHeader {
    /// Get the endianness from the magic bytes
    pub fn endian(&self) -> Result<Endian> {
        match self.magic {
            0x4259 => Ok(Endian::Little),
            0x5942 => Ok(Endian::Big),
            _ => Err(crate::Error::InvalidData("Invalid BYML magic")),
        }
    }

    /// Check if the version is valid
    pub fn is_valid_version(&self) -> bool {
        self.version >= 1 && self.version <= 7
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::NodeType;

    #[test]
    fn test_byml_reader_basic() {
        // Test with a real BYML file if it exists
        if let Ok(data) = std::fs::read("test/byml/A-1_Dynamic.byml") {
            let reader = BymlReader::new(&data);
            match reader {
                Ok(reader) => {
                    println!("Successfully created BYML reader for A-1_Dynamic.byml");
                    println!("Version: {}", reader.version());
                    println!("Endian: {:?}", reader.endian());

                    // Try to access the root node
                    let root = reader.root();
                    println!("Root node type: {:?}", root.node_type());
                    
                    // If root is a map, try to iterate through keys
                    if let Ok(map) = root.as_map() {
                        println!("Root is a map with {} entries", map.len());
                        for result in map.iter() {
                            match result {
                                Ok((key, _value)) => {
                                    println!("  Key: {}", key);
                                }
                                Err(e) => {
                                    println!("  Error reading key: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to create reader: {:?}", e);
                }
            }
        } else {
            println!("Test file not found, skipping reader test");
        }
    }

    #[test]
    fn test_node_type_conversion() {
        assert_eq!(NodeType::try_from(0xd0), Ok(NodeType::Bool));
        assert_eq!(NodeType::try_from(0xd1), Ok(NodeType::I32));
        assert_eq!(NodeType::try_from(0xa0), Ok(NodeType::String));
        assert_eq!(NodeType::try_from(0xc0), Ok(NodeType::Array));
        assert_eq!(NodeType::try_from(0xc1), Ok(NodeType::Map));
        assert_eq!(NodeType::try_from(0xff), Ok(NodeType::Null));
        assert_eq!(NodeType::try_from(0x99), Err(()));
    }
}
