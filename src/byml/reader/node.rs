//! BYML node reader implementation
//!
//! Provides zero-copy access to individual BYML nodes and containers.

use super::{BymlReader, ReaderError, ReaderResult};
use crate::{Endian, byml::NodeType};

#[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
use {
    crate::yaml::format_hex,
    base64::Engine,
    lexical_core,
    lexical,
};

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
        // For root node, we need to read the node type and handle containers specially
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

        // For root container nodes, the offset points directly to the container
        // The value_data is not meaningful for root containers
        let value_data = if matches!(node_type, NodeType::Array | NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap) {
            // For container root nodes, store the container offset in value_data  
            let mut data = [0u8; 8];
            data[0..4].copy_from_slice(&offset.to_le_bytes());
            data
        } else {
            // For primitive root nodes, read the value normally
            let mut value_data = [0u8; 8];
            let value_start = offset_usize + 1;
            if value_start + 8 <= data.len() {
                value_data.copy_from_slice(&data[value_start..value_start + 8]);
            }
            value_data
        };

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
                if offset >= data.len() {
                    return Err(ReaderError::InvalidOffset(offset as u32));
                }
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
                if offset >= data.len() {
                    return Err(ReaderError::InvalidOffset(offset as u32));
                }
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
                if offset >= data.len() {
                    return Err(ReaderError::InvalidOffset(offset as u32));
                }
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
                if offset >= data.len() {
                    return Err(ReaderError::InvalidOffset(offset as u32));
                }
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
    
    /// Convert this reader node to an owned Byml (allocates)
    /// 
    /// This method converts the zero-copy reader representation to the owned,
    /// mutable representation used by the standard BYML API.
    #[cfg(feature = "byml")]
    pub fn to_owned(&self) -> ReaderResult<crate::byml::Byml> {
        use crate::byml::Byml;
        use rustc_hash::FxHashMap;
        
        match self.node_type {
            NodeType::Null => Ok(Byml::Null),
            NodeType::Bool => Ok(Byml::Bool(self.as_bool()?)),
            NodeType::I32 => Ok(Byml::I32(self.as_i32()?)),
            NodeType::U32 => Ok(Byml::U32(self.as_u32()?)),
            NodeType::I64 => Ok(Byml::I64(self.as_i64()?)),
            NodeType::U64 => Ok(Byml::U64(self.as_u64()?)),
            NodeType::Float => Ok(Byml::Float(self.as_f32()?)),
            NodeType::Double => Ok(Byml::Double(self.as_f64()?)),
            NodeType::String => Ok(Byml::String(self.as_str()?.to_string().into())),
            NodeType::Binary => Ok(Byml::BinaryData(self.as_binary()?.to_vec())),
            NodeType::File => Ok(Byml::FileData(self.as_binary()?.to_vec())),
            NodeType::Array => {
                let array = self.as_array()?;
                let mut vec = Vec::with_capacity(array.len());
                for i in 0..array.len() {
                    if let Some(element) = array.get(i) {
                        vec.push(element.to_owned()?);
                    }
                }
                Ok(Byml::Array(vec))
            }
            NodeType::Map => {
                let map = self.as_map()?;
                let mut result = FxHashMap::default();
                for entry_result in map.iter() {
                    let (key, value) = entry_result?;
                    result.insert(key.to_string().into(), value.to_owned()?);
                }
                Ok(Byml::Map(result))
            }
            NodeType::HashMap => {
                let hash_map = self.as_hash_map()?;
                let mut result = FxHashMap::default();
                for entry_result in hash_map.iter() {
                    let (key, value) = entry_result?;
                    result.insert(key, value.to_owned()?);
                }
                Ok(Byml::HashMap(result))
            }
            NodeType::ValueHashMap => {
                let value_hash_map = self.as_hash_map()?;
                let mut result = FxHashMap::default();
                for entry_result in value_hash_map.iter() {
                    let (key, value) = entry_result?;
                    // ValueHashMap needs a (Byml, u32) tuple as value
                    // For now, use 0 as the second value since we don't have that info in the reader
                    result.insert(key, (value.to_owned()?, 0));
                }
                Ok(Byml::ValueHashMap(result))
            }
            NodeType::StringTable => {
                // StringTable is not a regular node type that should be converted
                Err(ReaderError::InvalidNodeType(self.node_type as u8))
            }
        }
    }

    /// Serialize this node to YAML text
    /// 
    /// This method provides direct YAML serialization from the zero-copy reader
    /// without allocating intermediate owned structures. The output exactly matches
    /// the YAML format produced by the owned API.
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    pub fn to_text(&self) -> ReaderResult<String> {
        
        // For non-container types, we can't serialize to YAML directly
        // Only container types (Array, Map, HashMap, ValueHashMap) and Null can be serialized
        match self.node_type {
            NodeType::Array | NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap => {
                let mut tree = ryml::Tree::default();
                tree.reserve(20000);
                
                // Set up the root node based on the type
                let mut root = tree.root_ref_mut().map_err(|e| ReaderError::InvalidFormat(format!("YAML tree error: {}", e)))?;
                match self.node_type {
                    NodeType::Array => {
                        root.change_type(ryml::NodeType::Seq).map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                    }
                    NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap => {
                        root.change_type(ryml::NodeType::Map).map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                    }
                    _ => unreachable!(),
                }
                
                // Build the YAML tree from the reader node
                Self::build_yaml_node(self, root)?;
                
                // Emit the YAML string
                tree.emit().map_err(|e| ReaderError::InvalidFormat(format!("YAML emit error: {}", e)))
            }
            NodeType::Null => Ok("null".to_string()),
            _ => Err(ReaderError::InvalidFormat("Can only serialize Hash, Array, or Null nodes to YAML".to_string())),
        }
    }

    /// Build a YAML tree node from a reader node (recursive helper)
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn build_yaml_node<'b, 'e>(
        reader_node: &BymlNodeReader<'_>,
        mut yaml_node: ryml::NodeRef<'b, 'e, '_, &'e mut ryml::Tree<'b>>,
    ) -> ReaderResult<()> {
        
        match reader_node.node_type {
            NodeType::Array => {
                let array = reader_node.as_array()?;
                let should_inline = array.len() < 10 && Self::array_is_simple(&array)?;
                
                if should_inline {
                    yaml_node.change_type(ryml::NodeType::Seq | ryml::NodeType::WipStyleFlowSl)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                } else {
                    yaml_node.change_type(ryml::NodeType::Seq)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                }
                
                for i in 0..array.len() {
                    if let Some(element) = array.get(i) {
                        let child = yaml_node.append_child()
                            .map_err(|e| ReaderError::InvalidFormat(format!("YAML append error: {}", e)))?;
                        Self::build_yaml_node(&element, child)?;
                    }
                }
            }
            NodeType::Map => {
                let map = reader_node.as_map()?;
                let should_inline = map.len() < 10 && Self::map_is_simple(&map)?;
                
                if should_inline {
                    yaml_node.change_type(ryml::NodeType::Map | ryml::NodeType::WipStyleFlowSl)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                } else {
                    yaml_node.change_type(ryml::NodeType::Map)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                }
                
                // Collect and sort keys for consistent output
                let mut entries = Vec::new();
                for entry_result in map.iter() {
                    let (key, value) = entry_result?;
                    entries.push((key.to_string(), value));
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                
                for (key, value) in entries {
                    let mut child = yaml_node.append_child()
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML append error: {}", e)))?;
                    child.set_key(&key)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML key error: {}", e)))?;
                    
                    if Self::string_needs_quotes(&key) {
                        let flags = child.node_type()
                            .map_err(|e| ReaderError::InvalidFormat(format!("YAML flags error: {}", e)))?;
                        child.set_type_flags(flags | ryml::NodeType::WipKeySquo)
                            .map_err(|e| ReaderError::InvalidFormat(format!("YAML flags error: {}", e)))?;
                    }
                    
                    Self::build_yaml_node(&value, child)?;
                }
            }
            NodeType::HashMap => {
                let hash_map = reader_node.as_hash_map()?;
                let should_inline = hash_map.len() < 10 && Self::hash_map_is_simple(&hash_map)?;
                
                if should_inline {
                    yaml_node.change_type(ryml::NodeType::Map | ryml::NodeType::WipStyleFlowSl)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                } else {
                    yaml_node.change_type(ryml::NodeType::Map)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                }
                
                // Set the tag for hash maps
                yaml_node.set_val_tag("!h")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
                
                // Collect and sort keys for consistent output
                let mut entries = Vec::new();
                for entry_result in hash_map.iter() {
                    let (key, value) = entry_result?;
                    entries.push((key, value));
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                
                for (key, value) in entries {
                    let mut child = yaml_node.append_child()
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML append error: {}", e)))?;
                    child.set_key(&key.to_string())
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML key error: {}", e)))?;
                    
                    Self::build_yaml_node(&value, child)?;
                }
            }
            NodeType::ValueHashMap => {
                let value_hash_map = reader_node.as_hash_map()?;
                let should_inline = value_hash_map.len() < 10 && Self::hash_map_is_simple(&value_hash_map)?;
                
                if should_inline {
                    yaml_node.change_type(ryml::NodeType::Map | ryml::NodeType::WipStyleFlowSl)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                } else {
                    yaml_node.change_type(ryml::NodeType::Map)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML type error: {}", e)))?;
                }
                
                // Set the tag for value hash maps
                yaml_node.set_val_tag("!vh")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
                
                // Collect and sort keys for consistent output  
                let mut entries = Vec::new();
                for entry_result in value_hash_map.iter() {
                    let (key, value) = entry_result?;
                    entries.push((key, value));
                }
                entries.sort_by(|a, b| a.0.cmp(&b.0));
                
                for (key, value) in entries {
                    let mut child = yaml_node.append_child()
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML append error: {}", e)))?;
                    child.set_key(&key.to_string())
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML key error: {}", e)))?;
                    
                    Self::build_yaml_node(&value, child)?;
                }
            }
            // Scalar types
            _ => Self::build_yaml_scalar(reader_node, yaml_node)?,
        }
        
        Ok(())
    }

    /// Build a YAML scalar node from a reader node
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn build_yaml_scalar<'b, 'e>(
        reader_node: &BymlNodeReader<'_>,
        mut yaml_node: ryml::NodeRef<'b, 'e, '_, &'e mut ryml::Tree<'b>>,
    ) -> ReaderResult<()> {
        use crate::yaml::*;
        
        match reader_node.node_type {
            NodeType::String => {
                let s = reader_node.as_str()?;
                yaml_node.set_val(s)
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                if Self::string_needs_quotes(s) {
                    let flags = yaml_node.node_type()
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML flags error: {}", e)))?;
                    yaml_node.set_type_flags(flags | ryml::NodeType::WipValDquo)
                        .map_err(|e| ReaderError::InvalidFormat(format!("YAML flags error: {}", e)))?;
                }
            }
            NodeType::Bool => {
                let b = reader_node.as_bool()?;
                yaml_node.set_val(if b { "true" } else { "false" })
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
            }
            NodeType::Float => {
                let f = reader_node.as_f32()?;
                let float_str = Self::write_float(f as f64)?;
                yaml_node.set_val(&float_str)
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
            }
            NodeType::Double => {
                let d = reader_node.as_f64()?;
                let double_str = Self::write_float(d)?;
                yaml_node.set_val(&double_str)
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!f64")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            NodeType::I32 => {
                let i = reader_node.as_i32()?;
                yaml_node.set_val(&lexical::to_string(i))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
            }
            NodeType::I64 => {
                let i = reader_node.as_i64()?;
                yaml_node.set_val(&lexical::to_string(i))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!l")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            NodeType::U32 => {
                let u = reader_node.as_u32()?;
                yaml_node.set_val(&format_hex!(&u))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!u")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            NodeType::U64 => {
                let u = reader_node.as_u64()?;
                yaml_node.set_val(&format_hex!(&u))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!ul")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            NodeType::Null => {
                yaml_node.set_val("null")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
            }
            NodeType::Binary => {
                let data = reader_node.as_binary()?;
                let arena = yaml_node.tree().arena_capacity();
                yaml_node.tree_mut().reserve_arena(arena + data.len());
                yaml_node.set_val(&base64::engine::general_purpose::STANDARD.encode(data))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!!binary")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            NodeType::File => {
                let data = reader_node.as_binary()?;
                let arena = yaml_node.tree().arena_capacity();
                yaml_node.tree_mut().reserve_arena(arena + data.len());
                yaml_node.set_val(&base64::engine::general_purpose::STANDARD.encode(data))
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML val error: {}", e)))?;
                yaml_node.set_val_tag("!!file")
                    .map_err(|e| ReaderError::InvalidFormat(format!("YAML tag error: {}", e)))?;
            }
            _ => return Err(ReaderError::InvalidNodeType(reader_node.node_type as u8)),
        }
        
        Ok(())
    }

    /// Helper to check if an array contains only simple (non-container) elements
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn array_is_simple(array: &BymlArrayReader<'_>) -> ReaderResult<bool> {
        for i in 0..array.len() {
            if let Some(element) = array.get(i) {
                match element.node_type() {
                    NodeType::Array | NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap => {
                        return Ok(false);
                    }
                    _ => {}
                }
            }
        }
        Ok(true)
    }

    /// Helper to check if a map contains only simple (non-container) values
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn map_is_simple(map: &BymlMapReader<'_>) -> ReaderResult<bool> {
        for entry_result in map.iter() {
            let (_key, value) = entry_result?;
            match value.node_type() {
                NodeType::Array | NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap => {
                    return Ok(false);
                }
                _ => {}
            }
        }
        Ok(true)
    }

    /// Helper to check if a hash map contains only simple (non-container) values
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn hash_map_is_simple(hash_map: &BymlHashMapReader<'_>) -> ReaderResult<bool> {
        for entry_result in hash_map.iter() {
            let (_key, value) = entry_result?;
            match value.node_type() {
                NodeType::Array | NodeType::Map | NodeType::HashMap | NodeType::ValueHashMap => {
                    return Ok(false);
                }
                _ => {}
            }
        }
        Ok(true)
    }

    /// Helper function to format floating point numbers for YAML
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn write_float(value: f64) -> ReaderResult<String> {
        use lexical_core::{FormattedSize, ToLexical};
        
        let mut buffer = [0u8; f64::FORMATTED_SIZE_DECIMAL + 1];
        let extra;
        let buf = if value.is_sign_negative() && value == 0.0 {
            buffer[0] = b'-';
            extra = 1;
            &mut buffer[1..]
        } else {
            extra = 0;
            &mut buffer[..f64::FORMATTED_SIZE_DECIMAL]
        };
        
        let len = value.to_lexical(buf).len() + extra;
        let result = std::str::from_utf8(&buffer[..len])
            .map_err(|_| ReaderError::StringEncoding(std::str::Utf8Error::from(std::str::from_utf8(&buffer[..len]).unwrap_err())))?;
        Ok(result.to_string())
    }

    /// Helper function to determine if a string needs quotes in YAML
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn string_needs_quotes(value: &str) -> bool {
        use lexical::parse;
        
        matches!(value, "true" | "false")
            || value.starts_with('!')
            || (value.contains('.')
                && (Self::is_infinity(value)
                    || Self::is_negative_infinity(value)
                    || Self::is_nan(value)
                    || parse::<f64, &[u8]>(value.as_bytes()).is_ok()))
            || parse::<u64, &[u8]>(value.as_bytes()).is_ok()
            || value == "null"
            || value == "!"
            || value == "NULL"
    }

    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn is_infinity(input: &str) -> bool {
        matches!(
            input,
            ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF"
        )
    }

    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn is_negative_infinity(input: &str) -> bool {
        matches!(input, "-.inf" | "-.Inf" | "-.INF")
    }

    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    fn is_nan(input: &str) -> bool {
        matches!(input, ".nan" | ".NaN" | ".NAN")
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

        Ok(BymlMapReader {
            reader,
            keys_offset,
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

        // Read hash map length (NodeType + u24)
        // First byte is NodeType, next 3 bytes are length as u24
        let len_bytes = match reader.endian_internal() {
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

        let is_value_hash_map = matches!(node_type, NodeType::ValueHashMap);

        Ok(BymlHashMapReader {
            reader,
            offset,
            len: len_bytes,
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
    pub fn get(&self, key: u32) -> Option<BymlNodeReader<'a>> {
        let data = self.reader.data();
        let endian = self.reader.endian_internal();
        let offset = self.offset as usize;
        
        // HashMap structure:
        // offset + 0: NodeType (1 byte) + length (3 bytes) - we already parsed this
        // offset + 4: entries (8 bytes each: hash(4) + value_offset(4))
        // offset + 4 + 8*len: node types (1 byte each)
        
        let entries_start = offset + 4;
        let types_start = entries_start + 8 * self.len as usize;
        
        // Linear search through entries (could be optimized with binary search if sorted)
        for i in 0..self.len {
            let entry_offset = entries_start + (i as usize) * 8;
            
            if entry_offset + 8 > data.len() {
                return None;
            }
            
            // Read hash key
            let entry_hash = match endian {
                Endian::Little => u32::from_le_bytes([
                    data[entry_offset],
                    data[entry_offset + 1],
                    data[entry_offset + 2],
                    data[entry_offset + 3],
                ]),
                Endian::Big => u32::from_be_bytes([
                    data[entry_offset],
                    data[entry_offset + 1],
                    data[entry_offset + 2],
                    data[entry_offset + 3],
                ]),
            };
            
            if entry_hash == key {
                // Found the key, read the value (4 bytes starting at entry_offset + 4)
                let mut value_data = [0u8; 8];
                if entry_offset + 8 <= data.len() {
                    value_data[0..4].copy_from_slice(&data[entry_offset + 4..entry_offset + 8]);
                }
                let type_offset = types_start + i as usize;
                
                if type_offset >= data.len() {
                    return None;
                }
                
                let node_type = data[type_offset];
                
                return Some(BymlNodeReader {
                    reader: self.reader,
                    node_type: NodeType::try_from(node_type).ok()?,
                    value_data,
                    offset: entry_offset as u32 + 4,
                });
            }
        }
        
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
    
    /// Iterate over all entries (expensive - O(n) scan)
    /// Returns an iterator that yields (hash_key, node) pairs
    pub fn iter(&'a self) -> BymlHashMapIterator<'a> {
        BymlHashMapIterator {
            reader: self,
            index: 0,
        }
    }
}

/// Iterator over BYML hash map entries
pub struct BymlHashMapIterator<'a> {
    reader: &'a BymlHashMapReader<'a>,
    index: usize,
}

impl<'a> Iterator for BymlHashMapIterator<'a> {
    type Item = ReaderResult<(u32, BymlNodeReader<'a>)>;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.reader.len() {
            return None;
        }
        
        let data = self.reader.reader.data();
        let endian = self.reader.reader.endian_internal();
        let offset = self.reader.offset as usize;
        
        let entries_start = offset + 4;
        let entry_offset = entries_start + self.index * 8;
        
        if entry_offset + 8 > data.len() {
            self.index = self.reader.len(); // Prevent further iteration
            return Some(Err(ReaderError::UnexpectedEnd(entry_offset as u32)));
        }
        
        // Read hash key
        let hash_key = match endian {
            Endian::Little => u32::from_le_bytes([
                data[entry_offset],
                data[entry_offset + 1], 
                data[entry_offset + 2],
                data[entry_offset + 3],
            ]),
            Endian::Big => u32::from_be_bytes([
                data[entry_offset],
                data[entry_offset + 1],
                data[entry_offset + 2],
                data[entry_offset + 3],
            ]),
        };
        
        // Read value data
        let mut value_data = [0u8; 8];
        value_data[0..4].copy_from_slice(&data[entry_offset + 4..entry_offset + 8]);
        
        // Get node type
        let types_start = entries_start + 8 * self.reader.len as usize;
        let type_offset = types_start as usize + self.index;
        
        if type_offset >= data.len() {
            self.index = self.reader.len(); // Prevent further iteration
            return Some(Err(ReaderError::UnexpectedEnd(type_offset as u32)));
        }
        
        let node_type_byte = data[type_offset];
        let node_type = match NodeType::try_from(node_type_byte) {
            Ok(nt) => nt,
            Err(_) => {
                self.index += 1;
                return Some(Err(ReaderError::InvalidNodeType(node_type_byte)));
            }
        };
        
        let node = BymlNodeReader::new(
            self.reader.reader,
            node_type,
            value_data,
            entry_offset as u32 + 4,
        );
        
        self.index += 1;
        Some(Ok((hash_key, node)))
    }
    
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.reader.len() - self.index;
        (remaining, Some(remaining))
    }
}
