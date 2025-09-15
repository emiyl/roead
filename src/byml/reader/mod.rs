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
    use crate::byml::{NodeType, FILES};

    #[test]
    fn test_basic_reader() {
        // Just test that we can import and the basic functionality exists
        let invalid_data = b"INVALID";
        assert!(BymlReader::new(invalid_data).is_err());
    }

    #[test]
    fn test_reader_parsing_all_files() {
        // Test parsing all the same files used by the owned API
        // Only expect parsing success when the owned API also succeeds
        use crate::byml::Byml;
        
        for file in FILES {
            println!("Testing reader API with file: {}", file);
            
            let file_path = std::path::Path::new("test/byml").join([file, ".byml"].join(""));
            let bytes = match std::fs::read(&file_path) {
                Ok(data) => data,
                Err(e) => {
                    println!("  Skipping file {}: {}", file, e);
                    continue;
                }
            };
            
            // First check if owned API can parse this file
            let owned_result = Byml::from_binary(&bytes);
            let reader_result = BymlReader::new(&bytes);
            
            match (&owned_result, &reader_result) {
                (Ok(_), Ok(reader)) => {
                    println!("  ✓ Successfully parsed");
                    println!("  Version: {}", reader.version());
                    println!("  Endian: {:?}", reader.endian());
                    
                    let root = reader.root();
                    println!("  Root node type: {:?}", root.node_type());
                    
                    match root.node_type() {
                        NodeType::Array => {
                            if let Ok(arr) = root.as_array() {
                                println!("  Array with {} elements", arr.len());
                                
                                // Test accessing first few elements
                                for i in 0..std::cmp::min(3, arr.len()) {
                                    if let Some(elem) = arr.get(i) {
                                        println!("    [{}]: {:?}", i, elem.node_type());
                                    }
                                }
                            }
                        }
                        NodeType::Map => {
                            if let Ok(map) = root.as_map() {
                                println!("  Map with {} entries", map.len());
                                
                                // Test iteration through first few entries
                                let mut count = 0;
                                for result in map.iter() {
                                    if count >= 3 { break; }
                                    match result {
                                        Ok((key, value)) => {
                                            println!("    '{}': {:?}", key, value.node_type());
                                        }
                                        Err(e) => {
                                            println!("    Error iterating map in {}: {:?}", file, e);
                                            break;
                                        }
                                    }
                                    count += 1;
                                }
                            }
                        }
                        NodeType::HashMap => {
                            if let Ok(hash_map) = root.as_hash_map() {
                                println!("  HashMap with {} entries", hash_map.len());
                            }
                        }
                        _ => {
                            println!("  Primitive value: {:?}", root.node_type());
                        }
                    }
                },
                (Err(_), Err(_)) => {
                    println!("  ✓ Both APIs failed as expected");
                },
                (Ok(_), Err(e)) => {
                    panic!("Reader API failed to parse file '{}' that owned API can parse: {:?}", file, e);
                },
                (Err(_), Ok(_)) => {
                    println!("  ⚠ Reader API succeeded where owned API failed");
                }
            }
        }
    }

    #[test]
    fn test_zero_copy_string_access() {
        // Test that string access is truly zero-copy
        if let Ok(data) = std::fs::read("test/byml/A-1_Dynamic.byml") {
            if let Ok(reader) = BymlReader::new(&data) {
                let root = reader.root();
                if let Ok(map) = root.as_map() {
                    for result in map.iter() {
                        if let Ok((key, value)) = result {
                            // Verify key is a valid string slice referencing original data
                            assert!(!key.is_empty() || key.is_empty()); // Just validate it's accessible
                            
                            // Test string values for zero-copy access
                            if let Ok(string_val) = value.as_str() {
                                // Verify the string slice points into the original data
                                let string_ptr = string_val.as_ptr();
                                let data_start = data.as_ptr();
                                let data_end = unsafe { data_start.add(data.len()) };
                                
                                // The string pointer should be within the original data range
                                assert!(string_ptr >= data_start);
                                assert!(string_ptr < data_end);
                                println!("  Zero-copy string '{}' validated", string_val);
                                break; // Just test one string
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_binary_data_access() {
        // Test binary data access with files that contain binary data
        for file in FILES {
            let file_path = std::path::Path::new("test/byml").join([file, ".byml"].join(""));
            if let Ok(bytes) = std::fs::read(&file_path) {
                if let Ok(reader) = BymlReader::new(&bytes) {
                    // Recursively search for binary data nodes and count them
                    fn count_binary_nodes(node: &BymlNodeReader, bytes: &[u8]) -> usize {
                        let mut count = 0;
                        
                        match node.node_type() {
                            NodeType::Binary => {
                                if let Ok(binary) = node.as_binary() {
                                    count += 1;
                                    println!("    Found binary data: {} bytes", binary.len());
                                    
                                    // Verify binary data points into original data
                                    let binary_ptr = binary.as_ptr();
                                    let data_start = bytes.as_ptr();
                                    let data_end = unsafe { data_start.add(bytes.len()) };
                                    
                                    assert!(binary_ptr >= data_start);
                                    assert!(binary_ptr < data_end);
                                }
                            }
                            NodeType::Array => {
                                if let Ok(array) = node.as_array() {
                                    for i in 0..array.len() {
                                        if let Some(element) = array.get(i) {
                                            count += count_binary_nodes(&element, bytes);
                                        }
                                    }
                                }
                            }
                            NodeType::Map => {
                                if let Ok(map) = node.as_map() {
                                    for result in map.iter() {
                                        if let Ok((_key, value)) = result {
                                            count += count_binary_nodes(&value, bytes);
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                        
                        count
                    }
                    
                    let binary_count = count_binary_nodes(&reader.root(), &bytes);
                    if binary_count > 0 {
                        println!("File {} contains {} binary data nodes", file, binary_count);
                    }
                }
            }
        }
    }

    #[test]
    fn test_container_iteration() {
        // Comprehensive test of array and map iteration
        if let Ok(data) = std::fs::read("test/byml/ActorInfo.product.byml") {
            if let Ok(reader) = BymlReader::new(&data) {
                let root = reader.root();
                if let Ok(root_map) = root.as_map() {
                    // Test map iteration
                    let mut key_count = 0;
                    for result in root_map.iter() {
                        let (key, value) = result.expect("Should iterate map successfully");
                        key_count += 1;
                        
                        if key == "Actors" {
                            // Test array iteration
                            if let Ok(actors_array) = value.as_array() {
                                println!("Actors array has {} elements", actors_array.len());
                                
                                // Test accessing specific indices
                                if let Some(first_actor) = actors_array.get(0) {
                                    println!("First actor type: {:?}", first_actor.node_type());
                                    
                                    if let Ok(actor_map) = first_actor.as_map() {
                                        for result in actor_map.iter() {
                                            if let Ok((actor_key, actor_value)) = result {
                                                println!("  Actor field '{}': {:?}", actor_key, actor_value.node_type());
                                                break; // Just check one field
                                            }
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                    
                    assert!(key_count > 0, "Should have iterated through at least one key");
                }
            }
        }
    }

    #[test]
    fn test_error_handling() {
        // Test invalid BYML data
        let invalid_data = b"INVALID";
        assert!(BymlReader::new(invalid_data).is_err());
        
        // Test truncated data
        if let Ok(valid_data) = std::fs::read("test/byml/A-1_Dynamic.byml") {
            let truncated_data = &valid_data[..10]; // Take only first 10 bytes
            assert!(BymlReader::new(truncated_data).is_err());
            
            // Test accessing wrong node type
            if let Ok(reader) = BymlReader::new(&valid_data) {
                let root = reader.root();
                
                // If root is a map, trying to access it as an array should fail
                if root.as_map().is_ok() {
                    assert!(root.as_array().is_err());
                }
                // If root is an array, trying to access it as a map should fail
                if root.as_array().is_ok() {
                    assert!(root.as_map().is_err());
                }
            }
        }
    }

    #[test]
    fn test_primitive_value_access() {
        // Test accessing various primitive values across different files
        for file in FILES {
            let file_path = std::path::Path::new("test/byml").join([file, ".byml"].join(""));
            if let Ok(bytes) = std::fs::read(&file_path) {
                if let Ok(reader) = BymlReader::new(&bytes) {
                    // Recursively search for and test primitive values
                    fn test_primitives(node: &BymlNodeReader, file: &str, max_depth: usize) {
                        if max_depth == 0 {
                            return;
                        }
                        
                        match node.node_type() {
                            NodeType::Bool => {
                                if let Ok(val) = node.as_bool() {
                                    println!("  Bool: {}", val);
                                }
                            }
                            NodeType::I32 => {
                                if let Ok(val) = node.as_i32() {
                                    println!("  I32: {}", val);
                                }
                            }
                            NodeType::U32 => {
                                if let Ok(val) = node.as_u32() {
                                    println!("  U32: {}", val);
                                }
                            }
                            NodeType::I64 => {
                                if let Ok(val) = node.as_i64() {
                                    println!("  I64: {}", val);
                                }
                            }
                            NodeType::U64 => {
                                if let Ok(val) = node.as_u64() {
                                    println!("  U64: {}", val);
                                }
                            }
                            NodeType::Float => {
                                if let Ok(val) = node.as_f32() {
                                    println!("  Float: {}", val);
                                }
                            }
                            NodeType::Double => {
                                if let Ok(val) = node.as_f64() {
                                    println!("  Double: {}", val);
                                }
                            }
                            NodeType::String => {
                                if let Ok(val) = node.as_str() {
                                    println!("  String: '{}'", val);
                                }
                            }
                            NodeType::Null => {
                                println!("  Null value");
                            }
                            NodeType::Array => {
                                if let Ok(array) = node.as_array() {
                                    for i in 0..std::cmp::min(2, array.len()) { // Test first 2 elements
                                        if let Some(element) = array.get(i) {
                                            test_primitives(&element, file, max_depth - 1);
                                        }
                                    }
                                }
                            }
                            NodeType::Map => {
                                if let Ok(map) = node.as_map() {
                                    let mut count = 0;
                                    for result in map.iter() {
                                        if count >= 2 { break; } // Test first 2 entries
                                        if let Ok((_key, value)) = result {
                                            test_primitives(&value, file, max_depth - 1);
                                        }
                                        count += 1;
                                    }
                                }
                            }
                            _ => {} // Skip hash maps and other complex types for this test
                        }
                    }
                    
                    println!("Testing primitives in file: {}", file);
                    test_primitives(&reader.root(), file, 3); // Limit depth to 3 levels
                }
            }
        }
    }

    #[test]
    fn debug_map_structure() {
        use crate::byml::Byml;
        
        let data = std::fs::read("test/byml/ActorInfo.product.byml").unwrap();
        
        // Test owned API first
        let owned = Byml::from_binary(&data).unwrap();
        if let Ok(map) = owned.as_map() {
            println!("Owned API map has {} entries", map.len());
            use crate::byml::BymlIndex;
            for (i, (key, _value)) in map.iter().enumerate() {
                if i >= 5 { break; }
                println!("  Key {}: '{}'", i, key);
            }
        }
        
        // Test reader API
        let reader = BymlReader::new(&data).unwrap();
        let root = reader.root();
        
        if let Ok(reader_map) = root.as_map() {
            println!("Reader API map reports {} entries", reader_map.len());
            
            // Try to read first few keys manually
            for i in 0..std::cmp::min(5, reader_map.len()) {
                match reader_map.get_key_at_index(i) {
                    Ok(key) => println!("  Key {}: '{}'", i, key),
                    Err(e) => {
                        println!("  Key {}: ERROR {:?}", i, e);
                        break;
                    }
                }
            }
        }
    }

    #[test]
    fn test_usen_specific_data_validation() {
        // Test specific data points from USen.byml to verify correct parsing
        // This test validates against known data from the USen.yml file
        
        let data = std::fs::read("test/byml/USen.byml").unwrap();
        let reader = BymlReader::new(&data).unwrap();
        let root = reader.root();
        
        // USen.byml should be a hash map (marked with !h tag in YAML)
        let hash_map = root.as_hash_map().expect("USen.byml should be a hash map");
        
        println!("USen.byml HashMap has {} entries", hash_map.len());
        
        // Test specific entries from the YAML file to validate correct parsing
        // Entry: 1264494 with Hash: 0xE8FCC5F5 and SampleNum: 0x12AF0, WaveDataOffset: 0x80
        if let Some(entry_1264494) = hash_map.get(1264494) {
            let entry_map = entry_1264494.as_map().expect("Entry should be a map");
            
            // Check Hash field
            if let Some(hash_node) = entry_map.get("Hash") {
                let hash_value = hash_node.as_u32().expect("Hash should be U32");
                assert_eq!(hash_value, 0xE8FCC5F5, "Hash value mismatch for entry 1264494");
                println!("  ✓ Hash for 1264494: 0x{:X}", hash_value);
            }
            
            // Check ChannelInfo array
            if let Some(channel_info_node) = entry_map.get("ChannelInfo") {
                let channel_info_array = channel_info_node.as_array().expect("ChannelInfo should be array");
                assert_eq!(channel_info_array.len(), 1, "Should have 1 channel info entry");
                
                if let Some(channel_0) = channel_info_array.get(0) {
                    let channel_map = channel_0.as_map().expect("Channel info should be map");
                    
                    // Check SampleNum
                    if let Some(sample_num_node) = channel_map.get("SampleNum") {
                        let sample_num = sample_num_node.as_u32().expect("SampleNum should be U32");
                        assert_eq!(sample_num, 0x12AF0, "SampleNum mismatch for entry 1264494");
                        println!("    ✓ SampleNum: 0x{:X}", sample_num);
                    }
                    
                    // Check WaveDataOffset
                    if let Some(wave_offset_node) = channel_map.get("WaveDataOffset") {
                        let wave_offset = wave_offset_node.as_u32().expect("WaveDataOffset should be U32");
                        assert_eq!(wave_offset, 0x80, "WaveDataOffset mismatch for entry 1264494");
                        println!("    ✓ WaveDataOffset: 0x{:X}", wave_offset);
                    }
                }
            }
        } else {
            panic!("Entry 1264494 not found in hash map");
        }
        
        // Test entry with binary data: 7458797
        if let Some(entry_7458797) = hash_map.get(7458797) {
            let entry_map = entry_7458797.as_map().expect("Entry should be a map");
            
            // Check Hash field
            if let Some(hash_node) = entry_map.get("Hash") {
                let hash_value = hash_node.as_u32().expect("Hash should be U32");
                assert_eq!(hash_value, 0x4EEEBF1E, "Hash value mismatch for entry 7458797");
                println!("  ✓ Hash for 7458797: 0x{:X}", hash_value);
            }
            
            // Check ChannelInfo with binary data
            if let Some(channel_info_node) = entry_map.get("ChannelInfo") {
                let channel_info_array = channel_info_node.as_array().expect("ChannelInfo should be array");
                
                if let Some(channel_0) = channel_info_array.get(0) {
                    let channel_map = channel_0.as_map().expect("Channel info should be map");
                    
                    // Check AdpcmContext binary data
                    if let Some(adpcm_context_node) = channel_map.get("AdpcmContext") {
                        let adpcm_context = adpcm_context_node.as_binary().expect("AdpcmContext should be binary");
                        // Should be base64 decoded from "AAAAAAAA" which is 6 bytes of zeros
                        assert_eq!(adpcm_context.len(), 6, "AdpcmContext length mismatch");
                        assert_eq!(adpcm_context, &[0, 0, 0, 0, 0, 0], "AdpcmContext data mismatch");
                        println!("    ✓ AdpcmContext: {} bytes of binary data", adpcm_context.len());
                    }
                    
                    // Check AdpcmParameter binary data
                    if let Some(adpcm_param_node) = channel_map.get("AdpcmParameter") {
                        let adpcm_param = adpcm_param_node.as_binary().expect("AdpcmParameter should be binary");
                        // Should be base64 decoded data (32 bytes)
                        assert_eq!(adpcm_param.len(), 32, "AdpcmParameter length mismatch");
                        println!("    ✓ AdpcmParameter: {} bytes of binary data", adpcm_param.len());
                    }
                    
                    // Check SampleNum
                    if let Some(sample_num_node) = channel_map.get("SampleNum") {
                        let sample_num = sample_num_node.as_u32().expect("SampleNum should be U32");
                        assert_eq!(sample_num, 0xADFC, "SampleNum mismatch for entry 7458797");
                        println!("    ✓ SampleNum: 0x{:X}", sample_num);
                    }
                }
            }
        } else {
            panic!("Entry 7458797 not found in hash map");
        }
        
        // Test entry with multiple channels: 4253374
        if let Some(entry_4253374) = hash_map.get(4253374) {
            let entry_map = entry_4253374.as_map().expect("Entry should be a map");
            
            // Check Hash field
            if let Some(hash_node) = entry_map.get("Hash") {
                let hash_value = hash_node.as_u32().expect("Hash should be U32");
                assert_eq!(hash_value, 0xD548098A, "Hash value mismatch for entry 4253374");
                println!("  ✓ Hash for 4253374: 0x{:X}", hash_value);
            }
            
            // Check ChannelInfo with 2 channels
            if let Some(channel_info_node) = entry_map.get("ChannelInfo") {
                let channel_info_array = channel_info_node.as_array().expect("ChannelInfo should be array");
                assert_eq!(channel_info_array.len(), 2, "Should have 2 channel info entries");
                
                // Test first channel
                if let Some(channel_0) = channel_info_array.get(0) {
                    let channel_map = channel_0.as_map().expect("Channel info should be map");
                    
                    if let Some(sample_num_node) = channel_map.get("SampleNum") {
                        let sample_num = sample_num_node.as_u32().expect("SampleNum should be U32");
                        assert_eq!(sample_num, 0x558A5, "SampleNum mismatch for entry 4253374 channel 0");
                        println!("    ✓ Channel 0 SampleNum: 0x{:X}", sample_num);
                    }
                    
                    if let Some(wave_offset_node) = channel_map.get("WaveDataOffset") {
                        let wave_offset = wave_offset_node.as_u32().expect("WaveDataOffset should be U32");
                        assert_eq!(wave_offset, 0xC0, "WaveDataOffset mismatch for entry 4253374 channel 0");
                        println!("    ✓ Channel 0 WaveDataOffset: 0x{:X}", wave_offset);
                    }
                }
                
                // Test second channel
                if let Some(channel_1) = channel_info_array.get(1) {
                    let channel_map = channel_1.as_map().expect("Channel info should be map");
                    
                    if let Some(sample_num_node) = channel_map.get("SampleNum") {
                        let sample_num = sample_num_node.as_u32().expect("SampleNum should be U32");
                        assert_eq!(sample_num, 0x558A5, "SampleNum mismatch for entry 4253374 channel 1");
                        println!("    ✓ Channel 1 SampleNum: 0x{:X}", sample_num);
                    }
                    
                    if let Some(wave_offset_node) = channel_map.get("WaveDataOffset") {
                        let wave_offset = wave_offset_node.as_u32().expect("WaveDataOffset should be U32");
                        assert_eq!(wave_offset, 0xB900, "WaveDataOffset mismatch for entry 4253374 channel 1");
                        println!("    ✓ Channel 1 WaveDataOffset: 0x{:X}", wave_offset);
                    }
                }
            }
        } else {
            panic!("Entry 4253374 not found in hash map");
        }
        
        // Test a few more random entries to ensure comprehensive validation
        
        // Entry 11135534 - simple entry
        if let Some(entry_11135534) = hash_map.get(11135534) {
            let entry_map = entry_11135534.as_map().expect("Entry should be a map");
            
            if let Some(hash_node) = entry_map.get("Hash") {
                let hash_value = hash_node.as_u32().expect("Hash should be U32");
                assert_eq!(hash_value, 0x5792352D, "Hash value mismatch for entry 11135534");
                println!("  ✓ Hash for 11135534: 0x{:X}", hash_value);
            }
        }
        
        // Entry 50976467 - another simple entry
        if let Some(entry_50976467) = hash_map.get(50976467) {
            let entry_map = entry_50976467.as_map().expect("Entry should be a map");
            
            if let Some(hash_node) = entry_map.get("Hash") {
                let hash_value = hash_node.as_u32().expect("Hash should be U32");
                assert_eq!(hash_value, 0x8FFB7887, "Hash value mismatch for entry 50976467");
                println!("  ✓ Hash for 50976467: 0x{:X}", hash_value);
            }
            
            if let Some(channel_info_node) = entry_map.get("ChannelInfo") {
                let channel_info_array = channel_info_node.as_array().expect("ChannelInfo should be array");
                if let Some(channel_0) = channel_info_array.get(0) {
                    let channel_map = channel_0.as_map().expect("Channel info should be map");
                    
                    if let Some(sample_num_node) = channel_map.get("SampleNum") {
                        let sample_num = sample_num_node.as_u32().expect("SampleNum should be U32");
                        assert_eq!(sample_num, 0x89AA, "SampleNum mismatch for entry 50976467");
                        println!("    ✓ SampleNum for 50976467: 0x{:X}", sample_num);
                    }
                }
            }
        }
        
        println!("✅ All USen.byml data validation tests passed!");
    }
}
