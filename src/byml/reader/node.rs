//! BYML node reader implementation
//!
//! Provides zero-copy access to individual BYML nodes and containers.

use super::{BymlReader, ReaderError, ReaderResult};
use crate::{Endian, byml::NodeType};

/// Zero-copy reader for individual BYML nodes
///
/// This struct provides access to BYML node data without copying. For primitive
/// types, data is read directly from the binary. For strings and binary data,
/// references to the original data are returned.
pub struct BymlNodeReader<'a> {
    /// Reference to the parent reader
    reader: &'a BymlReader<'a>,
    /// Type of this node
    node_type: NodeType,
    /// Raw value data for this node (8 bytes for inline values, offset for others)
    value_data: [u8; 8],
    /// Offset in original data (for container types)
    #[allow(dead_code)]
    offset: u32,
}

impl<'a> BymlNodeReader<'a> {
    /// Create a new node reader for the root node
    pub(crate) fn new_root(reader: &'a BymlReader<'a>, offset: u32) -> Self {
        // For root node, we need to read the node type and value from the offset
        let data = reader.data();
        let offset_usize = offset as usize;

        if offset_usize >= data.len() {
            // Return a null node for invalid offset
            return Self {
                reader,
                node_type: NodeType::Null,
                value_data: [0; 8],
                offset: 0,
            };
        }

        // Read node type
        let node_type_byte = data[offset_usize];
        let node_type = match NodeType::try_from(node_type_byte) {
            Ok(nt) => nt,
            Err(_) => NodeType::Null,
        };

        // For root node, the value starts right after the type byte
        let mut value_data = [0u8; 8];
        let value_start = offset_usize + 1;
        if value_start + 8 <= data.len() {
            value_data.copy_from_slice(&data[value_start..value_start + 8]);
        }

        Self {
            reader,
            node_type,
            value_data,
            offset,
        }
    }

    /// Create a new node reader from explicit parameters
    pub(crate) fn new(
        reader: &'a BymlReader<'a>,
        node_type: NodeType,
        value_data: [u8; 8],
        offset: u32,
    ) -> Self {
        Self {
            reader,
            node_type,
            value_data,
            offset,
        }
    }

    /// Get the type of this node
    pub fn node_type(&self) -> NodeType {
        self.node_type
    }

    /// Check if this node is null
    pub fn is_null(&self) -> bool {
        matches!(self.node_type, NodeType::Null)
    }

    /// Get the node value as a boolean
    pub fn as_bool(&self) -> ReaderResult<bool> {
        match self.node_type {
            NodeType::Bool => Ok(self.value_data[0] != 0),
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as an i32
    pub fn as_i32(&self) -> ReaderResult<i32> {
        match self.node_type {
            NodeType::I32 => {
                let bytes = [
                    self.value_data[0],
                    self.value_data[1],
                    self.value_data[2],
                    self.value_data[3],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => i32::from_le_bytes(bytes),
                    Endian::Big => i32::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as a u32
    pub fn as_u32(&self) -> ReaderResult<u32> {
        match self.node_type {
            NodeType::U32 => {
                let bytes = [
                    self.value_data[0],
                    self.value_data[1],
                    self.value_data[2],
                    self.value_data[3],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes(bytes),
                    Endian::Big => u32::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as an i64
    pub fn as_i64(&self) -> ReaderResult<i64> {
        match self.node_type {
            NodeType::I64 => {
                // i64 values are stored as offsets to 8-byte values
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                } as usize;

                let data = self.reader.data();
                if offset + 8 > data.len() {
                    return Err(ReaderError::UnexpectedEnd(offset as u32));
                }

                let bytes = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => i64::from_le_bytes(bytes),
                    Endian::Big => i64::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as a u64
    pub fn as_u64(&self) -> ReaderResult<u64> {
        match self.node_type {
            NodeType::U64 => {
                // u64 values are stored as offsets to 8-byte values
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                } as usize;

                let data = self.reader.data();
                if offset + 8 > data.len() {
                    return Err(ReaderError::UnexpectedEnd(offset as u32));
                }

                let bytes = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => u64::from_le_bytes(bytes),
                    Endian::Big => u64::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as an f32
    pub fn as_f32(&self) -> ReaderResult<f32> {
        match self.node_type {
            NodeType::Float => {
                let bytes = [
                    self.value_data[0],
                    self.value_data[1],
                    self.value_data[2],
                    self.value_data[3],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => f32::from_le_bytes(bytes),
                    Endian::Big => f32::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as an f64
    pub fn as_f64(&self) -> ReaderResult<f64> {
        match self.node_type {
            NodeType::Double => {
                // f64 values are stored as offsets to 8-byte values
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                } as usize;

                let data = self.reader.data();
                if offset + 8 > data.len() {
                    return Err(ReaderError::UnexpectedEnd(offset as u32));
                }

                let bytes = [
                    data[offset],
                    data[offset + 1],
                    data[offset + 2],
                    data[offset + 3],
                    data[offset + 4],
                    data[offset + 5],
                    data[offset + 6],
                    data[offset + 7],
                ];
                Ok(match self.reader.endian_internal() {
                    Endian::Little => f64::from_le_bytes(bytes),
                    Endian::Big => f64::from_be_bytes(bytes),
                })
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as a string (zero-copy)
    pub fn as_str(&self) -> ReaderResult<&'a str> {
        match self.node_type {
            NodeType::String => {
                let string_index = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                };
                self.reader.get_string(string_index)
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node value as binary data (zero-copy)
    pub fn as_binary(&self) -> ReaderResult<&'a [u8]> {
        match self.node_type {
            NodeType::Binary | NodeType::File => {
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                } as usize;

                let data = self.reader.data();
                if offset + 4 > data.len() {
                    return Err(ReaderError::UnexpectedEnd(offset as u32));
                }

                // Binary data starts with a length field
                let length = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ]),
                } as usize;

                if offset + 4 + length > data.len() {
                    return Err(ReaderError::UnexpectedEnd((offset + 4 + length) as u32));
                }

                Ok(&data[offset + 4..offset + 4 + length])
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node as an array reader
    pub fn as_array(&self) -> ReaderResult<BymlArrayReader<'a>> {
        match self.node_type {
            NodeType::Array => {
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                };
                BymlArrayReader::new(self.reader, offset)
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node as a map reader
    pub fn as_map(&self) -> ReaderResult<BymlMapReader<'a>> {
        match self.node_type {
            NodeType::Map => {
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                };
                BymlMapReader::new(self.reader, offset)
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }

    /// Get the node as a hash map reader
    pub fn as_hash_map(&self) -> ReaderResult<BymlHashMapReader<'a>> {
        match self.node_type {
            NodeType::HashMap | NodeType::ValueHashMap => {
                let offset = match self.reader.endian_internal() {
                    Endian::Little => u32::from_le_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                    Endian::Big => u32::from_be_bytes([
                        self.value_data[0],
                        self.value_data[1],
                        self.value_data[2],
                        self.value_data[3],
                    ]),
                };
                BymlHashMapReader::new(self.reader, offset, self.node_type)
            }
            _ => Err(ReaderError::InvalidNodeType(self.node_type as u8)),
        }
    }
}

/// Zero-copy reader for BYML arrays
pub struct BymlArrayReader<'a> {
    reader: &'a BymlReader<'a>,
    node_types: &'a [u8],
    values_offset: u32,
    len: u32,
}

impl<'a> BymlArrayReader<'a> {
    pub(crate) fn new(reader: &'a BymlReader<'a>, offset: u32) -> ReaderResult<Self> {
        let data = reader.data();
        let offset_usize = offset as usize;

        if offset_usize + 4 > data.len() {
            return Err(ReaderError::UnexpectedEnd(offset));
        }

        // Read node type (should be Array = 0xc0) and length (u24)
        let _node_type = data[offset_usize];
        let len = match reader.endian_internal() {
            Endian::Little => u32::from_le_bytes([
                data[offset_usize + 1],
                data[offset_usize + 2],
                data[offset_usize + 3],
                0,
            ]),
            Endian::Big => u32::from_be_bytes([
                0,
                data[offset_usize + 1],
                data[offset_usize + 2],
                data[offset_usize + 3],
            ]),
        };

        // Node types follow immediately after the header
        let node_types_start = offset_usize + 4;
        let node_types_end = node_types_start + len as usize;

        if node_types_end > data.len() {
            return Err(ReaderError::UnexpectedEnd(node_types_end as u32));
        }

        let node_types = &data[node_types_start..node_types_end];

        // Values start after node types, aligned to 4 bytes
        let values_offset = ((node_types_end + 3) & !3) as u32;

        Ok(BymlArrayReader {
            reader,
            node_types,
            values_offset,
            len,
        })
    }

    /// Get the length of the array
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if the array is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get an element at the given index
    pub fn get(&self, index: usize) -> Option<BymlNodeReader<'a>> {
        if index >= self.len as usize {
            return None;
        }

        let node_type_byte = self.node_types[index];
        let node_type = NodeType::try_from(node_type_byte).ok()?;

        // Calculate value offset for this element
        let value_offset = self.values_offset + (index as u32 * 8);
        let data = self.reader.data();
        let value_offset_usize = value_offset as usize;

        if value_offset_usize + 8 > data.len() {
            return None;
        }

        let mut value_data = [0u8; 8];
        value_data.copy_from_slice(&data[value_offset_usize..value_offset_usize + 8]);

        Some(BymlNodeReader::new(
            self.reader,
            node_type,
            value_data,
            value_offset,
        ))
    }

    /// Try to get an element at the given index with error handling
    pub fn try_get(&self, index: usize) -> ReaderResult<Option<BymlNodeReader<'a>>> {
        Ok(self.get(index))
    }
}

/// Zero-copy reader for BYML maps (string-keyed)
pub struct BymlMapReader<'a> {
    reader: &'a BymlReader<'a>,
    keys_offset: u32,
    values_offset: u32,
    len: u32,
}

impl<'a> BymlMapReader<'a> {
    pub(crate) fn new(reader: &'a BymlReader<'a>, offset: u32) -> ReaderResult<Self> {
        let data = reader.data();
        let offset_usize = offset as usize;

        if offset_usize + 4 > data.len() {
            return Err(ReaderError::UnexpectedEnd(offset));
        }

        // Read node type (should be Map = 0xc1) and length (u24)
        let _node_type = data[offset_usize];
        let len = match reader.endian_internal() {
            Endian::Little => u32::from_le_bytes([
                data[offset_usize + 1],
                data[offset_usize + 2], 
                data[offset_usize + 3],
                0,
            ]),
            Endian::Big => u32::from_be_bytes([
                0,
                data[offset_usize + 1],
                data[offset_usize + 2],
                data[offset_usize + 3],
            ]),
        };

        // Keys start immediately after the header
        let keys_offset = offset + 4;

        // Values start after keys (each key entry is 8 bytes: 3 bytes string index + 1 byte node type + 4 bytes value)
        let values_offset = keys_offset;

        Ok(BymlMapReader {
            reader,
            keys_offset,
            values_offset,
            len,
        })
    }

    /// Get the length of the map
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if the map is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a value by string key
    pub fn get(&self, key: &str) -> Option<BymlNodeReader<'a>> {
        // Linear search through keys (could be optimized with binary search)
        for i in 0..self.len {
            if let Ok(entry_key) = self.get_key_at_index(i as usize) {
                if entry_key == key {
                    return self.get_value_at_index(i as usize);
                }
            }
        }
        None
    }

    /// Try to get a value by string key with error handling
    pub fn try_get(&self, key: &str) -> ReaderResult<Option<BymlNodeReader<'a>>> {
        Ok(self.get(key))
    }

    /// Check if the map contains a key
    pub fn contains_key(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// Get the key at a given index
    pub(crate) fn get_key_at_index(&self, index: usize) -> ReaderResult<&'a str> {
        if index >= self.len as usize {
            return Err(ReaderError::InvalidFormat(
                "Index out of bounds".to_string(),
            ));
        }

        let data = self.reader.data();
        // Each entry is 8 bytes: 3 bytes string index + 1 byte node type + 4 bytes value
        let entry_offset = (self.keys_offset + index as u32 * 8) as usize;

        if entry_offset + 8 > data.len() {
            return Err(ReaderError::UnexpectedEnd(entry_offset as u32));
        }

        // String index is in first 3 bytes
        let string_index = match self.reader.endian_internal() {
            Endian::Little => u32::from_le_bytes([
                data[entry_offset],
                data[entry_offset + 1],
                data[entry_offset + 2],
                0,
            ]),
            Endian::Big => u32::from_be_bytes([
                0,
                data[entry_offset],
                data[entry_offset + 1], 
                data[entry_offset + 2],
            ]),
        };
        self.reader.get_string(string_index)
    }

    /// Get the value at a given index
    pub(crate) fn get_value_at_index(&self, index: usize) -> Option<BymlNodeReader<'a>> {
        if index >= self.len as usize {
            return None;
        }

        let data = self.reader.data();

        // Each entry is 8 bytes: 3 bytes string index + 1 byte node type + 4 bytes value
        let entry_offset = (self.keys_offset + index as u32 * 8) as usize;
        if entry_offset + 8 > data.len() {
            return None;
        }

        // Node type is at byte 3
        let node_type_byte = data[entry_offset + 3];
        let node_type = NodeType::try_from(node_type_byte).ok()?;

        // Value data is in bytes 4-7
        let mut value_data = [0u8; 8];
        value_data[0..4].copy_from_slice(&data[entry_offset + 4..entry_offset + 8]);

        Some(BymlNodeReader::new(
            self.reader,
            node_type,
            value_data,
            entry_offset as u32 + 4,
        ))
    }
}

/// Zero-copy reader for BYML hash maps (u32-keyed)
#[allow(dead_code)]
pub struct BymlHashMapReader<'a> {
    reader: &'a BymlReader<'a>,
    offset: u32,
    len: u32,
    is_value_hash_map: bool,
}

impl<'a> BymlHashMapReader<'a> {
    pub(crate) fn new(
        reader: &'a BymlReader<'a>,
        offset: u32,
        node_type: NodeType,
    ) -> ReaderResult<Self> {
        let data = reader.data();
        let offset_usize = offset as usize;

        if offset_usize + 4 > data.len() {
            return Err(ReaderError::UnexpectedEnd(offset));
        }

        // Read hash map length
        let len = u32::from_le_bytes([
            data[offset_usize],
            data[offset_usize + 1],
            data[offset_usize + 2],
            0,
        ]);

        let is_value_hash_map = matches!(node_type, NodeType::ValueHashMap);

        Ok(BymlHashMapReader {
            reader,
            offset,
            len,
            is_value_hash_map,
        })
    }

    /// Get the length of the hash map
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Check if the hash map is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get a value by hash key
    pub fn get(&self, _key: u32) -> Option<BymlNodeReader<'a>> {
        // Implementation would need binary search through hash entries
        // This is a simplified version
        None
    }

    /// Try to get a value by hash key with error handling
    pub fn try_get(&self, key: u32) -> ReaderResult<Option<BymlNodeReader<'a>>> {
        Ok(self.get(key))
    }

    /// Check if the hash map contains a key
    pub fn contains_key(&self, key: u32) -> bool {
        self.get(key).is_some()
    }
}
