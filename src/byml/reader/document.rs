//! BYML document reader implementation
//!
//! Provides the main `BymlReader` struct for parsing BYML documents with
//! zero-copy semantics.

use super::{BymlHeader, ReaderError, ReaderResult};
use crate::Endian;
use crate::byml::reader::node::BymlNodeReader;
use binrw::BinReaderExt;
use std::io::Cursor;

/// Zero-copy BYML document reader
///
/// This struct provides access to a BYML document without copying the underlying
/// data. All string and binary data is accessed through references to the original
/// byte slice.
pub struct BymlReader<'a> {
    /// Reference to the original binary data
    data: &'a [u8],
    /// Parsed BYML header
    header: BymlHeader,
    /// Reference to the string table section
    string_table: &'a [u8],
    /// Reference to the hash key table section (if present)
    #[allow(dead_code)]
    hash_key_table: Option<&'a [u8]>,
    /// Detected endianness
    endian: Endian,
}

impl<'a> BymlReader<'a> {
    /// Create a new BYML reader from binary data
    ///
    /// # Arguments
    /// * `data` - The binary BYML data to read from
    ///
    /// # Returns
    /// A new `BymlReader` instance or an error if the data is invalid
    ///
    /// # Example
    /// ```
    /// # use roead::byml::reader::BymlReader;
    /// let data = std::fs::read("test.byml")?;
    /// let reader = BymlReader::new(&data)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(data: &'a [u8]) -> ReaderResult<Self> {
        if data.len() < 16 {
            return Err(ReaderError::InvalidFormat(
                "Data too short for BYML header".to_string(),
            ));
        }

        // Parse header
        let mut cursor = Cursor::new(data);
        let header: BymlHeader = cursor.read_le()?;

        // Determine endianness and re-parse if needed
        let endian = header
            .endian()
            .map_err(|_| ReaderError::InvalidFormat("Invalid BYML magic bytes".to_string()))?;

        let header = if matches!(endian, Endian::Big) {
            let mut cursor = Cursor::new(data);
            cursor.read_be()?
        } else {
            header
        };

        // Validate version
        if !header.is_valid_version() {
            return Err(ReaderError::InvalidFormat(format!(
                "Unsupported BYML version: {}",
                header.version
            )));
        }

        // Validate and extract string table
        let string_table = Self::extract_table(data, header.string_table_offset, "string table")?;

        // Extract hash key table if present
        let hash_key_table = if header.hash_key_table_offset != 0 {
            Some(Self::extract_table(
                data,
                header.hash_key_table_offset,
                "hash key table",
            )?)
        } else {
            None
        };

        Ok(BymlReader {
            data,
            header,
            string_table,
            hash_key_table,
            endian,
        })
    }

    /// Extract a table (string or hash key table) from the data
    fn extract_table(data: &'a [u8], offset: u32, table_name: &str) -> ReaderResult<&'a [u8]> {
        let offset = offset as usize;
        if offset >= data.len() {
            return Err(ReaderError::InvalidOffset(offset as u32));
        }

        // Tables start with 1 byte node type + 3-byte entry count
        if offset + 4 > data.len() {
            return Err(ReaderError::UnexpectedEnd(offset as u32));
        }

        // Skip the node type byte (should be StringTable = 0xc2)
        // Read the 3-byte entry count that follows
        let entry_count = u32::from_le_bytes([
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            0, // Pad with zero for 4th byte
        ]);

        // Calculate minimum table size: 4 bytes header + entry_count * 4 bytes offsets
        let min_table_size = 4 + entry_count * 4;
        
        // We can't easily determine the exact end of the table since string data
        // follows the offset table, but we need at least the offset table
        let min_table_end = offset + min_table_size as usize;
        if min_table_end > data.len() {
            return Err(ReaderError::InvalidFormat(format!(
                "{} extends beyond data bounds",
                table_name
            )));
        }

        // Return the entire remaining data from the table start
        // The actual string data extends beyond the offset table
        Ok(&data[offset..])
    }

    /// Get the version of the BYML format
    pub fn version(&self) -> u16 {
        self.header.version
    }

    /// Get the endianness of the document
    pub fn endian(&self) -> Endian {
        self.endian
    }

    /// Get the root node of the document
    pub fn root(&'a self) -> BymlNodeReader<'a> {
        BymlNodeReader::new_root(self, self.header.root_node_offset)
    }
    
    /// Convert this reader to an owned Byml document (allocates)
    /// 
    /// This method converts the zero-copy reader representation to the owned,
    /// mutable representation used by the standard BYML API.
    #[cfg(feature = "byml")]
    pub fn to_owned(&self) -> ReaderResult<crate::byml::Byml> {
        self.root().to_owned()
    }

    /// Serialize the document to YAML text
    /// 
    /// This method provides direct YAML serialization from the zero-copy reader
    /// without allocating intermediate owned structures. The output exactly matches
    /// the YAML format produced by the owned API.
    #[cfg(all(feature = "yaml", any(feature = "byml", feature = "byml-read")))]
    pub fn to_text(&self) -> ReaderResult<String> {
        self.root().to_text()
    }

    /// Get a string from the string table by index
    pub(crate) fn get_string(&self, index: u32) -> ReaderResult<&'a str> {
        self.get_table_string(self.string_table, index, "string")
    }

    /// Get a hash key from the hash key table by index
    #[allow(dead_code)]
    pub(crate) fn get_hash_key(&self, index: u32) -> ReaderResult<&'a str> {
        match self.hash_key_table {
            Some(table) => self.get_table_string(table, index, "hash key"),
            None => Err(ReaderError::InvalidFormat(
                "No hash key table present".to_string(),
            )),
        }
    }

    /// Get a string from a table by index
    fn get_table_string(
        &self,
        table: &'a [u8],
        index: u32,
        table_name: &str,
    ) -> ReaderResult<&'a str> {
        if table.len() < 4 {
            return Err(ReaderError::InvalidFormat(format!(
                "{} table too short",
                table_name
            )));
        }

        // Get table entry count (bytes 1-3 after node type byte)
        let entry_count = match self.endian {
            Endian::Little => u32::from_le_bytes([table[1], table[2], table[3], 0]),
            Endian::Big => u32::from_be_bytes([0, table[1], table[2], table[3]]),
        };

        if index >= entry_count {
            return Err(ReaderError::InvalidFormat(format!(
                "{} index {} out of bounds (count: {})",
                table_name, index, entry_count
            )));
        }

        // Calculate offset to string pointer
        let ptr_offset = 4 + (index as usize * 4); // Skip header + index * 4 bytes
        if ptr_offset + 4 > table.len() {
            return Err(ReaderError::UnexpectedEnd(ptr_offset as u32));
        }

        // Read string offset (relative to table start)
        let string_offset = match self.endian {
            Endian::Little => u32::from_le_bytes([
                table[ptr_offset],
                table[ptr_offset + 1],
                table[ptr_offset + 2],
                table[ptr_offset + 3],
            ]),
            Endian::Big => u32::from_be_bytes([
                table[ptr_offset],
                table[ptr_offset + 1],
                table[ptr_offset + 2],
                table[ptr_offset + 3],
            ]),
        } as usize;

        // The offset is relative to the table start
        if string_offset >= table.len() {
            return Err(ReaderError::InvalidOffset(string_offset as u32));
        }

        // Find null terminator
        let string_start = string_offset;
        let string_data = &table[string_start..];
        let null_pos = string_data.iter().position(|&b| b == 0).ok_or_else(|| {
            ReaderError::InvalidFormat(format!("Unterminated string in {} table", table_name))
        })?;

        // Convert to UTF-8 string
        std::str::from_utf8(&string_data[..null_pos]).map_err(ReaderError::StringEncoding)
    }

    /// Get reference to underlying data
    pub(crate) fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Get endianness for internal use
    pub(crate) fn endian_internal(&self) -> Endian {
        self.endian
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_creation() {
        // Test with minimal valid BYML header
        let mut data = vec![0; 16];
        data[0] = 0x59; // 'Y'
        data[1] = 0x42; // 'B' - big endian magic
        data[2] = 0x00; // version low
        data[3] = 0x02; // version high (version 2)

        let reader = BymlReader::new(&data);
        // This should fail because we don't have proper tables
        assert!(reader.is_err());
    }

    #[test]
    fn test_invalid_magic() {
        let data = vec![0xFF, 0xFF, 0x02, 0x00]; // Invalid magic
        let reader = BymlReader::new(&data);
        assert!(reader.is_err());
    }

    #[test]
    fn test_invalid_version() {
        let mut data = vec![0; 16];
        data[0] = 0x59; // 'Y'
        data[1] = 0x42; // 'B'
        data[2] = 0x00; // version low  
        data[3] = 0x10; // version high (version 16 - invalid)

        let reader = BymlReader::new(&data);
        assert!(reader.is_err());
    }
}
